//! Unit tests of `state` (M3 plan, state.rs specification).

use super::*;
use crate::fsops::RealFs;
use std::fs;

const OP: &str = "0000000000001-0000abcd";

struct FixedClock;
impl Clock for FixedClock {
    fn now_rfc3339(&self) -> String {
        "2026-10-05T00:00:00Z".into()
    }
    fn now_millis(&self) -> u64 {
        1
    }
    fn entropy(&self) -> u32 {
        0xabcd
    }
}

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn canon(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap()
}

fn sd(root: &Path) -> StateDir {
    StateDir::new(root, TargetId::ClaudeCode)
}

// ---- op ids, state dir ----

#[test]
fn op_id_format_and_validation() {
    assert_eq!(new_op_id(&FixedClock), OP);
    assert!(valid_op_id(OP));
    assert!(valid_op_id(&new_op_id(&SystemClock)));
    for bad in [
        "../x",
        "0000000000001-0000ABCD",
        "000000000001-0000abcd",
        "0000000000001/0000abcd",
        "0000000000001-0000abc",
        "0000000000001-0000abcg",
        "",
        "0000000000001-0000abcd/..",
    ] {
        assert!(!valid_op_id(bad), "{bad:?}");
    }
}

#[test]
fn state_dir_paths_and_bad_op_id() {
    let s = sd(Path::new("/p"));
    assert_eq!(s.dir().path(), Path::new("/p/.unam/claude-code"));
    assert_eq!(
        s.applied().path(),
        Path::new("/p/.unam/claude-code/applied.json")
    );
    assert_eq!(
        s.journal().path(),
        Path::new("/p/.unam/claude-code/journal.jsonl")
    );
    assert_eq!(
        s.history_file(OP).unwrap().path(),
        Path::new("/p/.unam/claude-code/history/0000000000001-0000abcd.json")
    );
    assert_eq!(
        s.trash_dir(OP).unwrap().path(),
        Path::new("/p/.unam/claude-code/trash/0000000000001-0000abcd")
    );
    assert_eq!(
        lock_path(Path::new("/p")).path(),
        Path::new("/p/.unam/lock")
    );
    assert_eq!(
        s.history_file("../x").unwrap_err().code(),
        "journal-invalid"
    );
    assert_eq!(s.trash_dir("../x").unwrap_err().code(), "journal-invalid");
}

// ---- layout ----

#[cfg(unix)]
#[test]
fn check_state_layout_rows() {
    use std::os::unix::fs::symlink;
    let t = tmp();
    let outside = t.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("f"), b"x").unwrap();
    let fresh = |name: &str| {
        let p = t.path().join(name);
        fs::create_dir_all(&p).unwrap();
        p
    };
    // Absent and correct kinds are accepted.
    let ok = fresh("ok");
    check_state_layout(&ok, Some("claude-code")).unwrap();
    fs::create_dir_all(ok.join(".unam/claude-code/history")).unwrap();
    fs::create_dir_all(ok.join(".unam/claude-code/trash")).unwrap();
    fs::write(ok.join(".unam/lock"), b"").unwrap();
    fs::write(ok.join(".unam/claude-code/applied.json"), b"{}").unwrap();
    fs::write(ok.join(".unam/claude-code/journal.jsonl"), b"").unwrap();
    check_state_layout(&ok, Some("claude-code")).unwrap();

    let refused = |root: &Path| {
        let e = check_state_layout(root, Some("claude-code")).unwrap_err();
        assert_eq!(e.code(), "state-dir-invalid", "{}", root.display());
    };
    let a = fresh("a");
    symlink(&outside, a.join(".unam")).unwrap();
    refused(&a);
    let b = fresh("b");
    fs::create_dir_all(b.join(".unam")).unwrap();
    symlink(&outside, b.join(".unam/claude-code")).unwrap();
    refused(&b);
    for (name, sub) in [("c", "history"), ("d", "trash")] {
        let r = fresh(name);
        fs::create_dir_all(r.join(".unam/claude-code")).unwrap();
        symlink(&outside, r.join(".unam/claude-code").join(sub)).unwrap();
        refused(&r);
    }
    for (name, file) in [("e", "applied.json"), ("f", "journal.jsonl")] {
        let r = fresh(name);
        fs::create_dir_all(r.join(".unam/claude-code")).unwrap();
        symlink(outside.join("f"), r.join(".unam/claude-code").join(file)).unwrap();
        refused(&r);
        let r2 = fresh(&format!("{name}2"));
        fs::create_dir_all(r2.join(".unam/claude-code").join(file)).unwrap();
        refused(&r2);
    }
    let g = fresh("g");
    fs::create_dir_all(g.join(".unam/lock")).unwrap();
    refused(&g);
}

