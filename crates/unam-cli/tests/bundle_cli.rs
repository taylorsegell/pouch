//! `unam bundle save|ls|show|edit|duplicate|rm|add` (spec 5; M2 plan CLI
//! Contract, P10 to P14, P21, P22). Every test runs in a `TestEnv`; every
//! `bundle save` test plants a `.git` marker in its project first.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn env_fx() -> (TestEnv, PathBuf, SourceSnapshot) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let (code, v) = run_json(&env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, before)
}

fn json_in(env: &TestEnv, dir: &Path, args: &[&str]) -> (i32, Value) {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (code, out, err) = run_in(env, dir, &full);
    let v: Value = serde_json::from_str(&out)
        .unwrap_or_else(|e| panic!("stdout is not JSON ({e}): {out:?}\nstderr: {err}"));
    (code, v)
}

fn bundle_path(env: &TestEnv, file: &str) -> PathBuf {
    env.path("bundles").join(file)
}

fn write_bundle(env: &TestEnv, file: &str, text: &str) -> PathBuf {
    let p = bundle_path(env, file);
    fs::write(&p, text).unwrap();
    p
}

/// A project with a `.git` marker and the given selection text.
fn project(env: &TestEnv, name: &str, selection: Option<&str>) -> PathBuf {
    let p = env.path("work").join(name);
    fs::create_dir_all(&p).unwrap();
    git_marker(&p);
    if let Some(text) = selection {
        write_file(&p, ".unam/selection.yaml", text.as_bytes());
    }
    p
}

fn err_code(v: &Value) -> &str {
    v["errors"][0]["code"].as_str().unwrap()
}

fn has_key(v: &Value, k: &str) -> bool {
    v["data"].get(k).is_some()
}

fn canon(p: &Path) -> String {
    fs::canonicalize(p).unwrap().to_string_lossy().into_owned()
}

const SEL_TWO: &str =
    "version: 1\ntargets:\n  claude-code:\n    add: [code-review, fx:pytest-fixtures]\n";

