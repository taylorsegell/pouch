//! CLI contract of the activation commands (milestone M3: exit codes 2, 4,
//! 5, 6, 7, 11 paths, flags, target rules, strategies, ownership release,
//! carry, shadows, the gitignore offer, history cap, created directories,
//! and the G1 and G7 refusals). Every test ends with `fx.done()`.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::*;
use common::*;
use pouch_core::change::{self, Decision, Edit, Outcome};
use pouch_core::fsops::RealFs;
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

fn fails(fx: &Fx, p: &Path, args: &[&str], code: i32, err: &str) -> Value {
    let (c, v) = fx.json_raw(p, args);
    assert_eq!(c, code, "{args:?}: {v}");
    assert_eq!(v["errors"][0]["code"], err, "{args:?}: {v}");
    v
}

fn selection(p: &Path) -> Option<String> {
    fs::read_to_string(p.join(".pouch/selection.yaml")).ok()
}

fn receipt_names(p: &Path) -> Vec<String> {
    let mut v: Vec<String> = read_receipt(p, "claude-code")["entries"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap().to_string())
        .collect();
    v.sort();
    v
}

#[test]
fn add_unresolved_ref_blocks_with_nothing_written() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    fails(
        &fx,
        &p,
        &["add", "fx:nope/missing", "--yes"],
        4,
        "not-found",
    );
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn add_ambiguous_ref_exit_5_nothing_written() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let v = fails(
        &fx,
        &p,
        &["add", "schema-migration", "--yes"],
        5,
        "ambiguous",
    );
    assert_eq!(v["errors"][0]["candidates"].as_array().unwrap().len(), 2);
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn rm_unselected_is_exit_4_with_no_change() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let start = snapshot_project(&p);
    fails(
        &fx,
        &p,
        &["rm", "fx:testing/pytest-fixtures", "--yes"],
        4,
        "not-selected",
    );
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn rm_drops_ad_hoc_entry_and_records_remove_for_bundle_supplied() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["use", "hermes-only", "--replace", "--yes"]);
    ok(&fx, &p, &["add", "fx:data/schema-migration", "--yes"]);
    let v = ok(&fx, &p, &["rm", "fx:data/schema-migration", "--yes"]);
    assert!(v["data"]["selection"]["add"].as_array().unwrap().is_empty());
    assert!(
        v["data"]["selection"]["remove"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let v = ok(&fx, &p, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
    assert_eq!(
        v["data"]["selection"]["remove"],
        serde_json::json!(["fx:testing/pytest-fixtures"])
    );
    assert_eq!(children(&dest(&p)), vec!["alpha-root-skill", "code-review"]);
    fx.done();
}

#[test]
fn use_requires_replace_or_add() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let (code, _, err) = fx.run(&p, &["use", "hermes-only", "--yes"]);
    assert_eq!(code, 2, "{err}");
    let (code, _, err) = fx.run(&p, &["use", "hermes-only", "--replace", "--add", "--yes"]);
    assert_eq!(code, 2, "{err}");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn use_unknown_bundle_exit_4() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    fails(
        &fx,
        &p,
        &["use", "nope", "--replace", "--yes"],
        4,
        "bundle-not-found",
    );
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn use_add_is_idempotent() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["use", "hermes-only", "--add", "--yes"]);
    let before = selection(&p).unwrap();
    let v = ok(&fx, &p, &["use", "hermes-only", "--add", "--yes"]);
    assert_eq!(v["data"]["status"], "no-changes");
    assert_eq!(selection(&p).unwrap(), before);
    assert_eq!(before.matches("hermes-only").count(), 1);
    fx.done();
}

#[test]
fn deactivate_without_selection_is_no_changes() {
    let fx = Fx::new();
    let p = fx.project("p");
    let v = ok(&fx, &p, &["deactivate", "--yes"]);
    assert_eq!(v["data"]["status"], "no-changes");
    assert_eq!(v["data"]["selection"]["written"], false);
    assert!(selection(&p).is_none());
    assert!(!p.join(".claude").exists());
    fx.done();
}