// ---- lock ----

#[test]
fn lock_acquire_contend_release() {
    let t = tmp();
    let root = t.path();
    let first = ProjectLock::acquire(root, "add", &FixedClock).unwrap();
    let line = read_lock_line(root).unwrap();
    let v: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["pid"], std::process::id());
    assert_eq!(v["command"], "add");
    assert!(line.ends_with('\n') && line.lines().count() == 1);
    let e = ProjectLock::acquire(root, "rm", &FixedClock).unwrap_err();
    assert_eq!(e.code(), "locked");
    assert!(matches!(e, Error::Locked { pid: Some(p) } if p == std::process::id()));
    assert_eq!(
        read_lock_line(root).unwrap(),
        line,
        "contender never writes"
    );
    drop(first);
    let again = ProjectLock::acquire(root, "use", &FixedClock).unwrap();
    assert!(read_lock_line(root).unwrap().contains("\"use\""));
    drop(again);
    assert!(lock_path(root).path().is_file(), "never deleted");
}

#[test]
fn lock_with_garbage_content_gives_no_pid() {
    let t = tmp();
    let root = t.path();
    let held = ProjectLock::acquire(root, "add", &FixedClock).unwrap();
    // Overwrite the content in place (same inode) with garbage.
    fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(lock_path(root).path())
        .unwrap()
        .write_all(b"garbage")
        .unwrap();
    let e = ProjectLock::acquire(root, "rm", &FixedClock).unwrap_err();
    assert!(matches!(e, Error::Locked { pid: None }));
    drop(held);
}

