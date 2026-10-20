//! Acceptance criteria for selections (milestone M2: AC-10, AC-12, AC-35,
//! desired side), plus core-API checks of `save_selection` and
//! `detect_root`. Activation (AC-10 end to end) arrived in milestone M3; the
//! status side of AC-12 and AC-35 in milestone M4.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use pouch_core::bundle::load_dir;
use pouch_core::catalog::{Catalog, LoadOptions};
use pouch_core::config::{Config, Strategy};
use pouch_core::project::index::read_index;
use pouch_core::project::{
    RootDetection, Selection, TargetId, TargetSelection, detect_root, load_selection,
    save_selection,
};
use pouch_core::resolve::{Provenance, Resolution, resolve_selection};
use std::fs;
use std::path::{Path, PathBuf};

struct Fx {
    env: TestEnv,
    lib: PathBuf,
    lib2: PathBuf,
    before: SourceSnapshot,
    before2: SourceSnapshot,
}

impl Fx {
    fn new() -> Fx {
        let env = TestEnv::new();
        let lib = build_library(&env);
        let lib2 = build_second_library(&env);
        let before = snapshot_source(&lib);
        let before2 = snapshot_source(&lib2);
        for (alias, l) in [("fx", &lib), ("hs", &lib2)] {
            let (code, v) = run_json(&env, &["source", "add", alias, l.to_str().unwrap()]);
            assert_eq!(code, 0, "{v}");
        }
        copy_fixture_bundle(&env, "hermes-only.yaml");
        Fx {
            env,
            lib,
            lib2,
            before,
            before2,
        }
    }

    fn project(&self, name: &str) -> PathBuf {
        let p = self.env.path("work").join(name);
        fs::create_dir_all(&p).unwrap();
        git_marker(&p);
        p
    }

    fn resolve(&self, root: &Path) -> Resolution {
        let dirs = dirs_for(&self.env);
        let (config, _) = Config::load_or_create(&dirs).unwrap();
        let catalog = Catalog::load(
            &config.sources,
            &dirs,
            LoadOptions {
                use_cache: false,
                git_global: false,
            },
        )
        .unwrap();
        let sel = load_selection(root).unwrap().unwrap();
        resolve_selection(
            &sel.targets[&TargetId::ClaudeCode],
            &load_dir(&dirs),
            &catalog,
            config.defaults.strategy,
        )
    }

    fn done(&self) {
        assert_source_unchanged(&self.before, &self.lib);
        assert_source_unchanged(&self.before2, &self.lib2);
    }
}

