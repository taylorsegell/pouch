//! Acceptance criteria for verify and status (milestone M4: AC-14, AC-26,
//! AC-27, AC-28, AC-36, AC-39, AC-41). Every test runs through the shared
//! fixture `Fx` and ends with `fx.done()` (or `done_with` after a deliberate
//! edit of the library).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const FOOTER: &str = "Verified means discoverable under the target's documented scan rules. It does not mean the agent has read it.";

fn ok(fx: &Fx, p: &Path, args: &[&str]) -> Value {
    let (code, v) = fx.json_raw(p, args);
    assert_eq!(code, 0, "{args:?}: {v}");
    v
}

fn canon(p: &Path) -> String {
    fs::canonicalize(p).unwrap().to_string_lossy().into_owned()
}

#[cfg(unix)]
#[test]
fn ac_14_status_shows_source_path_destination_strategy_provenance_state_per_entry() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.write_bundle("pair", &["fx:sdlc/code-review", "hs:ops/deploy-checklist"]);
    ok(&fx, &p, &["use", "pair", "--replace", "--yes"]);
    ok(
        &fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let (code, v) = fx.json_raw(&p, &["status"]);
    assert_eq!(code, 0, "{v}");
    let lib = canon(&fx.lib);
    let lib2 = canon(&fx.lib2);
    let bundle = json!({"bundle": "pair"});
    let adhoc = json!({"adhoc": true});
    let want = [
        (
            "code-review",
            "fx:sdlc/code-review",
            "sdlc",
            format!("{lib}/sdlc/code-review"),
            json!([bundle.clone(), adhoc.clone()]),
        ),
        (
            "deploy-checklist",
            "hs:ops/deploy-checklist",
            "ops",
            format!("{lib2}/ops/deploy-checklist"),
            json!([bundle]),
        ),
        (
            "pytest-fixtures",
            "fx:testing/pytest-fixtures",
            "testing",
            format!("{lib}/testing/pytest-fixtures"),
            json!([adhoc]),
        ),
    ];
    assert_eq!(v["data"]["records"].as_array().unwrap().len(), 3);
    for (name, id, category, source, provenance) in want {
        let r = record(&v, name);
        assert_eq!(r["id"], id);
        assert_eq!(r["category"], category);
        assert_eq!(r["source_path"], source);
        assert_eq!(r["dest"], format!(".claude/skills/{name}"));
        assert_eq!(r["strategy"], "symlink");
        assert_eq!(r["provenance"], provenance, "{name}");
        assert_eq!(r["state"], "active");
    }
    assert_eq!(v["data"]["summary"]["active"], 3);
    // The human table carries the same facts.
    let (code, out, _) = fx.run(&p, &["status"]);
    assert_eq!(code, 0);
    for want in [
        "STATE",
        "active",
        "bundle:pair,adhoc",
        ".claude/skills/code-review",
        &format!("{lib}/sdlc/code-review"),
        "symlink",
    ] {
        assert!(out.contains(want), "{want}: {out}");
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_26_broken_destination_symlink_is_orphaned_and_status_exits_8() {
    let fx = Fx::new();
    let p = fx.project("p");
    // A scratch source that can disappear without touching the fixtures.
    let tmp = fx.env.path("work/scratch-lib");
    write_file(
        &tmp,
        "one/SKILL.md",
        skill_md("one", "A skill that goes away.").as_bytes(),
    );
    ok(
        &fx,
        &p,
        &["source", "add", "scratch", tmp.to_str().unwrap()],
    );
    ok(
        &fx,
        &p,
        &["add", "scratch:one", "fx:sdlc/code-review", "--yes"],
    );
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    fs::remove_dir_all(&tmp).unwrap();
    let (code, v) = fx.status(&p);
    assert_eq!(code, 8, "{v}");
    assert_eq!(v["ok"], false);
    assert_eq!(v["errors"][0]["code"], "diverged");
    let one = record(&v, "one");
    assert_eq!(one["state"], "orphaned");
    assert_eq!(one["reason"], "broken-link");
    assert_eq!(record(&v, "code-review")["state"], "active");
    assert_eq!(v["data"]["summary"]["orphaned"], 1);
    assert!(
        record_states(&v)
            .iter()
            .all(|(n, s)| n != "one" || s != "active")
    );
    let (code, out, err) = fx.run(&p, &["status"]);
    assert_eq!(code, 8);
    assert!(out.contains("orphaned"), "{out}");
    assert!(err.contains("error:"), "{err}");
    let (code, _) = fx.json(&p, &["reconcile"]);
    assert_eq!(code, 8);
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_27_shadowed_skill_reports_destination_and_shadowing_path() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let by = fx.plant_user_skill("code-review");
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    let r = record(&v, "code-review");
    assert_eq!(r["state"], "shadowed");
    assert_eq!(r["dest"], ".claude/skills/code-review");
    assert_eq!(
        r["shadowed_by"],
        fx.env.normalize(by.to_str().unwrap()),
        "{r}"
    );
    let (code, out, _) = fx.run(&p, &["status"]);
    assert_eq!(code, 0);
    assert!(out.contains("shadowed"), "{out}");
    assert!(out.contains(".claude/skills/code-review"), "{out}");
    assert!(out.contains(by.to_str().unwrap()), "{out}");
    // A shadow is a finding, not a divergence.
    let (code, v) = fx.json(&p, &["reconcile"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["findings"][0]["kind"], "shadowed");
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_28_no_output_claims_an_agent_read_loaded_or_applied_a_skill() {
    let fx = Fx::new();
    let p = fx.project("p");
    let mut outs: Vec<String> = Vec::new();
    let mut exits: Vec<i32> = Vec::new();
    let mut run = |args: &[&str]| {
        let (code, out, err) = fx.run(&p, args);
        exits.push(code);
        outs.push(format!("{out}\n{err}"));
        out
    };
    run(&["add", "fx:sdlc/code-review", "--yes"]);
    run(&["--json", "add", "fx:testing/pytest-fixtures", "--yes"]);
    let status_human = run(&["status"]);
    let status_json = run(&["--json", "status"]);
    run(&["reconcile"]);
    run(&["--json", "reconcile"]);
    run(&["doctor"]);
    run(&["--json", "doctor"]);
    run(&["rm", "fx:testing/pytest-fixtures", "--yes"]);
    run(&["--json", "rm", "fx:sdlc/code-review", "--yes"]);
    run(&["deactivate", "--yes"]);
    run(&["--json", "deactivate", "--yes"]);
    assert!(exits.iter().all(|c| *c == 0), "{exits:?}");
    // The spec 4.11 footer, verbatim, in the human line and in the JSON.
    assert!(
        status_human.lines().any(|l| l == FOOTER),
        "no footer line: {status_human}"
    );
    let v: Value = serde_json::from_str(&status_json).unwrap();
    assert_eq!(v["data"]["footer"], FOOTER);
    // No output claims more than discoverability. The footer itself holds
    // "has read" and is cut out before the scan.
    let banned = [
        "has read",
        "was read",
        "agent read",
        "has loaded",
        "was loaded by",
        "is using",
        "now using",
    ];
    for (i, text) in outs.iter().enumerate() {
        let lowered = text.replace(FOOTER, "").to_lowercase();
        for phrase in banned {
            assert!(
                !lowered.contains(phrase),
                "output {i} says {phrase:?}: {text}"
            );
        }
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_36_renamed_source_skill_is_orphaned_with_candidates_and_blocked_by_default() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    // The deliberate change: rename the skill directory in the library.
    fs::rename(
        fx.lib.join("sdlc/code-review"),
        fx.lib.join("sdlc/code-reviewer"),
    )
    .unwrap();
    let after_rename = snapshot_source(&fx.lib);
    let (code, v) = fx.status(&p);
    assert_eq!(code, 8, "{v}");
    let r = record(&v, "code-review");
    assert_eq!(r["state"], "orphaned");
    assert_eq!(r["reason"], "source-skill-missing");
    assert_eq!(r["candidates"], json!(["fx:sdlc/code-reviewer"]));
    let (_, out, _) = fx.run(&p, &["status"]);
    assert!(out.contains("fx:sdlc/code-reviewer"), "{out}");
    assert!(
        out.contains("`pouch rm fx:sdlc/code-review` then `pouch add fx:sdlc/code-reviewer`"),
        "{out}"
    );
    // Blocked by default: the unresolved reference stops every change.
    let start = snapshot_project(&p);
    for args in [
        &["add", "fx:testing/pytest-fixtures", "--yes"][..],
        &["reconcile", "--repair", "--yes"],
    ] {
        let (code, v) = fx.json_raw(&p, args);
        assert_eq!(code, 4, "{args:?}: {v}");
        assert_eq!(v["errors"][0]["code"], "not-found");
        assert_project_unchanged(&start, &p);
    }
    fx.done_with(&fx.lib, &after_rename);
}

#[cfg(unix)]
#[test]
fn ac_39_source_edit_under_follow_is_visible_at_destination_without_drift_warning() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    // The deliberate change: edit the skill body in the library.
    write_file(
        &fx.lib,
        "sdlc/code-review/SKILL.md",
        b"---\nname: code-review\ndescription: Edited after activation.\n---\n\nA new body line.\n",
    );
    let after_edit = snapshot_source(&fx.lib);
    let seen = fs::read_to_string(p.join(".claude/skills/code-review/SKILL.md")).unwrap();
    assert!(seen.contains("A new body line."), "{seen}");
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    assert!(record_states(&v).iter().all(|(_, s)| s == "active"));
    let text = v.to_string();
    assert!(!text.contains("drift"), "{text}");
    let v = ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["status"], "no-changes");
    assert!(!v.to_string().contains("drift"));
    fx.done_with(&fx.lib, &after_edit);
}

#[cfg(unix)]
#[test]
fn ac_41_contradictory_skills_activate_and_no_semantic_conflict_claim_appears() {
    let fx = Fx::new_with(|lib| {
        write_file(
            lib,
            "style/tabs/SKILL.md",
            b"---\nname: tabs\ndescription: Indent with tabs.\n---\n\nAlways indent code with tabs. Never use spaces.\n",
        );
        write_file(
            lib,
            "style/spaces/SKILL.md",
            b"---\nname: spaces\ndescription: Indent with spaces.\n---\n\nNever indent code with tabs. Always use four spaces.\n",
        );
    });
    let p = fx.project("p");
    let mut outs: Vec<String> = Vec::new();
    let mut run = |args: &[&str]| {
        let (code, out, err) = fx.run(&p, args);
        outs.push(format!("{out}\n{err}"));
        code
    };
    assert_eq!(
        run(&["add", "fx:style/tabs", "fx:style/spaces", "--yes"]),
        0
    );
    assert_eq!(run(&["--json", "status"]), 0);
    assert_eq!(run(&["status"]), 0);
    assert_eq!(run(&["--json", "reconcile"]), 0);
    assert_eq!(run(&["reconcile"]), 0);
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0);
    assert_eq!(
        record_states(&v),
        vec![
            ("spaces".to_string(), "active".to_string()),
            ("tabs".to_string(), "active".to_string())
        ]
    );
    for (i, text) in outs.iter().enumerate() {
        let lowered = text.to_lowercase();
        for word in ["contradict", "incompatible", "semantic"] {
            assert!(!lowered.contains(word), "output {i} says {word:?}: {text}");
        }
    }
    fx.done();
}
