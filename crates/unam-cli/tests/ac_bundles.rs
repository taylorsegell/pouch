//! Acceptance criteria for bundles (milestone M2: AC-7, AC-8, AC-9, AC-11).
//!
//! AC-11 is the riskiest M2 behavior (hand edits of user files); written first.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use unam_core::bundle::load_dir;
use unam_core::catalog::{Catalog, LoadOptions};
use unam_core::config::{Config, Strategy};
use unam_core::project::TargetSelection;
use unam_core::resolve::{ProblemKind, resolve_selection};

/// A `TestEnv` with the fixture library registered as `fx`.
fn env_with_fx() -> (TestEnv, PathBuf, SourceSnapshot) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let (code, v) = run_json(&env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, before)
}

/// Write `content` as the `commented` bundle in a fresh `TestEnv`, remove
/// `reference`, and assert the result is the old file minus exactly the one
/// line `removed_line` (compared as lines with their terminators).
fn remove_one_and_compare(content: &str, reference: &str, removed_line: &str) {
    let (env, lib, before) = env_with_fx();
    let path = env.path("bundles").join("commented.yaml");
    fs::write(&path, content).unwrap();
    let old_lines = split_lines(content);
    let hits: Vec<usize> = old_lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.as_str() == removed_line)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "fixture must hold the line once: {removed_line:?}"
    );
    let mut expected = old_lines.clone();
    expected.remove(hits[0]);

    let (code, out, err) = env.run(&["bundle", "rm", "commented", reference]);
    assert_eq!(code, 0, "stdout: {out}\nstderr: {err}");
    let new_lines = read_lines(&path);
    assert_eq!(
        new_lines, expected,
        "every other line must be byte-identical"
    );
    assert_eq!(fs::read(&path).unwrap(), expected.concat().into_bytes());
    assert_source_unchanged(&before, &lib);
}

fn fixture_text(name: &str) -> String {
    fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/bundles")
            .join(name),
    )
    .unwrap()
}

#[test]
fn ac_11_removing_one_entry_leaves_every_other_line_byte_identical() {
    let lf = fixture_text("commented.yaml");

    // Middle entry, LF.
    remove_one_and_compare(
        &lf,
        "fx:data/schema-migration",
        "  - 'fx:data/schema-migration'\n",
    );

    // CRLF plus BOM copy.
    let crlf = format!("\u{feff}{}", lf.replace('\n', "\r\n"));
    remove_one_and_compare(
        &crlf,
        "fx:data/schema-migration",
        "  - 'fx:data/schema-migration'\r\n",
    );

    // Dash at indent 0 copy.
    let indent0: String = split_lines(&lf)
        .into_iter()
        .map(|l| match l.strip_prefix("  - ") {
            Some(rest) => format!("- {rest}"),
            None => l,
        })
        .collect();
    remove_one_and_compare(
        &indent0,
        "fx:data/schema-migration",
        "- 'fx:data/schema-migration'\n",
    );

    // First item (it carries a trailing comment) and last item.
    remove_one_and_compare(
        &lf,
        "fx:sdlc/code-review",
        "  - fx:sdlc/code-review        # primary review flow\n",
    );
    remove_one_and_compare(&lf, "alpha-root-skill", "  - alpha-root-skill\n");
}

// ---- AC-7, AC-8, AC-9 (step H1) ----