fn texts(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn claude_code(bundles: &[&str], add: &[&str], remove: &[&str]) -> Selection {
    let mut s = Selection::default();
    s.targets.insert(
        TargetId::ClaudeCode,
        TargetSelection {
            strategy: None,
            bundles: texts(bundles),
            add: texts(add),
            remove: texts(remove),
        },
    );
    s
}

fn ids(r: &Resolution) -> Vec<String> {
    r.resolved.iter().map(|e| e.id.to_string()).collect()
}

#[test]
fn ac_10_saved_bundle_in_empty_project_yields_original_selection() {
    let fx = Fx::new();
    let dirs = dirs_for(&fx.env);
    let a = fx.project("proj-a");
    save_selection(
        &dirs,
        &a,
        &claude_code(
            &["hermes-only"],
            &["hs:ops/deploy-checklist"],
            &["fx:pytest-fixtures"],
        ),
    )
    .unwrap();
    let ra = fx.resolve(&a);
    assert!(ra.is_clean(), "{:?}", ra.problems);
    assert_eq!(
        ids(&ra),
        vec![
            "fx:sdlc/code-review",
            "fx:alpha-root-skill",
            "hs:ops/deploy-checklist"
        ]
    );

    let (code, out, err) = run_in(&fx.env, &a, &["bundle", "save", "my-set"]);
    assert_eq!(code, 0, "{out}\n{err}");
    let manifest = fs::read_to_string(fx.env.path("bundles").join("my-set.yaml")).unwrap();
    assert_eq!(
        manifest,
        "name: my-set\nskills:\n  - fx:sdlc/code-review\n  - fx:alpha-root-skill\n  - hs:ops/deploy-checklist\n"
    );

    let b = fx.project("proj-b");
    save_selection(&dirs, &b, &claude_code(&["my-set"], &[], &[])).unwrap();
    let rb = fx.resolve(&b);
    assert!(rb.is_clean(), "{:?}", rb.problems);
    assert_eq!(ids(&rb), ids(&ra));
    for (x, y) in ra.resolved.iter().zip(&rb.resolved) {
        assert_eq!((&x.name, x.strategy), (&y.name, y.strategy));
        assert_eq!(y.provenance, vec![Provenance::Bundle("my-set".into())]);
    }
    fx.done();
}

#[test]
fn ac_12_bundle_plus_add_plus_remove_resolves_with_provenance() {
    let fx = Fx::new();
    let p = fx.project("proj");
    save_selection(
        &dirs_for(&fx.env),
        &p,
        &claude_code(
            &["hermes-only"],
            &["hs:api/rest-design", "fx:sdlc/code-review"],
            &["fx:alpha-root-skill"],
        ),
    )
    .unwrap();
    let r = fx.resolve(&p);
    assert!(r.is_clean(), "{:?}", r.problems);
    assert_eq!(
        ids(&r),
        vec![
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "hs:api/rest-design"
        ]
    );
    let hermes = Provenance::Bundle("hermes-only".into());
    assert_eq!(
        r.resolved[0].provenance,
        vec![hermes.clone(), Provenance::Adhoc]
    );
    assert_eq!(r.resolved[0].winner(), Some(&Provenance::Adhoc));
    assert_eq!(r.resolved[1].provenance, vec![hermes]);
    assert_eq!(r.resolved[2].provenance, vec![Provenance::Adhoc]);
    assert!(
        r.resolved
            .iter()
            .all(|e| e.id.to_string() != "fx:alpha-root-skill")
    );
    // Status half (M4): `status` shows the same set as `selected`, with the
    // same provenance; nothing is applied, so nothing is orphaned.
    let (code, out, err) = run_in(&fx.env, &p, &["--json", "status"]);
    assert_eq!(code, 0, "{out} {err}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        record_states(&v),
        vec![
            ("code-review".to_string(), "selected".to_string()),
            ("pytest-fixtures".to_string(), "selected".to_string()),
            ("rest-design".to_string(), "selected".to_string()),
        ]
    );
    assert_eq!(
        record(&v, "code-review")["provenance"],
        serde_json::json!([{"bundle": "hermes-only"}, {"adhoc": true}])
    );
    assert_eq!(
        record(&v, "pytest-fixtures")["provenance"],
        serde_json::json!([{"bundle": "hermes-only"}])
    );
    assert_eq!(
        record(&v, "rest-design")["provenance"],
        serde_json::json!([{"adhoc": true}])
    );
    assert!(!p.join(".pouch/claude-code/applied.json").exists());
    fx.done();
}

#[test]
fn ac_35_hand_edited_selection_is_desired_not_applied() {
    let fx = Fx::new();
    for (name, text) in [
        (
            "with-version",
            "version: 1\ntargets:\n  claude-code:\n    bundles: [hermes-only]\n    add: [hs:api/rest-design]\n",
        ),
        (
            "without-version",
            "# hand written\ntargets:\n  claude-code:\n    strategy: copy\n    add:\n      - hs:api/rest-design\n      - code-review\n",
        ),
    ] {
        let p = fx.project(name);
        write_file(&p, ".pouch/selection.yaml", text.as_bytes());
        let r = fx.resolve(&p);
        assert!(r.is_clean(), "{name}: {:?}", r.problems);
        assert!(!r.resolved.is_empty());
        assert!(
            r.resolved
                .iter()
                .any(|e| e.id.to_string() == "hs:api/rest-design")
        );
        if name == "without-version" {
            assert!(r.resolved.iter().all(|e| e.strategy == Strategy::Copy));
        }
        // Status half (M4): desired, not applied, so `selected`, never
        // `active`, and `status` writes nothing.
        let (code, out, err) = run_in(&fx.env, &p, &["--json", "status"]);
        assert_eq!(code, 0, "{name}: {out} {err}");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let states = record_states(&v);
        assert!(!states.is_empty(), "{name}");
        assert!(
            states.iter().all(|(_, s)| s == "selected"),
            "{name}: {states:?}"
        );
        assert!(
            record(&v, "rest-design")["strategy"]
                == if name == "without-version" {
                    "copy"
                } else {
                    "symlink"
                },
            "{name}: {v}"
        );
        // Desired, not applied: no receipt and no destination directory.
        for q in [
            ".pouch/claude-code/applied.json",
            ".pouch/lock",
            ".claude",
            ".agents",
        ] {
            assert!(!p.join(q).exists(), "{name}: {q}");
        }
    }
    // Loading registered nothing in the project index.
    assert!(!fx.env.project_index().exists());
    assert!(read_index(&dirs_for(&fx.env)).projects.is_empty());
    fx.done();
}

#[test]
fn save_selection_registers_and_detect_root_finds_the_marker() {
    let fx = Fx::new();
    let p = fx.project("registered");
    let sub = p.join("src/deep");
    fs::create_dir_all(&sub).unwrap();
    let root = detect_root(&sub, Some(&fx.env.root)).unwrap();
    assert_eq!(root.how, RootDetection::Git);
    assert_eq!(root.path, fs::canonicalize(&p).unwrap());
    let out = save_selection(
        &dirs_for(&fx.env),
        &root.path,
        &claude_code(&[], &["code-review"], &[]),
    )
    .unwrap();
    assert!(out.created_dir);
    assert_eq!(out.gitignore_lines.len(), 5);
    assert_eq!(
        read_index(&dirs_for(&fx.env)).projects,
        vec![root.path.to_string_lossy().into_owned()]
    );
    assert!(fx.env.project_index().exists());
    fx.done();
}
