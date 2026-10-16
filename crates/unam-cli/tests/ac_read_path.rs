//! Acceptance criteria closed by milestone M1 (PRD 13.2): AC-1, AC-3, AC-4,
//! AC-5, AC-6, AC-38 (read side). Every test that keeps its source asserts the
//! source tree is unchanged.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;

const EXPECTED_IDS: [&str; 9] = [
    "fx:alpha-root-skill",
    "fx:bom-crlf/windows-skill",
    "fx:colon/value-with-colon",
    "fx:data/schema-migration",
    "fx:engineering/backend/api/error-handling",
    "fx:engineering/frontend/error-handling",
    "fx:mismatch/dir-name-differs",
    "fx:sdlc/code-review",
    "fx:testing/pytest-fixtures",
];

fn add(env: &TestEnv, lib: &std::path::Path) -> Value {
    let (code, v) = run_json(env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    v
}

fn all_ids(env: &TestEnv) -> Vec<String> {
    let (code, v) = run_json(env, &["search", ""]);
    assert_eq!(code, 0, "{v}");
    result_ids(&v)
}

#[test]
fn ac_01_register_non_git_directory_and_skills_discoverable() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    assert!(!lib.join(".git").exists());
    let before = snapshot_source(&lib);
    let v = add(&env, &lib);
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["kind"], "directory");
    assert_eq!(v["data"]["skill_count"], 9);
    let (code, s) = run_json(&env, &["search", "code-review"]);
    assert_eq!(code, 0);
    assert!(
        result_ids(&s).contains(&"fx:sdlc/code-review".to_string()),
        "{s}"
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_03_three_level_nested_categories_real_paths() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    add(&env, &lib);
    let ids = all_ids(&env);
    assert_eq!(ids, EXPECTED_IDS.to_vec());
    let (code, v) = run_json(&env, &["show", "fx:engineering/backend/api/error-handling"]);
    assert_eq!(code, 0, "{v}");
    let d = &v["data"];
    assert_eq!(d["category"], "engineering/backend/api");
    assert_eq!(d["rel_path"], "engineering/backend/api/error-handling");
    assert_eq!(d["name"], "error-handling");
    let shown = std::path::PathBuf::from(d["path"].as_str().unwrap());
    assert!(shown.join("SKILL.md").is_file());
    assert!(shown.ends_with("engineering/backend/api/error-handling"));
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_04_git_and_node_modules_yield_no_skills() {
    let env = TestEnv::new();
    let lib = build_library_with_root_git(&env);
    for p in [
        ".git/hooks/SKILL.md",
        "node_modules/pkg/SKILL.md",
        "vendor/node_modules/deep/SKILL.md",
        "vendor/some-repo/.git/SKILL.md",
    ] {
        assert!(lib.join(p).is_file(), "{p}");
    }
    let before = snapshot_source(&lib);
    let v = add(&env, &lib);
    assert_eq!(v["data"]["kind"], "git-repo");
    assert_eq!(v["data"]["skill_count"], 9);
    assert_eq!(all_ids(&env), EXPECTED_IDS.to_vec());
    let (_, s) = run_json(&env, &["search", "hidden-review"]);
    assert!(
        result_ids(&s)
            .iter()
            .all(|id| !id.contains("node_modules") && !id.contains(".git")),
        "{s}"
    );
    for r in [
        "fx:node_modules/pkg",
        "fx:vendor/node_modules/deep",
        "fx:vendor/some-repo/.git",
        "fx:.git/hooks",
        "hidden-review",
        "git-hook",
    ] {
        let (code, out) = run_json(&env, &["show", r]);
        assert_ne!(code, 0, "{r} must not resolve: {out}");
    }
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let v = add(&env, &lib);
    let diags = &v["data"]["diagnostics"];
    let bad = diags_with_code(diags, "unparseable-yaml");
    assert_eq!(bad.len(), 1, "{diags}");
    assert_eq!(bad[0]["severity"], "error");
    assert_eq!(bad[0]["path"], "broken/bad-yaml");
    let nodesc = diags_with_code(diags, "missing-description");
    assert_eq!(nodesc.len(), 1);
    assert_eq!(nodesc[0]["path"], "broken/no-description");

    let (code, s) = run_json(&env, &["show", "fx:mismatch/dir-name-differs"]);
    assert_eq!(code, 0, "{s}");
    assert_eq!(s["data"]["name"], "other-name");
    let mm = diags_with_code(&s["data"]["diagnostics"], "name-dir-mismatch");
    assert_eq!(mm.len(), 1);
    assert_eq!(mm[0]["severity"], "warning");

    let (code, s) = run_json(&env, &["show", "fx:broken/bad-yaml"]);
    assert_eq!(code, 4, "{s}");
    let (code, _) = run_json(&env, &["show", "fx:broken/no-description"]);
    assert_eq!(code, 4);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_06_search_spans_three_categories_one_result_set() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    add(&env, &lib);
    let (code, v) = run_json(&env, &["search", "review"]);
    assert_eq!(code, 0);
    let results = v["data"]["results"].as_array().unwrap();
    let mut categories = Vec::new();
    for want in [
        "fx:sdlc/code-review",
        "fx:testing/pytest-fixtures",
        "fx:data/schema-migration",
    ] {
        let r = results
            .iter()
            .find(|r| r["id"] == want)
            .unwrap_or_else(|| panic!("{want} missing from {v}"));
        categories.push(r["category"].as_str().unwrap().to_string());
    }
    assert_eq!(categories, vec!["sdlc", "testing", "data"]);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_38_unreachable_source_reported_no_mutation() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    add(&env, &lib);
    std::fs::remove_dir_all(&lib).unwrap();
    let config_before = std::fs::read(env.config_file()).unwrap();
    let cache_before = std::fs::read(env.cache_file("fx")).unwrap();

    let (code, v) = run_json(&env, &["source", "list"]);
    assert_eq!(code, 0, "{v}");
    let src = &v["data"]["sources"][0];
    assert_eq!(src["alias"], "fx");
    assert_eq!(src["reachable"], false);
    assert!(src["skill_count"].is_null());
    assert_eq!(
        diags_with_code(&v["warnings"], "source-unreachable").len(),
        1
    );

    let (code, v) = run_json(&env, &["search", "review"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        diags_with_code(&v["warnings"], "source-unreachable").len(),
        1
    );
    assert!(result_ids(&v).is_empty());

    let (code, v) = run_json(&env, &["show", "fx:sdlc/code-review"]);
    assert_eq!(code, 4, "{v}");
    let msg = v["errors"][0]["message"].as_str().unwrap();
    assert!(msg.contains("`fx`") && msg.contains("unreachable"), "{msg}");

    assert_eq!(std::fs::read(env.config_file()).unwrap(), config_before);
    assert_eq!(std::fs::read(env.cache_file("fx")).unwrap(), cache_before);
    let config = String::from_utf8(config_before).unwrap();
    assert!(config.contains("alias = \"fx\""));
}
