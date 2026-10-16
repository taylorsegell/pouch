//! Unit tests of `change` (M3 plan, change.rs specification). Tempdir
//! projects, an in-process catalog, `RealFs`.

use super::*;
use crate::catalog::LoadOptions;
use crate::config::SourceEntry;
use crate::fsops::{DestPath, OwnedExpectation, RealFs, RemoveOutcome, RestoreOutcome};
use crate::paths::{ENTERPRISE_SKILLS_VAR, Env};
use crate::project::index::read_index;
use crate::source::SourcePath;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;

struct FixedClock(std::cell::Cell<u64>);
impl Clock for FixedClock {
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

struct Fx {
    t: tempfile::TempDir,
    lib: PathBuf,
    dirs: Dirs,
    config: Config,
    catalog: Catalog,
    clock: FixedClock,
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn fx() -> Fx {
    let t = tempfile::tempdir().unwrap();
    let lib = t.path().join("lib");
    for (rel, name) in [
        ("sdlc/code-review", "code-review"),
        ("testing/pytest-fixtures", "pytest-fixtures"),
        ("data/schema-migration", "schema-migration"),
    ] {
        write(
            &lib.join(rel).join("SKILL.md"),
            &format!("---\nname: {name}\ndescription: About {name}.\n---\n"),
        );
    }
    let mut m: HashMap<String, OsString> = HashMap::new();
    for (k, sub) in [
        ("HOME", "home"),
        ("UNAM_CONFIG_DIR", "config"),
        ("UNAM_CACHE_DIR", "cache"),
        ("UNAM_BUNDLES_DIR", "bundles"),
        ("XDG_DATA_HOME", "data"),
        (ENTERPRISE_SKILLS_VAR, "enterprise"),
    ] {
        m.insert(k.into(), t.path().join(sub).into());
    }
    let dirs = Dirs::resolve(Env::from_map(m)).unwrap();
    let config = Config {
        sources: vec![SourceEntry {
            alias: "fx".into(),
            path: lib.to_string_lossy().into_owned(),
            precedence: 10,
        }],
        ..Config::default()
    };
    let catalog = Catalog::load(
        &config.sources,
        &dirs,
        LoadOptions {
            use_cache: false,
            git_global: false,
        },
    )
    .unwrap();
    write(
        &t.path().join("bundles/b1.yaml"),
        "name: b1\nskills:\n  - fx:sdlc/code-review\n  - fx:testing/pytest-fixtures\n",
    );
    Fx {
        t,
        lib,
        dirs,
        config,
        catalog,
        clock: FixedClock(std::cell::Cell::new(1)),
    }
}

impl Fx {
    fn project(&self, name: &str) -> ProjectRoot {
        let p = self.t.path().join("work").join(name);
        fs::create_dir_all(p.join(".git")).unwrap();
        ProjectRoot {
            path: fs::canonicalize(&p).unwrap(),
            how: RootDetection::Git,
        }
    }

    fn bundles(&self) -> BundleSet {
        crate::bundle::load_dir(&self.dirs)
    }

    fn run_edit(
        &self,
        root: &ProjectRoot,
        edit: Edit,
        fs_ops: &dyn FsOps,
        decide: &mut dyn FnMut(&Plan) -> Decision,
    ) -> CResult<Outcome> {
        let bundles = self.bundles();
        let req = ChangeRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            bundles: &bundles,
            root,
            target: TargetId::ClaudeCode,
            edit,
            command: "add",
            clock: &self.clock,
        };
        run(&req, fs_ops, decide)
    }