#[test]
fn deactivate_all_equals_default_with_one_adapter() {
    let fx = Fx::new();
    let a = fx.project("a");
    let b = fx.project("b");
    for p in [&a, &b] {
        ok(&fx, p, &["add", "fx:sdlc/code-review", "--yes"]);
    }
    let (_, va) = fx.json(&a, &["deactivate", "--yes"]);
    let (_, vb) = fx.json(&b, &["deactivate", "--all", "--yes"]);
    let strip = |v: &Value| {
        let mut d = v["data"].clone();
        d["project_root"] = Value::Null;
        d["selection"]["path"] = Value::Null;
        d["receipt"]["path"] = Value::Null;
        d.to_string()
            .replace("/work/a/", "/work/X/")
            .replace("/work/b/", "/work/X/")
    };
    assert_eq!(strip(&va), strip(&vb));
    assert_eq!(selection(&a), selection(&b));
    assert_eq!(children(&dest(&a)), children(&dest(&b)));
    assert_eq!(receipt_names(&a), receipt_names(&b));
    assert_eq!(
        read_receipt(&a, "claude-code")["created_dirs"],
        read_receipt(&b, "claude-code")["created_dirs"]
    );
    assert!(!a.join(".claude").exists() && !b.join(".claude").exists());
    fx.done();
}

#[test]
fn no_yes_without_a_terminal_refuses_exit_2_with_plan_and_changes_nothing() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let (code, out, err) = fx.run(&p, &["add", "fx:sdlc/code-review"]);
    assert_eq!(code, 2, "{err}");
    assert!(
        out.contains("  + sdlc/code-review"),
        "the plan is printed: {out}"
    );
    assert!(err.contains("Pass `--yes`"), "{err}");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn json_without_yes_refuses_exit_2() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let v = fails(
        &fx,
        &p,
        &["add", "fx:sdlc/code-review"],
        2,
        "confirmation-required",
    );
    assert_eq!(v["data"]["status"], "refused");
    assert_eq!(v["data"]["plan"]["steps"].as_array().unwrap().len(), 1);
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn empty_plan_needs_no_confirmation() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let (code, out, err) = fx.run(&p, &["add", "fx:sdlc/code-review"]);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("No changes"), "{out}");
    fx.done();
}

#[test]
fn conflicts_exit_6_before_any_prompt() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.plant_foreign(&p, "code-review");
    let (code, out, err) = fx.run(&p, &["add", "fx:sdlc/code-review"]);
    assert_eq!(code, 6, "{err}");
    assert!(out.contains("! conflict"), "{out}");
    assert!(!err.contains("Pass `--yes`"), "{err}");
    fx.done();
}

#[test]
fn agents_neutral_flag_value_is_a_supported_target() {
    let fx = Fx::new();
    let p = fx.project("p");
    // M5: `--target agents-neutral` is a registered target (it was
    // `target-unsupported` before the adapter existed).
    let (code, v) = fx.json(
        &p,
        &[
            "--target",
            "agents-neutral",
            "add",
            "fx:sdlc/code-review",
            "--yes",
        ],
    );
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["data"]["target"], "agents-neutral");
    assert_eq!(v["data"]["destination"], ".agents/skills");
    assert!(p.join(".agents/skills/code-review").exists());
    assert!(!p.join(".claude").exists());
    fx.done();
}

#[test]
fn invalid_agent_flag_value_exit_2() {
    let fx = Fx::new();
    let p = fx.project("p");
    fails(
        &fx,
        &p,
        &["add", "fx:sdlc/code-review", "--target", "cursor", "--yes"],
        2,
        "invalid-target",
    );
    fx.done();
}