#[test]
fn bundle_ls_empty_and_populated() {
    let (env, lib, before) = env_fx();
    let (code, out, _) = env.run(&["bundle", "ls"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("No bundles in "), "{out}");
    let (code, v) = run_json(&env, &["bundle", "ls"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["bundles"], serde_json::json!([]));
    copy_fixture_bundle(&env, "hermes-only.yaml");
    copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(&env, &["bundle", "ls"]);
    assert_eq!(code, 0);
    let slugs: Vec<&str> = v["data"]["bundles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["slug"].as_str().unwrap())
        .collect();
    assert_eq!(slugs, vec!["commented", "hermes-only"]);
    assert_eq!(
        v["data"]["bundles"][0]["description"],
        "Backend feature work."
    );
    let (_, out, _) = env.run(&["bundle", "ls"]);
    assert!(out.starts_with("SLUG  "), "{out}");
    assert!(
        out.contains("DESCRIPTION") && out.contains("hermes-only"),
        "{out}"
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_ls_reports_broken_bundle_and_keeps_going() {
    let (env, lib, before) = env_fx();
    copy_fixture_bundle(&env, "hermes-only.yaml");
    write_bundle(&env, "bad.yaml", "skills: [unclosed\n");
    write_bundle(&env, "blank.yml", "name: '  '\nskills: [a]\n");
    let (code, v) = run_json(&env, &["bundle", "ls"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["bundles"].as_array().unwrap().len(), 1);
    let d = &v["data"]["diagnostics"];
    assert_eq!(
        diags_with_code(d, "bundle-invalid-yaml")[0]["path"],
        "bad.yaml"
    );
    assert_eq!(
        diags_with_code(d, "bundle-name-blank")[0]["path"],
        "blank.yml"
    );
    assert_eq!(
        diags_with_code(&v["warnings"], "bundle-invalid-yaml").len(),
        1
    );
    let (code, _, err) = env.run(&["bundle", "ls"]);
    assert_eq!(code, 0);
    assert!(
        err.contains("bundle-invalid-yaml") && err.contains("bad.yaml"),
        "{err}"
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_show_resolves_each_entry() {
    let (env, lib, before) = env_fx();
    copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(&env, &["bundle", "show", "commented"]);
    assert_eq!(code, 0, "{v}");
    let d = &v["data"];
    assert_eq!(d["slug"], "commented");
    assert_eq!(d["description"], "Backend feature work.");
    assert_eq!(d["instruction"], "Start with the review flow.");
    assert_eq!(d["problems"], 0);
    assert_eq!(d["skills"].as_array().unwrap().len(), 4);
    assert_eq!(d["skills"][3]["id"], "fx:alpha-root-skill");
    let (_, out, _) = env.run(&["bundle", "show", "Commented"]);
    assert!(
        out.contains("  fx:sdlc/code-review  ->  fx:sdlc/code-review"),
        "{out}"
    );
    write_bundle(
        &env,
        "mixed.yaml",
        "skills: [nope, error-handling, 'fx:']\n",
    );
    let (code, v) = run_json(&env, &["bundle", "show", "mixed"]);
    assert_eq!(code, 0, "{v}");
    let statuses: Vec<&str> = v["data"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, vec!["unresolved", "ambiguous", "invalid"]);
    assert_eq!(v["data"]["problems"], 3);
    assert!(v["data"]["skills"][0]["id"].is_null());
    let (_, out, _) = env.run(&["bundle", "show", "mixed"]);
    assert!(out.contains("  nope  !! unresolved: "), "{out}");
    assert!(
        out.contains("!! ambiguous: fx:engineering/backend/api/error-handling"),
        "{out}"
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_show_unknown_exit_4() {
    let (env, lib, before) = env_fx();
    let (code, v) = run_json(&env, &["bundle", "show", "nope"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(err_code(&v), "bundle-not-found");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_writes_qualified_refs() {
    let (env, lib, before) = env_fx();
    let p = project(&env, "proj", Some(SEL_TWO));
    let (code, v) = json_in(&env, &p, &["bundle", "save", "Team Set"]);
    assert_eq!(code, 0, "{v}");
    let d = &v["data"];
    assert_eq!(d["slug"], "team-set");
    assert_eq!(d["name"], "Team Set");
    assert_eq!(d["target"], "claude-code");
    assert_eq!(d["skill_count"], 2);
    assert_eq!(
        d["skills"],
        serde_json::json!(["fx:sdlc/code-review", "fx:testing/pytest-fixtures"])
    );
    assert_eq!(d["overwritten"], false);
    assert_eq!(d["project_root"].as_str().unwrap(), canon(&p));
    assert_eq!(
        fs::read_to_string(bundle_path(&env, "team-set.yaml")).unwrap(),
        "name: Team Set\nskills:\n  - fx:sdlc/code-review\n  - fx:testing/pytest-fixtures\n"
    );
    let (code, out, _) = run_in(&env, &p, &["bundle", "save", "other"]);
    assert_eq!(code, 0);
    assert!(
        out.starts_with("Saved bundle other (2 skills) -> "),
        "{out}"
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_refuses_existing_without_force() {
    let (env, lib, before) = env_fx();
    let p = project(&env, "proj", Some(SEL_TWO));
    assert_eq!(json_in(&env, &p, &["bundle", "save", "set"]).0, 0);
    let path = bundle_path(&env, "set.yaml");
    let original = fs::read(&path).unwrap();
    let (code, v) = json_in(&env, &p, &["bundle", "save", "SET"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(err_code(&v), "bundle-exists");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_force_overwrites() {
    let (env, lib, before) = env_fx();
    write_bundle(&env, "set.yaml", "name: set\nskills: [alpha-root-skill]\n");
    let p = project(&env, "proj", Some(SEL_TWO));
    let (code, v) = json_in(&env, &p, &["bundle", "save", "set", "--force"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["overwritten"], true);
    let text = fs::read_to_string(bundle_path(&env, "set.yaml")).unwrap();
    assert!(
        text.contains("fx:testing/pytest-fixtures") && !text.contains("alpha"),
        "{text}"
    );
    let (_, out, _) = run_in(&env, &p, &["bundle", "save", "set", "--force"]);
    assert!(out.trim_end().ends_with("(overwrote existing)"), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_without_selection_exit_4() {
    let (env, lib, before) = env_fx();
    let p = project(&env, "empty", None);
    let (code, v) = json_in(&env, &p, &["bundle", "save", "x"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(err_code(&v), "no-selection");
    let q = project(&env, "no-targets", Some("version: 1\n"));
    let (code, v) = json_in(&env, &q, &["bundle", "save", "x"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(err_code(&v), "no-selection");
    let r = project(&env, "bad", Some("version: 1\nbundels: []\n"));
    let (code, v) = json_in(&env, &r, &["bundle", "save", "x"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(err_code(&v), "selection-invalid");
    assert!(!bundle_path(&env, "x.yaml").exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_with_two_selection_entries_is_exit_2() {
    let (env, lib, before) = env_fx();
    let p = project(
        &env,
        "proj",
        Some(
            "targets:\n  claude-code:\n    add: [code-review]\n  agents-neutral:\n    add: [code-review]\n",
        ),
    );
    let (code, v) = json_in(&env, &p, &["bundle", "save", "x"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(err_code(&v), "target-ambiguous");
    assert!(
        v["errors"][0]["remedy"]
            .as_str()
            .unwrap()
            .contains("pass `--target <id>`")
    );
    assert!(!bundle_path(&env, "x.yaml").exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_blocked_by_unresolved_ref_exit_4() {
    let (env, lib, before) = env_fx();
    let p = project(
        &env,
        "proj",
        Some(
            "targets:\n  claude-code:\n    bundles: [missing]\n    add: [nope, code-review, error-handling]\n",
        ),
    );
    let (code, v) = json_in(&env, &p, &["bundle", "save", "x"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(err_code(&v), "bundle-not-found");
    let codes: Vec<&str> = v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert_eq!(
        codes,
        vec!["bundle-missing", "ref-not-found", "ref-ambiguous"]
    );
    let q = project(
        &env,
        "amb",
        Some("targets:\n  claude-code:\n    add: [error-handling]\n"),
    );
    assert_eq!(json_in(&env, &q, &["bundle", "save", "x"]).0, 5);
    assert!(!bundle_path(&env, "x.yaml").exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_save_root_detection_from_subdirectory() {
    let (env, lib, before) = env_fx();
    let p = project(&env, "proj", Some(SEL_TWO));
    let sub = p.join("src/nested");
    fs::create_dir_all(&sub).unwrap();
    let (code, v) = json_in(&env, &sub, &["bundle", "save", "from-sub"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["project_root"].as_str().unwrap(), canon(&p));
    assert_eq!(v["data"]["skill_count"], 2);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_duplicate_cases() {
    let (env, lib, before) = env_fx();
    copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(&env, &["bundle", "duplicate", "commented", "Copy One"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["source"], "commented");
    assert_eq!(v["data"]["slug"], "copy-one");
    assert_eq!(v["data"]["overwritten"], false);
    let original = fs::read_to_string(bundle_path(&env, "commented.yaml")).unwrap();
    assert_eq!(
        fs::read_to_string(bundle_path(&env, "copy-one.yaml")).unwrap(),
        original.replace("name: commented", "name: Copy One")
    );
    let (code, v) = run_json(&env, &["bundle", "duplicate", "commented", "copy one"]);
    assert_eq!((code, err_code(&v)), (2, "bundle-exists"));
    let (code, v) = run_json(
        &env,
        &["bundle", "duplicate", "commented", "copy one", "--force"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["overwritten"], true);
    let (code, v) = run_json(&env, &["bundle", "duplicate", "nope", "x"]);
    assert_eq!((code, err_code(&v)), (4, "bundle-not-found"));
    let (code, v) = run_json(&env, &["bundle", "duplicate", "commented", "!!!"]);
    assert_eq!((code, err_code(&v)), (2, "invalid-bundle-name"));
    let (_, out, _) = env.run(&["bundle", "duplicate", "commented", "third"]);
    assert!(out.starts_with("Copied commented -> third ("), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rm_deletes_and_lists_referencing_projects() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let p = project(
        &env,
        "proj",
        Some("targets:\n  claude-code:\n    bundles: [Commented]\n"),
    );
    let other = project(
        &env,
        "other",
        Some("targets:\n  claude-code:\n    bundles: [else]\n"),
    );
    write_project_index(&env, &[&p, &other]);
    let (code, v) = run_json(&env, &["bundle", "rm", "commented"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["deleted"], true);
    assert_eq!(v["data"]["slug"], "commented");
    assert_eq!(v["data"]["path"].as_str().unwrap(), path.to_string_lossy());
    assert_eq!(
        v["data"]["referencing_projects"],
        serde_json::json!([p.to_string_lossy()])
    );
    assert!(!has_key(&v, "removed"));
    assert!(!path.exists());
    // Human output names the path and the referencing project.
    copy_fixture_bundle(&env, "commented.yaml");
    let (code, out, _) = env.run(&["bundle", "rm", "commented"]);
    assert_eq!(code, 0);
    assert!(
        out.contains(&format!("Deleted bundle commented ({})", path.display())),
        "{out}"
    );
    assert!(out.contains(&p.to_string_lossy().into_owned()), "{out}");
    assert!(out.contains("no longer resolves"), "{out}");
    // The selection keeps its reference.
    let sel = fs::read_to_string(p.join(".unam/selection.yaml")).unwrap();
    assert!(sel.contains("Commented"));
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rm_deletes_only_the_loaded_file_when_name_differs_from_slug() {
    let (env, lib, before) = env_fx();
    let target = write_bundle(
        &env,
        "Backend Dev.yaml",
        "name: Backend Dev\nskills: [code-review]\n",
    );
    let s1 = write_bundle(&env, "other.yaml", "skills: [code-review]\n");
    let s2 = write_bundle(
        &env,
        "backend-dev-copy.yaml",
        "skills: [alpha-root-skill]\n",
    );
    let before_bytes = (fs::read(&s1).unwrap(), fs::read(&s2).unwrap());
    let (code, v) = run_json(&env, &["bundle", "rm", "backend-dev"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["data"]["path"].as_str().unwrap(),
        target.to_string_lossy()
    );
    assert!(!target.exists());
    assert_eq!(
        (fs::read(&s1).unwrap(), fs::read(&s2).unwrap()),
        before_bytes
    );
    assert!(!bundle_path(&env, "backend-dev.yaml").exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rm_forms_are_separate() {
    let (env, lib, before) = env_fx();
    let one = write_bundle(
        &env,
        "one.yaml",
        "name: one\nskills:\n  - alpha-root-skill\n",
    );
    let original = fs::read(&one).unwrap();
    // The reference form never deletes, even when it refuses.
    let (code, v) = run_json(&env, &["bundle", "rm", "one", "alpha-root-skill"]);
    assert_eq!((code, err_code(&v)), (2, "bundle-would-be-empty"));
    assert!(!has_key(&v, "deleted"));
    assert_eq!(fs::read(&one).unwrap(), original);
    // The delete form never edits entries: it removes the file.
    let (code, v) = run_json(&env, &["bundle", "rm", "one"]);
    assert_eq!(code, 0, "{v}");
    assert!(!has_key(&v, "removed"));
    assert!(!one.exists());
    assert_source_unchanged(&before, &lib);
}

#[cfg(unix)]
#[test]
fn bundle_symlinked_manifest_cases() {
    let (env, lib, before) = env_fx();
    let outside = env.path("work").join("dotfiles").join("linked.yaml");
    fs::create_dir_all(outside.parent().unwrap()).unwrap();
    let body = "name: linked\nskills:\n  - code-review\n  - alpha-root-skill\n";
    fs::write(&outside, body).unwrap();
    let link = bundle_path(&env, "linked.yaml");
    std::os::unix::fs::symlink(&outside, &link).unwrap();
    let marker = env.path("work").join("editor-ran");
    let editor = write_scripted_editor(&env, &format!("touch '{}'", marker.display()));
    let p = project(&env, "proj", Some(SEL_TWO));
    let check = |code: i32, v: &Value| {
        assert_eq!((code, err_code(v)), (2, "bundle-edit-unsupported"), "{v}");
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&outside).unwrap(), body);
    };
    for args in [
        vec!["bundle", "add", "linked", "fx:data/schema-migration"],
        vec!["bundle", "rm", "linked", "code-review"],
        vec!["bundle", "rm", "linked", "code-review", "--rewrite"],
        vec!["bundle", "duplicate", "linked", "linked", "--force"],
    ] {
        let (code, v) = run_json(&env, &args);
        check(code, &v);
    }
    let (code, v) = json_in(&env, &p, &["bundle", "save", "linked", "--force"]);
    check(code, &v);
    let out = env
        .cmd()
        .env("EDITOR", &editor)
        .args(["--json", "bundle", "edit", "linked"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    check(out.status.code().unwrap(), &v);
    assert!(!marker.exists(), "the editor must never start");
    // Delete unlinks only the link.
    let (code, v) = run_json(&env, &["bundle", "rm", "linked"]);
    assert_eq!(code, 0, "{v}");
    assert!(fs::symlink_metadata(&link).is_err());
    assert_eq!(fs::read_to_string(&outside).unwrap(), body);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rm_unknown_exit_4() {
    let (env, lib, before) = env_fx();
    for args in [
        vec!["bundle", "rm", "nope"],
        vec!["bundle", "rm", "nope", "code-review"],
    ] {
        let (code, v) = run_json(&env, &args);
        assert_eq!((code, err_code(&v)), (4, "bundle-not-found"), "{args:?}");
    }
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_add_cases() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(
        &env,
        &[
            "bundle",
            "add",
            "commented",
            "fx:engineering/frontend/error-handling/",
            "fx:alpha-root-skill",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["data"]["added"],
        serde_json::json!([
            "fx:engineering/frontend/error-handling",
            "fx:alpha-root-skill"
        ])
    );
    assert_eq!(v["data"]["skill_count"], 6);
    assert_eq!(v["data"]["rewrite"], false);
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("  - alpha-root-skill\n  - fx:engineering/frontend/error-handling\n  - fx:alpha-root-skill\ninstruction:"), "{text}");
    assert!(text.contains("# primary review flow") && text.contains("# testing helpers"));
    let (code, out, _) = env.run(&["bundle", "add", "commented", "alpha-root-skill"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Added 0 skill(s) to commented")
            && out.contains("Already present: alpha-root-skill"),
        "{out}"
    );
    let snapshot = fs::read(&path).unwrap();
    let (code, v) = run_json(&env, &["bundle", "add", "commented", "nope"]);
    assert_eq!((code, err_code(&v)), (4, "not-found"));
    let (code, v) = run_json(&env, &["bundle", "add", "commented", "fx:"]);
    assert_eq!((code, err_code(&v)), (2, "invalid-ref"));
    let (code, v) = run_json(&env, &["bundle", "add", "nope", "code-review"]);
    assert_eq!((code, err_code(&v)), (4, "bundle-not-found"));
    let (code, _, _) = env.run(&["bundle", "add", "commented"]);
    assert_eq!(code, 2, "references are required");
    assert_eq!(fs::read(&path).unwrap(), snapshot);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_add_ambiguous_exit_5_no_write() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "hermes-only.yaml");
    let original = fs::read(&path).unwrap();
    let (code, v) = run_json(
        &env,
        &[
            "bundle",
            "add",
            "hermes-only",
            "fx:sdlc/code-review",
            "error-handling",
        ],
    );
    assert_eq!((code, err_code(&v)), (5, "ambiguous"));
    assert_eq!(v["errors"][0]["candidates"].as_array().unwrap().len(), 2);
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rm_skills_cases() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(&env, &["bundle", "rm", "commented", "code-review"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        v["data"]["removed"],
        serde_json::json!(["fx:sdlc/code-review"])
    );
    assert_eq!(v["data"]["skill_count"], 3);
    assert!(!has_key(&v, "deleted"));
    assert!(
        !fs::read_to_string(&path)
            .unwrap()
            .contains("fx:sdlc/code-review")
    );
    let (code, v) = run_json(&env, &["bundle", "rm", "commented", "fx:sdlc/code-review"]);
    assert_eq!((code, err_code(&v)), (4, "skill-not-in-bundle"));
    let (code, v) = run_json(
        &env,
        &[
            "bundle",
            "rm",
            "commented",
            "pytest-fixtures",
            "schema-migration",
            "alpha-root-skill",
        ],
    );
    assert_eq!((code, err_code(&v)), (2, "bundle-would-be-empty"));
    let (code, out, _) = env.run(&[
        "bundle",
        "rm",
        "commented",
        "pytest-fixtures",
        "fx:schema-migration",
    ]);
    assert_eq!(code, 0);
    assert!(out.starts_with("Removed 2 entries from commented"), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[cfg(unix)]
fn edit_with(env: &TestEnv, editor: &Path, name: &str) -> (i32, Value) {
    let out = env
        .cmd()
        .env("EDITOR", editor)
        .args(["--json", "bundle", "edit", name])
        .output()
        .unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_slice(&out.stdout).unwrap(),
    )
}

#[cfg(unix)]
fn edit_dir_empty(env: &TestEnv) -> bool {
    fs::read_dir(env.path("cache").join("edit"))
        .map(|r| r.count() == 0)
        .unwrap_or(true)
}

#[cfg(unix)]
#[test]
fn bundle_edit_with_scripted_editor_commits_valid_change() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let ed = write_scripted_editor(
        &env,
        "printf 'name: commented\\nskills:\\n  - alpha-root-skill\\n  - nope\\n' > \"$1\"",
    );
    let (code, v) = edit_with(&env, &ed, "commented");
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["changed"], true);
    assert_eq!(v["data"]["problems"], 1);
    assert_eq!(diags_with_code(&v["warnings"], "ref-not-found").len(), 1);
    assert_eq!(
        fs::read_to_string(&path).unwrap(),
        "name: commented\nskills:\n  - alpha-root-skill\n  - nope\n"
    );
    assert!(edit_dir_empty(&env));
    let noop = write_scripted_editor(&env, "true");
    let (code, v) = edit_with(&env, &noop, "commented");
    assert_eq!(code, 0);
    assert_eq!(v["data"]["changed"], false);
    assert_source_unchanged(&before, &lib);
}

#[cfg(unix)]
#[test]
fn bundle_edit_invalid_result_leaves_file_unchanged() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let original = fs::read(&path).unwrap();
    for body in [
        "printf 'skills: [\\n' > \"$1\"",
        "printf 'name: commented\\nskills: []\\n' > \"$1\"",
    ] {
        let ed = write_scripted_editor(&env, body);
        let (code, v) = edit_with(&env, &ed, "commented");
        assert_eq!((code, err_code(&v)), (1, "bundle-invalid"), "{body}");
        assert_eq!(fs::read(&path).unwrap(), original);
        assert!(edit_dir_empty(&env));
    }
    assert_source_unchanged(&before, &lib);
}

#[cfg(unix)]
#[test]
fn bundle_edit_editor_failure_leaves_file_unchanged() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let original = fs::read(&path).unwrap();
    let ed = write_scripted_editor(&env, "printf 'name: x\\nskills: [a]\\n' > \"$1\"\nexit 7");
    let (code, v) = edit_with(&env, &ed, "commented");
    assert_eq!((code, err_code(&v)), (1, "editor-failed"));
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(edit_dir_empty(&env));
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_edit_unset_editor_exit_2() {
    let (env, lib, before) = env_fx();
    copy_fixture_bundle(&env, "commented.yaml");
    let (code, v) = run_json(&env, &["bundle", "edit", "commented"]);
    assert_eq!((code, err_code(&v)), (2, "editor-not-set"));
    let (code, v) = run_json(&env, &["bundle", "edit", "nope"]);
    assert_eq!((code, err_code(&v)), (4, "bundle-not-found"));
    assert_source_unchanged(&before, &lib);
}

#[cfg(unix)]
#[test]
fn bundle_edit_slug_change_refused() {
    let (env, lib, before) = env_fx();
    let path = copy_fixture_bundle(&env, "commented.yaml");
    let original = fs::read(&path).unwrap();
    let ed = write_scripted_editor(
        &env,
        "printf 'name: renamed\\nskills: [code-review]\\n' > \"$1\"",
    );
    let (code, v) = edit_with(&env, &ed, "commented");
    assert_eq!((code, err_code(&v)), (1, "bundle-invalid"));
    assert!(
        v["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains("renaming"),
        "{v}"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(!bundle_path(&env, "renamed.yaml").exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_rewrite_flag_cases() {
    let (env, lib, before) = env_fx();
    let flow = "# flow style\nname: flow\nskills: [code-review, alpha-root-skill, 'fx:data/schema-migration']\nextra: {k: v}\n";
    let path = write_bundle(&env, "flow.yaml", flow);
    let (code, v) = run_json(&env, &["bundle", "rm", "flow", "code-review"]);
    assert_eq!((code, err_code(&v)), (2, "bundle-edit-unsupported"));
    assert!(
        v["errors"][0]["remedy"]
            .as_str()
            .unwrap()
            .contains("--rewrite")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), flow);
    let (code, v) = run_json(&env, &["bundle", "rm", "flow", "code-review", "--rewrite"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["rewrite"], true);
    assert_eq!(v["data"]["removed"], serde_json::json!(["code-review"]));
    let text = fs::read_to_string(&path).unwrap();
    assert!(
        !text.contains("# flow style") && text.contains("extra:"),
        "{text}"
    );
    let (code, v) = run_json(
        &env,
        &["bundle", "add", "flow", "pytest-fixtures", "--rewrite"],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["skill_count"], 3);
    // `--rewrite` requires references.
    let (code, _, err) = env.run(&["bundle", "rm", "flow", "--rewrite"]);
    assert_eq!(code, 2, "{err}");
    assert!(path.exists());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_edit_unsupported_shapes_refused() {
    let (env, lib, before) = env_fx();
    for (file, text) in [
        ("flow.yaml", "skills: [code-review, alpha-root-skill]\n"),
        (
            "multi.yaml",
            "skills:\n  - code-review\n  - alpha-root\n    -skill\n",
        ),
        (
            "anchor.yaml",
            "skills: &list\n  - code-review\n  - alpha-root-skill\n",
        ),
    ] {
        let path = write_bundle(&env, file, text);
        let name = file.trim_end_matches(".yaml");
        for args in [
            vec!["bundle", "rm", name, "code-review"],
            vec!["bundle", "add", name, "pytest-fixtures"],
        ] {
            let (code, v) = run_json(&env, &args);
            assert_eq!(
                (code, err_code(&v)),
                (2, "bundle-edit-unsupported"),
                "{file}: {v}"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), text);
        }
    }
    assert_source_unchanged(&before, &lib);
}

#[test]
fn bundle_commands_leave_sources_untouched() {
    let (env, lib, before) = env_fx();
    let lib2 = build_second_library(&env);
    let before2 = snapshot_source(&lib2);
    assert_eq!(
        run_json(&env, &["source", "add", "hs", lib2.to_str().unwrap()]).0,
        0
    );
    copy_fixture_bundle(&env, "hermes-only.yaml");
    copy_fixture_bundle(&env, "commented.yaml");
    let p = project(
        &env,
        "proj",
        Some(
            "targets:\n  claude-code:\n    bundles: [hermes-only]\n    add: [hs:ops/deploy-checklist]\n",
        ),
    );
    assert_eq!(json_in(&env, &p, &["bundle", "save", "saved"]).0, 0);
    for args in [
        vec!["bundle", "ls"],
        vec!["bundle", "show", "commented"],
        vec!["bundle", "duplicate", "commented", "dup"],
        vec!["bundle", "add", "dup", "hs:api/rest-design"],
        vec!["bundle", "rm", "dup", "hs:api/rest-design"],
        vec!["bundle", "add", "dup", "schema-migration"],
        vec!["bundle", "rm", "dup", "code-review", "--rewrite"],
        vec!["bundle", "rm", "dup"],
        vec!["bundle", "edit", "saved"],
        vec!["show", "fx:sdlc/code-review"],
    ] {
        let (code, v) = run_json(&env, &args);
        assert!([0, 2, 5].contains(&code), "{args:?}: {v}");
    }
    #[cfg(unix)]
    {
        let ed = write_scripted_editor(
            &env,
            "printf 'name: saved\\nskills: [code-review]\\n' > \"$1\"",
        );
        assert_eq!(edit_with(&env, &ed, "saved").0, 0);
    }
    assert_source_unchanged(&before, &lib);
    assert_source_unchanged(&before2, &lib2);
}