    fn add(&self, root: &ProjectRoot, refs: &[&str]) -> Outcome {
        let refs = refs.iter().map(|s| s.to_string()).collect();
        self.run_edit(root, Edit::Add(refs), &RealFs, &mut |_| Decision::Proceed)
            .map_err(|f| f.error)
            .unwrap()
    }
}

fn yes(_: &Plan) -> Decision {
    Decision::Proceed
}

fn tsel(bundles: &[&str], add: &[&str], remove: &[&str]) -> TargetSelection {
    let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect();
    TargetSelection {
        strategy: None,
        bundles: v(bundles),
        add: v(add),
        remove: v(remove),
    }
}

fn s(x: &str) -> String {
    x.to_string()
}

// ---- apply_edit ----

#[test]
fn apply_edit_table() {
    let f = fx();
    let b = f.bundles();
    let c = &f.catalog;
    // add is idempotent and stores the qualified id.
    let (t, changed) = apply_edit(
        &tsel(&[], &[], &[]),
        &Edit::Add(vec![s("code-review")]),
        c,
        &b,
    )
    .unwrap();
    assert!(changed);
    assert_eq!(t.add, vec!["fx:sdlc/code-review"]);
    let (_, changed) = apply_edit(&t, &Edit::Add(vec![s("fx:sdlc/code-review")]), c, &b).unwrap();
    assert!(!changed);
    // add drops a matching remove.
    let (t, _) = apply_edit(
        &tsel(&["b1"], &[], &["code-review"]),
        &Edit::Add(vec![s("code-review")]),
        c,
        &b,
    )
    .unwrap();
    assert!(t.remove.is_empty());
    // rm drops add.
    let (t, _) = apply_edit(
        &tsel(&[], &["fx:sdlc/code-review"], &[]),
        &Edit::Remove(vec![s("code-review")]),
        c,
        &b,
    )
    .unwrap();
    assert!(t.add.is_empty() && t.remove.is_empty());
    // rm records a remove for a bundle-supplied skill.
    let (t, _) = apply_edit(
        &tsel(&["b1"], &[], &[]),
        &Edit::Remove(vec![s("pytest-fixtures")]),
        c,
        &b,
    )
    .unwrap();
    assert_eq!(t.remove, vec!["fx:testing/pytest-fixtures"]);
    // rm of an unselected skill.
    let e = apply_edit(
        &tsel(&[], &[], &[]),
        &Edit::Remove(vec![s("schema-migration")]),
        c,
        &b,
    )
    .unwrap_err();
    assert_eq!((e.code(), e.exit_code()), ("not-selected", 4));
    // rm of a dangling add by exact text (source unreachable).
    let (t, _) = apply_edit(
        &tsel(&[], &["gone:x/y"], &[]),
        &Edit::Remove(vec![s("gone:x/y")]),
        c,
        &b,
    )
    .unwrap();
    assert!(t.add.is_empty());
    // use --replace clears add and remove and keeps the strategy.
    let mut start = tsel(&["other"], &["fx:sdlc/code-review"], &["x"]);
    start.strategy = Some(Strategy::Copy);
    let (t, _) = apply_edit(&start, &Edit::UseReplace(s("B1")), c, &b).unwrap();
    assert_eq!(t.bundles, vec!["b1"]);
    assert!(t.add.is_empty() && t.remove.is_empty());
    assert_eq!(t.strategy, Some(Strategy::Copy));
    // use --add is idempotent.
    let (t, _) = apply_edit(&tsel(&["b1"], &[], &[]), &Edit::UseAdd(s("b1")), c, &b).unwrap();
    assert_eq!(t.bundles, vec!["b1"]);
    // unknown bundle.
    let e = apply_edit(&tsel(&[], &[], &[]), &Edit::UseAdd(s("nope")), c, &b).unwrap_err();
    assert_eq!(e.code(), "bundle-not-found");
    // Errors change nothing: an invalid reference among valid ones.
    let e = apply_edit(
        &tsel(&[], &[], &[]),
        &Edit::Add(vec![s("code-review"), s("Bad:x")]),
        c,
        &b,
    )
    .unwrap_err();
    assert_eq!(e.exit_code(), 2);
    // deactivate.
    let (t, changed) = apply_edit(&tsel(&["b1"], &[], &[]), &Edit::Deactivate, c, &b).unwrap();
    assert_eq!(t, TargetSelection::default());
    assert!(changed);
}

// ---- resolve_target, resolve_root ----

#[test]
fn resolve_target_branches() {
    let f = fx();
    let root = f.project("t");
    let d = &f.dirs;
    let (t, det) = resolve_target(Some(TargetId::ClaudeCode), None, &root.path, d).unwrap();
    assert_eq!(t, TargetId::ClaudeCode);
    assert_eq!(det, Detection::Absent);
    let e = resolve_target(Some(TargetId::AgentsNeutral), None, &root.path, d).unwrap_err();
    assert_eq!(e.code(), "target-unsupported");
    let mut sel = Selection::default();
    sel.targets
        .insert(TargetId::ClaudeCode, TargetSelection::default());
    assert_eq!(
        resolve_target(None, Some(&sel), &root.path, d).unwrap().0,
        TargetId::ClaudeCode
    );
    sel.targets
        .insert(TargetId::AgentsNeutral, TargetSelection::default());
    let e = resolve_target(None, Some(&sel), &root.path, d).unwrap_err();
    assert_eq!(e.code(), "target-ambiguous");
    let mut only_an = Selection::default();
    only_an
        .targets
        .insert(TargetId::AgentsNeutral, TargetSelection::default());
    assert_eq!(
        resolve_target(None, Some(&only_an), &root.path, d)
            .unwrap_err()
            .code(),
        "target-unsupported"
    );
    fs::create_dir_all(root.path.join(".claude")).unwrap();
    let (t, det) = resolve_target(None, None, &root.path, d).unwrap();
    assert_eq!(t, TargetId::ClaudeCode);
    assert!(det.is_present());
}

#[test]
fn resolve_root_with_project_flag() {
    let f = fx();
    let outer = f.t.path().join("outer");
    let inner = outer.join("inner");
    fs::create_dir_all(outer.join(".git")).unwrap();
    fs::create_dir_all(&inner).unwrap();
    let r = resolve_root(Some(&inner), Path::new("/")).unwrap();
    assert_eq!(r.path, fs::canonicalize(&inner).unwrap());
    assert_eq!(r.how, RootDetection::Explicit);
    let r = resolve_root(None, &inner).unwrap();
    assert_eq!(r.path, fs::canonicalize(&outer).unwrap());
    let e = resolve_root(Some(&outer.join("missing")), Path::new("/")).unwrap_err();
    assert_eq!((e.code(), e.exit_code()), ("project-path-invalid", 2));
    fs::write(outer.join("file"), b"x").unwrap();
    let e = resolve_root(Some(&outer.join("file")), Path::new("/")).unwrap_err();
    assert_eq!(e.code(), "project-path-invalid");
}

// ---- ensure_project_outside_sources (G7) ----

#[test]
fn ensure_project_outside_sources_rows() {
    let f = fx();
    let canon = |p: &Path| ProjectRoot {
        path: fs::canonicalize(p).unwrap(),
        how: RootDetection::Explicit,
    };
    let check = |p: &Path| ensure_project_outside_sources(&f.config, &f.dirs, &canon(p));
    assert_eq!(check(&f.lib).unwrap_err().code(), "project-overlaps-source");
    fs::create_dir_all(f.lib.join("sdlc/proj")).unwrap();
    assert_eq!(
        check(&f.lib.join("sdlc/proj")).unwrap_err().code(),
        "project-overlaps-source"
    );
    assert_eq!(
        check(f.t.path()).unwrap_err().code(),
        "project-overlaps-source"
    );
    let sibling = f.t.path().join("lib-other");
    fs::create_dir_all(&sibling).unwrap();
    assert!(check(&sibling).is_ok());
    // An uncanonicalizable source is skipped.
    let mut cfg = f.config.clone();
    cfg.sources[0].path = "/definitely/missing/lib".into();
    assert!(ensure_project_outside_sources(&cfg, &f.dirs, &canon(&f.lib)).is_ok());
    // A refusal happens before the lock: no `.unam` is created.
    let root = canon(&f.lib.join("sdlc/proj"));
    let bundles = f.bundles();
    let req = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &root,
        target: TargetId::ClaudeCode,
        edit: Edit::Add(vec![s("code-review")]),
        command: "add",
        clock: &f.clock,
    };
    let e = run(&req, &RealFs, &mut yes).err().unwrap();
    assert_eq!(e.error.code(), "project-overlaps-source");
    assert!(!root.path.join(".unam").exists());
}