#[cfg(unix)]
#[test]
fn lock_refuses_symlinks_and_a_symlinked_unam() {
    use std::os::unix::fs::symlink;
    let t = tmp();
    let victim = t.path().join("victim");
    fs::write(&victim, b"precious").unwrap();
    let a = t.path().join("a");
    fs::create_dir_all(a.join(".unam")).unwrap();
    symlink(&victim, a.join(".unam/lock")).unwrap();
    assert_eq!(
        ProjectLock::acquire(&a, "add", &FixedClock)
            .unwrap_err()
            .code(),
        "state-dir-invalid"
    );
    assert_eq!(fs::read(&victim).unwrap(), b"precious");
    let outside = t.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    let b = t.path().join("b");
    fs::create_dir_all(b.join(".unam")).unwrap();
    symlink(outside.join("created"), b.join(".unam/lock")).unwrap();
    assert_eq!(
        ProjectLock::acquire(&b, "add", &FixedClock)
            .unwrap_err()
            .code(),
        "state-dir-invalid"
    );
    assert!(!outside.join("created").exists());
    let c = t.path().join("c");
    fs::create_dir_all(&c).unwrap();
    symlink(&outside, c.join(".unam")).unwrap();
    assert_eq!(
        ProjectLock::acquire(&c, "add", &FixedClock)
            .unwrap_err()
            .code(),
        "state-dir-invalid"
    );
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

// ---- selection hash ----

#[test]
fn selection_hash_stable_and_sensitive() {
    let base = TargetSelection {
        strategy: None,
        bundles: vec!["a".into(), "b".into()],
        add: vec!["fx:x/y".into()],
        remove: vec![],
    };
    let h = selection_hash(Strategy::Symlink, &base);
    assert_eq!(h, selection_hash(Strategy::Symlink, &base));
    assert!(h.as_str().starts_with("blake3:"));
    assert_ne!(h, selection_hash(Strategy::Copy, &base));
    let mut swapped = base.clone();
    swapped.bundles.reverse();
    assert_ne!(h, selection_hash(Strategy::Symlink, &swapped));
    let mut more = base.clone();
    more.add.push("fx:z".into());
    assert_ne!(h, selection_hash(Strategy::Symlink, &more));
    let mut rm = base.clone();
    rm.remove.push("fx:x/y".into());
    assert_ne!(h, selection_hash(Strategy::Symlink, &rm));
}

// ---- receipt ----

fn sample_receipt_text() -> String {
    let v = json!({
        "version": 1,
        "project_root": "/p",
        "target": "claude-code",
        "applied_at": "2026-10-05T00:00:00Z",
        "selection_hash": "blake3:00",
        "last_op_id": OP,
        "created_dirs": [".claude", ".claude/skills"],
        "entries": [
            {
                "kind": "skill",
                "skill_id": "fx:sdlc/code-review",
                "name": "code-review",
                "dest": ".claude/skills/code-review",
                "strategy": "symlink",
                "source_path": "/lib/sdlc/code-review",
                "tree_hash": "blake3:11",
                "git_commit": null,
                "pin": "follow",
                "provenance": [{"bundle": "a"}, {"adhoc": true}],
                "applied_at": "2026-10-05T00:00:00Z",
                "future_field": {"x": 1}
            },
            {"kind": "generated", "path": "AGENTS.md", "zeta": 1, "alpha": 2}
        ],
        "future_top": [1, 2]
    });
    let mut s = serde_json::to_string_pretty(&v).unwrap();
    s.push('\n');
    s
}

#[test]
fn receipt_round_trip_preserves_unknown_fields_and_other_entries() {
    let text = sample_receipt_text();
    let r = Receipt::parse(&text, Path::new("/x")).unwrap();
    assert_eq!(r.to_text(), text);
    assert_eq!(r.skill_entries().count(), 1);
    assert!(matches!(&r.entries[1], Entry::Other(_)));
    let s = r.skill_entries().next().unwrap();
    assert!(s.consistent(".claude/skills"));
    assert!(!s.consistent(".other"));
}

#[test]
fn receipt_rejections() {
    let p = Path::new("/x");
    assert_eq!(Receipt::parse("", p).unwrap_err().code(), "receipt-invalid");
    assert_eq!(
        Receipt::parse("{not", p).unwrap_err().code(),
        "receipt-invalid"
    );
    let v2 = sample_receipt_text().replace("\"version\": 1", "\"version\": 2");
    assert_eq!(
        Receipt::parse(&v2, p).unwrap_err().code(),
        "receipt-version-unsupported"
    );
    let missing = sample_receipt_text().replace("\"pin\": \"follow\",", "");
    assert_eq!(
        Receipt::parse(&missing, p).unwrap_err().code(),
        "receipt-invalid"
    );
}

#[test]
fn receipt_with_a_foreign_target_is_receipt_invalid() {
    let t = tmp();
    let s = sd(t.path());
    fs::create_dir_all(s.dir().path()).unwrap();
    let text = sample_receipt_text().replace("\"claude-code\"", "\"agents-neutral\"");
    fs::write(s.applied().path(), text).unwrap();
    let e = Receipt::load(&s, t.path(), TargetId::ClaudeCode).unwrap_err();
    assert_eq!(e.code(), "receipt-invalid");
}

#[test]
fn receipt_from_a_moved_project_loads_with_a_root_moved_warning() {
    let t = tmp();
    let s = sd(t.path());
    fs::create_dir_all(s.dir().path()).unwrap();
    fs::write(s.applied().path(), sample_receipt_text()).unwrap();
    let mut r = Receipt::load(&s, t.path(), TargetId::ClaudeCode).unwrap();
    assert!(r.root_moved(t.path()));
    // The next receipt written carries the current root.
    r.project_root = t.path().to_string_lossy().into_owned();
    assert!(!r.root_moved(t.path()));
    assert!(r.to_text().contains(&*t.path().to_string_lossy()));
    // Absent gives an empty receipt.
    let empty = Receipt::load(&sd(&t.path().join("none")), t.path(), TargetId::ClaudeCode).unwrap();
    assert!(empty.entries.is_empty());
    assert_eq!(empty.last_op_id, None);
}

// ---- history ----

fn hist(op_id: &str) -> HistoryEntry {
    HistoryEntry {
        version: 1,
        op_id: op_id.into(),
        command: "add".into(),
        target: "claude-code".into(),
        started_at: "s".into(),
        finished_at: "f".into(),
        outcome: "applied".into(),
        selection_before: SelectionBefore {
            existed: false,
            text: String::new(),
        },
        selection_after: SelectionAfter { text: "x".into() },
        receipt_before: Value::Null,
        receipt_after: json!({}),
    }
}

#[test]
fn history_write_and_prune() {
    let t = tmp();
    let s = sd(t.path());
    for i in 0..25u64 {
        write_history(&RealFs, &s, &hist(&format!("{:013}-0000abcd", i + 1))).unwrap();
    }
    fs::write(s.history_dir().path().join("notes.txt"), b"mine").unwrap();
    fs::create_dir_all(s.history_dir().path().join("sub")).unwrap();
    assert_eq!(prune_history(&RealFs, &s, 20).unwrap(), 5);
    let mut names: Vec<String> = fs::read_dir(s.history_dir().path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 22);
    assert!(names.contains(&"notes.txt".to_string()));
    assert!(names.contains(&"sub".to_string()));
    assert!(!names.contains(&"0000000000005-0000abcd.json".to_string()));
    assert!(names.contains(&"0000000000006-0000abcd.json".to_string()));
    let back: HistoryEntry = serde_json::from_slice(
        &fs::read(s.history_file("0000000000025-0000abcd").unwrap().path()).unwrap(),
    )
    .unwrap();
    assert_eq!(back, hist("0000000000025-0000abcd"));
}

#[test]
fn latest_history_is_newest_op_id() {
    let t = tmp();
    let s = sd(t.path());
    assert_eq!(read_latest_history(&s).unwrap(), None);
    for i in [3u64, 11, 7] {
        write_history(&RealFs, &s, &hist(&format!("{i:013}-0000abcd"))).unwrap();
    }
    fs::write(s.history_dir().path().join("9999999999999-zzzz.json"), b"x").unwrap();
    fs::write(s.history_dir().path().join("notes.txt"), b"mine").unwrap();
    let latest = read_latest_history(&s).unwrap().unwrap();
    assert_eq!(latest.op_id, "0000000000011-0000abcd");
    // A corrupt newest entry is an error, not a silent fallback.
    fs::write(
        s.history_file("0000000000012-0000abcd").unwrap().path(),
        b"{not json",
    )
    .unwrap();
    assert_eq!(
        read_latest_history(&s).unwrap_err().code(),
        "state-dir-invalid"
    );
}

struct FailRemove;
impl FsOps for FailRemove {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        RealFs.write_state(p, b)
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        RealFs.append_state(p, b)
    }
    fn remove_state(&self, _p: &StatePath) -> Result<()> {
        Err(Error::io("injected", std::io::Error::other("x")))
    }
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
        RealFs.remove_state_dir_if_empty(p)
    }
    fn create_dir(&self, d: &crate::fsops::DestPath) -> Result<bool> {
        RealFs.create_dir(d)
    }
    fn rmdir_if_empty(&self, d: &crate::fsops::DestPath) -> Result<bool> {
        RealFs.rmdir_if_empty(d)
    }
    fn symlink_dir(&self, s: &crate::source::SourcePath, d: &crate::fsops::DestPath) -> Result<()> {
        RealFs.symlink_dir(s, d)
    }
    fn copy_tree(
        &self,
        s: &crate::source::SourcePath,
        d: &crate::fsops::DestPath,
        tag: &str,
        h: &Hash,
    ) -> Result<Hash> {
        RealFs.copy_tree(s, d, tag, h)
    }
    fn remove_owned(
        &self,
        d: &crate::fsops::DestPath,
        e: &crate::fsops::OwnedExpectation,
        t: &StatePath,
    ) -> Result<crate::fsops::RemoveOutcome> {
        RealFs.remove_owned(d, e, t)
    }
    fn restore_owned(
        &self,
        d: &crate::fsops::DestPath,
        e: &crate::fsops::OwnedExpectation,
        t: &StatePath,
        s: Option<&crate::source::SourcePath>,
        tag: &str,
    ) -> Result<crate::fsops::RestoreOutcome> {
        RealFs.restore_owned(d, e, t, s, tag)
    }

    fn write_generated(
        &self,
        d: &crate::fsops::DestPath,
        c: &[u8],
        tag: &str,
    ) -> crate::error::Result<()> {
        RealFs.write_generated(d, c, tag)
    }
    fn remove_generated(
        &self,
        d: &crate::fsops::DestPath,
        h: &crate::hash::Hash,
        t: &crate::fsops::StatePath,
    ) -> crate::error::Result<crate::fsops::RemoveOutcome> {
        RealFs.remove_generated(d, h, t)
    }
}

