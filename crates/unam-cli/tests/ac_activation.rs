//! Acceptance criteria for activation (milestone M3: AC-2, AC-10 end to
//! end, AC-13, AC-15 to AC-19, AC-21 to AC-25, AC-29, AC-30, AC-40, S2).
//! Every test runs through the shared fixture `Fx` and ends with
//! `fx.done()` (sources and planted foreign fixtures unchanged).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn dest(p: &Path) -> PathBuf {
    p.join(".claude/skills")
}

fn children(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    v.sort();
    v
}

fn ok(fx: &Fx, p: &Path, args: &[&str]) -> Value {
    let (code, v) = fx.json_raw(p, args);
    assert_eq!(code, 0, "{args:?}: {v}");
    v
}

fn canonical_lib(fx: &Fx, rel: &str) -> PathBuf {
    fs::canonicalize(&fx.lib).unwrap().join(rel)
}

fn selection_text(p: &Path) -> String {
    fs::read_to_string(p.join(".unam/selection.yaml")).unwrap()
}

fn history_count(p: &Path) -> usize {
    fs::read_dir(p.join(".unam/claude-code/history"))
        .map(|rd| rd.count())
        .unwrap_or(0)
}

#[test]
fn ac_02_source_tree_unchanged_after_full_activate_modify_deactivate_cycle() {
    let fx = Fx::new();
    let sym = fx.project("sym");
    let copy = fx.project("copy");
    fx.write_selection(&copy, "targets:\n  claude-code:\n    strategy: copy\n");
    for p in [&sym, &copy] {
        for args in [
            &[
                "add",
                "fx:sdlc/code-review",
                "fx:testing/pytest-fixtures",
                "--yes",
            ][..],
            &["add", "fx:data/schema-migration", "--yes"],
            &["rm", "fx:testing/pytest-fixtures", "--yes"],
            &["deactivate", "--yes"],
        ] {
            ok(&fx, p, args);
            assert_source_unchanged(&fx.before, &fx.lib);
            assert_source_unchanged(&fx.before2, &fx.lib2);
        }
        assert!(!p.join(".claude").exists(), "{}", p.display());
    }
    fx.done();
}