// ---- run outcomes ----

#[test]
fn run_conflict_writes_no_selection_and_no_journal() {
    let f = fx();
    let root = f.project("c");
    write(
        &root.path.join(".claude/skills/code-review/SKILL.md"),
        "mine",
    );
    let out = f.add(&root, &["code-review"]);
    let Outcome::Blocked(p) = out else { panic!() };
    assert_eq!(p.plan.conflicts[0].code(), "foreign-at-dest");
    assert!(!selection_path(&root.path).exists());
    assert!(!root.path.join(".unam/claude-code").exists());
}

#[test]
fn run_empty_plan_with_changed_selection_writes_it() {
    let f = fx();
    let root = f.project("e");
    // A hand-written selection with a no-op remove entry, applied
    // through an edit that leaves the selection as it is.
    write(
        &selection_path(&root.path),
        "targets:\n  claude-code:\n    bundles: [b1]\n    remove: [\"fx:data/schema-migration\"]\n",
    );
    let out = f
        .run_edit(&root, Edit::UseAdd(s("b1")), &RealFs, &mut yes)
        .map_err(|x| x.error)
        .unwrap();
    assert!(matches!(out, Outcome::Applied(ref p, _) if !p.selection_written));
    // `use b1 --replace` clears the remove entry: the selection changes,
    // the desired set and provenance do not, so the plan is empty.
    let before = fs::read_to_string(selection_path(&root.path)).unwrap();
    let out = f
        .run_edit(&root, Edit::UseReplace(s("b1")), &RealFs, &mut |_| {
            panic!("no confirmation for an empty plan")
        })
        .map_err(|x| x.error)
        .unwrap();
    let Outcome::NoChanges(p) = out else { panic!() };
    assert!(p.selection_written);
    let after = fs::read_to_string(selection_path(&root.path)).unwrap();
    assert_ne!(before, after);
    assert!(!after.contains("schema-migration"));
    assert!(
        !StateDir::new(&root.path, TargetId::ClaudeCode)
            .journal()
            .path()
            .exists()
    );
}