#[test]
fn prune_failure_is_an_err_the_caller_can_downgrade() {
    let t = tmp();
    let s = sd(t.path());
    for i in 0..3u64 {
        write_history(&RealFs, &s, &hist(&format!("{:013}-0000abcd", i + 1))).unwrap();
    }
    assert!(prune_history(&FailRemove, &s, 1).is_err());
}

// ---- journal ----

fn begin_for(root: &Path, plan: Vec<JournalStep>) -> Begin {
    Begin {
        version: 1,
        op_id: OP.into(),
        command: "add".into(),
        target: "claude-code".into(),
        project_root: root.to_string_lossy().into_owned(),
        started_at: "s".into(),
        dest_rel: ".claude/skills".into(),
        dest_dirs_planned: vec![".claude".into(), ".claude/skills".into()],
        selection_before: SelectionBefore {
            existed: false,
            text: String::new(),
        },
        selection_after_text: "x".into(),
        selection_after_hash: "blake3:00".into(),
        selection_hash: "blake3:11".into(),
        receipt_before: Value::Null,
        plan,
    }
}

fn add_step(n: u32, name: &str) -> JournalStep {
    JournalStep::Add {
        n,
        id: format!("fx:a/{name}"),
        name: name.into(),
        strategy: Strategy::Symlink,
        source: format!("/lib/a/{name}"),
        tree_hash: "blake3:00".into(),
        provenance: vec![Provenance::Adhoc],
    }
}

