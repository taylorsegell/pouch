//! Change orchestration (M3 plan P11 to P13, G1, G2, G7, G11, Gap 13):
//! root and target resolution, selection edits, `prepare` (a plan, no
//! writes), and `run` (lock, journal check, plan, decide, selection write,
//! apply). Also `resume` and `rollback` with the same guards.

use crate::adapter::{Adapter, Detection, adapter_for, adapters};
use crate::apply::{
    self, ApplyCtx, ApplyReport, Inspection, JournalSummary, RecoverReport, SelectionPrelude,
};
use crate::bundle::{BundleSet, slugify};
use crate::catalog::{Catalog, canonical_ref_text};
use crate::config::{Config, DestinationPolicy, Strategy};
use crate::error::{Diagnostic, Error, Result};
use crate::fsops::{FsOps, StatePath};
use crate::generated::{self, GeneratedInput};
use crate::hash::Hash;
use crate::observe::observe;
use crate::paths::Dirs;
use crate::plan::{DiffInput, Plan, PlanHeader, diff};
use crate::project::selection::{gitignore_lines, parse_selection, render_selection};
use crate::project::{
    ProjectRoot, RootDetection, Selection, TargetId, TargetSelection, detect_root, selection_path,
};
use crate::resolve::{DesiredEntry, ProblemKind, resolve_selection};
use crate::state::{
    Clock, ProjectLock, Receipt, SelectionBefore, StateDir, check_state_layout, new_op_id,
    read_latest_history, selection_hash, strategy_text,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// A selection edit (P11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit {
    Add(Vec<String>),
    Remove(Vec<String>),
    UseReplace(String),
    UseAdd(String),
    Deactivate,
    /// `reconcile --repair` and relocate repair: the selection stays as it
    /// is; the plan re-aligns the destination with it (M4-7).
    Reconcile,
    /// `rollback` from history: make the target selection this one (M4-9).
    Restore(TargetSelection),
    /// `rollback` with no journal: `prepare` resolves this to `Restore` (or
    /// `Deactivate`) from the newest history entry, after `run` took the
    /// lock, so a locked project answers `Locked` first (AC-53).
    FromHistory,
}

/// The answer of the confirmation step (P13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Proceed,
    Decline,
    Refuse,
}

/// An error plus the diagnostics gathered before it.
#[derive(Debug)]
pub struct ChangeFailure {
    pub error: Error,
    pub warnings: Vec<Diagnostic>,
}

impl From<Error> for ChangeFailure {
    fn from(error: Error) -> Self {
        ChangeFailure {
            error,
            warnings: Vec::new(),
        }
    }
}

type CResult<T> = std::result::Result<T, ChangeFailure>;

/// A selection change request.
pub struct ChangeRequest<'a> {
    pub dirs: &'a Dirs,
    pub config: &'a Config,
    pub catalog: &'a Catalog,
    pub bundles: &'a BundleSet,
    pub root: &'a ProjectRoot,
    pub target: TargetId,
    pub edit: Edit,
    pub command: &'a str,
    pub clock: &'a dyn Clock,
}

/// What `prepare` computed (no file written).
pub struct Prepared {
    pub plan: Plan,
    pub tsel_before: TargetSelection,
    /// `None` for `deactivate` (the target key is removed).
    pub tsel_after: Option<TargetSelection>,
    pub selection_before: SelectionBefore,
    /// The whole selection after the edit.
    pub selection: Selection,
    pub selection_changed: bool,
    pub selection_after_text: String,
    pub selection_after_hash: String,
    pub warnings: Vec<Diagnostic>,
    pub receipt: Receipt,
    pub adapter: &'static dyn Adapter,
    /// Set by `run`: the selection file was written by this command.
    pub selection_written: bool,
    /// Set by `run`: the `.gitignore` lines to offer (first write only).
    pub gitignore_lines: Vec<String>,
}

/// How `run` ended.
pub enum Outcome {
    NoChanges(Box<Prepared>),
    Blocked(Box<Prepared>),
    Refused(Box<Prepared>),
    Declined(Box<Prepared>),
    Applied(Box<Prepared>, Box<ApplyReport>),
    Interrupted(JournalSummary),
}

// ---- root, sources, target ----