#[test]
fn run_refuse_and_decline_write_nothing() {
    let f = fx();
    for d in [Decision::Refuse, Decision::Decline] {
        let root = f.project(&format!("r-{d:?}"));
        let out = f
            .run_edit(
                &root,
                Edit::Add(vec![s("code-review")]),
                &RealFs,
                &mut |_| d,
            )
            .map_err(|x| x.error)
            .unwrap();
        assert!(matches!(
            (&out, d),
            (Outcome::Refused(_), Decision::Refuse) | (Outcome::Declined(_), Decision::Decline)
        ));
        assert!(!selection_path(&root.path).exists());
        assert!(!root.path.join(".claude").exists());
        assert!(!root.path.join(".unam/claude-code").exists());
    }
}

/// Logs every mutating call with its path.
#[derive(Default)]
struct LogFs {
    log: RefCell<Vec<String>>,
}
impl LogFs {
    fn note(&self, m: &str, p: &Path) {
        self.log.borrow_mut().push(format!(
            "{m} {}",
            p.file_name().unwrap_or_default().to_string_lossy()
        ));
    }
}
impl FsOps for LogFs {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.note("write_state", p.path());
        RealFs.write_state(p, b)
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.note("append_state", p.path());
        RealFs.append_state(p, b)
    }
    fn remove_state(&self, p: &StatePath) -> Result<()> {
        self.note("remove_state", p.path());
        RealFs.remove_state(p)
    }
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
        self.note("remove_state_dir_if_empty", p.path());
        RealFs.remove_state_dir_if_empty(p)
    }
    fn create_dir(&self, d: &DestPath) -> Result<bool> {
        self.note("create_dir", d.path());
        RealFs.create_dir(d)
    }
    fn rmdir_if_empty(&self, d: &DestPath) -> Result<bool> {
        self.note("rmdir_if_empty", d.path());
        RealFs.rmdir_if_empty(d)
    }
    fn symlink_dir(&self, src: &SourcePath, d: &DestPath) -> Result<()> {
        self.note("symlink_dir", d.path());
        RealFs.symlink_dir(src, d)
    }
    fn copy_tree(&self, src: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        self.note("copy_tree", d.path());
        RealFs.copy_tree(src, d, tag, h)
    }
    fn remove_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
    ) -> Result<RemoveOutcome> {
        self.note("remove_owned", d.path());
        RealFs.remove_owned(d, e, t)
    }
    fn restore_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
        src: Option<&SourcePath>,
        tag: &str,
    ) -> Result<RestoreOutcome> {
        self.note("restore_owned", d.path());
        RealFs.restore_owned(d, e, t, src, tag)
    }
}