fn write_journal(root: &Path, lines: &[String]) -> StateDir {
    let s = sd(root);
    fs::create_dir_all(s.dir().path()).unwrap();
    fs::write(s.journal().path(), lines.concat()).unwrap();
    s
}

fn begin_line(b: Begin) -> String {
    Record::Begin(Box::new(b)).to_line()
}

fn present(r: JournalRead) -> Journal {
    match r {
        JournalRead::Present(j) => *j,
        other => panic!("{other:?}"),
    }
}

#[test]
fn journal_reader_each_record_type() {
    let t = tmp();
    let root = canon(t.path());
    let plan = vec![
        add_step(1, "a"),
        JournalStep::Remove {
            n: 2,
            id: "fx:a/b".into(),
            name: "b".into(),
            expect: JournalExpect {
                strategy: Strategy::Copy,
                source: "/lib/a/b".into(),
                tree_hash: Some("blake3:00".into()),
            },
        },
        JournalStep::DropOrphan {
            n: 3,
            id: "fx:a/c".into(),
            name: "c".into(),
            reason: "missing".into(),
        },
        JournalStep::UpdateProvenance {
            n: 4,
            id: "fx:a/d".into(),
            name: "d".into(),
            provenance: vec![Provenance::Bundle("x".into())],
        },
        JournalStep::RemoveDir {
            n: 5,
            dir: ".claude/skills".into(),
        },
    ];
    let lines = vec![
        begin_line(begin_for(&root, plan)),
        Record::Dir {
            path: ".claude".into(),
        }
        .to_line(),
        Record::DirSkipped {
            path: ".claude".into(),
        }
        .to_line(),
        Record::Dir {
            path: ".claude/skills".into(),
        }
        .to_line(),
        Record::Started { n: 1 }.to_line(),
        Record::Done {
            n: 1,
            how: "ran".into(),
        }
        .to_line(),
        Record::Started { n: 2 }.to_line(),
        Record::Failed {
            n: 2,
            code: "io-error".into(),
            message: "m".into(),
        }
        .to_line(),
        Record::Skipped {
            n: 3,
            reason: "missing".into(),
        }
        .to_line(),
        Record::Started { n: 4 }.to_line(),
        Record::DirRemoved {
            path: ".claude/skills".into(),
        }
        .to_line(),
        Record::Receipt {
            outcome: MarkerOutcome::Partial,
        }
        .to_line(),
    ];
    let s = write_journal(&root, &lines);
    let j = present(read_journal(&s, ".claude/skills").unwrap());
    assert_eq!(j.states[&1], StepState::Done { how: "ran".into() });
    assert!(matches!(j.states[&2], StepState::Failed { .. }));
    assert!(j.states[&3].terminal());
    assert_eq!(j.states[&4], StepState::Started);
    assert_eq!(j.states[&5], StepState::Pending);
    assert_eq!(j.dirs_created, vec![".claude/skills".to_string()]);
    assert_eq!(j.dirs_removed, vec![".claude/skills".to_string()]);
    assert_eq!(j.marker, Some(MarkerOutcome::Partial));
    assert!(!j.torn_tail && j.ends_with_newline);
    assert_eq!(j.valid_text, lines.concat());
}