/// `--project <path>` gives the canonical existing directory (`Explicit`,
/// no walk to `.git`); otherwise the nearest `.git` from `cwd` (P12).
pub fn resolve_root(project: Option<&Path>, cwd: &Path) -> Result<ProjectRoot> {
    match project {
        Some(p) => {
            let invalid = |reason: String| Error::ProjectPathInvalid {
                path: p.to_string_lossy().into_owned(),
                reason,
            };
            let canonical = std::fs::canonicalize(p).map_err(|e| invalid(e.to_string()))?;
            if !canonical.is_dir() {
                return Err(invalid("not a directory".into()));
            }
            Ok(ProjectRoot {
                path: canonical,
                how: RootDetection::Explicit,
            })
        }
        None => detect_root(cwd, None),
    }
}

/// Refuse a project root that equals, lies inside, or contains a
/// registered source (G7; S3). Called before the lock, which would create
/// `.unam/` inside the project.
pub fn ensure_project_outside_sources(
    config: &Config,
    dirs: &Dirs,
    root: &ProjectRoot,
) -> Result<()> {
    for s in &config.sources {
        let Ok(expanded) = dirs.expand(&s.path) else {
            continue;
        };
        let Ok(src) = std::fs::canonicalize(&expanded) else {
            continue;
        };
        if root.path.starts_with(&src) || src.starts_with(&root.path) {
            return Err(Error::ProjectOverlapsSource {
                project: root.path.clone(),
                source_root: src,
            });
        }
    }
    Ok(())
}

/// Resolve the target (P12): the flag, else the selection's single target,
/// else adapter detection (none present defaults to `claude-code`).
pub fn resolve_target(
    flag: Option<TargetId>,
    selection: Option<&Selection>,
    root: &Path,
    dirs: &Dirs,
) -> Result<(TargetId, Detection)> {
    let target = match flag {
        Some(t) => t,
        None => match selection.map(|s| s.targets.len()).unwrap_or(0) {
            0 => adapters()
                .into_iter()
                .find(|a| a.detect(root, dirs).is_present())
                .map_or(TargetId::ClaudeCode, |a| a.id()),
            1 => *selection
                .and_then(|s| s.targets.keys().next())
                .unwrap_or(&TargetId::ClaudeCode),
            _ => {
                return Err(Error::TargetAmbiguous {
                    targets: selection
                        .map(|s| s.targets.keys().map(|t| t.to_string()).collect())
                        .unwrap_or_default(),
                });
            }
        },
    };
    let adapter = adapter_for(target).ok_or_else(|| Error::TargetUnsupported {
        target: target.as_str().to_string(),
    })?;
    Ok((target, adapter.detect(root, dirs)))
}

// ---- selection edits ----

fn resolves_to(catalog: &Catalog, text: &str, id: &str) -> bool {
    canonical_ref_text(text) == id
        || catalog
            .resolve_text(text)
            .is_ok_and(|s| s.id.to_string() == id)
}

/// Apply an edit to one target selection (P11). Every reference is
/// validated first; nothing changes on any error. The bool is "changed".
pub fn apply_edit(
    tsel: &TargetSelection,
    edit: &Edit,
    catalog: &Catalog,
    bundles: &BundleSet,
) -> Result<(TargetSelection, bool)> {
    let mut out = tsel.clone();
    match edit {
        Edit::Add(refs) => {
            let ids: Vec<String> = refs
                .iter()
                .map(|r| catalog.resolve_text(r).map(|s| s.id.to_string()))
                .collect::<Result<_>>()?;
            for id in ids {
                if !out.add.iter().any(|a| canonical_ref_text(a) == id) {
                    out.add.push(id.clone());
                }
                out.remove.retain(|r| !resolves_to(catalog, r, &id));
            }
        }
        Edit::Remove(refs) => {
            let mut ids: Vec<String> = Vec::new();
            for r in refs {
                match catalog.resolve_text(r) {
                    Ok(s) => ids.push(s.id.to_string()),
                    Err(e) => {
                        // An unreachable source: accept an exact `add` text.
                        let canon = canonical_ref_text(r);
                        if matches!(e, Error::NotFound { .. })
                            && out.add.iter().any(|a| canonical_ref_text(a) == canon)
                        {
                            ids.push(canon);
                        } else {
                            return Err(e);
                        }
                    }
                }
            }
            let current = resolve_selection(tsel, bundles, catalog, Strategy::Symlink);
            for (r, id) in refs.iter().zip(&ids) {
                let selected = current.resolved.iter().any(|d| d.id.to_string() == *id)
                    || out.add.iter().any(|a| canonical_ref_text(a) == *id);
                if !selected {
                    return Err(Error::NotSelected {
                        reference: r.trim().to_string(),
                    });
                }
            }
            for id in &ids {
                out.add.retain(|a| !resolves_to(catalog, a, id));
            }
            let after = resolve_selection(&out, bundles, catalog, Strategy::Symlink);
            for id in &ids {
                if after.resolved.iter().any(|d| d.id.to_string() == *id)
                    && !out.remove.iter().any(|x| canonical_ref_text(x) == *id)
                {
                    out.remove.push(id.clone());
                }
            }
        }
        Edit::UseReplace(b) => {
            let bundle = bundles.get(b).ok_or_else(|| Error::BundleNotFound {
                slug: bundle_slug(b),
            })?;
            out.bundles = vec![bundle.slug.clone()];
            out.add.clear();
            out.remove.clear();
        }
        Edit::UseAdd(b) => {
            let bundle = bundles.get(b).ok_or_else(|| Error::BundleNotFound {
                slug: bundle_slug(b),
            })?;
            if !out.bundles.iter().any(|x| slugify(x) == bundle.slug) {
                out.bundles.push(bundle.slug.clone());
            }
        }
        Edit::Deactivate => out = TargetSelection::default(),
        // `FromHistory` is resolved by `prepare` before it reaches here.
        Edit::Reconcile | Edit::FromHistory => {}
        Edit::Restore(t) => out = t.clone(),
    }
    let changed = out != *tsel;
    Ok((out, changed))
}

