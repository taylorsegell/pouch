//! `--json` output snapshots (spec 8). Temp paths are normalized to `[TMP]`
//! before parsing; timestamps are redacted. Run with `INSTA_UPDATE=no` in
//! every gate so a missing or changed snapshot fails.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;

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
    let out = env
        .cmd()
        .env("EDITOR", &editor)
        .args(["--json", "bundle", "edit", "commented"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let text = env.normalize(&String::from_utf8(out.stdout).unwrap());
    let v: Value = serde_json::from_str(&text).unwrap();
    insta::assert_json_snapshot!("bundle_edit", v);
}