#[test]
fn journal_torn_tail_and_missing_newline() {
    let t = tmp();
    let root = canon(t.path());
    let b = begin_line(begin_for(&root, vec![add_step(1, "a")]));
    let started = Record::Started { n: 1 }.to_line();
    let s = write_journal(&root, &[b.clone(), started.clone(), "{\"t\":\"do".into()]);
    let j = present(read_journal(&s, ".claude/skills").unwrap());
    assert!(j.torn_tail);
    assert_eq!(j.valid_text, format!("{b}{started}"));
    assert_eq!(j.states[&1], StepState::Started);
    // A final record without a newline is kept and reported.
    let s = write_journal(&root, &[b.clone(), started.trim_end().to_string()]);
    let j = present(read_journal(&s, ".claude/skills").unwrap());
    assert!(!j.torn_tail);
    assert!(!j.ends_with_newline);
    assert_eq!(j.valid_text, format!("{b}{started}"));
}

#[test]
fn journal_torn_begin_and_absent() {
    let t = tmp();
    let root = canon(t.path());
    let s = sd(&root);
    assert_eq!(
        read_journal(&s, ".claude/skills").unwrap(),
        JournalRead::Absent
    );
    let s = write_journal(&root, &[String::new()]);
    assert_eq!(
        read_journal(&s, ".claude/skills").unwrap(),
        JournalRead::TornBegin
    );
    let s = write_journal(&root, &["{\"t\":\"begin\",\"vers".into()]);
    assert_eq!(
        read_journal(&s, ".claude/skills").unwrap(),
        JournalRead::TornBegin
    );
}