#[test]
fn run_applied_writes_the_selection_before_the_journal_begin() {
    let f = fx();
    let root = f.project("a");
    let log = LogFs::default();
    let out = f
        .run_edit(&root, Edit::Add(vec![s("code-review")]), &log, &mut yes)
        .map_err(|x| x.error)
        .unwrap();
    assert!(matches!(out, Outcome::Applied(..)));
    let l = log.log.borrow();
    let sel = l
        .iter()
        .position(|x| x == "write_state selection.yaml")
        .unwrap();
    let begin = l
        .iter()
        .position(|x| x == "write_state journal.jsonl")
        .unwrap();
    assert!(sel < begin, "{l:?}");
    assert!(root.path.join(".claude/skills/code-review").exists());
}

#[test]
fn run_with_an_interrupted_journal_returns_interrupted_before_any_write() {
    let f = fx();
    let root = f.project("i");
    let state = StateDir::new(&root.path, TargetId::ClaudeCode);
    let begin = serde_json::json!({
        "t": "begin", "version": 1, "op_id": "0000000000001-0000abcd", "command": "add",
        "target": "claude-code", "project_root": root.path.to_string_lossy(), "started_at": "s",
        "dest_rel": ".claude/skills", "dest_dirs_planned": [],
        "selection_before": {"existed": false, "text": ""},
        "selection_after_text": "", "selection_after_hash": "h", "receipt_before": null,
        "plan": [{"op": "drop_orphan", "n": 1, "id": "fx:a/b", "name": "b", "reason": "missing"}]
    });
    write(state.journal().path(), &format!("{begin}\n"));
    let log = LogFs::default();
    let out = f
        .run_edit(&root, Edit::Add(vec![s("code-review")]), &log, &mut yes)
        .map_err(|x| x.error)
        .unwrap();
    assert!(matches!(out, Outcome::Interrupted(_)));
    assert!(log.log.borrow().is_empty());
    assert!(!selection_path(&root.path).exists());
}

#[test]
fn the_lock_is_held_during_decide() {
    let f = fx();
    let root = f.project("l");
    let mut held = false;
    f.run_edit(
        &root,
        Edit::Add(vec![s("code-review")]),
        &RealFs,
        &mut |_| {
            let e = ProjectLock::acquire(&root.path, "x", &f.clock).unwrap_err();
            held = e.code() == "locked";
            Decision::Decline
        },
    )
    .map_err(|x| x.error)
    .unwrap();
    assert!(held);
}

