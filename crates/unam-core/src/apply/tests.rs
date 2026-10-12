//! Unit tests of `apply` (M3 plan, apply.rs specification). Tempdir
//! projects; a local failing `FsOps` wrapper counts the three destination
//! calls (`symlink_dir`, `copy_tree`, `remove_owned`).

use super::*;
use crate::adapter::claude_code::ClaudeCode;
use crate::catalog::{Catalog, LoadOptions};
use crate::config::{SourceEntry, Strategy};
use crate::fsops::RealFs;
use crate::observe::observe;
use crate::plan::{DiffInput, PlanHeader, diff};
use crate::project::{RootDetection, TargetId};
use crate::resolve::{DesiredEntry, Pin, Provenance};
use crate::state::{HistoryEntry, JournalRead};
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};

// ---- clock ----

struct StepClock(Cell<u64>);
impl StepClock {
    fn new() -> Self {
        StepClock(Cell::new(1_000))
    }
}
impl Clock for StepClock {
    fn now_rfc3339(&self) -> String {
        let v = self.0.get();
        self.0.set(v + 1);
        format!("2026-10-05T00:00:{:02}.{v:03}Z", v % 60)
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

// ---- failing wrapper ----

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    None,
    FailBefore(u32),
    FailAfter(u32),
    CrashBefore(u32),
    CrashAfter(u32),
}

struct Faulty {
    mode: Mode,
    count: Cell<u32>,
    /// Every mutating call (method name), for ordering checks.
    log: RefCell<Vec<String>>,
    /// Panic inside `create_dir` (G6c row).
    crash_create_dir: bool,
}

impl Faulty {
    fn new(mode: Mode) -> Self {
        Faulty {
            mode,
            count: Cell::new(0),
            log: RefCell::new(Vec::new()),
            crash_create_dir: false,
        }
    }

    fn dest<T>(&self, f: impl FnOnce() -> Result<T>) -> Result<T> {
        let n = self.count.get() + 1;
        self.count.set(n);
        match self.mode {
            Mode::FailBefore(k) if k == n => Err(Error::io("injected", std::io::Error::other("x"))),
            Mode::CrashBefore(k) if k == n => panic!("crash before {n}"),
            Mode::FailAfter(k) if k == n => {
                f()?;
                Err(Error::io("injected", std::io::Error::other("x")))
            }
            Mode::CrashAfter(k) if k == n => {
                let _ = f();
                panic!("crash after {n}")
            }
            _ => f(),
        }
    }

    fn note(&self, m: &str) {
        self.log.borrow_mut().push(m.to_string());
    }
}

impl FsOps for Faulty {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.note("write_state");
        RealFs.write_state(p, b)
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.note("append_state");
        RealFs.append_state(p, b)
    }
    fn remove_state(&self, p: &StatePath) -> Result<()> {
        self.note("remove_state");
        RealFs.remove_state(p)
    }
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
        self.note("remove_state_dir_if_empty");
        RealFs.remove_state_dir_if_empty(p)
    }
    fn create_dir(&self, d: &DestPath) -> Result<bool> {
        self.note("create_dir");
        if self.crash_create_dir {
            panic!("crash in create_dir");
        }
        RealFs.create_dir(d)
    }
    fn rmdir_if_empty(&self, d: &DestPath) -> Result<bool> {
        self.note("rmdir_if_empty");
        RealFs.rmdir_if_empty(d)
    }
    fn symlink_dir(&self, s: &SourcePath, d: &DestPath) -> Result<()> {
        self.note("symlink_dir");
        self.dest(|| RealFs.symlink_dir(s, d))
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        self.note("copy_tree");
        self.dest(|| RealFs.copy_tree(s, d, tag, h))
    }
    fn remove_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
    ) -> Result<RemoveOutcome> {
        self.note("remove_owned");
        self.dest(|| RealFs.remove_owned(d, e, t))
    }
    fn restore_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
        s: Option<&SourcePath>,
        tag: &str,
    ) -> Result<RestoreOutcome> {
        self.note("restore_owned");
        RealFs.restore_owned(d, e, t, s, tag)
    }
}

// ---- fixture ----

const REL: &str = ".claude/skills";