fn assert_invalid(root: &Path, lines: &[String], why: &str) {
    let s = write_journal(root, lines);
    let e = read_journal(&s, ".claude/skills").unwrap_err();
    assert_eq!(e.code(), "journal-invalid", "{why}");
}

#[test]
fn journal_rejections() {
    let t = tmp();
    let root = canon(t.path());
    let good = || begin_for(&root, vec![add_step(1, "a")]);
    let b = begin_line(good());
    assert_invalid(&root, &[b.clone(), b.clone()], "duplicate begin");
    assert_invalid(
        &root,
        &[b.clone(), "{\"t\":\"exploded\"}\n".into()],
        "unknown type",
    );
    assert_invalid(
        &root,
        &[
            b.clone(),
            "garbage\n".into(),
            Record::Started { n: 1 }.to_line(),
        ],
        "bad middle line",
    );
    assert_invalid(
        &root,
        &[Record::Started { n: 1 }.to_line()],
        "record before begin",
    );
    assert_invalid(
        &root,
        &[b.clone(), Record::Started { n: 2 }.to_line()],
        "n out of range",
    );
    let mut x = good();
    x.plan = vec![add_step(1, "..")];
    assert_invalid(&root, &[begin_line(x)], "name ..");
    let mut x = good();
    x.dest_rel = "../..".into();
    assert_invalid(&root, &[begin_line(x)], "dest_rel mismatch");
    let mut x = good();
    x.op_id = "../x".into();
    assert_invalid(&root, &[begin_line(x)], "bad op id");
    let mut x = good();
    x.plan = vec![add_step(2, "a")];
    assert_invalid(&root, &[begin_line(x)], "plan gap");
    let mut x = good();
    x.plan = vec![JournalStep::RemoveDir {
        n: 1,
        dir: ".claude/other".into(),
    }];
    assert_invalid(&root, &[begin_line(x)], "forged remove_dir");
    assert_invalid(
        &root,
        &[b.clone(), Record::Dir { path: "..".into() }.to_line()],
        "forged dir record",
    );
    assert_invalid(
        &root,
        &[
            b.clone(),
            Record::DirSkipped {
                path: "/etc".into(),
            }
            .to_line(),
        ],
        "forged dir_skipped",
    );
    assert_invalid(
        &root,
        &[
            b.clone(),
            Record::DirRemoved {
                path: ".claude/x".into(),
            }
            .to_line(),
        ],
        "forged dir_removed",
    );
    let mut x = good();
    x.receipt_before = json!({"version": 1, "project_root": "/p", "target": "agents-neutral",
        "applied_at": "", "selection_hash": "", "last_op_id": null, "created_dirs": [], "entries": []});
    assert_invalid(&root, &[begin_line(x)], "receipt_before of another target");
    let mut x = good();
    x.receipt_before = json!({"version": 1, "project_root": "/p", "target": "claude-code",
        "applied_at": "", "selection_hash": "", "last_op_id": null, "created_dirs": [],
        "entries": [{"kind": "skill", "skill_id": "fx:a/b", "name": "b", "dest": "../../b",
            "strategy": "symlink", "source_path": "/s", "tree_hash": "blake3:00",
            "git_commit": null, "pin": "follow", "provenance": [], "applied_at": ""}]});
    assert_invalid(
        &root,
        &[begin_line(x)],
        "receipt_before entry escaping the destination",
    );
}

#[test]
fn journal_with_a_different_project_root_is_journal_invalid() {
    let t = tmp();
    let root = canon(t.path());
    let mut b = begin_for(&root, vec![add_step(1, "a")]);
    b.project_root = "/somewhere/else".into();
    let s = write_journal(&root, &[begin_line(b)]);
    let e = read_journal(&s, ".claude/skills").unwrap_err();
    assert_eq!(e.code(), "journal-invalid");
    assert!(e.to_string().contains("move the project back"));
}

// ---- generated bundle briefs (M5-8) ----

