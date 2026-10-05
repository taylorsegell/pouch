//! `unam source add|list|rm` (spec 4.2, 5; SRC-6; plan P8, P10, P11).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use std::fs;

fn lib_str(lib: &std::path::Path) -> &str {
    lib.to_str().unwrap()
}

#[test]
fn add_success_human() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let (code, out, err) = env.run(&["source", "add", "fx", lib_str(&lib)]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("Registered source fx -> "), "{out}");
    assert!(out.contains("(directory)"), "{out}");
    assert!(out.contains("Skills found: 9"), "{out}");
    assert!(
        err.contains("skipped[unparseable-yaml] broken/bad-yaml"),
        "{err}"
    );
    assert!(err.contains("note[duplicate-name]"), "{err}");
    assert_source_unchanged(&before, &lib);
}

#[test]
fn add_exit_2_cases_write_nothing() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let file = env.path("work").join("plain-file");
    fs::write(&file, "x").unwrap();
    let missing = env.path("work").join("missing");
    for (args, code_str) in [
        (
            vec!["source", "add", "fx", missing.to_str().unwrap()],
            "source-path-invalid",
        ),
        (
            vec!["source", "add", "fx", file.to_str().unwrap()],
            "source-path-invalid",
        ),
        (
            vec!["source", "add", "Bad_Alias", lib_str(&lib)],
            "invalid-alias",
        ),
    ] {
        let (code, v) = run_json(&env, &args);
        assert_eq!(code, 2, "{args:?}: {v}");
        assert_eq!(v["ok"], false);
        assert_eq!(v["errors"][0]["code"], code_str);
        assert!(v["errors"][0]["remedy"].is_string());
    }
    let (code, _) = run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    assert_eq!(code, 0);
    let config = fs::read(env.config_file()).unwrap();
    let other = env.path("work").join("other");
    fs::create_dir_all(&other).unwrap();
    let (code, v) = run_json(&env, &["source", "add", "fx", other.to_str().unwrap()]);
    assert_eq!(code, 2);
    assert_eq!(v["errors"][0]["code"], "alias-exists");
    let respelled = format!("{}/../", lib.join("sdlc").display());
    let (code, v) = run_json(&env, &["source", "add", "fx2", &respelled]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["errors"][0]["code"], "source-path-registered");
    assert_eq!(fs::read(env.config_file()).unwrap(), config);
}

