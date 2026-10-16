//! `unam search` (spec 4.3, 5).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;

fn setup() -> (TestEnv, std::path::PathBuf, SourceSnapshot) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    // A sibling category whose name extends `sdlc`, for the boundary check.
    write_file(
        &lib,
        "sdlcx/other/SKILL.md",
        skill_md("other", "Not in sdlc.").as_bytes(),
    );
    let before = snapshot_source(&lib);
    let (code, v) = run_json(&env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, before)
}

#[test]
fn category_prefix_and_boundary() {
    let (env, lib, before) = setup();
    let (code, v) = run_json(&env, &["search", "", "--category", "sdlc"]);
    assert_eq!(code, 0);
    assert_eq!(result_ids(&v), vec!["fx:sdlc/code-review"]);
    let (_, v) = run_json(&env, &["search", "", "--category", "engineering/"]);
    assert_eq!(
        result_ids(&v),
        vec![
            "fx:engineering/backend/api/error-handling",
            "fx:engineering/frontend/error-handling"
        ]
    );
    let (_, v) = run_json(&env, &["search", "", "--category", "sdlcx"]);
    assert_eq!(result_ids(&v), vec!["fx:sdlcx/other"]);
    assert_eq!(v["data"]["category"], "sdlcx");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn source_filter_and_unknown_alias() {
    let (env, lib, before) = setup();
    let (code, v) = run_json(&env, &["search", "review", "--source", "fx"]);
    assert_eq!(code, 0);
    assert!(!result_ids(&v).is_empty());
    assert_eq!(v["data"]["source"], "fx");
    for bad in ["nope", "Not_Valid"] {
        let (code, v) = run_json(&env, &["search", "review", "--source", bad]);
        assert_eq!(code, 2, "{v}");
        assert_eq!(v["errors"][0]["code"], "unknown-source");
    }
    assert_source_unchanged(&before, &lib);
}

#[test]
fn empty_query_lists_all_sorted_by_id() {
    let (env, lib, before) = setup();
    let (code, v) = run_json(&env, &["search", ""]);
    assert_eq!(code, 0);
    let ids = result_ids(&v);
    assert_eq!(ids.len(), 10);
    let mut sorted = ids.clone();
    sorted.sort();
    assert_eq!(ids, sorted);
    assert_eq!(v["data"]["total"], 10);
    assert_source_unchanged(&before, &lib);
}

#[test]
fn no_match_is_empty_exit_0() {
    let (env, lib, before) = setup();
    let (code, v) = run_json(&env, &["search", "zzqqxxjj"]);
    assert_eq!(code, 0);
    assert!(result_ids(&v).is_empty());
    let (code, out, _) = env.run(&["search", "zzqqxxjj"]);
    assert_eq!(code, 0);
    assert!(out.contains("No skills match"), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn exact_name_ranks_first() {
    let (env, lib, before) = setup();
    let (_, v) = run_json(&env, &["search", "code-review"]);
    assert_eq!(result_ids(&v)[0], "fx:sdlc/code-review");
    let (code, out, _) = env.run(&["search", "code-review"]);
    assert_eq!(code, 0);
    let first_row = out.lines().nth(1).unwrap();
    assert!(first_row.starts_with("fx:sdlc/code-review"), "{out}");
    assert!(out.starts_with("ID"), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn json_fields_present() {
    let (env, lib, before) = setup();
    let (_, v) = run_json(&env, &["search", "review"]);
    assert_eq!(v["command"], "search");
    assert!(v["warnings"].is_array());
    assert!(v["errors"].as_array().unwrap().is_empty());
    assert!(v["data"]["category"].is_null());
    assert!(v["data"]["source"].is_null());
    for r in v["data"]["results"].as_array().unwrap() {
        for k in ["id", "name", "category", "source", "description", "score"] {
            assert!(r.get(k).is_some(), "missing {k} in {r}");
        }
        assert!(r["score"].is_u64());
        assert!(r["diagnostics"].is_array());
    }
    // Skipped-skill notices are not repeated on search.
    assert!(diags_with_code(&v["warnings"], "unparseable-yaml").is_empty());
    assert_source_unchanged(&before, &lib);
}