struct Fx {
    _t: tempfile::TempDir,
    root: PathBuf,
    lib: PathBuf,
    dirs: crate::paths::Dirs,
    catalog: Catalog,
    clock: StepClock,
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn fx() -> Fx {
    let t = tempfile::tempdir().unwrap();
    let lib = t.path().join("lib");
    for name in ["one", "two", "three", "four", "five"] {
        write(
            &lib.join(format!("a/{name}/SKILL.md")),
            &format!("---\nname: {name}\ndescription: Skill {name}.\n---\n"),
        );
        write(&lib.join(format!("a/{name}/notes.md")), name);
    }
    let root = t.path().join("proj");
    fs::create_dir_all(&root).unwrap();
    let root = fs::canonicalize(&root).unwrap();
    let dirs = crate::project::index::tests_support::dirs_in(t.path());
    let mut f = Fx {
        _t: t,
        root,
        lib,
        dirs,
        catalog: Catalog::default(),
        clock: StepClock::new(),
    };
    f.reload();
    f
}

impl Fx {
    fn reload(&mut self) {
        self.catalog = Catalog::load(
            &[SourceEntry {
                alias: "fx".into(),
                path: self.lib.to_string_lossy().into_owned(),
                precedence: 10,
            }],
            &self.dirs,
            LoadOptions {
                use_cache: false,
                git_global: false,
            },
        )
        .unwrap();
    }

    fn ctx(&self) -> ApplyCtx<'_> {
        ApplyCtx::new(&self.root, &ClaudeCode, Some(&self.catalog), &self.clock).unwrap()
    }

    fn state(&self) -> StateDir {
        StateDir::new(&self.root, TargetId::ClaudeCode)
    }

    fn receipt(&self) -> Receipt {
        Receipt::load(&self.state(), &self.root, TargetId::ClaudeCode).unwrap()
    }

    fn plan(&self, names: &[&str], strategy: Strategy, deactivate: bool) -> Plan {
        let desired: Vec<DesiredEntry> = names
            .iter()
            .map(|n| {
                let id = SkillId::parse(&format!("fx:a/{n}")).unwrap();
                DesiredEntry {
                    name: n.to_string(),
                    id,
                    strategy,
                    pin: Pin::Follow,
                    provenance: vec![Provenance::Adhoc],
                }
            })
            .collect();
        let receipt = self.receipt();
        let mut names_set: BTreeSet<String> = desired.iter().map(|d| d.name.clone()).collect();
        let mut hash_names = BTreeSet::new();
        for e in receipt.skill_entries() {
            names_set.insert(e.name.clone());
            if e.strategy == Strategy::Copy {
                hash_names.insert(e.name.clone());
            }
        }
        let observed = observe(&self.root, REL, &names_set, &hash_names, &[]).unwrap();
        diff(&DiffInput {
            header: PlanHeader {
                root: self.root.clone(),
                root_how: RootDetection::Explicit,
                target: TargetId::ClaudeCode,
                dest_rel: REL.into(),
                strategy,
                detection_text: None,
                consumers: vec![],
            },
            op_id: crate::state::new_op_id(&self.clock),
            desired: &desired,
            carry_ids: &BTreeSet::new(),
            receipt: &receipt,
            observed: &observed,
            catalog: &self.catalog,
            selection_hash: Hash::of_bytes(names.join(",").as_bytes()),
            budget_tokens: 100_000,
            release_created_dirs: deactivate,
            repoint_moved: false,
        })
    }

    fn prelude(&self) -> SelectionPrelude {
        SelectionPrelude {
            command: "add".into(),
            selection_before: SelectionBefore {
                existed: false,
                text: String::new(),
            },
            selection_after_text: String::new(),
            selection_after_hash: Hash::of_bytes(b"").to_string(),
        }
    }

    fn apply(&self, names: &[&str], strategy: Strategy, fs: &dyn FsOps) -> Result<ApplyReport> {
        let p = self.plan(names, strategy, false);
        execute(&self.ctx(), &p, &self.prelude(), fs)
    }

    fn dest(&self) -> PathBuf {
        self.root.join(REL)
    }

    fn journal_exists(&self) -> bool {
        self.state().journal().path().exists()
    }
}