#[test]
fn a_locked_run_never_reads_the_journal() {
    let f = fx();
    let root = f.project("k");
    let state = StateDir::new(&root.path, TargetId::ClaudeCode);
    write(state.journal().path(), "garbage line one\ngarbage\n");
    let _guard = ProjectLock::acquire(&root.path, "hold", &f.clock).unwrap();
    let e = f
        .run_edit(&root, Edit::Add(vec![s("code-review")]), &RealFs, &mut yes)
        .err()
        .unwrap();
    assert_eq!(e.error.code(), "locked");
}

// ---- carry versus block ----

#[test]
fn owned_unreachable_is_carried_but_an_unresolved_add_blocks() {
    let mut f = fx();
    let root = f.project("u");
    f.add(&root, &["code-review"]);
    // The source becomes unreachable.
    let moved = f.t.path().join("lib-moved");
    fs::rename(&f.lib, &moved).unwrap();
    f.catalog = Catalog::load(
        &f.config.sources,
        &f.dirs,
        LoadOptions {
            use_cache: false,
            git_global: false,
        },
    )
    .unwrap();
    let bundles = f.bundles();
    let req = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &root,
        target: TargetId::ClaudeCode,
        edit: Edit::Deactivate,
        command: "x",
        clock: &f.clock,
    };
    // prepare with the selection unchanged (an add of an existing ref is
    // not possible offline), so use a hand selection check through prepare.
    let mut r2 = ChangeRequest {
        edit: Edit::UseAdd(s("b1")),
        ..req
    };
    r2.command = "use";
    let e = prepare(&r2).err().unwrap();
    assert_eq!(e.error.code(), "not-found", "b1 entries do not resolve");
    // Owned carry: a request that keeps the selection as it is.
    write(
        &f.t.path().join("bundles/empty.yaml"),
        "name: keep\nskills:\n  - fx:sdlc/code-review\n",
    );
    let bundles = f.bundles();
    let r3 = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &root,
        target: TargetId::ClaudeCode,
        edit: Edit::Remove(vec![s("fx:sdlc/code-review")]),
        command: "rm",
        clock: &f.clock,
    };
    let p = prepare(&r3).map_err(|x| x.error).unwrap();
    // Removed from the selection: the receipt entry is no longer carried
    // but cannot be proven (the link dangles yet matches): a Remove.
    assert!(p.plan.carried.is_empty());
    // Keep it selected: no edit (Add of the same text is unresolvable, so
    // check the carry through `diff` input built by prepare for UseAdd of
    // a bundle that names it).
    let r4 = ChangeRequest {
        edit: Edit::UseAdd(s("keep")),
        ..r3
    };
    let p = prepare(&r4).map_err(|x| x.error).unwrap();
    assert_eq!(p.plan.carried.len(), 1);
    assert!(p.plan.steps.is_empty());
    assert!(
        p.plan
            .warnings
            .iter()
            .any(|w| w.code == "owned-source-unreachable")
    );
    fs::rename(&moved, &f.lib).unwrap();
}

// ---- G11, root moved, registration ----

