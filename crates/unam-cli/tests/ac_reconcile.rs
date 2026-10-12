//! Acceptance criteria for reconcile, rollback from history, and source
//! relocation (milestone M4: AC-33, AC-34, AC-37), the PRD S8 check, the
//! `source rm` bundle-member guard, and the seven adversarial evidence rows
//! of the repair paths (STA-6). Every test ends with `fx.done()` (or
//! `done_at` / `done_with` after a deliberate change of the library).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::*;
use common::*;
use serde_json::Value;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use unam_core::change::{self, Decision, Edit};
use unam_core::fsops::RealFs;

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

fn selection_text(p: &Path) -> String {
    fs::read_to_string(p.join(".unam/selection.yaml")).unwrap()
}

fn canon(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap()
}

fn causes(v: &Value) -> Vec<(String, String)> {
    v["data"]["divergences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["cause"].as_str().unwrap().to_string(),
                d["name"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// `(op, name)` of the steps of a plan or the results of an apply.
fn ops_and_names(rows: &Value) -> Vec<(String, String)> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["op"].as_str().unwrap().to_string(),
                r["name"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

#[cfg(unix)]
#[test]
fn ac_33_rollback_from_history_restores_prior_destination_and_receipt() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    let a_dest = children(&dest(&p));
    let a_entries = receipt_entry_texts(&p, "claude-code");
    let a_selection = selection_text(&p);
    assert_eq!(a_entries.len(), 1);
    ok(&fx, &p, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    assert_ne!(children(&dest(&p)), a_dest);

    // Rollback with no journal restores the state before the last change.
    let v = ok(&fx, &p, &["rollback", "--yes"]);
    assert_eq!(v["data"]["command"], "rollback");
    assert_eq!(v["data"]["status"], "applied");
    assert_eq!(children(&dest(&p)), a_dest);
    assert_eq!(receipt_entry_texts(&p, "claude-code"), a_entries);
    assert_eq!(selection_text(&p), a_selection);
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    assert_eq!(
        record_states(&v),
        vec![("code-review".to_string(), "active".to_string())]
    );

    // The rollback wrote its own history entry, so a second one undoes it.
    ok(&fx, &p, &["rollback", "--yes"]);
    assert_eq!(children(&dest(&p)), vec!["code-review", "pytest-fixtures"]);
    let entries = receipt_entry_texts(&p, "claude-code");
    assert_eq!(entries.len(), 2);
    assert!(entries.contains(&a_entries[0]), "the A record is verbatim");
    let (code, _) = fx.status(&p);
    assert_eq!(code, 0);

    // The very first apply rolled back removes what it created.
    let q = fx.project("q");
    ok(&fx, &q, &["add", "fx:sdlc/code-review", "--yes"]);
    assert!(dest(&q).exists());
    ok(&fx, &q, &["rollback", "--yes"]);
    assert!(!q.join(".claude").exists());
    assert!(receipt_entry_texts(&q, "claude-code").is_empty());
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_34_hand_deleted_entry_reconcile_reports_exit_8_and_repair_restores() {
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
    let link_text = fs::read_link(dest(&p).join("code-review")).unwrap();
    fs::remove_file(dest(&p).join("code-review")).unwrap();

    let (code, v) = fx.status(&p);
    assert_eq!(code, 8, "{v}");
    assert_eq!(record(&v, "code-review")["state"], "orphaned");
    assert_eq!(record(&v, "code-review")["reason"], "missing");
    let (code, v) = fx.json(&p, &["reconcile"]);
    assert_eq!(code, 8, "{v}");
    assert_eq!(v["errors"][0]["code"], "diverged");
    assert_eq!(
        causes(&v),
        vec![("hand-deleted".to_string(), "code-review".to_string())]
    );
    assert_eq!(
        ops_and_names(&v["data"]["plan"]["steps"]),
        vec![("add".to_string(), "code-review".to_string())]
    );
    // Reporting changed nothing.
    assert!(!dest(&p).join("code-review").exists());

    let v = ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    assert_eq!(v["data"]["command"], "reconcile");
    assert_eq!(
        fs::read_link(dest(&p).join("code-review")).unwrap(),
        link_text
    );
    let (code, _) = fx.json(&p, &["reconcile"]);
    assert_eq!(code, 0);
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    assert_eq!(record(&v, "house-style")["state"], "foreign");
    fx.done();
}

#[cfg(unix)]
#[test]
fn ac_37_source_relocate_repairs_every_affected_project() {
    let fx = Fx::new();
    let p1 = fx.project("p1");
    let p2 = fx.project("p2");
    ok(
        &fx,
        &p1,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    ok(&fx, &p2, &["use", "hermes-only", "--replace", "--yes"]);
    // The deliberate change: move the library.
    let new_lib = fx.env.path("work/lib-moved");
    fs::rename(&fx.lib, &new_lib).unwrap();
    for p in [&p1, &p2] {
        let (code, v) = fx.status(p);
        assert_eq!(code, 8, "{v}");
        assert!(
            record_states(&v).iter().all(|(_, s)| s == "orphaned"),
            "{v}"
        );
    }
    let v = ok(
        &fx,
        &p1,
        &[
            "source",
            "relocate",
            "fx",
            new_lib.to_str().unwrap(),
            "--yes",
        ],
    );
    assert_eq!(v["data"]["status"], "relocated");
    assert_eq!(v["data"]["path"], new_lib.to_str().unwrap());
    let projects = v["data"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    assert!(projects.iter().all(|p| p["status"] == "repaired"), "{v}");
    for p in [&p1, &p2] {
        let (code, v) = fx.status(p);
        assert_eq!(code, 0, "{v}");
        assert!(record_states(&v).iter().all(|(_, s)| s == "active"), "{v}");
    }
    assert_eq!(
        fs::read_link(dest(&p1).join("code-review")).unwrap(),
        canon(&new_lib).join("sdlc/code-review")
    );
    let (_, list) = fx.json(&p1, &["source", "list"]);
    assert_eq!(
        list["data"]["sources"][0]["path"],
        fx.env.normalize(new_lib.to_str().unwrap())
    );
    fx.done_at(&new_lib);
}

#[cfg(unix)]
#[test]
fn s8_divergence_always_reported() {
    let fx = Fx::new();
    // One project per kind of divergence, each with its own skill.
    let hand_deleted = fx.project("hand-deleted");
    ok(
        &fx,
        &hand_deleted,
        &["add", "fx:testing/pytest-fixtures", "--yes"],
    );
    fs::remove_file(dest(&hand_deleted).join("pytest-fixtures")).unwrap();

    let skill_missing = fx.project("skill-missing");
    ok(
        &fx,
        &skill_missing,
        &["add", "fx:sdlc/code-review", "--yes"],
    );

    let modified = fx.project("modified");
    fx.write_selection(&modified, "targets:\n  claude-code:\n    strategy: copy\n");
    ok(
        &fx,
        &modified,
        &["add", "fx:data/schema-migration", "--yes"],
    );
    let copied = dest(&modified).join("schema-migration/SKILL.md");
    let mut text = fs::read_to_string(&copied).unwrap();
    text.push_str("\nEdited by hand.\n");
    fs::write(&copied, text).unwrap();

    let shadowed = fx.project("shadowed");
    ok(&fx, &shadowed, &["add", "hs:ops/deploy-checklist", "--yes"]);
    fx.plant_user_skill("deploy-checklist");

    let not_applied = fx.project("not-applied");
    ok(&fx, &not_applied, &["add", "fx:alpha-root-skill", "--yes"]);
    fx.write_selection(
        &not_applied,
        "version: 1\ntargets:\n  claude-code:\n    add:\n      - fx:alpha-root-skill\n      - hs:api/rest-design\n",
    );

    let deselected = fx.project("deselected");
    ok(&fx, &deselected, &["add", "hs:api/rest-design", "--yes"]);
    fx.write_selection(
        &deselected,
        "version: 1\ntargets:\n  claude-code:\n    add: []\n",
    );

    // The deliberate change: rename the skill that `skill-missing` holds.
    fs::rename(
        fx.lib.join("sdlc/code-review"),
        fx.lib.join("sdlc/code-reviewer"),
    )
    .unwrap();
    let after_rename = snapshot_source(&fx.lib);

    // (project, skill name, state in `status`, cause in `reconcile`)
    let cases: [(&PathBuf, &str, &str, Option<&str>); 6] = [
        (
            &hand_deleted,
            "pytest-fixtures",
            "orphaned",
            Some("hand-deleted"),
        ),
        (
            &skill_missing,
            "code-review",
            "orphaned",
            Some("source-skill-missing"),
        ),
        (
            &modified,
            "schema-migration",
            "orphaned",
            Some("hand-modified"),
        ),
        (&shadowed, "deploy-checklist", "shadowed", None),
        (&not_applied, "rest-design", "selected", Some("not-applied")),
        (&deselected, "rest-design", "active", Some("deselected")),
    ];
    for (project, name, state, cause) in cases {
        let (code, v) = fx.status(project);
        assert_eq!(record(&v, name)["state"], state, "{name}: {v}");
        assert_eq!(code, if state == "orphaned" { 8 } else { 0 }, "{name}: {v}");
        let (code, v) = fx.json(project, &["reconcile"]);
        match cause {
            Some(cause) => {
                assert_eq!(code, 8, "{name}: {v}");
                assert!(
                    causes(&v).contains(&(cause.to_string(), name.to_string())),
                    "{name}: {:?}",
                    causes(&v)
                );
            }
            None => {
                assert_eq!(code, 0, "{name}: {v}");
                assert!(causes(&v).is_empty());
                assert_eq!(v["data"]["findings"][0]["kind"], "shadowed");
                assert_eq!(v["data"]["findings"][0]["name"], name);
            }
        }
    }
    fx.done_with(&fx.lib, &after_rename);
}

#[test]
fn source_rm_counts_bundle_members() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["use", "hermes-only", "--replace", "--yes"]);
    // Only a bundle member from `fx` references it: no literal `fx:` ref.
    assert!(!selection_text(&p).contains("fx:"));
    let (code, v) = fx.json_raw(&p, &["source", "rm", "fx"]);
    assert_eq!(code, 3, "{v}");
    assert_eq!(v["errors"][0]["code"], "source-referenced");
    let (code, v) = fx.json_raw(&p, &["source", "list"]);
    assert_eq!(code, 0);
    assert_eq!(v["data"]["sources"].as_array().unwrap().len(), 2);
    ok(&fx, &p, &["source", "rm", "fx", "--force"]);
    // `bundle rm` never blocks, whatever selects the bundle.
    ok(&fx, &p, &["bundle", "rm", "hermes-only"]);
    // A source no selection or bundle uses goes without `--force`.
    ok(&fx, &p, &["source", "rm", "hs"]);
    fx.done();
}

// ---- adversarial evidence rows (repair, relocate, rollback delete) ----

/// A project with two applied skills, both links deleted by hand.
fn two_hand_deleted(fx: &Fx, name: &str) -> PathBuf {
    let p = fx.project(name);
    ok(
        fx,
        &p,
        &[
            "add",
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
    );
    for n in ["code-review", "pytest-fixtures"] {
        fs::remove_file(dest(&p).join(n)).unwrap();
    }
    p
}

#[cfg(unix)]
#[test]
fn evidence_repair_never_touches_foreign() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    fx.plant_foreign(&p, "house-style");
    fx.plant_file(&dest(&p).join("README.md"), b"my notes\n");
    let mine = fx.env.path("work/mine");
    fs::create_dir_all(&mine).unwrap();
    fx.plant_link(&dest(&p).join("my-link"), &mine);
    fs::remove_file(dest(&p).join("code-review")).unwrap();
    let (code, v) = fx.status(&p);
    assert_eq!(code, 8, "{v}");
    assert_eq!(record(&v, "house-style")["state"], "foreign");
    assert_eq!(record(&v, "my-link")["state"], "foreign");
    assert!(record_states(&v).iter().all(|(n, _)| n != "README.md"));
    ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    assert!(dest(&p).join("code-review").exists());
    // `fx.done()` proves every foreign byte and link text is as planted.
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_repair_never_overwrites_modified_entries() {
    let fx = Fx::new();
    // A copied entry whose file the user edited.
    let c = fx.project("copy");
    fx.write_selection(&c, "targets:\n  claude-code:\n    strategy: copy\n");
    ok(&fx, &c, &["add", "fx:data/schema-migration", "--yes"]);
    let file = dest(&c).join("schema-migration/SKILL.md");
    let mut text = fs::read_to_string(&file).unwrap();
    text.push_str("\nMy own rules.\n");
    fs::write(&file, &text).unwrap();
    let start = snapshot_project(&c);
    let (code, v) = fx.json_raw(&c, &["reconcile", "--repair", "--yes"]);
    assert_eq!(code, 6, "{v}");
    assert_eq!(v["errors"][0]["code"], "blocked-by-conflicts");
    assert_eq!(fs::read_to_string(&file).unwrap(), text);
    assert_project_unchanged(&start, &c);

    // A symlink the user pointed somewhere else.
    let l = fx.project("link");
    ok(&fx, &l, &["add", "fx:testing/pytest-fixtures", "--yes"]);
    let elsewhere = canon(&fx.lib2).join("api/rest-design");
    let link = dest(&l).join("pytest-fixtures");
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
    let start = snapshot_project(&l);
    let (code, v) = fx.json_raw(&l, &["reconcile", "--repair", "--yes"]);
    assert_eq!(code, 6, "{v}");
    assert_eq!(fs::read_link(&link).unwrap(), elsewhere);
    assert_project_unchanged(&start, &l);
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_deselected_modified_entry_is_dropped_from_receipt_only() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    fx.write_selection(&p, "version: 1\ntargets:\n  claude-code:\n    add: []\n");
    let elsewhere = canon(&fx.lib2).join("api/rest-design");
    let link = dest(&p).join("code-review");
    fs::remove_file(&link).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &link).unwrap();
    let v = ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    assert_eq!(v["data"]["status"], "applied");
    // The entry stays on disk; only the receipt record goes.
    assert_eq!(fs::read_link(&link).unwrap(), elsewhere);
    assert!(receipt_entry_texts(&p, "claude-code").is_empty());
    let (code, v) = fx.status(&p);
    assert_eq!(code, 0, "{v}");
    assert_eq!(record(&v, "code-review")["state"], "foreign");
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_repair_interrupted_resume_and_rollback_reach_zero_divergence() {
    quiet_crashes();
    let fx = Fx::new();
    // The call index of the second link created by a clean repair.
    let twin = two_hand_deleted(&fx, "twin");
    let counter = FailingFs::new(FailMode::None);
    {
        let c = core_ctx(&fx.env, &twin);
        let clock = StepClock::new();
        let req = c.request(Edit::Reconcile, "reconcile", &clock);
        change::run(&req, &counter, &mut |_| Decision::Proceed)
            .map_err(|f| f.error)
            .unwrap();
    }
    let second_link = counter
        .calls()
        .into_iter()
        .filter(|(_, m, _)| *m == "symlink_dir")
        .map(|(n, _, _)| n)
        .nth(1)
        .unwrap();

    for recovery in ["resume", "rollback"] {
        let p = two_hand_deleted(&fx, recovery);
        let before = snapshot_project(&p);
        let c = core_ctx(&fx.env, &p);
        let clock = StepClock::new();
        let req = c.request(Edit::Reconcile, "reconcile", &clock);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            change::run(
                &req,
                &FailingFs::new(FailMode::CrashBefore(second_link)),
                &mut |_| Decision::Proceed,
            )
        }));
        // Exactly one of the two links was created before the "crash".
        assert!(p.join(".unam/claude-code/journal.jsonl").exists());
        assert_eq!(children(&dest(&p)).len(), 1, "{recovery}");
        let clock = StepClock::starting_at(1_800_000_000_000);
        if recovery == "resume" {
            change::resume(&c.recover(&clock), &RealFs, &mut |_| Decision::Proceed)
                .map_err(|f| f.error)
                .unwrap();
            assert_eq!(children(&dest(&p)), ["code-review", "pytest-fixtures"]);
            assert_zero_divergence(&c, recovery);
        } else {
            change::rollback(&c.recover(&clock), &RealFs, &mut |_| Decision::Proceed)
                .map_err(|f| f.error)
                .unwrap();
            // Back to the state before the repair: the same two hand
            // deletions, nothing more and nothing less.
            assert_project_unchanged(&before, &p);
            let report = unam_core::reconcile::report(
                &unam_core::verify::VerifyRequest {
                    dirs: &c.dirs,
                    config: &c.config,
                    catalog: &c.catalog,
                    bundles: &c.bundles,
                    root: &c.root,
                    target: unam_core::project::TargetId::ClaudeCode,
                },
                &clock,
            )
            .unwrap();
            assert_eq!(report.divergences.len(), 2);
        }
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_repair_preview_equals_applied_steps() {
    let fx = Fx::new();
    let p = two_hand_deleted(&fx, "p");
    let (code, preview) = fx.json_raw(&p, &["reconcile"]);
    assert_eq!(code, 8);
    let planned = ops_and_names(&preview["data"]["plan"]["steps"]);
    assert_eq!(planned.len(), 2);
    let applied = ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    assert_eq!(ops_and_names(&applied["data"]["results"]), planned);
    assert!(
        applied["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|r| r["result"] == "done")
    );
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_repair_is_idempotent() {
    let fx = Fx::new();
    let p = two_hand_deleted(&fx, "p");
    ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    let start = snapshot_project(&p);
    let v = ok(&fx, &p, &["reconcile", "--repair", "--yes"]);
    assert_eq!(v["data"]["status"], "no-changes");
    assert_project_unchanged(&start, &p);
    let (_, out, _) = fx.run(&p, &["reconcile", "--repair", "--yes"]);
    assert!(out.contains("No changes"), "{out}");
    assert_project_unchanged(&start, &p);
    fx.done();
}

#[cfg(unix)]
#[test]
fn evidence_relocate_to_incomplete_library_reports_and_deletes_nothing() {
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
    // A library that holds only one of the two skills.
    let partial = fx.env.path("work/partial-lib");
    write_file(
        &partial,
        "sdlc/code-review/SKILL.md",
        skill_md("code-review", "Only this one moved.").as_bytes(),
    );
    let start = snapshot_project(&p);
    let v = ok(
        &fx,
        &p,
        &[
            "source",
            "relocate",
            "fx",
            partial.to_str().unwrap(),
            "--yes",
        ],
    );
    let projects = v["data"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0]["status"], "unplanned", "{v}");
    assert!(
        projects[0]["error"]
            .as_str()
            .unwrap()
            .contains("no skill matches")
    );
    // Nothing in the project was removed or rewritten ...
    assert_project_unchanged(&start, &p);
    // ... the configuration still moved, and the project shows what is
    // missing instead of hiding it.
    let (_, list) = fx.json_raw(&p, &["source", "list"]);
    assert_eq!(
        list["data"]["sources"][0]["path"],
        partial.to_str().unwrap()
    );
    let (code, s) = fx.status(&p);
    assert_eq!(code, 8, "{s}");
    assert_eq!(
        record(&s, "pytest-fixtures")["reason"],
        "source-skill-missing"
    );
    fx.done();
}