#[test]
fn zero_skills_registers_with_warning() {
    let env = TestEnv::new();
    let empty = env.path("work").join("empty");
    fs::create_dir_all(&empty).unwrap();
    let (code, v) = run_json(&env, &["source", "add", "empty", empty.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["skill_count"], 0);
    assert_eq!(diags_with_code(&v["warnings"], "zero-skills").len(), 1);
    let (_, l) = run_json(&env, &["source", "list"]);
    assert_eq!(l["data"]["sources"][0]["alias"], "empty");
}

#[test]
fn relative_path_stored_canonical_and_tilde_as_typed() {
    let env = TestEnv::new();
    let rel = env.path("cwd").join("rel-lib/cat/s/SKILL.md");
    fs::create_dir_all(rel.parent().unwrap()).unwrap();
    fs::write(&rel, skill_md("s", "Relative.")).unwrap();
    let (code, v) = run_json(&env, &["source", "add", "rel", "rel-lib"]);
    assert_eq!(code, 0, "{v}");
    let canonical = fs::canonicalize(env.path("cwd").join("rel-lib")).unwrap();
    assert_eq!(v["data"]["path"], canonical.to_str().unwrap());

    let home_skill = env.path("home").join("skills/cat/h/SKILL.md");
    fs::create_dir_all(home_skill.parent().unwrap()).unwrap();
    fs::write(&home_skill, skill_md("h", "Home.")).unwrap();
    let (code, v) = run_json(&env, &["source", "add", "home", "~/skills"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["path"], "~/skills");
    assert_eq!(v["data"]["skill_count"], 1);
    let config = fs::read_to_string(env.config_file()).unwrap();
    assert!(config.contains("path = \"~/skills\""), "{config}");
    assert!(config.contains(&format!("path = \"{}\"", canonical.display())));
}

#[test]
fn list_order_last_scan_and_reachability() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let second = env.path("work").join("second");
    fs::create_dir_all(second.join("a/s")).unwrap();
    fs::write(second.join("a/s/SKILL.md"), skill_md("s", "Second.")).unwrap();
    // Registered with --no-cache: no cache yet, so last_scan is null.
    let (code, _) = run_json(&env, &["--no-cache", "source", "add", "zz", lib_str(&lib)]);
    assert_eq!(code, 0);
    let (code, _) = run_json(
        &env,
        &[
            "--no-cache",
            "source",
            "add",
            "aa",
            second.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0);
    let (code, v) = run_json(&env, &["--no-cache", "source", "list"]);
    assert_eq!(code, 0);
    let s = v["data"]["sources"].as_array().unwrap();
    assert_eq!(s[0]["alias"], "zz");
    assert_eq!(s[0]["precedence"], 10);
    assert_eq!(s[1]["alias"], "aa");
    assert_eq!(s[1]["precedence"], 20);
    assert!(s[0]["last_scan"].is_null());
    assert!(!env.cache_file("zz").exists());
    let (_, v) = run_json(&env, &["source", "list"]);
    assert!(v["data"]["sources"][0]["last_scan"].is_string());
    assert!(env.cache_file("zz").exists());
    fs::remove_dir_all(&second).unwrap();
    let (code, v) = run_json(&env, &["source", "list"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["sources"][1]["reachable"], false);
    assert_eq!(v["data"]["sources"][1]["kind"], "unknown");
    let (code, out, _) = env.run(&["source", "list"]);
    assert_eq!(code, 0);
    assert!(out.contains("ALIAS"), "{out}");
    assert!(out.contains("unreachable"), "{out}");
}

#[test]
fn precedence_tie_warning() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let other = env.path("work").join("other");
    fs::create_dir_all(&other).unwrap();
    fs::write(
        env.config_file(),
        format!(
            "version = 1\n\n[[sources]]\nalias = \"zz\"\npath = \"{}\"\nprecedence = 10\n\n[[sources]]\nalias = \"aa\"\npath = \"{}\"\nprecedence = 10\n",
            lib.display(),
            other.display()
        ),
    )
    .unwrap();
    let (code, v) = run_json(&env, &["source", "list"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(diags_with_code(&v["warnings"], "precedence-tie").len(), 1);
    let s = v["data"]["sources"].as_array().unwrap();
    assert_eq!(s[0]["alias"], "aa");
    assert_eq!(s[1]["alias"], "zz");
}

#[test]
fn config_created_on_first_load_not_by_bare_unam() {
    let env = TestEnv::new();
    let (code, _, _) = env.run(&[]);
    assert_eq!(code, 0);
    assert!(!env.config_file().exists());
    let (code, _, _) = env.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(!env.config_file().exists());
    let (code, v) = run_json(&env, &["source", "list"]);
    assert_eq!(code, 0);
    assert!(v["data"]["sources"].as_array().unwrap().is_empty());
    let text = fs::read_to_string(env.config_file()).unwrap();
    assert!(text.contains("version = 1"));
}

fn write_selection(root: &std::path::Path, text: &str) {
    fs::create_dir_all(root.join(".unam")).unwrap();
    fs::write(root.join(".unam/selection.yaml"), text).unwrap();
}

fn write_index(env: &TestEnv, text: &str) {
    let p = env.project_index();
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

#[test]
fn rm_cases() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    // Unknown alias: exit 4.
    let (code, v) = run_json(&env, &["source", "rm", "nope"]);
    assert_eq!(code, 4);
    assert_eq!(v["errors"][0]["code"], "source-not-found");
    // Index absent: exit 0 (P11), cache file deleted.
    run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    assert!(env.cache_file("fx").exists());
    let (code, v) = run_json(&env, &["source", "rm", "fx"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["removed"], true);
    assert_eq!(v["data"]["forced"], false);
    assert!(!env.cache_file("fx").exists());
    // Referenced: exit 3; forced: exit 0 with the dangling project listed.
    run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    let proj = env.path("work").join("project");
    write_selection(
        &proj,
        "version: 1\ntargets:\n  claude-code:\n    add: [\"fx:sdlc/code-review\"]\n",
    );
    write_index(
        &env,
        &serde_json::json!({"version": 1, "projects": [proj]}).to_string(),
    );
    let (code, v) = run_json(&env, &["source", "rm", "fx"]);
    assert_eq!(code, 3, "{v}");
    assert_eq!(v["errors"][0]["code"], "source-referenced");
    let (code, out, err) = env.run(&["source", "rm", "fx", "--force"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains(proj.to_str().unwrap()), "{out}");
    assert!(out.contains("unresolvable"), "{out}");
    // Corrupt index: warning, exit 0.
    run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    write_index(&env, "{corrupt");
    let (code, v) = run_json(&env, &["source", "rm", "fx"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        diags_with_code(&v["warnings"], "project-index-unreadable").len(),
        1
    );
    assert_source_unchanged(&before, &lib);
}

#[test]
fn cache_write_failure_is_warning_not_error() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    // A regular file where the catalog directory must go: creation fails for any uid.
    fs::write(env.path("cache").join("catalog"), "not a directory").unwrap();
    let (code, v) = run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(
        diags_with_code(&v["warnings"], "cache-write-failed").len(),
        1
    );
    assert_eq!(v["data"]["skill_count"], 9);
    assert_eq!(v["data"]["alias"], "fx");
}

#[test]
fn global_gitignore_is_honored_in_git_tree_under_isolation() {
    let env = TestEnv::new();
    let lib = build_library_with_root_git(&env);
    let git_dir = env.path("xdg-config").join("git");
    fs::create_dir_all(&git_dir).unwrap();
    fs::write(git_dir.join("ignore"), "testing/\n").unwrap();
    let (code, v) = run_json(&env, &["source", "add", "fx", lib_str(&lib)]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["skill_count"], 8);
    let (_, s) = run_json(&env, &["search", "pytest"]);
    assert!(!result_ids(&s).contains(&"fx:testing/pytest-fixtures".to_string()));
}