#[test]
fn ac_10_saved_bundle_applied_to_empty_project_yields_original_selection() {
    let fx = Fx::new();
    let a = fx.project("a");
    ok(&fx, &a, &["use", "hermes-only", "--replace", "--yes"]);
    ok(&fx, &a, &["add", "hs:ops/deploy-checklist", "--yes"]);
    // A holds a literal `add` reference to `hs`: the source rm guard sees it.
    let before_config = fs::read(fx.env.config_file()).unwrap();
    let (code, v) = fx.json_raw(&a, &["source", "rm", "hs"]);
    assert_eq!(code, 3, "{v}");
    assert_eq!(v["errors"][0]["code"], "source-referenced");
    let canon_a = fx.canon(&a).to_string_lossy().into_owned();
    assert!(
        v["errors"][0]["message"]
            .as_str()
            .unwrap()
            .contains(&canon_a),
        "{v}"
    );
    assert_eq!(fs::read(fx.env.config_file()).unwrap(), before_config);
    ok(&fx, &a, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
    ok(&fx, &a, &["bundle", "save", "my-set"]);
    let b = fx.project("b");
    ok(&fx, &b, &["use", "my-set", "--replace", "--yes"]);
    let summary = |p: &Path| -> Vec<(String, String, String, String)> {
        let mut v: Vec<_> = read_receipt(p, "claude-code")["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e["skill_id"].as_str().unwrap().to_string(),
                    e["name"].as_str().unwrap().to_string(),
                    e["strategy"].as_str().unwrap().to_string(),
                    e["tree_hash"].as_str().unwrap().to_string(),
                )
            })
            .collect();
        v.sort();
        v
    };
    assert_eq!(summary(&a), summary(&b));
    assert_eq!(summary(&a).len(), 3);
    assert_eq!(children(&dest(&a)), children(&dest(&b)));
    for e in read_receipt(&b, "claude-code")["entries"]
        .as_array()
        .unwrap()
    {
        assert_eq!(e["provenance"], serde_json::json!([{"bundle": "my-set"}]));
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_13_two_skills_from_two_categories_both_applied_with_no_category_step() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let d = dest(&p);
    assert_eq!(children(&d), vec!["code-review", "pytest-fixtures"]);
    for (name, rel) in [
        ("code-review", "sdlc/code-review"),
        ("pytest-fixtures", "testing/pytest-fixtures"),
    ] {
        let link = d.join(name);
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_link(&link).unwrap(), canonical_lib(&fx, rel));
        assert!(
            fs::read_to_string(link.join("SKILL.md"))
                .unwrap()
                .contains(name)
        );
    }
    assert_eq!(receipt_entry_texts(&p, "claude-code").len(), 2);
    let sel = selection_text(&p);
    assert!(sel.contains("fx:sdlc/code-review") && sel.contains("fx:testing/pytest-fixtures"));
    // Status half (M4): both entries are `active`.
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        record_states(&v),
        vec![
            ("code-review".to_string(), "active".to_string()),
            ("pytest-fixtures".to_string(), "active".to_string()),
        ]
    );
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_15_yes_still_prints_the_plan_and_the_plan_matches_the_filesystem() {
    let fx = Fx::new();
    let p = fx.project("p");
    let (code, out, _) = fx.run(
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    assert_eq!(code, 0);
    for want in ["Project   ", "Target    claude-code", "Strategy  symlink"] {
        assert!(out.contains(want), "{want}: {out}");
    }
    assert_eq!(out.matches("\n  + ").count(), 2, "{out}");
    // A second change in two `--json` runs.
    for args in [
        &["rm", "fx:testing/pytest-fixtures", "--yes"][..],
        &["add", "fx:data/schema-migration", "--yes"],
    ] {
        let v = ok(&fx, &p, args);
        let steps = v["data"]["plan"]["steps"].as_array().unwrap().clone();
        assert!(!steps.is_empty());
        for s in &steps {
            let at = p.join(s["dest"].as_str().unwrap());
            match s["op"].as_str().unwrap() {
                "add" => {
                    assert_eq!(
                        fs::read_link(&at).unwrap(),
                        PathBuf::from(s["source"].as_str().unwrap())
                    );
                }
                "remove" => assert!(fs::symlink_metadata(&at).is_err()),
                other => panic!("unexpected step {other}"),
            }
        }
        for u in v["data"]["plan"]["unchanged"].as_array().unwrap() {
            assert!(fs::symlink_metadata(p.join(u["dest"].as_str().unwrap())).is_ok());
        }
    }
    let receipted: Vec<String> = read_receipt(&p, "claude-code")["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    for c in children(&dest(&p)) {
        assert!(
            receipted.contains(&c),
            "{c} is neither planned nor receipted"
        );
    }
    // Status half (M4): `status` reports exactly the entries the plans
    // produced, all `active`.
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    let mut shown: Vec<String> = record_states(&v).into_iter().map(|(n, _)| n).collect();
    shown.sort();
    assert_eq!(shown, children(&dest(&p)));
    assert!(record_states(&v).iter().all(|(_, s)| s == "active"));
    fx.done();
}

#[test]
fn ac_16_second_identical_activation_makes_zero_changes_and_says_so() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let applied = p.join(".unam/claude-code/applied.json");
    let sel = p.join(".unam/selection.yaml");
    let stamp = |f: &Path| {
        (
            fs::read(f).unwrap(),
            fs::metadata(f).unwrap().modified().unwrap(),
        )
    };
    let (a0, s0) = (stamp(&applied), stamp(&sel));
    let (code, out, _) = fx.run(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains("No changes"), "{out}");
    let v = ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["status"], "no-changes");
    assert!(v["data"]["plan"]["steps"].as_array().unwrap().is_empty());
    assert_eq!(stamp(&applied), a0);
    assert_eq!(stamp(&sel), s0);
    assert_eq!(history_count(&p), 1);
    assert!(!p.join(".unam/claude-code/journal.jsonl").exists());
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_17_remove_one_add_one_changes_exactly_two_entries_and_a_third_record_is_verbatim() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "fx:data/schema-migration",
            "--yes",
        ],
    );
    let d = dest(&p);
    let kept = ["code-review", "schema-migration"];
    let inodes: Vec<u64> = kept.iter().map(|n| ino(&d.join(n))).collect();
    let texts = |p: &Path| -> Vec<String> {
        receipt_entry_texts(p, "claude-code")
            .into_iter()
            .filter(|t| {
                kept.iter()
                    .any(|k| t.contains(&format!("\"name\":\"{k}\"")))
            })
            .collect()
    };
    let before_texts = texts(&p);
    let before_children = children(&d);
    ok(&fx, &p, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
    ok(&fx, &p, &["add", "hs:ops/deploy-checklist", "--yes"]);
    let after_children = children(&d);
    let gone: Vec<&String> = before_children
        .iter()
        .filter(|c| !after_children.contains(c))
        .collect();
    let new: Vec<&String> = after_children
        .iter()
        .filter(|c| !before_children.contains(c))
        .collect();
    assert_eq!(gone, vec!["pytest-fixtures"]);
    assert_eq!(new, vec!["deploy-checklist"]);
    let after_inodes: Vec<u64> = kept.iter().map(|n| ino(&d.join(n))).collect();
    assert_eq!(inodes, after_inodes);
    assert_eq!(before_texts, texts(&p));
    assert_eq!(before_texts.len(), 2);
    fx.done();
}

