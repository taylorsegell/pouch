//! Verify (spec 4.11; M4 plan B2): classify every entry of a project's
//! target into the six states by comparing the three stores: desired
//! (`selection.yaml`), applied (`applied.json`), and observed (the
//! destination on disk). Read-only: no lock, no write.

use crate::adapter::adapter_for;
use crate::bundle::BundleSet;
use crate::catalog::frontmatter::parse_skill_md;
use crate::catalog::{Catalog, SkillId};
use crate::config::{Config, Strategy};
use crate::error::{Diagnostic, Error, Result};
use crate::fsops::{BriefObserved, ObservedKind};
use crate::generated::observe_briefs;
use crate::observe::{DestRootState, list_foreign, observe};
use crate::paths::Dirs;
use crate::plan::is_owned;
use crate::project::{ProjectRoot, TargetId, load_selection};
use crate::resolve::{ProblemKind, Provenance, canonical_provenance, resolve_selection};
use crate::state::{Receipt, SkillEntry, StateDir, strategy_text};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The sentence `status` always carries (spec 4.11, VER-5, AC-28).
pub const FOOTER: &str = "Verified means discoverable under the target's documented scan rules. It does not mean the agent has read it.";

/// The six entry states (spec 4.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntryState {
    Selected,
    Applied,
    Active,
    Shadowed,
    Foreign,
    Orphaned,
}

impl EntryState {
    pub fn as_str(self) -> &'static str {
        match self {
            EntryState::Selected => "selected",
            EntryState::Applied => "applied",
            EntryState::Active => "active",
            EntryState::Shadowed => "shadowed",
            EntryState::Foreign => "foreign",
            EntryState::Orphaned => "orphaned",
        }
    }
}

/// Why an entry is `orphaned` (M4-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The destination is absent.
    Missing,
    /// The destination link does not resolve.
    BrokenLink,
    /// The destination differs from the receipt (link text, kind, or hash).
    Modified,
    /// The receipt ID is no longer in the catalog while its source is
    /// reachable.
    SourceSkillMissing,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Missing => "missing",
            Reason::BrokenLink => "broken-link",
            Reason::Modified => "modified",
            Reason::SourceSkillMissing => "source-skill-missing",
        }
    }
}

/// One classified entry (AC-14 fields plus the optional extras).
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: Option<String>,
    pub name: String,
    pub category: String,
    /// Absolute source path re-derived from the registry (VER-1).
    pub source_path: Option<String>,
    pub dest: String,
    pub strategy: Option<Strategy>,
    pub provenance: Vec<Provenance>,
    pub state: EntryState,
    pub reason: Option<Reason>,
    pub candidates: Vec<String>,
    pub shadowed_by: Option<String>,
}

impl Record {
    /// The `--json` record.
    pub fn to_json(&self) -> Value {
        let mut v = json!({
            "id": self.id,
            "name": self.name,
            "category": self.category,
            "source_path": self.source_path,
            "dest": self.dest,
            "strategy": self.strategy.map(strategy_text),
            "provenance": serde_json::to_value(&self.provenance).unwrap_or(Value::Null),
            "state": self.state.as_str(),
        });
        if let Some(o) = v.as_object_mut() {
            if let Some(r) = self.reason {
                o.insert("reason".into(), json!(r.as_str()));
            }
            if !self.candidates.is_empty() {
                o.insert("candidates".into(), json!(self.candidates));
            }
            if let Some(by) = &self.shadowed_by {
                o.insert("shadowed_by".into(), json!(by));
            }
        }
        v
    }
}

/// Input of `verify`.
pub struct VerifyRequest<'a> {
    pub dirs: &'a Dirs,
    pub config: &'a Config,
    pub catalog: &'a Catalog,
    pub bundles: &'a BundleSet,
    pub root: &'a ProjectRoot,
    pub target: TargetId,
}

/// What `verify` found.
#[derive(Debug, Clone, PartialEq)]
pub struct VerifyReport {
    pub target: TargetId,
    pub dest_rel: String,
    pub records: Vec<Record>,
    /// IDs of the resolved desired set.
    pub desired_ids: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub journal_present: bool,
}

impl VerifyReport {
    pub fn count(&self, state: EntryState) -> usize {
        self.records.iter().filter(|r| r.state == state).count()
    }

