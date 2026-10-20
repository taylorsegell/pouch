//! Every action the picker takes, each mapped to a named core function the
//! CLI also calls (spec 10 M6 parity). This is the only module that calls
//! `pouch-core`; it never touches the filesystem itself. Writes happen inside
//! `change::run`, `change::resume`, `change::rollback`, and `bundle::crud::save`.

use crate::output::render_plan;
use pouch_core::adapter::{ApplyContext, RefreshNote, adapter_for, adapters};
use pouch_core::bundle::{self, BundleSet};
use pouch_core::catalog::search::{self, Hit};
use pouch_core::catalog::{Catalog, LoadOptions};
use pouch_core::change::{
    self, ChangeFailure, ChangeRequest, Decision, Edit, Outcome, Prepared, RecoverOutcome,
    RecoverRequest,
};
use pouch_core::config::Config;
use pouch_core::error::{Error, Result};
use pouch_core::fsops::RealFs;
use pouch_core::paths::Dirs;
use pouch_core::plan::Plan;
use pouch_core::project::{self, ProjectRoot, TargetId};
use pouch_core::state::SystemClock;
use pouch_core::verify::{self, VerifyReport, VerifyRequest};

/// A TUI action. Each one names the core function it calls; `ALL` and the
/// exhaustive match in `core_entry` keep the parity test honest. The table
/// is read by the `tui_parity_*` tests only, hence the `allow`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ResolveRoot,
    LoadCatalog,
    ListBundles,
    ResolveTarget,
    CycleTarget,
    Search,
    Verify,
    Preview,
    Apply,
    CheckJournal,
    Resume,
    Rollback,
    SaveBundle,
}

// BEGIN parity table (the parity test removes this block before it looks for the needles)
#[allow(dead_code)]
impl Action {
    pub const ALL: [Action; 13] = [
        Action::ResolveRoot,
        Action::LoadCatalog,
        Action::ListBundles,
        Action::ResolveTarget,
        Action::CycleTarget,
        Action::Search,
        Action::Verify,
        Action::Preview,
        Action::Apply,
        Action::CheckJournal,
        Action::Resume,
        Action::Rollback,
        Action::SaveBundle,
    ];

    /// `(text this module contains, text the CLI source contains)` for the
    /// core function behind the action. No wildcard arm: a new variant must
    /// be listed here.
    pub fn core_entry(self) -> (&'static str, &'static str) {
        match self {
            Action::ResolveRoot => ("change::resolve_root", "change::resolve_root"),
            Action::LoadCatalog => ("Catalog::load", "Catalog::load"),
            Action::ListBundles => ("bundle::load_dir", "bundle::load_dir"),
            Action::ResolveTarget => ("change::resolve_target", "change::resolve_target"),
            Action::CycleTarget => ("adapters()", "adapter_for"),
            Action::Search => ("search::search", "search::search"),
            Action::Verify => ("verify::verify", "verify::verify"),
            // The CLI reaches `prepare` only inside `change::run`.
            Action::Preview => ("change::prepare", "change::run"),
            Action::Apply => ("change::run", "change::run"),
            Action::CheckJournal => ("change::has_journal", "change::has_journal"),
            Action::Resume => ("change::resume", "change::resume"),
            Action::Rollback => ("change::rollback", "change::rollback"),
            Action::SaveBundle => ("bundle::crud::save", "bundle::save"),
        }
    }
}
// END parity table

/// What the picker needs from the process: directories, configuration, the
/// project root, the catalog, and the bundles.
pub struct Session {
    pub dirs: Dirs,
    pub config: Config,
    pub root: ProjectRoot,
    pub catalog: Catalog,
    pub bundles: BundleSet,
    pub options: LoadOptions,
}

impl Session {
    /// Resolve the root, load the catalog (warm cache when present), and
    /// load the bundles.
    pub fn load(
        dirs: Dirs,
        config: Config,
        options: LoadOptions,
        project_flag: Option<&std::path::Path>,
        cwd: &std::path::Path,
    ) -> Result<Session> {
        let root = change::resolve_root(project_flag, cwd)?;
        let catalog = Catalog::load(&config.sources, &dirs, options)?;
        let bundles = bundle::load_dir(&dirs);
        Ok(Session {
            dirs,
            config,
            root,
            catalog,
            bundles,
            options,
        })
    }

    pub fn reload_bundles(&mut self) {
        self.bundles = bundle::load_dir(&self.dirs);
    }

    /// The targets the picker can switch between.
    pub fn targets() -> Vec<TargetId> {
        adapters().iter().map(|a| a.id()).collect()
    }