#[test]
fn project_flag_overrides_detection() {
    let fx = Fx::new();
    let outer = fx.project("outer");
    let inner = outer.join("inner");
    fs::create_dir_all(&inner).unwrap();
    let cwd = fx.env.path("cwd");
    let v = ok(
        &fx,
        &cwd,
        &[
            "add",
            "fx:sdlc/code-review",
            "--project",
            inner.to_str().unwrap(),
            "--yes",
        ],
    );
    assert_eq!(
        v["data"]["project_root"],
        fx.canon(&inner).to_string_lossy().as_ref()
    );
    assert_eq!(v["data"]["root_detection"], "explicit");
    assert!(dest(&inner).join("code-review").exists());
    assert!(!outer.join(".claude").exists());
    assert!(!outer.join(".pouch").exists());
    fx.done();
}

#[test]
fn project_flag_missing_path_exit_2() {
    let fx = Fx::new();
    let cwd = fx.env.path("cwd");
    let missing = fx.env.path("work/missing");
    fails(
        &fx,
        &cwd,
        &[
            "add",
            "fx:sdlc/code-review",
            "--project",
            missing.to_str().unwrap(),
            "--yes",
        ],
        2,
        "project-path-invalid",
    );
    assert!(!missing.exists());
    fx.done();
}

#[test]
fn two_selection_entries_need_the_agent_flag() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.write_selection(
        &p,
        "targets:\n  claude-code:\n    add: [code-review]\n  agents-neutral:\n    add: [code-review]\n",
    );
    let v = fails(
        &fx,
        &p,
        &["add", "fx:testing/pytest-fixtures", "--yes"],
        2,
        "target-ambiguous",
    );
    assert!(
        v["errors"][0]["remedy"]
            .as_str()
            .unwrap()
            .contains("--target <id>")
    );
    ok(
        &fx,
        &p,
        &[
            "--target",
            "claude-code",
            "add",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    assert_eq!(children(&dest(&p)), vec!["code-review", "pytest-fixtures"]);
    fx.done();
}

#[test]
fn copy_strategy_applies_and_removes_a_managed_copy() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.write_selection(&p, "targets:\n  claude-code:\n    strategy: copy\n");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let copy = dest(&p).join("code-review");
    let meta = fs::symlink_metadata(&copy).unwrap();
    assert!(meta.is_dir() && !meta.file_type().is_symlink());
    assert!(copy.join("references/checklist.md").is_file());
    ok(&fx, &p, &["rm", "fx:sdlc/code-review", "--yes"]);
    assert!(fs::symlink_metadata(&copy).is_err());
    let trash = p.join(".pouch/claude-code/trash");
    assert!(children(&trash).is_empty(), "{:?}", children(&trash));
    fx.done();
}

#[cfg(unix)]
#[test]
fn symlink_strategy_removal_unlinks_only_the_link() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let link = dest(&p).join("code-review");
    let target = fs::read_link(&link).unwrap();
    ok(&fx, &p, &["rm", "fx:sdlc/code-review", "--yes"]);
    assert!(fs::symlink_metadata(&link).is_err());
    assert!(target.join("SKILL.md").is_file());
    fx.done();
}

/// Run `rm fx:sdlc/code-review` through the core API, calling `edit`
/// inside the confirmation window (after the plan, before the apply).
fn rm_with_edit_after_plan(fx: &Fx, p: &Path, edit: &dyn Fn()) -> pouch_core::apply::ApplyReport {
    let c = core_ctx(&fx.env, p);
    let clock = StepClock::new();
    let req = c.request(
        Edit::Remove(vec!["fx:sdlc/code-review".into()]),
        "rm",
        &clock,
    );
    let out = in_lock_section(|| {
        change::run(&req, &RealFs, &mut |_| {
            edit();
            Decision::Proceed
        })
    })
    .map_err(|f| f.error)
    .unwrap();
    let Outcome::Applied(_, report) = out else {
        panic!("not applied")
    };
    *report
}