#[test]
fn ac_18_replace_and_add_from_the_same_start_give_different_end_states() {
    let fx = Fx::new();
    fx.write_bundle("my-set", &["hs:ops/deploy-checklist", "hs:api/rest-design"]);
    let p1 = fx.project("p1");
    let p2 = fx.project("p2");
    for p in [&p1, &p2] {
        ok(&fx, p, &["use", "hermes-only", "--replace", "--yes"]);
        assert_eq!(children(&dest(p)).len(), 3);
    }
    ok(&fx, &p1, &["use", "my-set", "--replace", "--yes"]);
    // Registration (Gap 13): the canonical root, exactly once.
    let index: Value = serde_json::from_slice(&fs::read(fx.env.project_index()).unwrap()).unwrap();
    let canon1 = fx.canon(&p1).to_string_lossy().into_owned();
    let hits = index["projects"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|x| x.as_str() == Some(canon1.as_str()))
        .count();
    assert_eq!(hits, 1, "{index}");
    assert_eq!(
        children(&dest(&p1)),
        vec!["deploy-checklist", "rest-design"]
    );
    let s1 = selection_text(&p1);
    assert!(s1.contains("bundles:\n    - my-set\n"), "{s1}");
    assert!(s1.contains("add: []") && s1.contains("remove: []"), "{s1}");
    ok(&fx, &p2, &["use", "my-set", "--add", "--yes"]);
    assert_eq!(children(&dest(&p2)).len(), 5);
    let s2 = selection_text(&p2);
    assert!(s2.contains("- hermes-only\n    - my-set"), "{s2}");
    assert_eq!(
        read_receipt(&p1, "claude-code")["entries"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        read_receipt(&p2, "claude-code")["entries"]
            .as_array()
            .unwrap()
            .len(),
        5
    );
    assert_ne!(children(&dest(&p1)), children(&dest(&p2)));
    fx.done();
}

#[test]
fn ac_19_operations_in_project_a_leave_project_b_unchanged_by_hash() {
    let fx = Fx::new();
    let a = fx.project("a");
    let b = fx.project("b");
    ok(&fx, &a, &["add", "fx:sdlc/code-review", "--yes"]);
    ok(&fx, &b, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    let snap = snapshot_source(&b);
    ok(&fx, &a, &["add", "fx:data/schema-migration", "--yes"]);
    ok(&fx, &a, &["rm", "fx:sdlc/code-review", "--yes"]);
    ok(&fx, &a, &["use", "hermes-only", "--replace", "--yes"]);
    ok(&fx, &a, &["deactivate", "--yes"]);
    assert_source_unchanged(&snap, &b);
    fx.done();
}

#[test]
fn ac_21_returning_to_a_project_shows_its_prior_selection_unchanged() {
    let fx = Fx::new();
    let a = fx.project("a");
    let b = fx.project("b");
    ok(
        &fx,
        &a,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    ok(&fx, &b, &["use", "hermes-only", "--replace", "--yes"]);
    let sel_a = fs::read(a.join(".unam/selection.yaml")).unwrap();
    let rec_a = fs::read(a.join(".unam/claude-code/applied.json")).unwrap();
    let snap_b = snapshot_source(&b);
    let (code, out, _) = fx.run(&a, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains("No changes"), "{out}");
    let v = ok(&fx, &a, &["add", "fx:sdlc/code-review", "--yes"]);
    let unchanged: Vec<&str> = v["data"]["plan"]["unchanged"]
        .as_array()
        .unwrap()
        .iter()
        .map(|u| u["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        unchanged,
        vec!["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]
    );
    assert_eq!(fs::read(a.join(".unam/selection.yaml")).unwrap(), sel_a);
    assert_eq!(
        fs::read(a.join(".unam/claude-code/applied.json")).unwrap(),
        rec_a
    );
    assert_source_unchanged(&snap_b, &b);
    // Status half (M4): the prior selection and states are unchanged.
    let (code, status_a) = fx.status(&a);
    assert_eq!(code, 0, "{status_a}");
    assert_eq!(
        record_states(&status_a),
        vec![
            ("code-review".to_string(), "active".to_string()),
            ("pytest-fixtures".to_string(), "active".to_string()),
        ]
    );
    let (code, status_b) = fx.status(&b);
    assert_eq!(code, 0, "{status_b}");
    assert_eq!(record_states(&status_b).len(), 3);
    assert!(record_states(&status_b).iter().all(|(_, s)| s == "active"));
    assert_eq!(fs::read(a.join(".unam/selection.yaml")).unwrap(), sel_a);
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_22_collision_with_a_foreign_skill_refuses_names_both_paths_and_changes_nothing() {
    let fx = Fx::new();
    let own = fx.env.path("work/users-own-dir");
    fs::create_dir_all(&own).unwrap();
    fx.plant_file(&own.join("keep.md"), b"mine\n");
    type Plant<'a> = Box<dyn Fn(&Path) + 'a>;
    let variants: Vec<(&str, Plant<'_>)> = vec![
        (
            "dir",
            Box::new(|p: &Path| {
                fx.plant_foreign(p, "code-review");
            }),
        ),
        (
            "file",
            Box::new(|p: &Path| {
                fx.plant_file(&dest(p).join("code-review"), b"a plain file\n");
            }),
        ),
        (
            "link",
            Box::new(|p: &Path| {
                fx.plant_link(&dest(p).join("code-review"), &own);
            }),
        ),
        (
            "broken",
            Box::new(|p: &Path| {
                fx.plant_link(
                    &dest(p).join("code-review"),
                    Path::new("/nonexistent/target"),
                );
            }),
        ),
        (
            "empty",
            Box::new(|p: &Path| {
                fs::create_dir_all(dest(p).join("code-review")).unwrap();
            }),
        ),
    ];
    for (name, plant) in variants {
        let p = fx.project(name);
        plant(&p);
        let start = snapshot_project(&p);
        let (code, v) = fx.json_raw(&p, &["add", "fx:sdlc/code-review", "--yes"]);
        assert_eq!(code, 6, "{name}: {v}");
        assert_eq!(v["errors"][0]["code"], "blocked-by-conflicts");
        let c = &v["data"]["plan"]["conflicts"][0];
        assert_eq!(c["code"], "foreign-at-dest", "{name}");
        let msg = c["message"].as_str().unwrap();
        assert!(msg.contains(".claude/skills/code-review"), "{msg}");
        let src = canonical_lib(&fx, "sdlc/code-review");
        assert!(msg.contains(&*src.to_string_lossy()), "{msg}");
        assert_eq!(c["source"], src.to_string_lossy().as_ref());
        for f in [
            "selection.yaml",
            "claude-code/applied.json",
            "claude-code/journal.jsonl",
        ] {
            assert!(!p.join(".unam").join(f).exists(), "{name}: {f}");
        }
        assert_project_unchanged(&start, &p);
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_23_deactivate_all_removes_exactly_the_two_owned_and_leaves_foreign_and_directory() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    fx.plant_foreign(&p, "house-style");
    let v = ok(&fx, &p, &["deactivate", "--all", "--yes"]);
    assert_eq!(v["data"]["status"], "applied");
    assert_eq!(children(&dest(&p)), vec!["house-style"]);
    assert!(dest(&p).is_dir());
    assert_eq!(
        read_receipt(&p, "claude-code")["entries"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert!(!selection_text(&p).contains("claude-code"));
    fx.done();
}

#[test]
fn ac_24_no_agent_config_file_is_created_or_modified() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.plant_file(&p.join("CLAUDE.md"), b"# project rules\n");
    fx.plant_file(&p.join(".claude/settings.json"), b"{\"x\": 1}\n");
    fx.plant_file(&p.join(".mcp.json"), b"{}\n");
    let outside = |p: &Path| -> Vec<String> {
        snapshot_project(p)
            .0
            .into_iter()
            .map(|e| e.0)
            .filter(|r| {
                !(r.starts_with(".claude/skills")
                    || r.starts_with(".unam")
                    || r.starts_with(".git"))
            })
            .collect()
    };
    let start = outside(&p);
    let (code, out, _) = fx.run(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Add these lines to .gitignore (unam never edits it):"),
        "{out}"
    );
    assert!(out.contains(".unam/*/applied.json"), "{out}");
    let mut got = outside(&p);
    got.retain(|r| r != ".claude/skills");
    assert_eq!(got, start);
    ok(&fx, &p, &["deactivate", "--yes"]);
    assert_eq!(outside(&p), start);
    assert!(!p.join(".gitignore").exists());
    fx.done();
}

#[test]
fn ac_25_every_created_entry_is_in_the_receipt_with_source_id_and_strategy() {
    let fx = Fx::new();
    let sym = fx.project("sym");
    let copy = fx.project("copy");
    fx.write_selection(&copy, "targets:\n  claude-code:\n    strategy: copy\n");
    for p in [&sym, &copy] {
        ok(
            &fx,
            p,
            &[
                "add",
                "fx:sdlc/code-review",
                "hs:ops/deploy-checklist",
                "--yes",
            ],
        );
        let r = read_receipt(p, "claude-code");
        let entries = r["entries"].as_array().unwrap();
        assert_eq!(entries.len(), children(&dest(p)).len());
        for e in entries {
            for k in [
                "skill_id",
                "strategy",
                "dest",
                "name",
                "pin",
                "provenance",
                "applied_at",
            ] {
                assert!(!e[k].is_null(), "{k}: {e}");
            }
            assert_eq!(e["pin"], "follow");
            let name = e["name"].as_str().unwrap();
            assert_eq!(e["dest"], format!(".claude/skills/{name}"));
            let src = PathBuf::from(e["source_path"].as_str().unwrap());
            let catalog_hash = unam_core::hash::tree_hash(&src, &[".git", ".DS_Store"])
                .unwrap()
                .to_string();
            assert_eq!(e["tree_hash"], catalog_hash.as_str());
            let at = p.join(e["dest"].as_str().unwrap());
            if e["strategy"] == "copy" {
                let got = unam_core::hash::tree_hash(&at, &[".git", ".DS_Store"])
                    .unwrap()
                    .to_string();
                assert_eq!(got, catalog_hash);
            } else {
                assert_eq!(fs::read_link(&at).unwrap(), src);
                assert_eq!(src, fs::canonicalize(&src).unwrap());
            }
        }
    }
    fx.done();
}

#[test]
fn ac_29_claude_code_output_states_no_restart_required_with_evidence() {
    let fx = Fx::new();
    let existing = fx.project("existing");
    fs::create_dir_all(dest(&existing)).unwrap();
    let (code, out, _) = fx.run(&existing, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains("no restart or reload required"), "{out}");
    assert!(out.contains("code.claude.com/docs/en/skills"), "{out}");
    assert!(!out.contains("now active"));
    let v = ok(
        &fx,
        &existing,
        &["add", "fx:testing/pytest-fixtures", "--yes"],
    );
    assert_eq!(v["data"]["refresh"]["kind"], "none");
    assert!(
        v["data"]["refresh"]["evidence"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
    let fresh = fx.project("fresh");
    let (code, out, _) = fx.run(&fresh, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(
        out.contains("not detected; .claude/skills will be created"),
        "{out}"
    );
    assert!(!out.contains("now active"));
    let fresh2 = fx.project("fresh2");
    let v = ok(&fx, &fresh2, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["refresh"]["kind"], "command");
    assert_eq!(v["data"]["refresh"]["command"], "/reload-skills");
    fx.done();
}

#[test]
fn ac_30_every_removal_prints_the_loaded_instructions_caveat() {
    let fx = Fx::new();
    const SES2: &str = "Removed from future discovery. A conversation that already loaded these instructions keeps them until that conversation ends.";
    let p = fx.project("p");
    let (code, out, _) = fx.run(
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    assert_eq!(code, 0);
    assert!(!out.contains(SES2));
    let v = ok(&fx, &p, &["add", "fx:data/schema-migration", "--yes"]);
    assert!(v["data"]["removal_caveat"].is_null());
    let (code, out, _) = fx.run(&p, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains(SES2), "{out}");
    let v = ok(&fx, &p, &["use", "hermes-only", "--replace", "--yes"]);
    assert_eq!(v["data"]["removal_caveat"], SES2);
    let (code, out, _) = fx.run(&p, &["deactivate", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains(SES2), "{out}");
    fx.done();
}

#[test]
fn ac_40_skill_in_two_bundles_appears_once_with_two_provenance_records() {
    let fx = Fx::new();
    fx.write_bundle("b1", &["fx:sdlc/code-review"]);
    fx.write_bundle("b2", &["fx:sdlc/code-review"]);
    let p = fx.project("p");
    ok(&fx, &p, &["use", "b1", "--replace", "--yes"]);
    let v = ok(&fx, &p, &["use", "b2", "--add", "--yes"]);
    let steps = v["data"]["plan"]["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 1);
    assert_eq!(steps[0]["op"], "update_provenance");
    assert_eq!(children(&dest(&p)), vec!["code-review"]);
    let r = read_receipt(&p, "claude-code");
    assert_eq!(r["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        r["entries"][0]["provenance"],
        serde_json::json!([{"bundle": "b1"}, {"bundle": "b2"}])
    );
    let v = ok(&fx, &p, &["use", "b2", "--replace", "--yes"]);
    assert!(
        v["data"]["plan"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .all(|s| s["op"] != "remove")
    );
    let r = read_receipt(&p, "claude-code");
    assert_eq!(
        r["entries"][0]["provenance"],
        serde_json::json!([{"bundle": "b2"}])
    );
    assert_eq!(children(&dest(&p)), vec!["code-review"]);
    fx.done();
}

#[test]
fn s2_one_command_add() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    let before = read_receipt(&p, "claude-code");
    let inv = Invocations::default();
    let n0 = inv.count();
    let (code, v) = inv.run_json(&fx, &p, &["add", "fx:data/schema-migration", "--yes"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(inv.count() - n0, 1);
    let after = read_receipt(&p, "claude-code");
    assert_eq!(
        after["entries"].as_array().unwrap().len(),
        before["entries"].as_array().unwrap().len() + 1
    );
    assert_ne!(after["last_op_id"], before["last_op_id"]);
    assert!(dest(&p).join("schema-migration").exists());
    fx.done();
}