/// The edit that rolls back from history (M4-9): restore the target
/// selection the newest history entry held before its change, or
/// `Deactivate` when that selection had no such target. Read-only;
/// `NoHistory` when the target has none.
pub fn history_edit(root: &Path, target: TargetId) -> Result<Edit> {
    check_state_layout(root, Some(target.as_str()))?;
    let state = StateDir::new(root, target);
    let entry = read_latest_history(&state)?.ok_or_else(|| Error::NoHistory {
        target: target.as_str().to_string(),
    })?;
    let before = parse_selection(&entry.selection_before.text, &selection_path(root))?;
    Ok(match before.targets.get(&target) {
        Some(t) => Edit::Restore(t.clone()),
        None => Edit::Deactivate,
    })
}

/// True when a journal exists for `target`, or (no target) for any target.
pub fn has_journal(root: &Path, target: Option<TargetId>) -> bool {
    let exists =
        |t: TargetId| std::fs::symlink_metadata(StateDir::new(root, t).journal().path()).is_ok();
    match target {
        Some(t) => exists(t),
        None => adapters().into_iter().any(|a| exists(a.id())),
    }
}

fn bundle_slug(text: &str) -> String {
    let s = slugify(text);
    if s.is_empty() {
        text.trim().to_string()
    } else {
        s
    }
}

// ---- prepare ----

/// Skills a scan location holds: child directories with a `SKILL.md`
/// (a symlink to such a directory counts).
fn skills_in(dir: &Path) -> usize {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return 0;
    };
    rd.filter_map(std::result::Result::ok)
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter(|e| e.path().join("SKILL.md").is_file())
        .count()
}

/// Warnings about a destination other agents also read (M5-7, EX11): an
/// ancestor `.agents/skills` that contributes skills (EDG-11), a skill that
/// is also placed for the other adapter (spec 4.7, reported once per skill),
/// and the unimplemented `isolated` destination policy (PRD D3).
fn shared_destination_warnings(
    req: &ChangeRequest<'_>,
    adapter: &dyn Adapter,
    root: &Path,
    desired: &[DesiredEntry],
) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    for loc in adapter.scan_locations(root, req.dirs) {
        if loc.scope != crate::adapter::Scope::Ancestor {
            continue;
        }
        let n = skills_in(&loc.path);
        if n > 0 {
            out.push(
                Diagnostic::warning(
                    "ancestor-skills",
                    format!(
                        "ancestor {} contributes {n} skill(s) to this project (codex walks upward)",
                        loc.path.display()
                    ),
                )
                .with_path(loc.path.to_string_lossy().into_owned()),
            );
        }
    }
    let mut dup: BTreeSet<String> = BTreeSet::new();
    for other in adapters() {
        if other.id() == req.target {
            continue;
        }
        let state = StateDir::new(root, other.id());
        let Ok(r) = Receipt::load(&state, root, other.id()) else {
            continue;
        };
        let theirs: BTreeSet<&str> = r.skill_entries().map(|e| e.name.as_str()).collect();
        for d in desired {
            if theirs.contains(d.name.as_str()) {
                dup.insert(d.name.clone());
            }
        }
    }
    for name in dup {
        out.push(Diagnostic::warning(
            "cross-adapter-duplicate",
            format!(
                "{name} is placed for both targets: duplicate for cursor and windsurf-devin via compat read of .claude/skills"
            ),
        ));
    }
    if req.target == TargetId::AgentsNeutral
        && req.config.defaults.destination_policy == DestinationPolicy::Isolated
    {
        out.push(Diagnostic::warning(
            "destination-policy-isolated",
            "destination_policy = isolated is not implemented in v1; using shared (PRD D3)",
        ));
    }
    out
}