    /// The starting target: the flag, else the selection's, else the first
    /// detected adapter (ambiguity falls back to the first adapter).
    pub fn initial_target(&self, flag: Option<TargetId>) -> (TargetId, Option<String>) {
        let selection = project::load_selection(&self.root.path).ok().flatten();
        match change::resolve_target(flag, selection.as_ref(), &self.root.path, &self.dirs) {
            Ok((t, _)) => (t, None),
            Err(e) => {
                let first = Session::targets()
                    .first()
                    .copied()
                    .unwrap_or(TargetId::ClaudeCode);
                let note = format!(
                    "ambiguous target: using {}, press t to switch ({})",
                    first.as_str(),
                    crate::output::neutralize(&e.to_string())
                );
                (first, Some(note))
            }
        }
    }

    pub fn search(&self, query: &str, category: Option<&str>) -> Vec<Hit> {
        search::search(&self.catalog, query, category, None)
    }

    pub fn verify(&self, target: TargetId) -> Result<VerifyReport> {
        verify::verify(&VerifyRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            bundles: &self.bundles,
            root: &self.root,
            target,
        })
    }

    /// The first target that has an interrupted-apply journal.
    pub fn journal_target(&self) -> Option<TargetId> {
        Session::targets()
            .into_iter()
            .find(|t| change::has_journal(&self.root.path, Some(*t)))
    }

    fn request<'a>(
        &'a self,
        target: TargetId,
        edit: Edit,
        command: &'a str,
        clock: &'a SystemClock,
    ) -> ChangeRequest<'a> {
        ChangeRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            bundles: &self.bundles,
            root: &self.root,
            target,
            edit,
            command,
            clock,
        }
    }

    /// The plan for an edit; nothing is written.
    pub fn preview(
        &self,
        target: TargetId,
        edit: Edit,
        command: &str,
    ) -> std::result::Result<Prepared, ChangeFailure> {
        let clock = SystemClock;
        change::prepare(&self.request(target, edit, command, &clock))
    }

    /// Apply an edit through the journaled executor. The key press in the
    /// plan view confirms the plan the user saw (`confirmed`, the rendered
    /// text). `change::run` prepares the plan again under the project lock;
    /// when that plan renders differently the decision is `Decline`, and the
    /// bool is true so the caller can say why.
    pub fn apply(
        &self,
        target: TargetId,
        edit: Edit,
        command: &str,
        confirmed: &str,
    ) -> (std::result::Result<Outcome, ChangeFailure>, bool) {
        let clock = SystemClock;
        let req = self.request(target, edit, command, &clock);
        let changed = std::cell::Cell::new(false);
        let mut decide = |plan: &Plan| {
            let note = self.refresh_note(target, plan);
            if render_plan(plan, note.as_ref()) == confirmed {
                Decision::Proceed
            } else {
                changed.set(true);
                Decision::Decline
            }
        };
        let result = change::run(&req, &RealFs, &mut decide);
        (result, changed.get())
    }

    /// Finish (`resume`) or undo (`rollback`) an interrupted apply.
    pub fn recover(
        &self,
        target: TargetId,
        resume: bool,
    ) -> std::result::Result<RecoverOutcome, ChangeFailure> {
        let clock = SystemClock;
        let req = RecoverRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            root: &self.root,
            target: Some(target),
            clock: &clock,
        };
        let mut decide = |_: &pouch_core::apply::JournalSummary| Decision::Proceed;
        if resume {
            change::resume(&req, &RealFs, &mut decide)
        } else {
            change::rollback(&req, &RealFs, &mut decide)
        }
    }

    /// Save skill references as a new bundle (never overwrites).
    pub fn save_bundle(&self, name: &str, ids: &[String]) -> Result<bundle::crud::SaveOutcome> {
        bundle::crud::save(&self.dirs, &self.bundles, name, ids, false)
    }

    /// The `After apply` note of a plan.
    pub fn refresh_note(&self, target: TargetId, plan: &Plan) -> Option<RefreshNote> {
        adapter_for(target).map(|a| {
            a.refresh_note(ApplyContext {
                dest_root_existed: plan.dest_dirs_to_create.is_empty(),
            })
        })
    }

    /// The adapter's `Then ...` lines (pouch never runs them).
    pub fn followups(&self, target: TargetId) -> Vec<String> {
        adapter_for(target)
            .map(|a| {
                a.followups(&self.root.path, &self.dirs)
                    .into_iter()
                    .map(|f| format!("{}  ({})", f.command, f.reason))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The agents that read a shared destination (`agents-neutral` only).
    pub fn shared_consumers(&self, target: TargetId) -> Vec<String> {
        if target == TargetId::AgentsNeutral {
            adapter_for(target)
                .map(|a| a.consumers().iter().map(|c| (*c).to_string()).collect())
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }
}

/// A footer message for a core error: the message, then the hint.
pub fn error_text(e: &Error) -> String {
    use crate::output::neutralize;
    let message = neutralize(&e.to_string()).into_owned();
    match e.remedy() {
        Some(r) => format!("error: {message} (hint: {})", neutralize(r)),
        None => format!("error: {message}"),
    }
}