#[test]
fn unchanged_selection_has_after_text_and_hash_equal_to_before() {
    let f = fx();
    let root = f.project("g11");
    f.add(&root, &["code-review"]);
    // Delete the link by hand: the same add repairs it with an unchanged
    // selection.
    fs::remove_file(root.path.join(".claude/skills/code-review")).unwrap();
    let before = fs::read_to_string(selection_path(&root.path)).unwrap();
    let bundles = f.bundles();
    let req = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &root,
        target: TargetId::ClaudeCode,
        edit: Edit::Add(vec![s("code-review")]),
        command: "add",
        clock: &f.clock,
    };
    let p = prepare(&req).map_err(|x| x.error).unwrap();
    assert!(!p.selection_changed);
    assert_eq!(p.selection_after_text, before);
    assert_eq!(
        p.selection_after_hash,
        Hash::of_bytes(before.as_bytes()).to_string()
    );
    // Crash the repair, then roll back: no selection-not-restored.
    struct Crash;
    impl FsOps for Crash {
        fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
            RealFs.write_state(p, b)
        }
        fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
            RealFs.append_state(p, b)
        }
        fn remove_state(&self, p: &StatePath) -> Result<()> {
            RealFs.remove_state(p)
        }
        fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
            RealFs.remove_state_dir_if_empty(p)
        }
        fn create_dir(&self, d: &DestPath) -> Result<bool> {
            RealFs.create_dir(d)
        }
        fn rmdir_if_empty(&self, d: &DestPath) -> Result<bool> {
            RealFs.rmdir_if_empty(d)
        }
        fn symlink_dir(&self, s: &SourcePath, d: &DestPath) -> Result<()> {
            RealFs.symlink_dir(s, d)?;
            panic!("crash after symlink")
        }
        fn copy_tree(&self, s: &SourcePath, d: &DestPath, t: &str, h: &Hash) -> Result<Hash> {
            RealFs.copy_tree(s, d, t, h)
        }
        fn remove_owned(
            &self,
            d: &DestPath,
            e: &OwnedExpectation,
            t: &StatePath,
        ) -> Result<RemoveOutcome> {
            RealFs.remove_owned(d, e, t)
        }
        fn restore_owned(
            &self,
            d: &DestPath,
            e: &OwnedExpectation,
            t: &StatePath,
            s: Option<&SourcePath>,
            tag: &str,
        ) -> Result<RestoreOutcome> {
            RealFs.restore_owned(d, e, t, s, tag)
        }
    }
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&req, &Crash, &mut yes)));
    let rr = RecoverRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        root: &root,
        target: None,
        clock: &f.clock,
    };
    let out = rollback(&rr, &RealFs, &mut |_| Decision::Proceed)
        .map_err(|x| x.error)
        .unwrap();
    let RecoverOutcome::Recovered(report, _) = out else {
        panic!()
    };
    assert!(
        !report
            .warnings
            .iter()
            .any(|w| w.code == "selection-not-restored"),
        "{:?}",
        report.warnings
    );
    assert_eq!(
        fs::read_to_string(selection_path(&root.path)).unwrap(),
        before
    );
}

#[test]
fn receipt_root_moved_warning_in_prepare() {
    let f = fx();
    let root = f.project("m1");
    f.add(&root, &["code-review"]);
    let moved_to = f.t.path().join("work/m2");
    fs::rename(&root.path, &moved_to).unwrap();
    let moved = ProjectRoot {
        path: fs::canonicalize(&moved_to).unwrap(),
        how: RootDetection::Git,
    };
    let bundles = f.bundles();
    let req = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &moved,
        target: TargetId::ClaudeCode,
        edit: Edit::Add(vec![s("code-review")]),
        command: "add",
        clock: &f.clock,
    };
    let p = prepare(&req).map_err(|x| x.error).unwrap();
    assert!(p.warnings.iter().any(|w| w.code == "receipt-root-moved"));
}