fn read_selection_text(root: &Path) -> Result<(bool, String)> {
    let path = selection_path(root);
    match std::fs::read_to_string(&path) {
        Ok(t) => Ok((true, t)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((false, String::new())),
        Err(e) => Err(Error::SelectionRead { path, source: e }),
    }
}

/// Compute the plan for a request (P11, P12, P17; G3, G10, G11). Writes
/// nothing.
pub fn prepare(req: &ChangeRequest<'_>) -> CResult<Prepared> {
    let adapter = adapter_for(req.target).ok_or_else(|| Error::TargetUnsupported {
        target: req.target.as_str().to_string(),
    })?;
    let root = &req.root.path;
    let edit = match &req.edit {
        Edit::FromHistory => history_edit(root, req.target)?,
        other => other.clone(),
    };
    let (existed, text) = read_selection_text(root)?;
    let mut selection = parse_selection(&text, &selection_path(root))?;
    let tsel_before = selection
        .targets
        .get(&req.target)
        .cloned()
        .unwrap_or_default();
    let deactivate = edit == Edit::Deactivate;
    let strategy = tsel_before.strategy.unwrap_or(req.config.defaults.strategy);
    if !adapter.strategies().contains(&strategy) {
        return Err(Error::StrategyUnsupported {
            target: req.target.as_str().to_string(),
            strategy: strategy_text(strategy).to_string(),
        }
        .into());
    }
    let (tsel_after, changed) = if deactivate {
        let had = selection.targets.remove(&req.target).is_some();
        (None, had)
    } else {
        let (t, changed) = apply_edit(&tsel_before, &edit, req.catalog, req.bundles)?;
        selection.targets.insert(req.target, t.clone());
        (Some(t), changed)
    };
    let state = StateDir::new(root, req.target);
    let receipt = Receipt::load(&state, root, req.target)?;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let receipt_ids: BTreeSet<String> = receipt
        .skill_entries()
        .map(|s| s.skill_id.clone())
        .collect();
    let mut carry_ids: BTreeSet<String> = BTreeSet::new();
    let mut desired = Vec::new();
    if let Some(t) = &tsel_after {
        let resolution = resolve_selection(t, req.bundles, req.catalog, strategy);
        let mut blocking = Vec::new();
        for p in &resolution.problems {
            if p.kind == ProblemKind::Unresolved
                && receipt_ids.contains(&p.reference)
                && !p.unreachable.is_empty()
            {
                carry_ids.insert(p.reference.clone());
            } else {
                blocking.push(p);
            }
        }
        if let Some(first) = blocking.first() {
            return Err(ChangeFailure {
                error: first.to_error(),
                warnings: blocking.iter().map(|p| p.to_diagnostic()).collect(),
            });
        }
        desired = resolution.resolved;
    }
    let mut names: BTreeSet<String> = desired.iter().map(|d| d.name.clone()).collect();
    let mut hash_names: BTreeSet<String> = BTreeSet::new();
    for e in receipt.skill_entries() {
        names.insert(e.name.clone());
        if e.strategy == Strategy::Copy {
            hash_names.insert(e.name.clone());
        }
    }
    let rel = adapter.destination();
    let observed = observe(
        root,
        rel,
        &names,
        &hash_names,
        &adapter.scan_locations(root, req.dirs),
    )?;
    let detection = adapter.detect(root, req.dirs);
    let header = PlanHeader {
        root: root.clone(),
        root_how: req.root.how,
        target: req.target,
        dest_rel: rel.to_string(),
        strategy,
        detection_text: (!detection.is_present())
            .then(|| format!("not detected; {rel} will be created")),
        consumers: adapter.consumers().iter().map(|c| c.to_string()).collect(),
    };
    let plan_tsel = tsel_after.clone().unwrap_or_default();
    // Generated bundle briefs (spec 4.8 step 7): wanted for the bundles the
    // target selection names; observed together with the receipted ones.
    let wanted = tsel_after
        .as_ref()
        .map(|t| generated::wanted(t, req.bundles))
        .unwrap_or_default();
    let mut brief_names: BTreeSet<String> = wanted.iter().map(|w| w.name.clone()).collect();
    for g in receipt.generated_entries() {
        if g.consistent(rel) {
            brief_names.insert(g.name);
        }
    }
    let generated_input = GeneratedInput {
        observed: generated::observe_briefs(root, rel, &observed.dest_root, &brief_names)?,
        wanted,
    };
    let mut plan = diff(&DiffInput {
        header,
        op_id: new_op_id(req.clock),
        desired: &desired,
        carry_ids: &carry_ids,
        receipt: &receipt,
        observed: &observed,
        catalog: req.catalog,
        selection_hash: selection_hash(strategy, &plan_tsel),
        budget_tokens: req.config.defaults.catalog_budget_tokens,
        release_created_dirs: deactivate,
        repoint_moved: edit == Edit::Reconcile,
        generated: &generated_input,
    });
    plan.warnings
        .extend(shared_destination_warnings(req, adapter, root, &desired));
    let (selection_after_text, selection_after_hash) = if changed {
        let rendered = render_selection(&selection)?;
        let h = Hash::of_bytes(rendered.as_bytes()).to_string();
        (rendered, h)
    } else {
        (text.clone(), Hash::of_bytes(text.as_bytes()).to_string())
    };
    if receipt.root_moved(root) {
        warnings.push(Diagnostic::warning(
            "receipt-root-moved",
            format!(
                "the receipt was written for {}; this project is now at {} (the next write records the new root)",
                receipt.project_root,
                root.display()
            ),
        ));
    }
    Ok(Prepared {
        plan,
        tsel_before,
        tsel_after,
        selection_before: SelectionBefore {
            existed,
            text: text.clone(),
        },
        selection,
        selection_changed: changed,
        selection_after_text,
        selection_after_hash,
        warnings,
        receipt,
        adapter,
        selection_written: false,
        gitignore_lines: Vec::new(),
    })
}

// ---- run ----

/// Register the root in the project index after a selection write (Gap 13);
/// never fails the command.
pub fn register_project(dirs: &Dirs, root: &Path, warnings: &mut Vec<Diagnostic>) {
    match crate::project::index::register(dirs, root) {
        Ok(diags) => warnings.extend(diags),
        Err(e) => warnings.push(Diagnostic::warning(
            "project-index-failed",
            format!("cannot update the project index: {e}"),
        )),
    }
}

fn write_selection(req: &ChangeRequest<'_>, fs: &dyn FsOps, prepared: &mut Prepared) -> Result<()> {
    if !prepared.selection_changed {
        return Ok(());
    }
    let path = StatePath::new(selection_path(&req.root.path));
    fs.write_state(&path, prepared.selection_after_text.as_bytes())?;
    prepared.selection_written = true;
    if !prepared.selection_before.existed {
        prepared.gitignore_lines = gitignore_lines();
    }
    register_project(req.dirs, &req.root.path, &mut prepared.warnings);
    Ok(())
}

/// Run a change (P9, P11, P13; G1, G2, G7, Gap 13). The lock is held until
/// this returns, across `decide`.
pub fn run(
    req: &ChangeRequest<'_>,
    fs: &dyn FsOps,
    decide: &mut dyn FnMut(&Plan) -> Decision,
) -> CResult<Outcome> {
    ensure_project_outside_sources(req.config, req.dirs, req.root)?;
    let _lock = ProjectLock::acquire(&req.root.path, req.command, req.clock)?;
    check_state_layout(&req.root.path, Some(req.target.as_str()))?;
    let adapter = adapter_for(req.target).ok_or_else(|| Error::TargetUnsupported {
        target: req.target.as_str().to_string(),
    })?;
    let ctx = ApplyCtx::new(&req.root.path, adapter, Some(req.catalog), req.clock)?;
    let notes = match apply::inspect(&ctx, fs)? {
        Inspection::Interrupted(summary) => return Ok(Outcome::Interrupted(summary)),
        Inspection::Clear { notes } => notes,
    };
    let mut prepared = prepare(req)?;
    prepared.warnings.extend(
        notes
            .into_iter()
            .map(|n| Diagnostic::info("journal-finalized", n)),
    );
    if prepared.plan.is_blocked() {
        return Ok(Outcome::Blocked(Box::new(prepared)));
    }
    if prepared.plan.steps.is_empty() {
        apply::release_stale_brief_rows(&ctx, fs, &prepared.receipt)?;
        write_selection(req, fs, &mut prepared)?;
        return Ok(Outcome::NoChanges(Box::new(prepared)));
    }
    match decide(&prepared.plan) {
        Decision::Refuse => return Ok(Outcome::Refused(Box::new(prepared))),
        Decision::Decline => return Ok(Outcome::Declined(Box::new(prepared))),
        Decision::Proceed => {}
    }
    apply::ensure_dest_root(&ctx)?;
    write_selection(req, fs, &mut prepared)?;
    let prelude = SelectionPrelude {
        command: req.command.to_string(),
        selection_before: prepared.selection_before.clone(),
        selection_after_text: prepared.selection_after_text.clone(),
        selection_after_hash: prepared.selection_after_hash.clone(),
    };
    let report =
        apply::execute(&ctx, &prepared.plan, &prelude, fs).map_err(|error| ChangeFailure {
            error,
            warnings: prepared.warnings.clone(),
        })?;
    Ok(Outcome::Applied(Box::new(prepared), Box::new(report)))
}

// ---- resume and rollback ----

/// A `resume` or `rollback` request.
pub struct RecoverRequest<'a> {
    pub dirs: &'a Dirs,
    pub config: &'a Config,
    pub catalog: &'a Catalog,
    pub root: &'a ProjectRoot,
    pub target: Option<TargetId>,
    pub clock: &'a dyn Clock,
}

