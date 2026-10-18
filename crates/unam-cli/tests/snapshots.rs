//! `--json` output snapshots (spec 8). Temp paths are normalized to `[TMP]`
//! before parsing; timestamps are redacted. Run with `INSTA_UPDATE=no` in
//! every gate so a missing or changed snapshot fails.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::*;
use common::*;
use serde_json::Value;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use unam_core::change::{self, Decision, Edit};

fn json(env: &TestEnv, args: &[&str]) -> (i32, Value) {
    let (code, v) = run_json(env, args);
    let text = env.normalize(&serde_json::to_string(&v).unwrap());
    (code, serde_json::from_str(&text).unwrap())
}

fn setup() -> (TestEnv, String) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let lib = lib.to_str().unwrap().to_string();
    (env, lib)
}

#[test]
fn add_success() {
    let (env, lib) = setup();
    let (code, v) = json(&env, &["source", "add", "fx", &lib]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["diagnostics"].as_array().unwrap().len(), 6);
    insta::assert_json_snapshot!("add_success", v);
}

#[test]
fn add_bad_path() {
    let (env, _lib) = setup();
    let missing = env.path("work").join("missing");
    let (code, v) = json(&env, &["source", "add", "fx", missing.to_str().unwrap()]);
    assert_eq!(code, 2);
    insta::assert_json_snapshot!("add_bad_path", v);
}

#[test]
fn list() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["source", "list"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("list", v, { ".data.sources[].last_scan" => "[last_scan]" });
}

#[test]
fn rm_success() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["source", "rm", "fx"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("rm_success", v);
}

#[test]
fn rm_referenced() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let proj = env.path("work").join("project");
    write_file(
        &proj,
        ".unam/selection.yaml",
        b"version: 1\ntargets:\n  claude-code:\n    add: [\"fx:sdlc/code-review\"]\n",
    );
    let index = env.project_index();
    write_file(
        index.parent().unwrap(),
        "projects.json",
        serde_json::json!({"version": 1, "projects": [proj]})
            .to_string()
            .as_bytes(),
    );
    let (code, v) = json(&env, &["source", "rm", "fx"]);
    assert_eq!(code, 3);
    insta::assert_json_snapshot!("rm_referenced", v);
}

#[test]
fn search_review() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["search", "review"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("search_review", v);
}

#[test]
fn show_found() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["show", "fx:sdlc/code-review"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("show_found", v);
}

#[test]
fn show_not_found() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["show", "code-reviw"]);
    assert_eq!(code, 4);
    insta::assert_json_snapshot!("show_not_found", v);
}

#[test]
fn show_ambiguous() {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    let (code, v) = json(&env, &["show", "error-handling"]);
    assert_eq!(code, 5);
    insta::assert_json_snapshot!("show_ambiguous", v);
}

#[test]
fn usage_bare() {
    let env = TestEnv::new();
    let (code, out, _) = env.run(&[]);
    assert_eq!(code, 0);
    insta::assert_snapshot!("usage_bare", out);
}

// ---- milestone M2: bundle commands ----

fn json_in(env: &TestEnv, dir: &std::path::Path, args: &[&str]) -> (i32, Value) {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (code, out, err) = run_in(env, dir, &full);
    let v: Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}\nstderr: {err}"));
    let text = env.normalize(&serde_json::to_string(&v).unwrap());
    (code, serde_json::from_str(&text).unwrap())
}

/// `fx` registered plus the `commented` fixture bundle.
fn bundle_setup() -> TestEnv {
    let (env, lib) = setup();
    json(&env, &["source", "add", "fx", &lib]);
    copy_fixture_bundle(&env, "commented.yaml");
    env
}

