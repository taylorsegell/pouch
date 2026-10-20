//! Acceptance criteria for interruption and recovery (milestone M3: AC-31,
//! AC-32, AC-53) plus the recovery supplement rows (G2, G6). Crashes are
//! simulated in process with `catch_unwind` over the `FsOps` seam; the CLI
//! is then run against the state a killed process would leave.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::*;
use common::*;
use pouch_core::change::{self, Decision, Edit, Outcome, RecoverOutcome};
use pouch_core::fsops::RealFs;
use pouch_core::state::ProjectLock;
use serde_json::Value;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

fn dest(p: &Path) -> PathBuf {
    p.join(".claude/skills")
}

fn journal(p: &Path) -> PathBuf {
    p.join(".pouch/claude-code/journal.jsonl")
}

fn refs(r: &[&str]) -> Vec<String> {
    r.iter().map(|s| s.to_string()).collect()
}

/// Run an `add` in process under `fs`, catching a simulated crash.
fn crash_add(fx: &Fx, p: &Path, r: &[&str], fs_ops: &FailingFs) {
    quiet_crashes();
    let c = core_ctx(&fx.env, p);
    let clock = StepClock::new();
    let req = c.request(Edit::Add(refs(r)), "add", &clock);
    let _ = catch_unwind(AssertUnwindSafe(|| {
        in_lock_section(|| change::run(&req, fs_ops, &mut |_| Decision::Proceed))
    }));
}

/// The index of the first call matching `method` and a path fragment, in
/// a clean counting run of the same change on a twin project.
fn call_index(fx: &Fx, twin: &str, r: &[&str], method: &str, fragment: &str) -> u32 {
    let p = fx.project(twin);
    let counter = FailingFs::new(FailMode::None);
    crash_add(fx, &p, r, &counter);
    counter
        .calls()
        .into_iter()
        .find(|(_, m, path)| *m == method && path.to_string_lossy().contains(fragment))
        .map(|(n, _, _)| n)
        .unwrap()
}