/// `(rel path, kind, content)` of every entry under `root`.
fn tree(root: &Path) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            let m = fs::symlink_metadata(&p).unwrap();
            if m.file_type().is_symlink() {
                out.push((
                    rel,
                    "link".into(),
                    fs::read_link(&p).unwrap().to_string_lossy().into_owned(),
                ));
            } else if m.is_dir() {
                out.push((rel, "dir".into(), String::new()));
                stack.push(p);
            } else {
                out.push((
                    rel,
                    "file".into(),
                    String::from_utf8_lossy(&fs::read(&p).unwrap()).into_owned(),
                ));
            }
        }
    }
    out.sort();
    out
}

/// Replace the fixture's library path with `[LIB]`.
fn norm(f: &Fx, s: &str) -> String {
    let lib = fs::canonicalize(&f.lib).unwrap();
    s.replace(&*lib.to_string_lossy(), "[LIB]")
}

/// Skill entries with `applied_at` masked and paths normalized.
fn masked_entries(f: &Fx, r: &Receipt) -> Vec<String> {
    r.skill_entries()
        .map(|s| {
            let mut v = s.to_value();
            v["applied_at"] = Value::String("[TIME]".into());
            norm(f, &v.to_string())
        })
        .collect()
}

fn dest_listing(f: &Fx) -> Vec<(String, String, String)> {
    tree(&f.dest())
        .into_iter()
        .map(|(a, b, c)| (a, b, norm(f, &c)))
        .collect()
}

// ---- execute ----

#[test]
fn clean_apply_of_three_adds_writes_records_in_order() {
    let f = fx();
    let rec = RecordingFs::default();
    let r = f
        .apply(&["one", "two", "three"], Strategy::Symlink, &rec)
        .unwrap();
    assert_eq!(r.status, ApplyStatus::Applied);
    assert!(r.history_written);
    let kinds: Vec<String> = rec
        .lines
        .borrow()
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["t"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            "begin", "started", "dir", "dir", "done", "started", "done", "started", "done",
            "receipt"
        ]
    );
    assert_eq!(f.receipt().skill_entries().count(), 3);
    assert_eq!(
        fs::read_dir(f.state().history_dir().path())
            .unwrap()
            .count(),
        1
    );
    assert!(!f.journal_exists());
    assert_eq!(
        f.receipt().created_dirs,
        vec![".claude".to_string(), ".claude/skills".to_string()]
    );
    assert_eq!(r.refresh.kind, "command");
    assert_eq!(r.new_dirs.len(), 2);
}

/// Records every journal line written or appended.
#[derive(Default)]
struct RecordingFs {
    lines: RefCell<Vec<String>>,
}
impl RecordingFs {
    fn capture(&self, p: &StatePath, b: &[u8]) {
        if p.path().ends_with("journal.jsonl") {
            for l in String::from_utf8_lossy(b).lines() {
                self.lines.borrow_mut().push(l.to_string());
            }
        }
    }
}
impl FsOps for RecordingFs {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.capture(p, b);
        RealFs.write_state(p, b)
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.capture(p, b);
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
        RealFs.symlink_dir(s, d)
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        RealFs.copy_tree(s, d, tag, h)
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

#[test]
fn continue_past_failure_keeps_the_journal_and_writes_no_history() {
    let f = fx();
    let r = f
        .apply(
            &["one", "two", "three"],
            Strategy::Symlink,
            &Faulty::new(Mode::FailBefore(1)),
        )
        .unwrap();
    assert_eq!(r.status, ApplyStatus::Partial);
    assert!(!r.history_written);
    assert!(matches!(r.results[0].outcome, StepOutcome::Failed { .. }));
    assert_eq!(r.results[1].outcome, StepOutcome::Done);
    assert_eq!(r.results[2].outcome, StepOutcome::Done);
    assert_eq!(f.receipt().skill_entries().count(), 2);
    assert!(f.journal_exists());
    let JournalRead::Present(j) = read_journal(&f.state(), REL).unwrap() else {
        panic!()
    };
    assert_eq!(j.marker, Some(MarkerOutcome::Partial));
    assert!(!f.state().history_dir().path().exists());
}

#[test]
fn skip_only_run_is_finalized_as_applied_with_skips() {
    let f = fx();
    f.apply(&["one", "two"], Strategy::Copy, &RealFs).unwrap();
    let p = f.plan(&["two"], Strategy::Copy, false);
    // The copy is edited after planning: the Remove is skipped, not deleted.
    fs::write(f.dest().join("one/notes.md"), b"edited").unwrap();
    let r = execute(&f.ctx(), &p, &f.prelude(), &RealFs).unwrap();
    assert_eq!(r.status, ApplyStatus::AppliedWithSkips);
    assert_eq!(
        r.results[0].outcome,
        StepOutcome::Skipped(SkipReason::Modified)
    );
    assert_eq!(fs::read(f.dest().join("one/notes.md")).unwrap(), b"edited");
    assert!(!f.journal_exists());
    assert!(r.history_written);
    // Ownership released.
    assert!(f.receipt().skill_entries().all(|s| s.name != "one"));
}

#[cfg(unix)]
#[test]
fn dest_occupied_and_skipped_remove_makes_the_same_name_add_skip() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    fs::create_dir_all(f.dest().join("one")).unwrap();
    let r = execute(&f.ctx(), &p, &f.prelude(), &RealFs).unwrap();
    assert_eq!(
        r.results[0].outcome,
        StepOutcome::Skipped(SkipReason::DestOccupied)
    );
    assert!(fs::read_dir(f.dest().join("one")).unwrap().next().is_none());
    // Strategy change on a retargeted link: Remove skipped, Add occupied.
    let g = fx();
    g.apply(&["one"], Strategy::Symlink, &RealFs).unwrap();
    let p = g.plan(&["one"], Strategy::Copy, false);
    fs::remove_file(g.dest().join("one")).unwrap();
    std::os::unix::fs::symlink(&g.lib, g.dest().join("one")).unwrap();
    let r = execute(&g.ctx(), &p, &g.prelude(), &RealFs).unwrap();
    assert_eq!(
        r.results[0].outcome,
        StepOutcome::Skipped(SkipReason::Modified)
    );
    assert_eq!(
        r.results[1].outcome,
        StepOutcome::Skipped(SkipReason::DestOccupied)
    );
    assert_eq!(fs::read_link(g.dest().join("one")).unwrap(), g.lib);
}