#[test]
fn modified_owned_copy_is_released_and_never_deleted() {
    let fx = Fx::new();
    // Edited before the command: the plan releases ownership (P7,
    // drop_orphan modified) and nothing is deleted.
    let p = fx.project("p");
    fx.write_selection(&p, "targets:\n  claude-code:\n    strategy: copy\n");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let edited = dest(&p).join("code-review/references/checklist.md");
    fs::write(&edited, b"my own notes\n").unwrap();
    let v = ok(&fx, &p, &["rm", "fx:sdlc/code-review", "--yes"]);
    let step = &v["data"]["plan"]["steps"][0];
    assert_eq!(step["op"], "drop_orphan");
    assert_eq!(step["reason"], "modified");
    assert_eq!(fs::read(&edited).unwrap(), b"my own notes\n");
    assert!(receipt_names(&p).is_empty());
    // Edited after the plan was shown: the removal is skipped `modified`
    // (exit 7 class), the copy is intact, the receipt entry is gone.
    let q = fx.project("q");
    fx.write_selection(&q, "targets:\n  claude-code:\n    strategy: copy\n");
    ok(&fx, &q, &["add", "fx:sdlc/code-review", "--yes"]);
    let edited_q = dest(&q).join("code-review/references/checklist.md");
    let report = rm_with_edit_after_plan(&fx, &q, &|| {
        fs::write(&edited_q, b"late edit\n").unwrap();
    });
    assert_eq!(
        report.status,
        pouch_core::apply::ApplyStatus::AppliedWithSkips
    );
    assert_eq!(
        report.results[0].outcome,
        pouch_core::plan::StepOutcome::Skipped(pouch_core::fsops::SkipReason::Modified)
    );
    assert_eq!(fs::read(&edited_q).unwrap(), b"late edit\n");
    assert!(receipt_names(&q).is_empty());
    assert!(!q.join(".pouch/claude-code/journal.jsonl").exists());
    fx.done();
}

#[cfg(unix)]
#[test]
fn relinked_owned_symlink_is_released_and_never_deleted() {
    let fx = Fx::new();
    let mine = fx.env.path("work/mine");
    fs::create_dir_all(&mine).unwrap();
    // Relinked before the command: released by the plan.
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let link = dest(&p).join("code-review");
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&mine, &link).unwrap();
    let v = ok(&fx, &p, &["rm", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["plan"]["steps"][0]["op"], "drop_orphan");
    assert_eq!(fs::read_link(&link).unwrap(), mine);
    assert!(receipt_names(&p).is_empty());
    // Relinked after the plan was shown: skipped `modified`, link kept.
    let q = fx.project("q");
    ok(&fx, &q, &["add", "fx:sdlc/code-review", "--yes"]);
    let link_q = dest(&q).join("code-review");
    let report = rm_with_edit_after_plan(&fx, &q, &|| {
        fs::remove_file(&link_q).unwrap();
        std::os::unix::fs::symlink(&mine, &link_q).unwrap();
    });
    assert_eq!(
        report.results[0].outcome,
        pouch_core::plan::StepOutcome::Skipped(pouch_core::fsops::SkipReason::Modified)
    );
    assert_eq!(fs::read_link(&link_q).unwrap(), mine);
    assert!(receipt_names(&q).is_empty());
    fx.done();
}

#[test]
fn unreachable_source_owned_entry_is_carried_not_removed() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &[
            "add",
            "hs:ops/deploy-checklist",
            "fx:sdlc/code-review",
            "--yes",
        ],
    );
    let moved = fx.env.path("work/lib2-moved");
    fs::rename(&fx.lib2, &moved).unwrap();
    let (code, v) = fx.json_raw(&p, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    fs::rename(&moved, &fx.lib2).unwrap();
    assert_eq!(code, 0, "{v}");
    let carried = v["data"]["plan"]["carried"].as_array().unwrap();
    assert_eq!(carried.len(), 1);
    assert_eq!(carried[0]["id"], "hs:ops/deploy-checklist");
    assert!(
        v["data"]["plan"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "owned-source-unreachable")
    );
    assert!(fs::symlink_metadata(dest(&p).join("deploy-checklist")).is_ok());
    assert_eq!(
        receipt_names(&p),
        vec!["code-review", "deploy-checklist", "pytest-fixtures"]
    );
    fx.done();
}

