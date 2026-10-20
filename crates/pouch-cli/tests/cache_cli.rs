//! Catalog cache behavior through the CLI (spec 4.3, DIS-9; plan P7).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use std::fs;

fn setup() -> (TestEnv, std::path::PathBuf, SourceSnapshot) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let (code, v) = run_json(&env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, before)
}

fn description(env: &TestEnv, args: &[&str]) -> String {
    let (code, v) = run_json(env, args);
    assert_eq!(code, 0, "{v}");
    v["data"]["description"].as_str().unwrap().to_string()
}

#[test]
fn cache_file_created_after_first_scan() {
    let (env, lib, before) = setup();
    let text = fs::read_to_string(env.cache_file("fx")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], 1);
    assert_eq!(v["alias"], "fx");
    assert_eq!(v["skills"].as_array().unwrap().len(), 9);
    assert!(v["skills"][0]["fingerprint"].is_array());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn poisoned_cache_reused_and_no_cache_bypasses_it() {
    let (env, lib, before) = setup();
    let p = env.cache_file("fx");
    let text = fs::read_to_string(&p).unwrap();
    let poisoned = text.replace(
        "Guidance for reviewing code changes before merge.",
        "MARKER",
    );
    assert_ne!(text, poisoned);
    fs::write(&p, &poisoned).unwrap();
    assert_eq!(
        description(&env, &["show", "fx:sdlc/code-review"]),
        "MARKER"
    );
    let real = description(&env, &["--no-cache", "show", "fx:sdlc/code-review"]);
    assert_eq!(real, "Guidance for reviewing code changes before merge.");
    assert_eq!(fs::read_to_string(&p).unwrap(), poisoned);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn in_place_edit_detected_by_mtime() {
    let (env, lib, _before) = setup();
    let skill = lib.join("sdlc/code-review/SKILL.md");
    let text = fs::read_to_string(&skill).unwrap();
    let edited = text.replace("Guidance for reviewing", "Guidance for checking ");
    assert_eq!(text.len(), edited.len());
    fs::write(&skill, &edited).unwrap();
    fs::File::options()
        .write(true)
        .open(&skill)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(2_000_000))
        .unwrap();
    let d = description(&env, &["show", "fx:sdlc/code-review"]);
    assert!(d.starts_with("Guidance for checking "), "{d}");
}

#[test]
fn new_nested_skill_appears_and_deleted_disappears() {
    let (env, lib, _before) = setup();
    write_file(
        &lib,
        "brand/new/deeply/nested/SKILL.md",
        skill_md("nested", "Brand new.").as_bytes(),
    );
    fs::remove_dir_all(lib.join("data/schema-migration")).unwrap();
    let (_, v) = run_json(&env, &["search", ""]);
    let ids = result_ids(&v);
    assert!(ids.contains(&"fx:brand/new/deeply/nested".to_string()));
    assert!(!ids.contains(&"fx:data/schema-migration".to_string()));
    let cache = fs::read_to_string(env.cache_file("fx")).unwrap();
    assert!(cache.contains("brand/new/deeply/nested"));
    assert!(!cache.contains("data/schema-migration"));
}

#[test]
fn corrupt_cache_ignored_then_rewritten() {
    let (env, lib, before) = setup();
    let p = env.cache_file("fx");
    fs::write(&p, "{ corrupt").unwrap();
    let (code, v) = run_json(&env, &["search", ""]);
    assert_eq!(code, 0);
    assert_eq!(result_ids(&v).len(), 9);
    let text = fs::read_to_string(&p).unwrap();
    assert!(serde_json::from_str::<serde_json::Value>(&text).is_ok());
    assert_source_unchanged(&before, &lib);
}

#[test]
fn no_cache_on_fresh_env_creates_no_cache_file() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let (code, _) = run_json(
        &env,
        &["--no-cache", "source", "add", "fx", lib.to_str().unwrap()],
    );
    assert_eq!(code, 0);
    let (code, _) = run_json(&env, &["--no-cache", "search", "review"]);
    assert_eq!(code, 0);
    assert!(!env.cache_file("fx").exists());
    assert!(!env.path("cache").join("catalog").exists());
}