// ---- the matrix ----

/// Start: `one` applied (symlink). Change: remove `one`, add two, three,
/// four: four destination calls.
fn matrix_start() -> Fx {
    let f = fx();
    f.apply(&["one"], Strategy::Symlink, &RealFs).unwrap();
    f
}

fn run_injected(f: &Fx, mode: Mode) {
    let p = f.plan(&["two", "three", "four"], Strategy::Symlink, false);
    assert_eq!(p.steps.len(), 4);
    let faulty = Faulty::new(mode);
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
}

#[test]
fn every_mode_at_every_step_then_resume_or_rollback_is_consistent() {
    let clean = matrix_start();
    run_injected(&clean, Mode::None);
    let clean_dest = dest_listing(&clean);
    let clean_receipt = masked_entries(&clean, &clean.receipt());
    assert!(!clean.journal_exists());
    for k in 1..=4 {
        for mode in [
            Mode::FailBefore(k),
            Mode::FailAfter(k),
            Mode::CrashBefore(k),
            Mode::CrashAfter(k),
        ] {
            // Resume.
            let f = matrix_start();
            run_injected(&f, mode);
            assert!(f.journal_exists(), "{mode:?}");
            let r = resume(&f.ctx(), &RealFs).unwrap();
            assert_ne!(r.status, ApplyStatus::Partial, "{mode:?}: {:?}", r.results);
            assert_eq!(dest_listing(&f), clean_dest, "resume {mode:?}");
            assert_eq!(
                masked_entries(&f, &f.receipt()),
                clean_receipt,
                "resume {mode:?}"
            );
            assert!(!f.journal_exists());
            // Rollback.
            let g = matrix_start();
            let g_start = tree(&g.root);
            run_injected(&g, mode);
            let r = rollback(&g.ctx(), &RealFs).unwrap();
            assert_eq!(r.status, ApplyStatus::Applied, "{mode:?}: {:?}", r.warnings);
            assert_eq!(tree(&g.root), g_start, "rollback {mode:?}");
        }
    }
}

// ---- inspect ----

#[test]
fn inspect_rows() {
    let f = fx();
    let ctx = f.ctx();
    assert_eq!(
        inspect(&ctx, &RealFs).unwrap(),
        Inspection::Clear { notes: vec![] }
    );
    // Torn begin is discarded.
    write(f.state().journal().path(), "{\"t\":\"beg");
    let Inspection::Clear { notes } = inspect(&ctx, &RealFs).unwrap() else {
        panic!()
    };
    assert_eq!(notes.len(), 1);
    assert!(!f.journal_exists());
    // A partial journal is interrupted.
    f.apply(
        &["one", "two"],
        Strategy::Symlink,
        &Faulty::new(Mode::FailBefore(1)),
    )
    .unwrap();
    assert!(matches!(
        inspect(&ctx, &RealFs).unwrap(),
        Inspection::Interrupted(_)
    ));
}

