//! `pouch show` (spec 4.6 reference forms, spec 5; plan D6, P12).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;

fn setup() -> (TestEnv, std::path::PathBuf, SourceSnapshot) {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let (code, v) = run_json(&env, &["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    (env, lib, before)
}

#[test]
fn three_reference_forms_and_top_level() {
    let (env, lib, before) = setup();
    for r in [
        "fx:sdlc/code-review",
        "fx:code-review",
        "code-review",
        "fx:sdlc/code-review/",
    ] {
        let (code, v) = run_json(&env, &["show", r]);
        assert_eq!(code, 0, "{r}: {v}");
        assert_eq!(v["data"]["id"], "fx:sdlc/code-review");
    }
    let (code, v) = run_json(&env, &["show", "fx:alpha-root-skill"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["category"], "");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn not_found_with_candidates() {
    let (env, lib, before) = setup();
    let (code, v) = run_json(&env, &["show", "code-reviw"]);
    assert_eq!(code, 4, "{v}");
    assert_eq!(v["errors"][0]["code"], "not-found");
    let c = v["errors"][0]["candidates"].as_array().unwrap();
    assert_eq!(c[0], "fx:sdlc/code-review");
    let (code, _, err) = env.run(&["show", "code-reviw"]);
    assert_eq!(code, 4);
    assert!(err.contains("error: "), "{err}");
    assert!(err.contains("fx:sdlc/code-review"), "{err}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn ambiguous_never_picks() {
    let (env, lib, before) = setup();
    for r in ["error-handling", "fx:error-handling"] {
        let (code, v) = run_json(&env, &["show", r]);
        assert_eq!(code, 5, "{r}: {v}");
        assert_eq!(v["errors"][0]["code"], "ambiguous");
        let c: Vec<&str> = v["errors"][0]["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert_eq!(
            c,
            vec![
                "fx:engineering/backend/api/error-handling",
                "fx:engineering/frontend/error-handling"
            ]
        );
        assert!(v["data"].as_object().unwrap().is_empty());
    }
    for r in [
        "fx:engineering/backend/api/error-handling",
        "fx:engineering/frontend/error-handling",
    ] {
        let (code, _) = run_json(&env, &["show", r]);
        assert_eq!(code, 0, "{r}");
    }
    assert_source_unchanged(&before, &lib);
}

#[test]
fn invalid_ref_exit_2() {
    let (env, _lib, _before) = setup();
    for r in ["fx:", ":x", "fx:a/../b", "BAD:x"] {
        let (code, v) = run_json(&env, &["show", r]);
        assert_eq!(code, 2, "{r}: {v}");
        assert_eq!(v["errors"][0]["code"], "invalid-ref");
    }
}

#[test]
fn frontmatter_order_supporting_and_diagnostics() {
    let (env, lib, before) = setup();
    let (_, v) = run_json(&env, &["show", "fx:sdlc/code-review"]);
    let keys: Vec<&String> = v["data"]["frontmatter"]
        .as_object()
        .unwrap()
        .keys()
        .collect();
    assert_eq!(
        keys,
        vec![
            "name",
            "description",
            "version",
            "zeta-unknown",
            "alpha-unknown"
        ]
    );
    let supporting: Vec<&str> = v["data"]["supporting"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    for f in ["references/checklist.md", "scripts/run.sh"] {
        assert!(supporting.contains(&f), "{supporting:?}");
    }
    assert!(
        v["data"]["tree_hash"]
            .as_str()
            .unwrap()
            .starts_with("blake3:")
    );
    // M2 plan P16: `bundles` is always present (empty without bundles).
    assert_eq!(v["data"]["bundles"], serde_json::json!([]));
    let (_, v) = run_json(&env, &["show", "fx:colon/value-with-colon"]);
    assert_eq!(
        diags_with_code(&v["data"]["diagnostics"], "colon-retry-applied").len(),
        1
    );
    let (code, out, _) = env.run(&["show", "fx:mismatch/dir-name-differs"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("other-name  (fx:mismatch/dir-name-differs)"),
        "{out}"
    );
    assert!(out.contains("warning[name-dir-mismatch]"), "{out}");
    assert!(out.contains("Supporting files (0):"), "{out}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn unreachable_source_names_it() {
    let (env, lib, _before) = setup();
    std::fs::remove_dir_all(&lib).unwrap();
    let (code, v) = run_json(&env, &["show", "code-review"]);
    assert_eq!(code, 4, "{v}");
    assert!(v["errors"][0]["message"].as_str().unwrap().contains("`fx`"));
    assert_eq!(
        diags_with_code(&v["warnings"], "source-unreachable").len(),
        1
    );
}

#[test]
fn human_output_neutralizes_control_characters() {
    let env = TestEnv::new();
    let lib = env.path("work").join("esc");
    write_file(
        &lib,
        "cat/alert/SKILL.md",
        b"---\nname: alert\ndescription: \"red \\x1b[31mALERT\"\n---\n",
    );
    let (code, _) = run_json(&env, &["source", "add", "esc", lib.to_str().unwrap()]);
    assert_eq!(code, 0);
    for args in [vec!["show", "esc:cat/alert"], vec!["search", "alert"]] {
        let out = env.cmd().args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(0));
        assert!(!out.stdout.contains(&0x1b), "{args:?} leaked ESC");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(text.contains("\\x1b[31mALERT"), "{text}");
    }
    let out = env
        .cmd()
        .args(["--json", "show", "esc:cat/alert"])
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("\\u001b[31mALERT"), "{text}");
}

#[test]
fn show_lists_bundles_referencing_it() {
    let (env, lib, before) = setup();
    // Empty case.
    let (code, v) = run_json(&env, &["show", "fx:sdlc/code-review"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["bundles"], serde_json::json!([]));
    let (_, out, _) = env.run(&["show", "fx:sdlc/code-review"]);
    assert!(out.contains("Bundles: (none)\n"), "{out}");
    // Two bundles reference it (bare and qualified), one does not, one only
    // ambiguously.
    write_file(
        &env.path("bundles"),
        "zeta.yaml",
        b"skills: [fx:sdlc/code-review]\n",
    );
    write_file(
        &env.path("bundles"),
        "alpha.yaml",
        b"skills: [code-review, alpha-root-skill]\n",
    );
    write_file(
        &env.path("bundles"),
        "other.yaml",
        b"skills: [alpha-root-skill]\n",
    );
    write_file(
        &env.path("bundles"),
        "amb.yaml",
        b"skills: [error-handling]\n",
    );
    let (code, v) = run_json(&env, &["show", "code-review"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["bundles"], serde_json::json!(["alpha", "zeta"]));
    let keys: Vec<&String> = v["data"].as_object().unwrap().keys().collect();
    let at = keys.iter().position(|k| *k == "tree_hash").unwrap();
    assert_eq!(keys[at + 1], "bundles");
    let (_, out, _) = env.run(&["show", "code-review"]);
    assert!(out.contains("Bundles: alpha, zeta\n"), "{out}");
    let (_, v) = run_json(&env, &["show", "fx:engineering/frontend/error-handling"]);
    assert_eq!(v["data"]["bundles"], serde_json::json!([]));
    assert_source_unchanged(&before, &lib);
}