#[test]
fn bundle_save() {
    let env = bundle_setup();
    let proj = env.path("work").join("project");
    git_marker(&proj);
    write_file(
        &proj,
        ".unam/selection.yaml",
        b"version: 1\ntargets:\n  claude-code:\n    add: [code-review, fx:pytest-fixtures]\n",
    );
    let (code, v) = json_in(&env, &proj, &["bundle", "save", "Team Set"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("bundle_save", v);
}

#[test]
fn bundle_ls() {
    let env = bundle_setup();
    copy_fixture_bundle(&env, "hermes-only.yaml");
    let (code, v) = json(&env, &["bundle", "ls"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("bundle_ls", v);
}

#[test]
fn bundle_show() {
    let env = bundle_setup();
    let (code, v) = json(&env, &["bundle", "show", "commented"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("bundle_show", v);
}

#[test]
fn bundle_show_not_found() {
    let env = bundle_setup();
    let (code, v) = json(&env, &["bundle", "show", "Back End"]);
    assert_eq!(code, 4);
    insta::assert_json_snapshot!("bundle_show_not_found", v);
}

#[test]
fn bundle_add() {
    let env = bundle_setup();
    let (code, v) = json(
        &env,
        &[
            "bundle",
            "add",
            "commented",
            "fx:engineering/frontend/error-handling",
            "alpha-root-skill",
        ],
    );
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("bundle_add", v);
}

#[test]
fn bundle_rm_skill() {
    let env = bundle_setup();
    let (code, v) = json(
        &env,
        &["bundle", "rm", "commented", "fx:data/schema-migration"],
    );
    assert_eq!(code, 0);
    assert!(v["data"].get("deleted").is_none());
    insta::assert_json_snapshot!("bundle_rm_skill", v);
}

#[test]
fn bundle_rm_bundle() {
    let env = bundle_setup();
    let proj = env.path("work").join("project");
    write_file(
        &proj,
        ".unam/selection.yaml",
        b"version: 1\ntargets:\n  claude-code:\n    bundles: [commented]\n",
    );
    write_project_index(&env, &[&proj]);
    let (code, v) = json(&env, &["bundle", "rm", "commented"]);
    assert_eq!(code, 0);
    assert!(v["data"].get("removed").is_none());
    insta::assert_json_snapshot!("bundle_rm_bundle", v);
}

#[test]
fn bundle_duplicate() {
    let env = bundle_setup();
    let (code, v) = json(
        &env,
        &["bundle", "duplicate", "commented", "Commented Copy"],
    );
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("bundle_duplicate", v);
}

#[cfg(unix)]
#[test]
fn bundle_edit() {
    let env = bundle_setup();
    let editor = write_scripted_editor(
        &env,
        "printf 'name: commented\\ndescription: Edited.\\nskills:\\n  - alpha-root-skill\\n' > \"$1\"",
    );
    let mut cmd = env.cmd();
    cmd.env("EDITOR", &editor)
        .args(["--json", "bundle", "edit", "commented"]);
    let out = common::gated_output(cmd);
    assert_eq!(out.status.code(), Some(0));
    let text = env.normalize(&String::from_utf8(out.stdout).unwrap());
    let v: Value = serde_json::from_str(&text).unwrap();
    insta::assert_json_snapshot!("bundle_edit", v);
}

// ---- milestone M3: activation commands (`[TMP]`, `[OP_ID]`, `[TIME]`) ----

fn crash_add(fx: &Fx, p: &Path, refs: &[&str], mode: FailMode) {
    quiet_crashes();
    let c = core_ctx(&fx.env, p);
    let clock = StepClock::new();
    let req = c.request(
        Edit::Add(refs.iter().map(|s| s.to_string()).collect()),
        "add",
        &clock,
    );
    let ff = FailingFs::new(mode);
    let _ = catch_unwind(AssertUnwindSafe(|| {
        common::in_lock_section(|| change::run(&req, &ff, &mut |_| Decision::Proceed))
    }));
}

#[test]
fn add_apply() {
    let fx = Fx::new();
    let p = fx.project("p");
    let (code, v) = fx.json(
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("add_apply", v);
    fx.done();
}

#[test]
fn add_no_changes() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    let (code, v) = fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("add_no_changes", v);
    fx.done();
}

#[test]
fn add_conflict() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.plant_foreign(&p, "code-review");
    let (code, v) = fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 6);
    insta::assert_json_snapshot!("add_conflict", v);
    fx.done();
}

#[test]
fn rm_apply() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let (code, v) = fx.json(&p, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("rm_apply", v);
    fx.done();
}

#[test]
fn use_replace() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(&p, &["add", "fx:data/schema-migration", "--yes"]);
    let (code, v) = fx.json(&p, &["use", "hermes-only", "--replace", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("use_replace", v);
    fx.done();
}

#[test]
fn deactivate_apply() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    let (code, v) = fx.json(&p, &["deactivate", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("deactivate_apply", v);
    fx.done();
}

#[test]
fn apply_partial_failure() {
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
    let (code, v) = fx.json(
        &p,
        &["add", "fx:bad/escaper", "fx:sdlc/code-review", "--yes"],
    );
    assert_eq!(code, 7);
    insta::assert_json_snapshot!("apply_partial_failure", v);
    fx.done();
}

#[test]
fn refused_without_yes() {
    let fx = Fx::new();
    let p = fx.project("p");
    let (code, v) = fx.json(&p, &["add", "fx:sdlc/code-review"]);
    assert_eq!(code, 2);
    insta::assert_json_snapshot!("refused_without_yes", v);
    fx.done();
}

#[test]
fn interrupted_exit_10() {
    let fx = Fx::new();
    let p = fx.project("p");
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review", "fx:testing/pytest-fixtures"],
        FailMode::CrashAfter(5),
    );
    let (code, v) = fx.json(&p, &["add", "fx:data/schema-migration", "--yes"]);
    assert_eq!(code, 10);
    insta::assert_json_snapshot!("interrupted_exit_10", v);
    fx.done();
}

#[test]
fn locked_exit_11() {
    let fx = Fx::new();
    let p = fx.project("p");
    let clock = StepClock::new();
    let _guard = unam_core::state::ProjectLock::acquire(&fx.canon(&p), "test", &clock).unwrap();
    let (code, v) = fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 11);
    assert_eq!(v["data"]["locked"]["pid"], std::process::id());
    // The holder pid is this test process: replace it in the data and in
    // the message text.
    let pid = std::process::id().to_string();
    let text = v
        .to_string()
        .replace(&format!(r#""pid":{pid}"#), r#""pid":"[PID]""#)
        .replace(&format!("process {pid}"), "process [PID]");
    let v: Value = serde_json::from_str(&text).unwrap();
    insta::assert_json_snapshot!("locked_exit_11", v);
    fx.done();
}

#[test]
fn resume_apply() {
    let fx = Fx::new();
    let p = fx.project("p");
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review", "fx:testing/pytest-fixtures"],
        FailMode::CrashAfter(8),
    );
    let (code, v) = fx.json(&p, &["resume", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("resume_apply", v);
    fx.done();
}

#[test]
fn rollback_apply() {
    let fx = Fx::new();
    let p = fx.project("p");
    crash_add(
        &fx,
        &p,
        &["fx:sdlc/code-review", "fx:testing/pytest-fixtures"],
        FailMode::CrashAfter(8),
    );
    let (code, v) = fx.json(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("rollback_apply", v);
    fx.done();
}

// ---- milestone M4: status, reconcile, history rollback, doctor, relocate ----

/// A project showing most states: `code-review` active, `pytest-fixtures`
/// shadowed by a user-scope skill, `alpha-root-skill` orphaned (its link
/// deleted by hand), `house-style` foreign, `rest-design` selected only.
#[cfg(unix)]
fn mixed_states_project(fx: &Fx) -> std::path::PathBuf {
    let p = fx.project("p");
    fx.json(&p, &["use", "hermes-only", "--replace", "--yes"]);
    fx.plant_foreign(&p, "house-style");
    fx.plant_user_skill("pytest-fixtures");
    std::fs::remove_file(p.join(".claude/skills/alpha-root-skill")).unwrap();
    fx.write_selection(
        &p,
        "version: 1\ntargets:\n  claude-code:\n    bundles:\n      - hermes-only\n    add:\n      - hs:api/rest-design\n",
    );
    p
}

#[cfg(unix)]
#[test]
fn status_json() {
    let fx = Fx::new();
    let p = mixed_states_project(&fx);
    let (code, v) = fx.json(&p, &["status"]);
    assert_eq!(code, 8);
    insta::assert_json_snapshot!("status_json", v);
    fx.done();
}

#[cfg(unix)]
#[test]
fn reconcile_json() {
    let fx = Fx::new();
    let p = mixed_states_project(&fx);
    let (code, v) = fx.json(&p, &["reconcile"]);
    assert_eq!(code, 8);
    insta::assert_json_snapshot!("reconcile_json", v);
    fx.done();
}

#[cfg(unix)]
#[test]
fn rollback_history_json() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    fx.json(&p, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    let (code, v) = fx.json(&p, &["rollback", "--yes"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("rollback_history_json", v);
    fx.done();
}

#[cfg(unix)]
#[test]
fn doctor_json() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    let (code, v) = fx.json(&p, &["doctor"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("doctor_json", v);
    fx.done();
}

#[cfg(unix)]
#[test]
fn source_relocate_json() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let new_lib = fx.env.path("work/lib-moved");
    std::fs::rename(&fx.lib, &new_lib).unwrap();
    let (code, v) = fx.json(
        &p,
        &[
            "source",
            "relocate",
            "fx",
            new_lib.to_str().unwrap(),
            "--yes",
        ],
    );
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("source_relocate_json", v);
    fx.done_at(&new_lib);
}

// ---- milestone M5: agents-neutral and generated briefs ----

#[test]
fn plan_agents_neutral_json() {
    let fx = Fx::new();
    let p = fx.project("p");
    // Refused without `--yes`: the plan, its consumers, and nothing applied.
    let (code, v) = fx.json(
        &p,
        &["--target", "agents-neutral", "add", "fx:sdlc/code-review"],
    );
    assert_eq!(code, 2);
    insta::assert_json_snapshot!("plan_agents_neutral_json", v);
    fx.done();
}

#[test]
fn status_agents_neutral_json() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.json(
        &p,
        &[
            "--target",
            "agents-neutral",
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let (code, v) = fx.json(&p, &["--target", "agents-neutral", "status"]);
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("status_agents_neutral_json", v);
    fx.done();
}

#[test]
fn add_brief_json() {
    let fx = Fx::new();
    // The unmodified fixture: an instruction and no opt-out (EX1).
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    let p = fx.project("p");
    let (code, v) = fx.json(
        &p,
        &[
            "--target",
            "agents-neutral",
            "use",
            "hermes-only",
            "--replace",
            "--yes",
        ],
    );
    assert_eq!(code, 0);
    insta::assert_json_snapshot!("add_brief_json", v);
    fx.done();
}