#[test]
fn inspect_finalizes_a_completed_apply_left_behind() {
    // Crash after the receipt write, before the marker: the receipt holds
    // last_op_id; inspect finalizes and writes history once.
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    let faulty = FinalCrash::default();
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    assert!(f.journal_exists());
    assert_eq!(f.receipt().last_op_id.as_deref(), Some(p.op_id.as_str()));
    let hist = f.state().history_file(&p.op_id).unwrap();
    assert!(
        hist.path().exists(),
        "history is written before the receipt"
    );
    fs::remove_file(hist.path()).unwrap();
    let Inspection::Clear { notes } = inspect(&ctx, &RealFs).unwrap() else {
        panic!()
    };
    assert!(notes[0].contains("finalized"));
    assert!(!f.journal_exists());
    assert!(hist.path().exists());
    let entry: HistoryEntry = serde_json::from_slice(&fs::read(hist.path()).unwrap()).unwrap();
    assert_eq!(entry.op_id, p.op_id);
    assert_eq!(
        fs::read_dir(f.state().history_dir().path())
            .unwrap()
            .count(),
        1
    );
}

/// Panics on the append that follows the receipt write.
#[derive(Default)]
struct FinalCrash {
    receipt_written: Cell<bool>,
}
impl FsOps for FinalCrash {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        RealFs.write_state(p, b)?;
        if p.path().ends_with("applied.json") {
            self.receipt_written.set(true);
        }
        Ok(())
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        if self.receipt_written.get() {
            panic!("crash before the marker");
        }
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
        RealFs.symlink_dir(s, d)
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        RealFs.copy_tree(s, d, tag, h)
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

// ---- resume ----

#[test]
fn resume_re_derives_the_source_from_the_live_catalog() {
    let mut f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    {
        let ctx = f.ctx();
        let prelude = f.prelude();
        let faulty = Faulty::new(Mode::CrashBefore(1));
        let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    }
    // The skill is renamed in the source.
    write(
        &f.lib.join("a/one/SKILL.md"),
        "---\nname: renamed\ndescription: x\n---\n",
    );
    f.reload();
    let r = resume(&f.ctx(), &RealFs).unwrap();
    assert_eq!(r.status, ApplyStatus::Partial);
    assert!(
        matches!(&r.results[0].outcome, StepOutcome::Failed { code, .. } if code == "source-changed")
    );
    assert!(fs::symlink_metadata(f.dest().join("one")).is_err());
}

#[test]
fn resume_refuses_an_invalid_journal_and_touches_nothing() {
    let f = fx();
    let canary = f.dest().join("x");
    write(&canary.join("SKILL.md"), "mine");
    let mut b = serde_json::json!({
        "t": "begin", "version": 1, "op_id": "0000000000001-0000abcd", "command": "add",
        "target": "claude-code", "project_root": f.root.to_string_lossy(), "started_at": "s",
        "dest_rel": REL, "dest_dirs_planned": [],
        "selection_before": {"existed": false, "text": ""},
        "selection_after_text": "", "selection_after_hash": "h", "receipt_before": null,
        "plan": [{"op": "remove", "n": 1, "id": "fx:a/x", "name": "../x",
                  "expect": {"strategy": "symlink", "source": "/nonexistent"}}]
    });
    write(f.state().journal().path(), &format!("{b}\n"));
    let before = tree(&f.root);
    assert_eq!(
        resume(&f.ctx(), &RealFs).unwrap_err().code(),
        "journal-invalid"
    );
    assert_eq!(
        rollback(&f.ctx(), &RealFs).unwrap_err().code(),
        "journal-invalid"
    );
    assert_eq!(tree(&f.root), before);
    b["plan"][0]["name"] = "x".into();
    b["project_root"] = "/elsewhere".into();
    write(f.state().journal().path(), &format!("{b}\n"));
    assert_eq!(
        resume(&f.ctx(), &RealFs).unwrap_err().code(),
        "journal-invalid"
    );
    assert_eq!(fs::read(canary.join("SKILL.md")).unwrap(), b"mine");
}

#[test]
fn resume_rewrites_a_torn_tail_before_appending() {
    let f = fx();
    let p = f.plan(&["one", "two"], Strategy::Symlink, false);
    {
        let ctx = f.ctx();
        let prelude = f.prelude();
        let faulty = Faulty::new(Mode::CrashAfter(1));
        let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    }
    let j = f.state().journal();
    let mut text = fs::read_to_string(j.path()).unwrap();
    text.push_str("{\"t\":\"do");
    fs::write(j.path(), &text).unwrap();
    let r = resume(&f.ctx(), &RealFs).unwrap();
    assert_eq!(r.status, ApplyStatus::Applied);
    assert_eq!(f.receipt().skill_entries().count(), 2);
}

#[test]
fn resume_rewrites_a_journal_without_a_trailing_newline() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    {
        let ctx = f.ctx();
        let prelude = f.prelude();
        let faulty = Faulty::new(Mode::CrashBefore(1));
        let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    }
    let j = f.state().journal();
    let text = fs::read_to_string(j.path()).unwrap();
    fs::write(j.path(), text.trim_end()).unwrap();
    let rec = RecordingFs::default();
    resume(&f.ctx(), &rec).unwrap();
    assert!(!f.journal_exists());
    // Every captured line parsed as one record (no merged lines).
    for l in rec.lines.borrow().iter() {
        let v: Value = serde_json::from_str(l).unwrap();
        assert!(v.get("t").is_some());
    }
}