    /// Counts per state, in state order.
    pub fn summary_json(&self) -> Value {
        let mut m = serde_json::Map::new();
        for s in [
            EntryState::Selected,
            EntryState::Applied,
            EntryState::Active,
            EntryState::Shadowed,
            EntryState::Foreign,
            EntryState::Orphaned,
        ] {
            m.insert(s.as_str().to_string(), json!(self.count(s)));
        }
        Value::Object(m)
    }
}

struct Judged {
    state: EntryState,
    reason: Option<Reason>,
    candidates: Vec<String>,
    shadowed_by: Option<String>,
}

fn judge(
    req: &VerifyRequest<'_>,
    r: &SkillEntry,
    id: Option<&SkillId>,
    obs: &ObservedKind,
    shadow_by: Option<PathBuf>,
) -> Judged {
    let alias_reachable = id
        .and_then(|i| req.catalog.source(&i.source))
        .is_some_and(|s| s.reachable);
    let in_catalog = id.is_some_and(|i| req.catalog.skill(i).is_some());
    let skill_gone = alias_reachable && !in_catalog;
    let dangling = matches!(
        obs,
        ObservedKind::Symlink {
            resolves: false,
            ..
        }
    );
    let orphan = if *obs == ObservedKind::Missing {
        Some(Reason::Missing)
    } else if !is_owned(r, obs) {
        Some(Reason::Modified)
    } else if dangling {
        Some(if skill_gone {
            Reason::SourceSkillMissing
        } else {
            Reason::BrokenLink
        })
    } else if skill_gone {
        Some(Reason::SourceSkillMissing)
    } else {
        None
    };
    if let Some(reason) = orphan {
        let candidates = if reason == Reason::SourceSkillMissing {
            match req.catalog.resolve_text(&r.skill_id) {
                Err(Error::NotFound { candidates, .. }) => candidates,
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        return Judged {
            state: EntryState::Orphaned,
            reason: Some(reason),
            candidates,
            shadowed_by: None,
        };
    }
    let parsed = std::fs::read(req.root.path.join(&r.dest).join("SKILL.md"))
        .ok()
        .and_then(|b| parse_skill_md(&b, &r.name).ok());
    let (state, shadowed_by) = if parsed.is_none() {
        (EntryState::Applied, None)
    } else if let Some(by) = shadow_by {
        (
            EntryState::Shadowed,
            Some(by.to_string_lossy().into_owned()),
        )
    } else {
        (EntryState::Active, None)
    };
    Judged {
        state,
        reason: None,
        candidates: Vec::new(),
        shadowed_by,
    }
}

fn category_of(catalog: &Catalog, id: Option<&SkillId>) -> String {
    match id {
        Some(i) => catalog.skill(i).map_or_else(
            || i.rel_path.parent_str().to_string(),
            |s| s.category.clone(),
        ),
        None => String::new(),
    }
}

/// Classify the entries of `req.target` (spec 4.11). Read-only.
pub fn verify(req: &VerifyRequest<'_>) -> Result<VerifyReport> {
    let adapter = adapter_for(req.target).ok_or_else(|| Error::TargetUnsupported {
        target: req.target.as_str().to_string(),
    })?;
    let root = &req.root.path;
    let rel = adapter.destination();
    let selection = load_selection(root)?;
    let tsel = selection
        .as_ref()
        .and_then(|s| s.targets.get(&req.target))
        .cloned()
        .unwrap_or_default();
    let strategy = tsel.strategy.unwrap_or(req.config.defaults.strategy);
    let resolution = resolve_selection(&tsel, req.bundles, req.catalog, strategy);
    let state = StateDir::new(root, req.target);
    let receipt = Receipt::load(&state, root, req.target)?;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // Receipt view: consistent entries, first one per name.
    let mut view: Vec<&SkillEntry> = Vec::new();
    let mut known_names: BTreeSet<String> = BTreeSet::new();
    let mut receipt_ids: BTreeSet<String> = BTreeSet::new();
    for e in receipt.skill_entries() {
        known_names.insert(e.name.clone());
        receipt_ids.insert(e.skill_id.clone());
        if e.consistent(rel) && !view.iter().any(|v| v.name == e.name) {
            view.push(e);
        } else {
            diagnostics.push(Diagnostic::warning(
                "receipt-entry-ignored",
                format!(
                    "receipt entry {} ({}) does not match the destination; it is not classified",
                    e.skill_id, e.dest
                ),
            ));
        }
    }

    // Receipted bundle briefs are known names (never `foreign`) and are
    // checked below (M5-11).
    let briefs: Vec<_> = receipt
        .generated_entries()
        .filter(|g| g.consistent(rel))
        .collect();
    for g in &briefs {
        known_names.insert(g.name.clone());
    }
    let mut names: BTreeSet<String> = resolution.resolved.iter().map(|d| d.name.clone()).collect();
    let mut hash_names: BTreeSet<String> = BTreeSet::new();
    for e in &view {
        names.insert(e.name.clone());
        if e.strategy == Strategy::Copy {
            hash_names.insert(e.name.clone());
        }
    }
    let observed = observe(
        root,
        rel,
        &names,
        &hash_names,
        &adapter.scan_locations(root, req.dirs),
    )?;
    let brief_names: BTreeSet<String> = briefs.iter().map(|g| g.name.clone()).collect();
    let brief_obs = observe_briefs(root, rel, &observed.dest_root, &brief_names)?;
    for g in &briefs {
        match brief_obs.get(&g.name) {
            None | Some(BriefObserved::Missing) => diagnostics.push(
                Diagnostic::warning(
                    "brief-missing",
                    format!(
                        "{} (the brief of bundle {}) is gone from the destination",
                        g.dest, g.bundle
                    ),
                )
                .with_path(g.dest.clone())
                .with_remedy("Run `unam reconcile --repair` to write it again."),
            ),
            Some(BriefObserved::Brief { hash }) if hash.as_str() == g.content_hash => {}
            Some(_) => diagnostics.push(
                Diagnostic::warning(
                    "brief-modified",
                    format!(
                        "{} (the brief of bundle {}) was changed after unam wrote it; it is left as it is",
                        g.dest, g.bundle
                    ),
                )
                .with_path(g.dest.clone()),
            ),
        }
    }
    match &observed.dest_root {
        DestRootState::Outside { resolved } => diagnostics.push(Diagnostic::warning(
            "dest-outside-project",
            format!(
                "{rel} resolves to {resolved}, outside the project; nothing was inspected there"
            ),
        )),
        DestRootState::NotDir => diagnostics.push(Diagnostic::warning(
            "dest-not-directory",
            format!("{rel} exists and is not a directory"),
        )),
        _ => {}
    }

    let mut records: Vec<Record> = Vec::new();
    for r in &view {
        let id = SkillId::parse(&r.skill_id).ok();
        let obs = observed.kind(&r.name);
        let shadow_by = observed
            .shadows
            .iter()
            .find(|s| s.name == r.name)
            .map(|s| s.by.clone());
        let j = judge(req, r, id.as_ref(), &obs, shadow_by);
        let source_path = id
            .as_ref()
            .and_then(|i| req.catalog.abs_path(i))
            .map_or_else(
                || r.source_path.clone(),
                |p| p.to_string_lossy().into_owned(),
            );
        records.push(Record {
            id: Some(r.skill_id.clone()),
            name: r.name.clone(),
            category: category_of(req.catalog, id.as_ref()),
            source_path: Some(source_path),
            dest: r.dest.clone(),
            strategy: Some(r.strategy),
            provenance: canonical_provenance(&r.provenance),
            state: j.state,
            reason: j.reason,
            candidates: j.candidates,
            shadowed_by: j.shadowed_by,
        });
    }
    for d in &resolution.resolved {
        let id_text = d.id.to_string();
        if receipt_ids.contains(&id_text) {
            continue;
        }
        records.push(Record {
            id: Some(id_text),
            name: d.name.clone(),
            category: category_of(req.catalog, Some(&d.id)),
            source_path: req
                .catalog
                .abs_path(&d.id)
                .map(|p| p.to_string_lossy().into_owned()),
            dest: format!("{rel}/{}", d.name),
            strategy: Some(d.strategy),
            provenance: canonical_provenance(&d.provenance),
            state: EntryState::Selected,
            reason: None,
            candidates: Vec::new(),
            shadowed_by: None,
        });
    }
    for (name, _) in list_foreign(root, rel, &known_names)? {
        records.push(Record {
            id: None,
            dest: format!("{rel}/{name}"),
            name,
            category: String::new(),
            source_path: None,
            strategy: None,
            provenance: Vec::new(),
            state: EntryState::Foreign,
            reason: None,
            candidates: Vec::new(),
            shadowed_by: None,
        });
    }
    records.sort_by(|a, b| {
        (a.name.as_bytes(), a.state, &a.id).cmp(&(b.name.as_bytes(), b.state, &b.id))
    });

    for p in &resolution.problems {
        if p.kind == ProblemKind::Unresolved && receipt_ids.contains(&p.reference) {
            continue;
        }
        diagnostics.push(p.to_diagnostic());
    }
    let journal_present = std::fs::symlink_metadata(state.journal().path()).is_ok();
    if journal_present {
        diagnostics.push(
            Diagnostic::warning(
                "interrupted-apply",
                "an apply was interrupted; the states above may not be final",
            )
            .with_remedy("Run `unam resume` to finish it, or `unam rollback` to undo it."),
        );
    }
    Ok(VerifyReport {
        target: req.target,
        dest_rel: rel.to_string(),
        records,
        desired_ids: resolution
            .resolved
            .iter()
            .map(|d| d.id.to_string())
            .collect(),
        diagnostics,
        journal_present,
    })
}

/// A real project plus library for the verify, reconcile, and doctor unit
/// tests: apply through `change::run`, then damage the destination.
#[cfg(test)]
pub(crate) mod fixture {
    use super::*;
    use crate::catalog::LoadOptions;
    use crate::change::{ChangeRequest, Decision, Edit, Outcome};
    use crate::config::SourceEntry;
    use crate::fsops::RealFs;
    use crate::paths::{ENTERPRISE_SKILLS_VAR, Env};
    use crate::project::RootDetection;
    use crate::state::Clock;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::path::Path;

    pub struct Ticks(std::cell::Cell<u64>);

    impl Clock for Ticks {
        fn now_rfc3339(&self) -> String {
            "2026-10-05T00:00:00Z".into()
        }
        fn now_millis(&self) -> u64 {
            let v = self.0.get();
            self.0.set(v + 1);
            v
        }
        fn entropy(&self) -> u32 {
            0xabcd
        }
    }

    pub struct Fx {
        /// Keeps the temp directory alive.
        pub _guard: tempfile::TempDir,
        /// The canonical temp directory.
        pub base: PathBuf,
        pub lib: PathBuf,
        pub dirs: Dirs,
        pub config: Config,
        pub catalog: Catalog,
        pub bundles: BundleSet,
        pub root: ProjectRoot,
        pub clock: Ticks,
    }

    pub fn write(p: &Path, text: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    pub fn write_skill(lib: &Path, rel: &str, name: &str) {
        write(
            &lib.join(rel).join("SKILL.md"),
            &format!("---\nname: {name}\ndescription: About {name}.\n---\n"),
        );
    }

    pub fn dirs_in(base: &Path) -> Dirs {
        let mut m: HashMap<String, OsString> = HashMap::new();
        for (k, sub) in [
            ("HOME", "home"),
            ("UNAM_CONFIG_DIR", "config"),
            ("UNAM_CACHE_DIR", "cache"),
            ("UNAM_BUNDLES_DIR", "bundles"),
            ("XDG_DATA_HOME", "data"),
            (ENTERPRISE_SKILLS_VAR, "enterprise"),
        ] {
            m.insert(k.into(), base.join(sub).into());
        }
        Dirs::resolve(Env::from_map(m)).unwrap()
    }

    pub fn load_catalog(config: &Config, dirs: &Dirs) -> Catalog {
        Catalog::load(
            &config.sources,
            dirs,
            LoadOptions {
                use_cache: false,
                git_global: false,
            },
        )
        .unwrap()
    }

    pub fn fx() -> Fx {
        let t = tempfile::tempdir().unwrap();
        let base = std::fs::canonicalize(t.path()).unwrap();
        let lib = base.join("lib");
        for (rel, name) in [
            ("sdlc/code-review", "code-review"),
            ("testing/pytest-fixtures", "pytest-fixtures"),
            ("data/schema-migration", "schema-migration"),
        ] {
            write_skill(&lib, rel, name);
        }
        let dirs = dirs_in(&base);
        let config = Config {
            sources: vec![SourceEntry {
                alias: "fx".into(),
                path: lib.to_string_lossy().into_owned(),
                precedence: 10,
            }],
            ..Config::default()
        };
        let catalog = load_catalog(&config, &dirs);
        let p = base.join("work/p");
        std::fs::create_dir_all(p.join(".git")).unwrap();
        Fx {
            base,
            lib,
            dirs,
            config,
            catalog,
            bundles: BundleSet::default(),
            root: ProjectRoot {
                path: std::fs::canonicalize(&p).unwrap(),
                how: RootDetection::Git,
            },
            clock: Ticks(std::cell::Cell::new(1)),
            _guard: t,
        }
    }

    impl Fx {
        pub fn run(&self, edit: Edit) -> Outcome {
            let req = ChangeRequest {
                dirs: &self.dirs,
                config: &self.config,
                catalog: &self.catalog,
                bundles: &self.bundles,
                root: &self.root,
                target: TargetId::ClaudeCode,
                edit,
                command: "add",
                clock: &self.clock,
            };
            crate::change::run(&req, &RealFs, &mut |_| Decision::Proceed)
                .map_err(|f| f.error)
                .unwrap()
        }

        pub fn add(&self, refs: &[&str]) {
            let out = self.run(Edit::Add(refs.iter().map(|s| s.to_string()).collect()));
            assert!(matches!(out, Outcome::Applied(..)));
        }

        pub fn dest(&self, name: &str) -> PathBuf {
            self.root.path.join(".claude/skills").join(name)
        }

        pub fn request(&self) -> VerifyRequest<'_> {
            VerifyRequest {
                dirs: &self.dirs,
                config: &self.config,
                catalog: &self.catalog,
                bundles: &self.bundles,
                root: &self.root,
                target: TargetId::ClaudeCode,
            }
        }

        pub fn verify(&self) -> VerifyReport {
            verify(&self.request()).unwrap()
        }

        /// Reload the catalog (after a change to the library or the config).
        pub fn reload(&mut self) {
            self.catalog = load_catalog(&self.config, &self.dirs);
        }
    }

    pub fn state_of(report: &VerifyReport, name: &str) -> EntryState {
        report
            .records
            .iter()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("no record for {name}"))
            .state
    }
}

#[cfg(test)]
mod tests {
    use super::fixture::*;
    use super::*;

    #[cfg(unix)]
    #[test]
    fn record_has_id_name_category_source_dest_strategy_provenance_state() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        let report = f.verify();
        assert_eq!(report.records.len(), 1);
        let r = &report.records[0];
        assert_eq!(r.id.as_deref(), Some("fx:sdlc/code-review"));
        assert_eq!(r.name, "code-review");
        assert_eq!(r.category, "sdlc");
        assert_eq!(
            r.source_path.as_deref(),
            Some(f.lib.join("sdlc/code-review").to_str().unwrap())
        );
        assert_eq!(r.dest, ".claude/skills/code-review");
        assert_eq!(r.strategy, Some(Strategy::Symlink));
        assert_eq!(r.provenance, vec![Provenance::Adhoc]);
        assert_eq!(r.state, EntryState::Active);
        let v = r.to_json();
        for k in [
            "id",
            "name",
            "category",
            "source_path",
            "dest",
            "strategy",
            "provenance",
            "state",
        ] {
            assert!(v.get(k).is_some(), "missing {k}");
        }
        assert!(v.get("reason").is_none() && v.get("candidates").is_none());
        assert!(!report.journal_present);
    }

    #[cfg(unix)]
    #[test]
    fn selected_foreign_and_applied_states() {
        let f = fx();
        f.add(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]);
        // Unparseable SKILL.md behind a resolving link: applied.
        std::fs::write(
            f.lib.join("testing/pytest-fixtures/SKILL.md"),
            "no frontmatter",
        )
        .unwrap();
        // A foreign directory, a plain file, and a dot-name.
        write(&f.dest("house-style/SKILL.md"), "x");
        write(&f.dest("README.md"), "x");
        write(&f.dest(".DS_Store"), "x");
        // Desired but not applied (a hand-edited selection).
        write(
            &crate::project::selection_path(&f.root.path),
            "version: 1\ntargets:\n  claude-code:\n    add:\n      - fx:sdlc/code-review\n      - fx:testing/pytest-fixtures\n      - fx:data/schema-migration\n",
        );
        let report = f.verify();
        assert_eq!(state_of(&report, "code-review"), EntryState::Active);
        assert_eq!(state_of(&report, "pytest-fixtures"), EntryState::Applied);
        assert_eq!(state_of(&report, "house-style"), EntryState::Foreign);
        assert_eq!(state_of(&report, "schema-migration"), EntryState::Selected);
        assert!(report.records.iter().all(|r| r.name != "README.md"));
        assert!(report.records.iter().all(|r| r.name != ".DS_Store"));
        assert_eq!(report.records.len(), 4);
    }

    #[cfg(unix)]
    #[test]
    fn broken_symlink_is_orphaned_never_active() {
        let mut f = fx();
        f.add(&["fx:sdlc/code-review"]);
        // The link target goes away while the source stays reachable: the
        // skill is gone from the catalog too (`source-skill-missing`).
        std::fs::remove_dir_all(f.lib.join("sdlc/code-review")).unwrap();
        f.reload();
        let report = f.verify();
        let r = &report.records[0];
        assert_eq!(r.state, EntryState::Orphaned);
        assert_eq!(r.reason, Some(Reason::SourceSkillMissing));
        // An unreachable source: a plain broken link.
        let mut g = fx();
        g.add(&["fx:sdlc/code-review"]);
        std::fs::remove_dir_all(&g.lib).unwrap();
        g.reload();
        let r = &g.verify().records[0];
        assert_eq!(r.state, EntryState::Orphaned);
        assert_eq!(r.reason, Some(Reason::BrokenLink));
        // A missing destination and a hand-replaced link.
        let h = fx();
        h.add(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]);
        std::fs::remove_file(h.dest("code-review")).unwrap();
        std::fs::remove_file(h.dest("pytest-fixtures")).unwrap();
        std::os::unix::fs::symlink(
            h.lib.join("data/schema-migration"),
            h.dest("pytest-fixtures"),
        )
        .unwrap();
        let report = h.verify();
        let by = |n: &str| report.records.iter().find(|r| r.name == n).unwrap();
        assert_eq!(by("code-review").reason, Some(Reason::Missing));
        assert_eq!(by("pytest-fixtures").reason, Some(Reason::Modified));
        assert_eq!(report.count(EntryState::Orphaned), 2);
    }

    #[cfg(unix)]
    #[test]
    fn shadowed_by_user_scope_reports_both_paths() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        let user = f.base.join("home/.claude/skills/code-review");
        write(
            &user.join("SKILL.md"),
            "---\nname: code-review\ndescription: Mine.\n---\n",
        );
        let report = f.verify();
        let r = &report.records[0];
        assert_eq!(r.state, EntryState::Shadowed);
        assert_eq!(r.dest, ".claude/skills/code-review");
        assert_eq!(r.shadowed_by.as_deref(), Some(user.to_str().unwrap()));
    }

    #[cfg(unix)]
    #[test]
    fn renamed_source_skill_is_orphaned_with_candidates() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        std::fs::rename(
            f.lib.join("sdlc/code-review"),
            f.lib.join("sdlc/code-reviewer"),
        )
        .unwrap();
        let mut f = f;
        f.reload();
        let report = f.verify();
        let r = &report.records[0];
        assert_eq!(r.state, EntryState::Orphaned);
        assert_eq!(r.reason, Some(Reason::SourceSkillMissing));
        assert_eq!(r.candidates, vec!["fx:sdlc/code-reviewer".to_string()]);
        // The selection ref that no longer resolves is not repeated as a
        // separate diagnostic.
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    }

    #[cfg(unix)]
    #[test]
    fn follow_mode_source_edit_stays_active() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        write(
            &f.lib.join("sdlc/code-review/SKILL.md"),
            "---\nname: code-review\ndescription: Edited later.\n---\nNew body.\n",
        );
        let report = f.verify();
        assert_eq!(report.records[0].state, EntryState::Active);
        assert!(report.diagnostics.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_leftover_journal_adds_a_warning_not_a_failure() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        write(&f.root.path.join(".unam/claude-code/journal.jsonl"), "");
        let report = f.verify();
        assert!(report.journal_present);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "interrupted-apply")
        );
    }
}