#[test]
fn invalid_skill_name_is_a_blocking_conflict() {
    let fx = Fx::new_with(|lib| {
        write_file(
            lib,
            "odd/backslash/SKILL.md",
            b"---\nname: 'bad\\name'\ndescription: A name that cannot be a directory.\n---\n",
        );
    });
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let v = fails(
        &fx,
        &p,
        &["add", "fx:odd/backslash", "--yes"],
        6,
        "blocked-by-conflicts",
    );
    assert_eq!(v["data"]["plan"]["conflicts"][0]["code"], "invalid-name");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[test]
fn duplicate_name_is_a_blocking_conflict() {
    let fx = Fx::new();
    let p = fx.project("p");
    let start = snapshot_project(&p);
    let v = fails(
        &fx,
        &p,
        &[
            "add",
            "fx:engineering/backend/api/error-handling",
            "fx:engineering/frontend/error-handling",
            "--yes",
        ],
        6,
        "blocked-by-conflicts",
    );
    assert_eq!(v["data"]["plan"]["conflicts"][0]["code"], "duplicate-name");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[cfg(unix)]
#[test]
fn dest_root_symlink_outside_project_is_a_blocking_conflict() {
    let fx = Fx::new();
    let p = fx.project("p");
    let outside = fx.env.path("work/outside");
    fs::create_dir_all(outside.join("skills")).unwrap();
    fx.plant_file(&outside.join("skills/keep.md"), b"outside\n");
    std::os::unix::fs::symlink(&outside, p.join(".claude")).unwrap();
    let v = fails(
        &fx,
        &p,
        &["add", "fx:sdlc/code-review", "--yes"],
        6,
        "blocked-by-conflicts",
    );
    assert_eq!(
        v["data"]["plan"]["conflicts"][0]["code"],
        "dest-outside-project"
    );
    assert_eq!(children(&outside.join("skills")), vec!["keep.md"]);
    fx.done();
}

#[test]
fn shadow_warning_names_the_higher_scope_skill() {
    let fx = Fx::new();
    let p = fx.project("p");
    let enterprise = fx.env.path("enterprise").join("code-review");
    let user = fx
        .env
        .path("home")
        .join(".claude")
        .join("skills")
        .join("code-review");
    for d in [&enterprise, &user] {
        write_file(
            d,
            "SKILL.md",
            skill_md("code-review", "Higher scope.").as_bytes(),
        );
    }
    let v = ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let shadows: Vec<&Value> = v["data"]["plan"]["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["code"] == "shadowed")
        .collect();
    assert_eq!(shadows.len(), 2, "{v}");
    assert!(
        shadows[0]["message"]
            .as_str()
            .unwrap()
            .contains("enterprise scope wins")
    );
    assert!(
        shadows[1]["message"]
            .as_str()
            .unwrap()
            .contains("user scope wins")
    );
    assert_eq!(shadows[1]["path"], user.to_string_lossy().as_ref());
    fx.done();
}

#[test]
fn first_selection_write_offers_gitignore_lines_and_never_writes_gitignore() {
    let fx = Fx::new();
    let a = fx.project("a");
    let (code, out, _) = fx.run(&a, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains("Add these lines to .gitignore (pouch never edits it):"));
    for l in [
        ".pouch/lock",
        ".pouch/*/applied.json",
        ".pouch/*/journal.jsonl",
        ".pouch/*/history/",
        ".pouch/*/trash/",
    ] {
        assert!(out.contains(&format!("  {l}\n")), "{l}: {out}");
    }
    let b = fx.project("b");
    let v = ok(&fx, &b, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["gitignore_lines"].as_array().unwrap().len(), 5);
    let v = ok(&fx, &b, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    assert!(v["data"]["gitignore_lines"].as_array().unwrap().is_empty());
    assert!(!a.join(".gitignore").exists() && !b.join(".gitignore").exists());
    fx.done();
}

#[test]
fn history_is_capped_at_twenty_entries() {
    let fx = Fx::new();
    let p = fx.project("p");
    let c = core_ctx(&fx.env, &p);
    let clock = StepClock::new();
    let history = p.join(".pouch/claude-code/history");
    let mut ids: Vec<String> = Vec::new();
    for i in 0..22 {
        let edit = if i % 2 == 0 {
            Edit::Add(vec!["fx:sdlc/code-review".into()])
        } else {
            Edit::Remove(vec!["fx:sdlc/code-review".into()])
        };
        let req = c.request(edit, "add", &clock);
        let out = in_lock_section(|| change::run(&req, &RealFs, &mut |_| Decision::Proceed))
            .map_err(|f| f.error)
            .unwrap();
        let Outcome::Applied(_, report) = out else {
            panic!("{i}")
        };
        ids.push(format!("{}.json", report.op_id));
        if i == 0 {
            fs::write(history.join("notes.txt"), b"mine").unwrap();
        }
    }
    let mut names = children(&history);
    assert!(names.contains(&"notes.txt".to_string()));
    names.retain(|n| n.ends_with(".json"));
    assert_eq!(names.len(), 20);
    // The two oldest are gone (op ids sort chronologically).
    ids.sort();
    assert_eq!(names, ids[2..].to_vec());
    fx.done();
}

#[test]
fn deactivate_removes_created_directories_only_when_empty() {
    let fx = Fx::new();
    let a = fx.project("a");
    ok(&fx, &a, &["add", "fx:sdlc/code-review", "--yes"]);
    ok(&fx, &a, &["deactivate", "--yes"]);
    assert!(!a.join(".claude").exists());
    let b = fx.project("b");
    ok(&fx, &b, &["add", "fx:sdlc/code-review", "--yes"]);
    fx.plant_file(&b.join(".claude/settings.json"), b"{}\n");
    ok(&fx, &b, &["deactivate", "--yes"]);
    assert!(!dest(&b).exists());
    assert!(b.join(".claude/settings.json").exists());
    let c = fx.project("c");
    ok(&fx, &c, &["add", "fx:sdlc/code-review", "--yes"]);
    fx.plant_foreign(&c, "house-style");
    ok(&fx, &c, &["deactivate", "--yes"]);
    assert_eq!(children(&dest(&c)), vec!["house-style"]);
    fx.done();
}

#[test]
fn resume_and_rollback_without_a_journal_exit_4() {
    let fx = Fx::new();
    let p = fx.project("p");
    // `resume` has no journal; `rollback` also has no history to fall back to.
    for (cmd, code) in [("resume", "no-journal"), ("rollback", "no-history")] {
        let v = fails(&fx, &p, &[cmd, "--yes"], 4, code);
        let remedy = v["errors"][0]["remedy"].as_str().unwrap();
        assert!(!remedy.contains("milestone"), "{remedy}");
    }
    fx.done();
}

#[test]
fn activation_commands_leave_sources_untouched() {
    let fx = Fx::new();
    let sym = fx.project("sym");
    let copy = fx.project("copy");
    fx.write_selection(&copy, "targets:\n  claude-code:\n    strategy: copy\n");
    for p in [&sym, &copy] {
        ok(&fx, p, &["use", "hermes-only", "--replace", "--yes"]);
        ok(&fx, p, &["add", "hs:ops/deploy-checklist", "--yes"]);
        ok(&fx, p, &["rm", "fx:testing/pytest-fixtures", "--yes"]);
        ok(&fx, p, &["use", "hermes-only", "--add", "--yes"]);
        ok(&fx, p, &["deactivate", "--yes"]);
        fails(&fx, p, &["resume", "--yes"], 4, "no-journal");
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn foreign_files_are_never_modified_by_any_command() {
    let fx = Fx::new();
    let p = fx.project("p");
    fx.plant_foreign(&p, "house-style");
    fx.plant_file(&dest(&p).join("README.md"), b"my notes\n");
    let mine = fx.env.path("work/mine");
    fs::create_dir_all(&mine).unwrap();
    fx.plant_link(&dest(&p).join("my-link"), &mine);
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    ok(&fx, &p, &["use", "hermes-only", "--replace", "--yes"]);
    ok(&fx, &p, &["rm", "fx:sdlc/code-review", "--yes"]);
    ok(&fx, &p, &["deactivate", "--yes"]);
    assert_eq!(
        children(&dest(&p)),
        vec!["README.md", "house-style", "my-link"]
    );
    fx.done();
}

#[cfg(unix)]
#[test]
fn crafted_state_symlinks_are_refused_and_the_outside_file_is_untouched() {
    use std::os::unix::fs::symlink;
    let fx = Fx::new();
    let outside = fx.env.path("work/outside-dir");
    fs::create_dir_all(&outside).unwrap();
    let victim = fx.env.path("work/victim.txt");
    fx.plant_file(&victim, b"precious\n");
    type Plant<'a> = Box<dyn Fn(&Path) + 'a>;
    let cases: Vec<(&str, Plant<'_>)> = vec![
        (
            "lock-victim",
            Box::new(|p: &Path| {
                fs::create_dir_all(p.join(".pouch")).unwrap();
                symlink(&victim, p.join(".pouch/lock")).unwrap();
            }),
        ),
        (
            "lock-dangling",
            Box::new(|p: &Path| {
                fs::create_dir_all(p.join(".pouch")).unwrap();
                symlink(outside.join("created-by-pouch"), p.join(".pouch/lock")).unwrap();
            }),
        ),
        (
            "pouch-dir",
            Box::new(|p: &Path| {
                symlink(&outside, p.join(".pouch")).unwrap();
            }),
        ),
        (
            "state-dir",
            Box::new(|p: &Path| {
                fs::create_dir_all(p.join(".pouch")).unwrap();
                symlink(&outside, p.join(".pouch/claude-code")).unwrap();
            }),
        ),
        (
            "journal",
            Box::new(|p: &Path| {
                fs::create_dir_all(p.join(".pouch/claude-code")).unwrap();
                symlink(&victim, p.join(".pouch/claude-code/journal.jsonl")).unwrap();
            }),
        ),
    ];
    for (name, plant) in cases {
        let p = fx.project(name);
        plant(&p);
        let v = fails(
            &fx,
            &p,
            &["add", "fx:sdlc/code-review", "--yes"],
            1,
            "state-dir-invalid",
        );
        assert!(!outside.join("created-by-pouch").exists(), "{name}: {v}");
        assert!(
            children(&outside).is_empty(),
            "{name}: {:?}",
            children(&outside)
        );
        assert!(!p.join(".claude").exists(), "{name}");
    }
    fx.done();
}

#[test]
fn project_root_inside_a_source_is_refused_before_the_lock() {
    let fx = Fx::new_with(|lib| {
        fs::create_dir_all(lib.join("inner-project/.git")).unwrap();
    });
    let inner = fx.lib.join("inner-project");
    fails(
        &fx,
        &inner,
        &["add", "fx:sdlc/code-review", "--yes"],
        2,
        "project-overlaps-source",
    );
    assert!(!inner.join(".pouch").exists());
    assert!(!inner.join(".claude").exists());
    fx.done();
}

#[test]
fn project_flag_equal_to_a_source_is_refused() {
    let fx = Fx::new();
    let cwd = fx.env.path("cwd");
    for cmd in [
        &["add", "fx:sdlc/code-review", "--yes"][..],
        &["deactivate", "--yes"],
        &["rollback", "--yes"],
    ] {
        let mut args: Vec<&str> = cmd.to_vec();
        args.extend(["--project", fx.lib.to_str().unwrap()]);
        fails(&fx, &cwd, &args, 2, "project-overlaps-source");
    }
    assert!(!fx.lib.join(".pouch").exists());
    fx.done();
}