#[test]
fn resume_of_an_interrupted_remove_dir_counts_a_missing_directory_as_removed() {
    let f = fx();
    f.apply(&["one"], Strategy::Symlink, &RealFs).unwrap();
    // deactivate: remove one, then remove_dir skills, remove_dir .claude.
    let p = f.plan(&[], Strategy::Symlink, true);
    assert_eq!(
        p.steps.iter().map(Step::op).collect::<Vec<_>>(),
        vec!["remove", "remove_dir", "remove_dir"]
    );
    {
        let ctx = f.ctx();
        let prelude = f.prelude();
        let faulty = CrashOnRmdir::default();
        let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    }
    // The first rmdir happened, then the crash; resume finishes.
    assert!(!f.dest().exists());
    let r = resume(&f.ctx(), &RealFs).unwrap();
    assert_eq!(r.status, ApplyStatus::Applied);
    assert!(!f.root.join(".claude").exists());
    assert!(f.receipt().created_dirs.is_empty());
}

/// Performs the first `rmdir_if_empty`, then panics.
#[derive(Default)]
struct CrashOnRmdir {
    done: Cell<bool>,
}
impl FsOps for CrashOnRmdir {
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
        if self.done.get() {
            panic!("crash");
        }
        let r = RealFs.rmdir_if_empty(d);
        self.done.set(true);
        if r.is_ok() {
            panic!("crash after rmdir");
        }
        r
    }
    fn symlink_dir(&self, s: &SourcePath, d: &DestPath) -> Result<()> {
        RealFs.symlink_dir(s, d)
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        RealFs.copy_tree(s, d, tag, h)
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

// ---- rollback ----

#[test]
fn rollback_can_be_repeated_after_a_crash_during_rollback() {
    let f = matrix_start();
    let start = tree(&f.root);
    run_injected(&f, Mode::CrashAfter(3));
    {
        let ctx = f.ctx();
        let faulty = Faulty::new(Mode::CrashBefore(1));
        let _ = catch_unwind(AssertUnwindSafe(|| rollback(&ctx, &faulty)));
    }
    assert!(f.journal_exists());
    rollback(&f.ctx(), &RealFs).unwrap();
    assert_eq!(tree(&f.root), start);
}

#[test]
fn stale_temp_cleanup_removes_only_this_operation() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Copy, false);
    {
        let ctx = f.ctx();
        let prelude = f.prelude();
        let faulty = Faulty::new(Mode::CrashBefore(1));
        let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    }
    let mine = f.dest().join(format!(".unam-tmp-{}-one", p.op_id));
    let other = f.dest().join(".unam-tmp-0000000000009-0000abcd-one");
    fs::create_dir_all(&mine).unwrap();
    fs::create_dir_all(&other).unwrap();
    rollback(&f.ctx(), &RealFs).unwrap();
    assert!(!mine.exists());
    assert!(other.exists());
}