#[test]
fn journal_round_trips_the_generated_step_ops_inside_the_begin_record() {
    let t = tmp();
    let root = canon(t.path());
    let plan = vec![
        JournalStep::RemoveGenerated {
            n: 1,
            bundle: "b".into(),
            name: "b-brief".into(),
            expect_hash: "blake3:aa".into(),
        },
        JournalStep::WriteGenerated {
            n: 2,
            bundle: "b".into(),
            name: "b-brief".into(),
            content: "---\nname: b-brief\n---\n\nText \u{e9}\n".into(),
            content_hash: "blake3:bb".into(),
            adopt: false,
        },
    ];
    let line = begin_line(begin_for(&root, plan.clone()));
    // The new ops are step ops (tag `op`) inside `begin`, not record types.
    assert!(line.contains("\"op\":\"write_generated\""), "{line}");
    assert!(line.contains("\"op\":\"remove_generated\""), "{line}");
    assert!(line.starts_with("{\"t\":\"begin\""), "{line}");
    let s = write_journal(
        &root,
        &[
            line,
            Record::Started { n: 1 }.to_line(),
            Record::Done {
                n: 1,
                how: "ran".into(),
            }
            .to_line(),
        ],
    );
    let j = present(read_journal(&s, ".claude/skills").unwrap());
    assert_eq!(j.begin.plan, plan);
    assert_eq!(j.begin.plan[0].name(), Some("b-brief"));
    assert_eq!(j.begin.plan[1].n(), 2);
    assert_eq!(j.states[&1], StepState::Done { how: "ran".into() });
    assert_eq!(j.states[&2], StepState::Pending);
}

#[test]
fn journal_rejects_a_generated_step_with_an_invalid_name() {
    let t = tmp();
    let root = canon(t.path());
    for name in ["..", "a/b", ""] {
        let b = begin_for(
            &root,
            vec![JournalStep::WriteGenerated {
                n: 1,
                bundle: "b".into(),
                name: name.into(),
                content: "x".into(),
                content_hash: "blake3:00".into(),
                adopt: false,
            }],
        );
        assert_invalid(&root, &[begin_line(b)], "generated name");
    }
}

#[test]
fn generated_entry_is_a_typed_view_of_an_other_entry() {
    let v = json!({
        "kind": "generated",
        "generated": "bundle-brief",
        "bundle": "backend-dev",
        "name": "backend-dev-brief",
        "dest": ".claude/skills/backend-dev-brief",
        "content_hash": "blake3:11",
        "applied_at": "2026-10-02T21:40:00Z"
    });
    let g = GeneratedEntry::from_value(&v).unwrap();
    assert_eq!(g.bundle, "backend-dev");
    assert!(g.consistent(".claude/skills"));
    assert!(!g.consistent(".agents/skills"));
    assert_eq!(g.to_value(), v);
    // Not a brief: another `generated` kind, or missing fields.
    assert!(
        GeneratedEntry::from_value(&json!({"kind": "generated", "path": "AGENTS.md"})).is_none()
    );
    assert!(
        GeneratedEntry::from_value(&json!({"kind": "generated", "generated": "bundle-brief"}))
            .is_none()
    );
    assert!(GeneratedEntry::from_value(&json!({"kind": "skill"})).is_none());
    // A receipt exposes only the typed ones, and keeps the others verbatim.
    let mut r = Receipt::parse(&sample_receipt_text(), Path::new("/x")).unwrap();
    assert_eq!(r.generated_entries().count(), 0);
    r.entries.push(Entry::Other(v));
    assert_eq!(r.generated_entries().count(), 1);
    let again = Receipt::parse(&r.to_text(), Path::new("/x")).unwrap();
    assert_eq!(again, r);
    // A name that cannot be a destination is not consistent.
    let mut bad = g.clone();
    bad.name = "..".into();
    bad.dest = ".claude/skills/..".into();
    assert!(!bad.consistent(".claude/skills"));
}