/// `fx` plus the second library registered as `hs` (precedence 20).
fn env_with_fx_and_hs() -> (TestEnv, PathBuf, PathBuf, SourceSnapshot, SourceSnapshot) {
    let (env, lib, before) = env_with_fx();
    let lib2 = build_second_library(&env);
    let before2 = snapshot_source(&lib2);
    let (code, v) = run_json(&env, &["source", "add", "hs", lib2.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, lib2, before, before2)
}

/// The catalog as the CLI sees it, loaded in-process (no cache writes).
fn catalog_for(env: &TestEnv) -> Catalog {
    let dirs = dirs_for(env);
    let (config, _) = Config::load_or_create(&dirs).unwrap();
    Catalog::load(
        &config.sources,
        &dirs,
        LoadOptions {
            use_cache: false,
            git_global: false,
        },
    )
    .unwrap()
}

fn bundles_only(names: &[&str]) -> TargetSelection {
    TargetSelection {
        bundles: names.iter().map(|s| s.to_string()).collect(),
        ..TargetSelection::default()
    }
}

fn shown_ids(v: &Value) -> Vec<String> {
    v["data"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap_or("").to_string())
        .collect()
}

#[test]
fn ac_07_hermes_only_bundle_with_bare_names_loads_unchanged() {
    let (env, lib, before) = env_with_fx();
    let path = copy_fixture_bundle(&env, "hermes-only.yaml");
    let original = fs::read(&path).unwrap();

    let (code, v) = run_json(&env, &["bundle", "ls"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["bundles"][0]["slug"], "hermes-only");
    assert_eq!(v["data"]["bundles"][0]["skill_count"], 3);
    assert_eq!(v["data"]["bundles"][0]["has_instruction"], true);
    assert!(v["data"]["diagnostics"].as_array().unwrap().is_empty());

    let (code, v) = run_json(&env, &["bundle", "show", "hermes-only"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["description"], "Load 3 skills as a bundle");
    assert_eq!(
        shown_ids(&v),
        vec![
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "fx:alpha-root-skill"
        ]
    );
    assert_eq!(v["data"]["problems"], 0);

    let set = load_dir(&dirs_for(&env));
    let b = set.get("hermes-only").unwrap();
    assert_eq!(b.description, "Load 3 skills as a bundle");
    assert_eq!(
        b.skills,
        vec!["code-review", "pytest-fixtures", "alpha-root-skill"]
    );
    assert_eq!(
        b.instruction.as_deref(),
        Some("Review the change, then check the test fixtures.\nKeep the root skill in mind.")
    );
    assert_eq!(fs::read(&path).unwrap(), original, "loading never rewrites");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates() {
    let (env, lib, lib2, before, before2) = env_with_fx_and_hs();
    let hermes = copy_fixture_bundle(&env, "hermes-only.yaml");
    let amb = env.path("bundles").join("amb.yaml");
    fs::write(
        &amb,
        "name: amb\nskills:\n  - error-handling\n  - code-review\n",
    )
    .unwrap();
    let both = [
        "fx:engineering/backend/api/error-handling",
        "fx:engineering/frontend/error-handling",
    ];

    // Core: one Ambiguous problem with both IDs; the entry is not resolved.
    let set = load_dir(&dirs_for(&env));
    let r = resolve_selection(
        &bundles_only(&["amb"]),
        &set,
        &catalog_for(&env),
        Strategy::Symlink,
    );
    assert_eq!(r.problems.len(), 1);
    assert_eq!(r.problems[0].kind, ProblemKind::Ambiguous);
    assert_eq!(r.problems[0].candidates, both);
    let ids: Vec<String> = r.resolved.iter().map(|e| e.id.to_string()).collect();
    assert_eq!(ids, vec!["fx:sdlc/code-review"]);

    // CLI: show reports the status, exit 0.
    let (code, v) = run_json(&env, &["bundle", "show", "amb"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["skills"][0]["status"], "ambiguous");
    assert_eq!(
        v["data"]["skills"][0]["candidates"],
        serde_json::json!(both)
    );
    assert_eq!(v["data"]["problems"], 1);

    // CLI: add refuses with exit 5 naming both IDs; manifest untouched.
    let original = fs::read(&hermes).unwrap();
    let (code, v) = run_json(&env, &["bundle", "add", "hermes-only", "error-handling"]);
    assert_eq!(code, 5, "{v}");
    assert_eq!(v["errors"][0]["code"], "ambiguous");
    assert_eq!(v["errors"][0]["candidates"], serde_json::json!(both));
    let (code, v) = run_json(&env, &["bundle", "add", "hermes-only", "schema-migration"]);
    assert_eq!(code, 5, "{v}");
    assert_eq!(
        v["errors"][0]["candidates"],
        serde_json::json!(["fx:data/schema-migration", "hs:data/schema-migration"])
    );
    let (code, _, err) = env.run(&["bundle", "add", "hermes-only", "schema-migration"]);
    assert_eq!(code, 5);
    assert!(
        err.contains("fx:data/schema-migration") && err.contains("hs:data/schema-migration"),
        "{err}"
    );
    assert_eq!(fs::read(&hermes).unwrap(), original);

    // Nothing activates.
    let cwd = env.path("cwd");
    for p in [
        ".claude",
        ".agents",
        ".unam/claude-code/applied.json",
        ".unam/lock",
    ] {
        assert!(!cwd.join(p).exists(), "{p}");
    }
    assert_source_unchanged(&before, &lib);
    assert_source_unchanged(&before2, &lib2);
}

#[test]
fn ac_09_bundle_spans_two_categories_and_two_sources() {
    let (env, lib, lib2, before, before2) = env_with_fx_and_hs();
    fs::write(
        env.path("bundles").join("span.yaml"),
        "name: span\nskills:\n  - fx:sdlc/code-review\n  - hs:ops/deploy-checklist\n  - fx:data/schema-migration\n  - hs:data/schema-migration\n",
    )
    .unwrap();
    let set = load_dir(&dirs_for(&env));
    let r = resolve_selection(
        &bundles_only(&["span"]),
        &set,
        &catalog_for(&env),
        Strategy::Symlink,
    );
    assert!(r.problems.is_empty(), "{:?}", r.problems);
    let ids: Vec<String> = r.resolved.iter().map(|e| e.id.to_string()).collect();
    assert_eq!(
        ids,
        vec![
            "fx:sdlc/code-review",
            "hs:ops/deploy-checklist",
            "fx:data/schema-migration",
            "hs:data/schema-migration"
        ]
    );
    let mut categories: Vec<&str> = r
        .resolved
        .iter()
        .map(|e| e.id.rel_path.parent_str())
        .collect();
    categories.sort();
    categories.dedup();
    assert_eq!(categories, vec!["data", "ops", "sdlc"]);
    let mut sources: Vec<&str> = r.resolved.iter().map(|e| e.id.source.as_str()).collect();
    sources.sort();
    sources.dedup();
    assert_eq!(sources, vec!["fx", "hs"]);

    let (code, v) = run_json(&env, &["bundle", "show", "span"]);
    assert_eq!(code, 0, "{v}");
    let statuses: Vec<&str> = v["data"]["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["status"].as_str().unwrap())
        .collect();
    assert_eq!(statuses, vec!["resolved"; 4]);
    assert_eq!(shown_ids(&v), ids);
    assert_source_unchanged(&before, &lib);
    assert_source_unchanged(&before2, &lib2);
}