#[test]
fn rollback_restores_the_selection_only_when_its_hash_still_matches() {
    let f = fx();
    let sel = StatePath::new(crate::project::selection_path(&f.root));
    for edited in [false, true] {
        let p = f.plan(&["one"], Strategy::Symlink, false);
        fs::create_dir_all(sel.path().parent().unwrap()).unwrap();
        fs::write(sel.path(), b"after").unwrap();
        let prelude = SelectionPrelude {
            command: "add".into(),
            selection_before: SelectionBefore {
                existed: true,
                text: "before".into(),
            },
            selection_after_text: "after".into(),
            selection_after_hash: Hash::of_bytes(b"after").to_string(),
        };
        {
            let ctx = f.ctx();
            let faulty = Faulty::new(Mode::CrashAfter(1));
            let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
        }
        if edited {
            fs::write(sel.path(), b"hand edit").unwrap();
        }
        let r = rollback(&f.ctx(), &RealFs).unwrap();
        if edited {
            assert_eq!(fs::read(sel.path()).unwrap(), b"hand edit");
            assert!(
                r.warnings
                    .iter()
                    .any(|w| w.code == "selection-not-restored")
            );
        } else {
            assert_eq!(fs::read(sel.path()).unwrap(), b"before");
            assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        }
    }
}

#[test]
fn rollback_removes_the_history_entry_of_the_undone_apply() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        execute(&ctx, &p, &prelude, &FinalCrash::default())
    }));
    let hist = f.state().history_file(&p.op_id).unwrap();
    assert!(hist.path().exists());
    rollback(&ctx, &RealFs).unwrap();
    assert!(!hist.path().exists());
    assert!(!f.root.join(".claude").exists());
    assert!(!f.state().dir().path().exists());
    assert!(f.root.join(".unam").exists());
}

#[test]
fn rollback_ends_with_empty_state_directories_removed_bottom_up() {
    let f = matrix_start();
    run_injected(&f, Mode::CrashAfter(1));
    let trash = f.state().trash_root();
    fs::create_dir_all(trash.path().join("other")).unwrap();
    fs::write(trash.path().join("other/planted"), b"x").unwrap();
    rollback(&f.ctx(), &RealFs).unwrap();
    assert!(trash.path().join("other/planted").exists());
    assert!(
        f.state().history_dir().path().exists(),
        "non-empty history stays"
    );
    assert!(f.root.join(".unam").exists());
}

#[test]
fn execute_resume_rollback_call_ensure_dest_root_first() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    fs::write(f.root.join(".claude"), b"a file").unwrap();
    let faulty = Faulty::new(Mode::None);
    let e = execute(&f.ctx(), &p, &f.prelude(), &faulty).unwrap_err();
    assert_eq!(e.code(), "dest-not-directory");
    assert!(faulty.log.borrow().is_empty(), "{:?}", faulty.log.borrow());
    // A journal left behind, then the root swapped outside.
    fs::remove_file(f.root.join(".claude")).unwrap();
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        execute(&ctx, &p, &prelude, &Faulty::new(Mode::CrashBefore(1)))
    }));
    #[cfg(unix)]
    {
        let outside = f._t.path().join("outside");
        fs::create_dir_all(outside.join("skills")).unwrap();
        fs::remove_dir_all(f.root.join(".claude")).unwrap();
        std::os::unix::fs::symlink(&outside, f.root.join(".claude")).unwrap();
        for which in ["resume", "rollback"] {
            let faulty = Faulty::new(Mode::None);
            let e = if which == "resume" {
                resume(&f.ctx(), &faulty).unwrap_err()
            } else {
                rollback(&f.ctx(), &faulty).unwrap_err()
            };
            assert_eq!(e.code(), "dest-outside-project", "{which}");
            assert!(faulty.log.borrow().is_empty(), "{which}");
        }
    }
}