/// Destination listing (name, kind, link text or tree hash).
fn dest_state(p: &Path) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = match fs::read_dir(dest(p)) {
        Ok(rd) => rd
            .map(|e| {
                let e = e.unwrap();
                let path = e.path();
                let name = e.file_name().to_string_lossy().into_owned();
                let m = fs::symlink_metadata(&path).unwrap();
                let what = if m.file_type().is_symlink() {
                    format!("link {}", fs::read_link(&path).unwrap().display())
                } else if m.is_dir() {
                    format!(
                        "dir {}",
                        pouch_core::hash::tree_hash(&path, &[".git", ".DS_Store"]).unwrap()
                    )
                } else {
                    "file".to_string()
                };
                (name, what)
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

/// The receipt with volatile fields masked (`None` when absent).
fn receipt_state(p: &Path) -> Option<Value> {
    let path = p.join(".pouch/claude-code/applied.json");
    let mut v: Value = serde_json::from_slice(&fs::read(path).ok()?).unwrap();
    v["applied_at"] = Value::Null;
    v["last_op_id"] = Value::Null;
    v["project_root"] = Value::Null;
    if let Some(entries) = v["entries"].as_array_mut() {
        for e in entries {
            e["applied_at"] = Value::Null;
        }
    }
    Some(v)
}

fn selection_state(p: &Path) -> Option<String> {
    fs::read_to_string(p.join(".pouch/selection.yaml")).ok()
}

/// Destination, receipt, and selection (the AC-31 comparison set).
fn change_state(p: &Path) -> (Vec<(String, String)>, Option<Value>, Option<String>) {
    (dest_state(p), receipt_state(p), selection_state(p))
}

#[test]
fn ac_31_interrupted_apply_leaves_the_journal_and_the_next_command_detects_it_without_acting() {
    let fx = Fx::new();
    let p = fx.project("p");
    let all = [
        "fx:sdlc/code-review",
        "fx:testing/pytest-fixtures",
        "fx:data/schema-migration",
        "hs:ops/deploy-checklist",
    ];
    crash_add(&fx, &p, &all, &FailingFs::new(FailMode::CrashAfter(2)));
    assert!(journal(&p).exists());
    let start = snapshot_project(&p);
    for args in [
        &["add", "fx:sdlc/code-review", "--yes"][..],
        &["rm", "fx:sdlc/code-review", "--yes"],
        &["use", "hermes-only", "--replace", "--yes"],
        &["deactivate", "--yes"],
    ] {
        let (code, v) = fx.json_raw(&p, args);
        assert_eq!(code, 10, "{args:?}: {v}");
        assert_eq!(v["errors"][0]["code"], "interrupted");
        assert!(v["data"]["interrupted"]["steps"].as_array().unwrap().len() == 4);
        assert_eq!(
            v["data"]["next"],
            serde_json::json!(["pouch resume", "pouch rollback"])
        );
        assert_project_unchanged(&start, &p);
    }
    fx.done();
}

/// The AC-31 fault matrix over the scenarios named in `only`; every
/// `resume` and every `rollback` must also leave `reconcile` with zero
/// divergence (spec 8, PRD S7).
fn run_fault_matrix(only: &[&str]) {
    quiet_crashes();
    let fx = Fx::new();
    fx.write_bundle("swap", &["fx:sdlc/code-review", "fx:data/schema-migration"]);
    type Build = fn(&Fx, &str) -> Scenario;
    let builders: Vec<(&str, Build)> = [
        ("a", scenario_a as Build),
        ("b", scenario_b as Build),
        ("c", scenario_c as Build),
    ]
    .into_iter()
    .filter(|(label, _)| only.contains(label))
    .collect();
    let full = std::env::var("POUCH_FULL_FAULT_MATRIX").is_ok_and(|v| v == "1");
    println!(
        "fault matrix mode: {}",
        if full {
            "FULL (POUCH_FULL_FAULT_MATRIX=1)"
        } else {
            "strided sample (call 1, every third call, last call); set POUCH_FULL_FAULT_MATRIX=1 for the full matrix"
        }
    );
    let mut runs = 0;
    for (label, build) in builders {
        let clean = build(&fx, &format!("{label}-clean"));
        let counter = FailingFs::new(FailMode::None);
        let out = clean
            .run(&fx.env, &counter, &StepClock::new())
            .map_err(|f| f.error)
            .unwrap();
        assert!(matches!(out, Outcome::Applied(..)), "{label}");
        let total = counter.total();
        assert!(total >= 5, "{label}: {total} calls");
        println!("fault matrix scenario {label}: {total} mutating calls x 4 modes x 2 recoveries");
        let clean_state = change_state(&clean.project);
        if label == "c" {
            let r = read_receipt(&clean.project, "claude-code");
            assert_eq!(
                r["created_dirs"],
                serde_json::json!([".claude", ".claude/skills"])
            );
            assert_eq!(
                fs::read_dir(clean.project.join(".pouch/claude-code/history"))
                    .unwrap()
                    .count(),
                1
            );
        }
        for n in 1..=total {
            if !full && n != 1 && n != total && n % 3 != 0 {
                continue;
            }
            for mode in FailMode::all_at(n) {
                for recovery in ["resume", "rollback"] {
                    runs += 1;
                    let ctx = format!("{label} {mode:?} {recovery}");
                    let s = build(&fx, &format!("{label}-{n}-{runs}"));
                    let start = snapshot_project(&s.project);
                    let start_sel = selection_state(&s.project);
                    let start_dest = dest_state(&s.project);
                    let start_receipt = receipt_state(&s.project);
                    let ff = FailingFs::new(mode);
                    let _ =
                        catch_unwind(AssertUnwindSafe(|| s.run(&fx.env, &ff, &StepClock::new())));
                    assert_ordering_invariant(&s.project, &ctx);
                    let c = s.ctx(&fx.env);
                    let clock = StepClock::new();
                    if !journal(&s.project).exists() {
                        if change_state(&s.project) == clean_state {
                            // The apply finished before the injection.
                            continue;
                        }
                        // Before the `begin` record: nothing applied.
                        assert_eq!(dest_state(&s.project), start_dest, "{ctx}");
                        assert_eq!(receipt_state(&s.project), start_receipt, "{ctx}");
                        let sel = selection_state(&s.project);
                        assert!(
                            sel == start_sel || sel.as_deref() == clean_state.2.as_deref(),
                            "{ctx}: selection {sel:?}"
                        );
                        let before = snapshot_project(&s.project);
                        for e in [
                            in_lock_section(|| {
                                change::resume(&c.recover(&clock), &RealFs, &mut |_| {
                                    Decision::Proceed
                                })
                            })
                            .err()
                            .unwrap()
                            .error,
                            in_lock_section(|| {
                                change::rollback(&c.recover(&clock), &RealFs, &mut |_| {
                                    Decision::Proceed
                                })
                            })
                            .err()
                            .unwrap()
                            .error,
                        ] {
                            assert_eq!(e.code(), "no-journal", "{ctx}");
                        }
                        assert_project_unchanged(&before, &s.project);
                        s.run(&fx.env, &RealFs, &StepClock::new())
                            .map_err(|f| f.error)
                            .unwrap();
                        assert_eq!(change_state(&s.project), clean_state, "{ctx}: re-run");
                        continue;
                    }
                    let req = c.recover(&clock);
                    if recovery == "resume" {
                        let out = in_lock_section(|| {
                            change::resume(&req, &RealFs, &mut |_| Decision::Proceed)
                        })
                        .map_err(|f| f.error)
                        .unwrap();
                        assert!(matches!(out, RecoverOutcome::Recovered(..)), "{ctx}");
                        assert_eq!(change_state(&s.project), clean_state, "{ctx}");
                        assert!(!journal(&s.project).exists(), "{ctx}");
                        assert_zero_divergence(&c, &ctx);
                        if label == "c" {
                            let r = read_receipt(&s.project, "claude-code");
                            assert_eq!(
                                r["created_dirs"],
                                serde_json::json!([".claude", ".claude/skills"]),
                                "{ctx}"
                            );
                            assert_eq!(
                                fs::read_dir(s.project.join(".pouch/claude-code/history"))
                                    .unwrap()
                                    .count(),
                                1,
                                "{ctx}"
                            );
                        }
                    } else {
                        let out = in_lock_section(|| {
                            change::rollback(&req, &RealFs, &mut |_| Decision::Proceed)
                        })
                        .map_err(|f| f.error)
                        .unwrap();
                        let RecoverOutcome::Recovered(report, _) = out else {
                            panic!("{ctx}")
                        };
                        assert!(
                            report
                                .warnings
                                .iter()
                                .all(|w| !w.code.starts_with("rollback-")),
                            "{ctx}: {:?}",
                            report.warnings
                        );
                        assert_project_unchanged(&start, &s.project);
                        assert_zero_divergence(&c, &ctx);
                        if label == "c" {
                            assert!(!s.project.join(".claude").exists(), "{ctx}");
                            assert!(
                                !s.project.join(".pouch/claude-code/applied.json").exists(),
                                "{ctx}"
                            );
                            assert!(
                                !s.project.join(".pouch/claude-code/history").exists(),
                                "{ctx}"
                            );
                        }
                    }
                }
            }
        }
    }
    println!("fault matrix runs: {runs}");
    assert!(runs > 30 * only.len(), "{runs} matrix runs");
    fx.done();
}

#[test]
fn ac_31_resume_and_rollback_reach_a_consistent_state_for_every_injected_failure() {
    run_fault_matrix(&["a", "b", "c"]);
}

#[test]
fn s7_interruption_zero_divergence() {
    run_fault_matrix(&["a"]);
}

#[test]
fn ac_32_one_of_three_entries_fails_the_other_two_remain_applied_and_the_failure_is_named() {
    let fx = Fx::new_with(|lib| {
        write_file(
            lib,
            "bad/escaper/SKILL.md",
            skill_md("escaper", "Holds a link that leaves the skill.").as_bytes(),
        );
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", lib.join("bad/escaper/leak")).unwrap();
    });
    let p = fx.project("p");
    fx.write_selection(&p, "targets:\n  claude-code:\n    strategy: copy\n");
    let start = snapshot_project(&p);
    let (code, v) = fx.json_raw(
        &p,
        &[
            "add",
            "fx:bad/escaper",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    assert_eq!(code, 7, "{v}");
    assert_eq!(v["errors"][0]["code"], "apply-failed");
    let results = v["data"]["results"].as_array().unwrap();
    let failed: Vec<&Value> = results.iter().filter(|r| r["result"] == "failed").collect();
    let done: Vec<&Value> = results.iter().filter(|r| r["result"] == "done").collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(done.len(), 2);
    assert_eq!(failed[0]["code"], "copy-refused");
    assert!(
        failed[0]["message"].as_str().unwrap().contains("leak"),
        "{v}"
    );
    assert!(dest(&p).join("code-review").is_dir());
    assert!(dest(&p).join("pytest-fixtures").is_dir());
    assert!(!dest(&p).join("escaper").exists());
    assert_eq!(receipt_entry_texts(&p, "claude-code").len(), 2);
    assert!(journal(&p).exists());
    let (code, _) = fx.json_raw(&p, &["add", "fx:data/schema-migration", "--yes"]);
    assert_eq!(code, 10);
    let (code, v) = fx.json_raw(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn ac_53_second_mutating_command_refuses_while_locked_and_changes_nothing() {
    let fx = Fx::new();
    let p = fx.project("p");
    let clock = StepClock::new();
    let guard = ProjectLock::acquire(&fx.canon(&p), "test", &clock).unwrap();
    let lock_bytes = fs::read(p.join(".pouch/lock")).unwrap();
    let start = snapshot_project(&p);
    for args in [
        &["add", "fx:sdlc/code-review", "--yes"][..],
        &["rm", "fx:sdlc/code-review", "--yes"],
        &["use", "hermes-only", "--replace", "--yes"],
        &["deactivate", "--yes"],
        &["resume", "--yes"],
        &["rollback", "--yes"],
    ] {
        let (code, v) = fx.json_raw(&p, args);
        assert_eq!(code, 11, "{args:?}: {v}");
        assert_eq!(v["errors"][0]["code"], "locked");
        assert_eq!(v["data"]["locked"]["pid"], std::process::id());
        assert_project_unchanged(&start, &p);
        assert_eq!(fs::read(p.join(".pouch/lock")).unwrap(), lock_bytes);
    }
    drop(guard);
    // A sibling test's spawn may still hold a forked copy of the lock fd.
    spawn_fence();
    let (code, v) = fx.json_raw(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{v}");
    fx.done();
}

#[test]
fn recovery_finalizes_a_crash_between_receipt_write_and_journal_delete() {
    let fx = Fx::new();
    let r = ["fx:sdlc/code-review"];
    let k = call_index(&fx, "twin", &r, "write_state", "applied.json");
    let p = fx.project("p");
    crash_add(&fx, &p, &r, &FailingFs::new(FailMode::CrashAfter(k)));
    assert!(journal(&p).exists());
    let receipt_before = fs::read(p.join(".pouch/claude-code/applied.json")).unwrap();
    let (code, out, err) = fx.run(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{err}");
    assert!(err.contains("finalized a completed apply"), "{err}");
    assert!(out.contains("No changes"), "{out}");
    assert!(!journal(&p).exists());
    assert_eq!(
        fs::read_dir(p.join(".pouch/claude-code/history"))
            .unwrap()
            .count(),
        1
    );
    // Not replayed: the receipt is the one the interrupted apply wrote.
    assert_eq!(
        fs::read(p.join(".pouch/claude-code/applied.json")).unwrap(),
        receipt_before
    );
    fx.done();
}

#[test]
fn recovery_tolerates_a_torn_final_journal_line() {
    let fx = Fx::new();
    let p = fx.project("p");
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review", "fx:testing/pytest-fixtures"],
        &FailingFs::new(FailMode::CrashBefore(6)),
    );
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(journal(&p))
        .unwrap();
    std::io::Write::write_all(&mut f, b"{\"t\":\"done\",\"n\"").unwrap();
    drop(f);
    let (code, v) = fx.json_raw(&p, &["add", "fx:data/schema-migration", "--yes"]);
    assert_eq!(code, 10, "the torn journal is still read: {v}");
    let (code, v) = fx.json_raw(&p, &["resume", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert!(dest(&p).join("code-review").exists());
    assert!(dest(&p).join("pytest-fixtures").exists());
    assert!(!journal(&p).exists());
    fx.done();
}

#[test]
fn crafted_journal_cannot_escape_the_destination_root() {
    let fx = Fx::new();
    let outside = fx.env.path("work/outside");
    fx.plant_file(&outside.join("canary.md"), b"canary\n");
    let long = "a".repeat(300);
    let begin = |p: &Path| -> Value {
        serde_json::json!({
            "t": "begin", "version": 1, "op_id": "0000000000001-0000abcd", "command": "add",
            "target": "claude-code", "project_root": fx.canon(p).to_string_lossy(),
            "started_at": "2026-10-05T00:00:00Z", "dest_rel": ".claude/skills",
            "dest_dirs_planned": [],
            "selection_before": {"existed": false, "text": ""},
            "selection_after_text": "", "selection_after_hash": "h", "selection_hash": "h",
            "receipt_before": null,
            "plan": [{"op": "remove", "n": 1, "id": "fx:a/b", "name": "x",
                      "expect": {"strategy": "symlink", "source": outside.join("canary.md").to_string_lossy()}}]
        })
    };
    let receipt = |target: &str, dest: &str| -> Value {
        serde_json::json!({"version": 1, "project_root": "/p", "target": target,
            "applied_at": "", "selection_hash": "", "last_op_id": null, "created_dirs": [],
            "entries": [{"kind": "skill", "skill_id": "fx:a/b", "name": "b", "dest": dest,
                "strategy": "symlink", "source_path": "/s", "tree_hash": "blake3:00",
                "git_commit": null, "pin": "follow", "provenance": [], "applied_at": ""}]})
    };
    type Craft = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Craft)> = vec![
        (
            "dotdot",
            Box::new(|v: &mut Value| v["plan"][0]["name"] = "..".into()),
        ),
        (
            "slash",
            Box::new(|v: &mut Value| v["plan"][0]["name"] = "a/b".into()),
        ),
        (
            "absolute",
            Box::new(|v: &mut Value| v["plan"][0]["name"] = "/etc".into()),
        ),
        (
            "long",
            Box::new(move |v: &mut Value| v["plan"][0]["name"] = long.clone().into()),
        ),
        (
            "temp",
            Box::new(|v: &mut Value| v["plan"][0]["name"] = ".pouch-tmp-x".into()),
        ),
        (
            "op-id",
            Box::new(|v: &mut Value| v["op_id"] = "../x".into()),
        ),
        (
            "dest-rel",
            Box::new(|v: &mut Value| v["dest_rel"] = "../..".into()),
        ),
        (
            "root",
            Box::new(|v: &mut Value| v["project_root"] = "/somewhere/else".into()),
        ),
        (
            "receipt-target",
            Box::new(move |v: &mut Value| {
                v["receipt_before"] = receipt("agents-neutral", ".claude/skills/b")
            }),
        ),
        (
            "receipt-dest",
            Box::new(move |v: &mut Value| {
                v["receipt_before"] = receipt("claude-code", "../../outside/b")
            }),
        ),
    ];
    for (name, craft) in cases {
        let p = fx.project(name);
        fs::create_dir_all(dest(&p)).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("canary.md"), dest(&p).join("x")).unwrap();
        let mut v = begin(&p);
        craft(&mut v);
        write_file(
            &p,
            ".pouch/claude-code/journal.jsonl",
            format!("{v}\n").as_bytes(),
        );
        let start = snapshot_project(&p);
        for cmd in ["resume", "rollback"] {
            let (code, out) = fx.json_raw(&p, &[cmd, "--yes"]);
            assert_eq!(code, 1, "{name} {cmd}: {out}");
            assert_eq!(out["errors"][0]["code"], "journal-invalid", "{name} {cmd}");
            assert_project_unchanged(&start, &p);
        }
    }
    fx.done();
}

#[test]
fn rollback_restores_selection_only_when_unchanged() {
    let fx = Fx::new();
    let p = fx.project("p");
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review"],
        &FailingFs::new(FailMode::CrashAfter(4)),
    );
    assert!(journal(&p).exists());
    fs::write(p.join(".pouch/selection.yaml"), b"# hand edit\n").unwrap();
    let (code, v) = fx.json_raw(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        fs::read(p.join(".pouch/selection.yaml")).unwrap(),
        b"# hand edit\n"
    );
    assert!(
        v["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "selection-not-restored"),
        "{v}"
    );
    assert!(!dest(&p).join("code-review").exists());
    fx.done();
}

#[test]
fn rollback_can_be_repeated_after_a_crash_during_rollback() {
    quiet_crashes();
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review", "fx:testing/pytest-fixtures"],
        &FailingFs::new(FailMode::CrashAfter(7)),
    );
    assert!(journal(&p).exists());
    let c = core_ctx(&fx.env, &p);
    let clock = StepClock::new();
    let ff = FailingFs::new(FailMode::CrashBefore(2));
    let r = catch_unwind(AssertUnwindSafe(|| {
        in_lock_section(|| change::rollback(&c.recover(&clock), &ff, &mut |_| Decision::Proceed))
    }));
    assert!(crashed(&r));
    assert!(journal(&p).exists());
    let (code, v) = fx.json_raw(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[cfg(unix)]
#[test]
fn resume_and_rollback_refuse_when_the_destination_root_points_outside_the_project() {
    let fx = Fx::new();
    let p = fx.project("p");
    let r = ["fx:sdlc/code-review", "fx:testing/pytest-fixtures"];
    let k = call_index(&fx, "twin", &r, "symlink_dir", "code-review");
    crash_add(&fx, &p, &r, &FailingFs::new(FailMode::CrashAfter(k)));
    let journal_bytes = fs::read(journal(&p)).unwrap();
    // `.claude` replaced by a symlink to an outside directory holding an
    // entry that matches the journal.
    let outside = fx.env.path("work/outside");
    let link_text = fs::read_link(dest(&p).join("code-review")).unwrap();
    fs::create_dir_all(outside.join("skills")).unwrap();
    fx.plant_link(&outside.join("skills/code-review"), &link_text);
    fs::remove_file(dest(&p).join("code-review")).unwrap();
    fs::remove_dir(dest(&p)).unwrap();
    fs::remove_dir(p.join(".claude")).unwrap();
    std::os::unix::fs::symlink(&outside, p.join(".claude")).unwrap();
    for cmd in ["resume", "rollback"] {
        let (code, v) = fx.json_raw(&p, &[cmd, "--yes"]);
        assert_eq!(code, 6, "{cmd}: {v}");
        assert_eq!(v["errors"][0]["code"], "dest-outside-project");
        assert_eq!(fs::read(journal(&p)).unwrap(), journal_bytes);
    }
    // NotDirectory variant: a file at `.claude/skills`.
    fs::remove_file(p.join(".claude")).unwrap();
    fs::create_dir_all(p.join(".claude")).unwrap();
    fs::write(dest(&p), b"a file").unwrap();
    for cmd in ["resume", "rollback"] {
        let (code, v) = fx.json_raw(&p, &[cmd, "--yes"]);
        assert_eq!(code, 6, "{cmd}: {v}");
        assert_eq!(v["errors"][0]["code"], "dest-not-directory");
        assert_eq!(fs::read(journal(&p)).unwrap(), journal_bytes);
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn execute_refuses_when_the_destination_root_was_swapped_during_the_prompt() {
    let fx = Fx::new();
    let p = fx.project("p");
    fs::create_dir_all(dest(&p)).unwrap();
    let outside = fx.env.path("work/outside");
    fs::create_dir_all(outside.join("skills")).unwrap();
    fx.plant_file(&outside.join("skills/keep.md"), b"outside\n");
    let c = core_ctx(&fx.env, &p);
    let clock = StepClock::new();
    let req = c.request(Edit::Add(refs(&["fx:sdlc/code-review"])), "add", &clock);
    let swap = || {
        fs::remove_dir_all(p.join(".claude")).unwrap();
        std::os::unix::fs::symlink(&outside, p.join(".claude")).unwrap();
    };
    let e = in_lock_section(|| {
        change::run(&req, &RealFs, &mut |_| {
            swap();
            Decision::Proceed
        })
    })
    .err()
    .unwrap();
    assert_eq!(e.error.code(), "dest-outside-project");
    assert!(!p.join(".pouch/selection.yaml").exists());
    assert!(!journal(&p).exists());
    assert!(!p.join(".pouch/claude-code/applied.json").exists());
    assert_eq!(fs::read_dir(outside.join("skills")).unwrap().count(), 1);
    fx.done();
}

#[test]
fn resume_after_a_crash_following_a_torn_tail_rewrites_the_journal() {
    quiet_crashes();
    let fx = Fx::new();
    let r = ["fx:sdlc/code-review", "fx:testing/pytest-fixtures"];
    let clean = fx.project("clean");
    crash_add(&fx, &clean, &r, &FailingFs::new(FailMode::None));
    let p = fx.project("p");
    crash_add(&fx, &p, &r, &FailingFs::new(FailMode::CrashBefore(6)));
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(journal(&p))
        .unwrap();
    std::io::Write::write_all(&mut f, b"{\"t\":\"sta").unwrap();
    drop(f);
    let c = core_ctx(&fx.env, &p);
    let clock = StepClock::new();
    let ff = FailingFs::new(FailMode::CrashAfter(1));
    let res = catch_unwind(AssertUnwindSafe(|| {
        in_lock_section(|| change::resume(&c.recover(&clock), &ff, &mut |_| Decision::Proceed))
    }));
    assert!(crashed(&res));
    assert_eq!(ff.calls()[0].1, "write_state");
    for line in fs::read_to_string(journal(&p)).unwrap().lines() {
        let v: Value = serde_json::from_str(line).unwrap();
        assert!(v.get("t").is_some(), "{line}");
    }
    let (code, v) = fx.json_raw(&p, &["resume", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert!(!journal(&p).exists());
    let names = |q: &Path| -> Vec<String> { dest_state(q).into_iter().map(|(n, _)| n).collect() };
    assert_eq!(names(&p), names(&clean));
    assert_eq!(
        receipt_entry_texts(&p, "claude-code").len(),
        receipt_entry_texts(&clean, "claude-code").len()
    );
    fx.done();
}

#[test]
fn rollback_removes_a_history_entry_written_before_the_crash() {
    let fx = Fx::new();
    let r = ["fx:sdlc/code-review"];
    let k = call_index(&fx, "twin", &r, "write_state", "/history/");
    let p = fx.project("p");
    let start = snapshot_project(&p);
    crash_add(&fx, &p, &r, &FailingFs::new(FailMode::CrashAfter(k)));
    let history = p.join(".pouch/claude-code/history");
    assert_eq!(fs::read_dir(&history).unwrap().count(), 1);
    let (code, v) = fx.json_raw(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert!(!history.exists());
    assert_project_unchanged(&start, &p);
    fx.done();
}

/// Regression for the forked-descriptor flake: spawns on other threads must
/// never overlap an in-process lock section. All loops are fixed-count.
#[test]
fn lock_sections_never_overlap_a_spawn() {
    let fx = Fx::new();
    let p = fx.project("p");
    let root = fx.canon(&p);
    std::thread::scope(|scope| {
        for _ in 0..3 {
            scope.spawn(|| {
                for _ in 0..200 {
                    let (code, _, _) = run_in(&fx.env, &p, &["--version"]);
                    assert_eq!(code, 0);
                }
            });
        }
        scope.spawn(|| {
            let clock = StepClock::new();
            for i in 0..2000 {
                in_lock_section(|| {
                    let a = ProjectLock::acquire(&root, "test", &clock)
                        .unwrap_or_else(|e| panic!("acquire {i}: {e:?}"));
                    drop(a);
                    let b = ProjectLock::acquire(&root, "test", &clock)
                        .unwrap_or_else(|e| panic!("re-acquire {i}: {e:?}"));
                    drop(b);
                });
            }
        });
    });
    fx.done();
}