#[test]
fn run_registers_the_project_on_every_selection_write() {
    let f = fx();
    let canon_of = |r: &ProjectRoot| r.path.to_string_lossy().into_owned();
    // Applied run.
    let a = f.project("reg-a");
    f.add(&a, &["code-review"]);
    assert_eq!(read_index(&f.dirs).projects, vec![canon_of(&a)]);
    // Empty plan with a changed selection.
    let b = f.project("reg-b");
    let bundles = f.bundles();
    let req = ChangeRequest {
        dirs: &f.dirs,
        config: &f.config,
        catalog: &f.catalog,
        bundles: &bundles,
        root: &b,
        target: TargetId::ClaudeCode,
        edit: Edit::Deactivate,
        command: "deactivate",
        clock: &f.clock,
    };
    // Deactivate of a project without a selection writes nothing.
    let out = run(&req, &RealFs, &mut yes).map_err(|x| x.error).unwrap();
    assert!(matches!(out, Outcome::NoChanges(ref p) if !p.selection_written));
    assert!(!read_index(&f.dirs).projects.contains(&canon_of(&b)));
    // Deactivate of a selected project writes and registers once.
    let out = run(&ChangeRequest { root: &a, ..req }, &RealFs, &mut yes)
        .map_err(|x| x.error)
        .unwrap();
    assert!(matches!(out, Outcome::Applied(ref p, _) if p.selection_written));
    assert_eq!(read_index(&f.dirs).projects, vec![canon_of(&a)]);
    // An unreadable index (a directory at its path) is a warning only.
    let c = f.project("reg-c");
    fs::remove_file(f.dirs.project_index_file()).unwrap();
    fs::create_dir_all(f.dirs.project_index_file()).unwrap();
    let out = f.add(&c, &["code-review"]);
    let Outcome::Applied(p, _) = out else {
        panic!()
    };
    assert!(
        p.warnings
            .iter()
            .any(|w| w.code == "project-index-unreadable")
    );
    fs::remove_dir(f.dirs.project_index_file()).unwrap();
    // A read-only data directory with the index absent: project-index-failed.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
            .unwrap_or(false)
        {
            println!("self-skip: running as root (named residual)");
            return;
        }
        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
            }
        }
        fs::create_dir_all(&f.dirs.data).unwrap();
        fs::set_permissions(&f.dirs.data, fs::Permissions::from_mode(0o500)).unwrap();
        let _restore = Restore(f.dirs.data.clone());
        let d = f.project("reg-d");
        let out = f.add(&d, &["code-review"]);
        let Outcome::Applied(p, _) = out else {
            panic!()
        };
        assert!(
            p.warnings.iter().any(|w| w.code == "project-index-failed"),
            "{:?}",
            p.warnings
        );
    }
}

// ---- history rollback and reconcile edits (milestone M4) ----

#[test]
fn history_edit_restores_target_selection_before() {
    let f = fx();
    let root = f.project("p");
    let e = history_edit(&root.path, TargetId::ClaudeCode).unwrap_err();
    assert_eq!(e.code(), "no-history");
    f.add(&root, &["fx:sdlc/code-review"]);
    // The first apply had no selection: the history edit clears the target.
    assert_eq!(
        history_edit(&root.path, TargetId::ClaudeCode).unwrap(),
        Edit::Deactivate
    );
    f.add(&root, &["fx:testing/pytest-fixtures"]);
    let edit = history_edit(&root.path, TargetId::ClaudeCode).unwrap();
    assert_eq!(
        edit,
        Edit::Restore(tsel(&[], &["fx:sdlc/code-review"], &[]))
    );
    let out = f
        .run_edit(&root, edit, &RealFs, &mut |_| Decision::Proceed)
        .map_err(|c| c.error)
        .unwrap();
    assert!(matches!(out, Outcome::Applied(..)));
    let state = StateDir::new(&root.path, TargetId::ClaudeCode);
    let receipt = Receipt::load(&state, &root.path, TargetId::ClaudeCode).unwrap();
    let names: Vec<&str> = receipt.skill_entries().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["code-review"]);
    assert!(!has_journal(&root.path, None));
    assert!(!has_journal(&root.path, Some(TargetId::ClaudeCode)));
}

#[test]
fn reconcile_and_restore_edits_in_apply_edit() {
    let f = fx();
    let before = tsel(&[], &["fx:sdlc/code-review"], &[]);
    let bundles = f.bundles();
    let (out, changed) = apply_edit(&before, &Edit::Reconcile, &f.catalog, &bundles).unwrap();
    assert_eq!((out, changed), (before.clone(), false));
    let other = tsel(&["b1"], &[], &[]);
    let (out, changed) =
        apply_edit(&before, &Edit::Restore(other.clone()), &f.catalog, &bundles).unwrap();
    assert_eq!((out, changed), (other.clone(), true));
    let (_, changed) =
        apply_edit(&other, &Edit::Restore(other.clone()), &f.catalog, &bundles).unwrap();
    assert!(!changed);
}