#[test]
fn dir_record_is_written_before_create_dir() {
    let f = fx();
    let p = f.plan(&["one"], Strategy::Symlink, false);
    let mut faulty = Faulty::new(Mode::None);
    faulty.crash_create_dir = true;
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| execute(&ctx, &p, &prelude, &faulty)));
    let JournalRead::Present(j) = read_journal(&f.state(), REL).unwrap() else {
        panic!()
    };
    assert_eq!(j.dirs_created, vec![".claude".to_string()]);
    rollback(&ctx, &RealFs).unwrap();
    assert!(!f.root.join(".claude").exists());
    assert!(!f.state().dir().path().exists());
    // An existing directory is recorded as skipped (not created).
    let g = fx();
    fs::create_dir_all(g.root.join(".claude")).unwrap();
    let p = g.plan(&["one"], Strategy::Symlink, false);
    let rec = RecordingFs::default();
    execute(&g.ctx(), &p, &g.prelude(), &rec).unwrap();
    let kinds: Vec<String> = rec
        .lines
        .borrow()
        .iter()
        .map(|l| {
            serde_json::from_str::<Value>(l).unwrap()["t"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert_eq!(kinds[1], "started");
    assert_eq!(kinds[2], "dir");
    assert_eq!(g.receipt().created_dirs, vec![".claude/skills".to_string()]);
}

#[test]
fn remove_dir_steps_run_in_reverse_creation_order_and_keep_non_empty_directories() {
    let f = fx();
    f.apply(&["one"], Strategy::Symlink, &RealFs).unwrap();
    write(&f.root.join(".claude/settings.json"), "{}");
    let p = f.plan(&[], Strategy::Symlink, true);
    let r = execute(&f.ctx(), &p, &f.prelude(), &RealFs).unwrap();
    assert_eq!(r.status, ApplyStatus::Applied);
    assert!(!f.dest().exists());
    assert!(f.root.join(".claude/settings.json").exists());
    assert!(r.warnings.iter().any(|w| w.code == "created-dir-kept"));
    assert_eq!(f.receipt().created_dirs, vec![".claude".to_string()]);
    assert_eq!(r.removed_dirs, vec![".claude/skills".to_string()]);
}

#[test]
fn rollback_recreates_removed_created_directories_before_restoring_skills() {
    let f = fx();
    f.apply(&["one"], Strategy::Copy, &RealFs).unwrap();
    let start = tree(&f.root);
    let p = f.plan(&[], Strategy::Copy, true);
    // Crash after the last rmdir (the journal stays).
    let ctx = f.ctx();
    let prelude = f.prelude();
    let _ = catch_unwind(AssertUnwindSafe(|| {
        execute(&ctx, &p, &prelude, &FinalCrash::default())
    }));
    assert!(!f.root.join(".claude").exists());
    rollback(&ctx, &RealFs).unwrap();
    assert_eq!(tree(&f.root), start);
}

#[test]
fn history_prune_keeps_twenty() {
    let f = fx();
    for i in 0..21u64 {
        write_history(
            &RealFs,
            &f.state(),
            &HistoryEntry {
                version: 1,
                op_id: format!("{:013}-0000abcd", i + 1),
                command: "add".into(),
                target: "claude-code".into(),
                started_at: String::new(),
                finished_at: String::new(),
                outcome: "applied".into(),
                selection_before: SelectionBefore {
                    existed: false,
                    text: String::new(),
                },
                selection_after: SelectionAfter {
                    text: String::new(),
                },
                receipt_before: Value::Null,
                receipt_after: Value::Null,
            },
        )
        .unwrap();
    }
    f.apply(&["one"], Strategy::Symlink, &RealFs).unwrap();
    assert_eq!(
        fs::read_dir(f.state().history_dir().path())
            .unwrap()
            .count(),
        20
    );
}

#[test]
fn created_dirs_are_recorded_and_removed_only_when_empty() {
    let f = fx();
    f.apply(&["one", "two"], Strategy::Symlink, &RealFs)
        .unwrap();
    assert_eq!(f.receipt().created_dirs.len(), 2);
    write(&f.dest().join("house-style/SKILL.md"), "mine");
    let p = f.plan(&[], Strategy::Symlink, true);
    execute(&f.ctx(), &p, &f.prelude(), &RealFs).unwrap();
    assert!(f.dest().join("house-style/SKILL.md").exists());
    assert_eq!(f.receipt().created_dirs.len(), 2, "kept, not empty");
}

#[test]
fn journal_step_conversions_round_trip() {
    let f = fx();
    f.apply(&["one"], Strategy::Copy, &RealFs).unwrap();
    let p = f.plan(&["two"], Strategy::Copy, false);
    for s in &p.steps {
        assert_eq!(&step_from_journal(&journal_step(s)), s);
    }
}