/// How a recovery ended.
pub enum RecoverOutcome {
    Recovered(Box<RecoverReport>, JournalSummary),
    Declined(JournalSummary),
    Refused(JournalSummary),
}

fn recover_target(req: &RecoverRequest<'_>) -> Result<TargetId> {
    if let Some(t) = req.target {
        return Ok(t);
    }
    let with_journal: Vec<TargetId> = adapters()
        .into_iter()
        .map(|a| a.id())
        .filter(|t| {
            std::fs::symlink_metadata(StateDir::new(&req.root.path, *t).journal().path()).is_ok()
        })
        .collect();
    match with_journal.as_slice() {
        [one] => Ok(*one),
        [] => Err(Error::NoJournal {
            target: TargetId::ClaudeCode.as_str().to_string(),
        }),
        many => Err(Error::TargetAmbiguous {
            targets: many.iter().map(|t| t.to_string()).collect(),
        }),
    }
}

fn recover(
    req: &RecoverRequest<'_>,
    fs: &dyn FsOps,
    decide: &mut dyn FnMut(&JournalSummary) -> Decision,
    action: &str,
) -> CResult<RecoverOutcome> {
    ensure_project_outside_sources(req.config, req.dirs, req.root)?;
    let _lock = ProjectLock::acquire(&req.root.path, action, req.clock)?;
    let target = recover_target(req)?;
    let adapter = adapter_for(target).ok_or_else(|| Error::TargetUnsupported {
        target: target.as_str().to_string(),
    })?;
    check_state_layout(&req.root.path, Some(target.as_str()))?;
    let ctx = ApplyCtx::new(&req.root.path, adapter, Some(req.catalog), req.clock)?;
    let summary = apply::journal_summary(&ctx)?;
    match decide(&summary) {
        Decision::Refuse => return Ok(RecoverOutcome::Refused(summary)),
        Decision::Decline => return Ok(RecoverOutcome::Declined(summary)),
        Decision::Proceed => {}
    }
    let report = if action == "resume" {
        apply::resume(&ctx, fs)?
    } else {
        apply::rollback(&ctx, fs)?
    };
    Ok(RecoverOutcome::Recovered(Box::new(report), summary))
}

/// Finish an interrupted apply.
pub fn resume(
    req: &RecoverRequest<'_>,
    fs: &dyn FsOps,
    decide: &mut dyn FnMut(&JournalSummary) -> Decision,
) -> CResult<RecoverOutcome> {
    recover(req, fs, decide, "resume")
}

/// Undo an interrupted apply.
pub fn rollback(
    req: &RecoverRequest<'_>,
    fs: &dyn FsOps,
    decide: &mut dyn FnMut(&JournalSummary) -> Decision,
) -> CResult<RecoverOutcome> {
    recover(req, fs, decide, "rollback")
}

/// The project-relative path of the selection file (for output).
pub fn selection_file(root: &Path) -> PathBuf {
    selection_path(root)
}

#[cfg(test)]
mod tests;
