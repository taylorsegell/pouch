//! Generated bundle briefs (milestone M5: spec 4.8 step 7; AC-17 and AC-23
//! hold for generated entries). Run for both targets where cheap. Every
//! test runs through the shared fixture `Fx` and ends with `fx.done()`
//! (sources unchanged, planted foreign fixtures byte-identical). The brief
//! tests are the only ones that use the unmodified `hermes-only` fixture;
//! the other suites copy it with the brief opted out (EX1).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::*;
use common::*;
use serde_json::Value;
use std::fs;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use unam_core::change::{self, Decision, Edit, Outcome, RecoverOutcome};
use unam_core::fsops::RealFs;
use unam_core::hash::Hash;
use unam_core::project::TargetId;

const TARGETS: [(&str, &str); 2] = [
    ("claude-code", ".claude/skills"),
    ("agents-neutral", ".agents/skills"),
];
const INSTRUCTION: &str = "Review the change, then check the test fixtures.";

fn ok(fx: &Fx, p: &Path, args: &[&str]) -> Value {
    let (code, v) = fx.json_raw(p, args);
    assert_eq!(code, 0, "{args:?}: {v}");
    v
}

/// `reconcile` reports zero divergence and no journal remains (G6).
fn zero(fx: &Fx, p: &Path, target: &str) {
    assert_zero_divergence_in(&fx.env, p, TargetId::parse(target).unwrap(), target);
}

fn brief_dir(p: &Path, dest: &str, slug: &str) -> PathBuf {
    p.join(dest).join(format!("{slug}-brief"))
}

/// The receipt's generated entries (full JSON values).
fn generated_entries(p: &Path, target: &str) -> Vec<Value> {
    read_receipt(p, target)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "generated")
        .cloned()
        .collect()
}

fn ops(v: &Value) -> Vec<String> {
    v["data"]["plan"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["op"].as_str().unwrap().to_string())
        .collect()
}

fn warning_codes(v: &Value) -> Vec<String> {
    v["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap().to_string())
        .collect()
}

fn all_files_under(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        for e in rd {
            let p = e.unwrap().path();
            if fs::symlink_metadata(&p).unwrap().is_dir() {
                stack.push(p);
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn brief_written_on_add_and_listed_in_receipt() {
    let fx = Fx::new();
    // The unmodified fixture: an instruction and no opt-out (EX1).
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        // The plan (refused without --yes) shows the brief and the honesty line.
        let (code, out, _) = fx.run(&p, &["--target", t, "use", "hermes-only", "--replace"]);
        assert_ne!(code, 0, "{out}");
        assert!(out.contains("generated  hermes-only-brief"), "{out}");
        assert!(out.contains(INSTRUCTION), "{out}");
        assert!(
            out.contains("A bundle brief is a skill the agent can discover; unam cannot make the agent read it."),
            "{out}"
        );
        assert!(
            !brief_dir(&p, dest, "hermes-only").exists(),
            "{t}: refused plan wrote"
        );
        let v = ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        assert_eq!(v["data"]["status"], "applied");
        // The plan step carries bundle, name, hash; not the text.
        let step = v["data"]["plan"]["steps"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["op"] == "write_generated")
            .unwrap_or_else(|| panic!("{v}"));
        assert_eq!(step["bundle"], "hermes-only");
        assert_eq!(step["name"], "hermes-only-brief");
        assert_eq!(step["dest"], format!("{dest}/hermes-only-brief"));
        assert!(
            step["content_hash"]
                .as_str()
                .unwrap()
                .starts_with("blake3:")
        );
        assert!(step.get("content").is_none());
        // Skills first, the brief last (spec 4.8 step 8).
        let o = ops(&v);
        assert_eq!(o.last().unwrap(), "write_generated", "{o:?}");
        // The file: a directory holding only SKILL.md with the instruction.
        let dir = brief_dir(&p, dest, "hermes-only");
        let names: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, vec!["SKILL.md"]);
        let bytes = fs::read(dir.join("SKILL.md")).unwrap();
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(
            text.starts_with("---\nname: hermes-only-brief\ndescription: "),
            "{text}"
        );
        assert!(
            text.ends_with(&format!("{INSTRUCTION}\nKeep the root skill in mind.\n")),
            "{text}"
        );
        // The receipt entry (spec 2.4).
        let entries = generated_entries(&p, t);
        assert_eq!(entries.len(), 1, "{entries:?}");
        let e = &entries[0];
        assert_eq!(e["generated"], "bundle-brief");
        assert_eq!(e["bundle"], "hermes-only");
        assert_eq!(e["name"], "hermes-only-brief");
        assert_eq!(e["dest"], format!("{dest}/hermes-only-brief"));
        assert_eq!(e["content_hash"], Hash::of_bytes(&bytes).to_string());
        assert!(e["applied_at"].as_str().is_some());
        // The result row.
        let r = v["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["op"] == "write_generated")
            .unwrap();
        assert_eq!(r["result"], "done");
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_unchanged_is_a_no_op_and_record_verbatim() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        let before = generated_entries(&p, t);
        let dir = brief_dir(&p, dest, "hermes-only");
        let inode = ino(&dir);
        let bytes = fs::read(dir.join("SKILL.md")).unwrap();
        // The same selection again: nothing to do (AC-16 for generated).
        let v = ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        assert_eq!(v["data"]["status"], "no-changes", "{v}");
        assert_eq!(generated_entries(&p, t), before);
        // An unrelated change: the brief record is carried verbatim (AC-17).
        let v = ok(
            &fx,
            &p,
            &["--target", t, "add", "hs:ops/deploy-checklist", "--yes"],
        );
        assert_eq!(v["data"]["status"], "applied");
        assert!(
            !ops(&v).iter().any(|o| o.ends_with("_generated")),
            "{:?}",
            ops(&v)
        );
        assert_eq!(generated_entries(&p, t), before);
        assert_eq!(ino(&dir), inode);
        assert_eq!(fs::read(dir.join("SKILL.md")).unwrap(), bytes);
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_opt_out_writes_none() {
    let fx = Fx::new();
    fx.write_bundle_with("quiet", &["code-review"], Some("Be quiet."), Some(false));
    fx.write_bundle_with("blank", &["pytest-fixtures"], None, None);
    fx.write_bundle_with(
        "loud",
        &["fx:data/schema-migration"],
        Some("Say it."),
        Some(true),
    );
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        let v = ok(
            &fx,
            &p,
            &["--target", t, "use", "quiet", "--replace", "--yes"],
        );
        assert!(
            !ops(&v).iter().any(|o| o.ends_with("_generated")),
            "{:?}",
            ops(&v)
        );
        assert!(!brief_dir(&p, dest, "quiet").exists());
        let v = ok(&fx, &p, &["--target", t, "use", "blank", "--add", "--yes"]);
        assert!(
            !ops(&v).iter().any(|o| o.ends_with("_generated")),
            "{:?}",
            ops(&v)
        );
        assert!(!brief_dir(&p, dest, "blank").exists());
        assert!(generated_entries(&p, t).is_empty());
        // An explicit `true` writes one.
        ok(&fx, &p, &["--target", t, "use", "loud", "--add", "--yes"]);
        assert!(brief_dir(&p, dest, "loud").join("SKILL.md").is_file());
        assert_eq!(generated_entries(&p, t).len(), 1);
        // A bundle reached only through `add` entries has no brief.
        let q = fx.project(&format!("q-{t}"));
        ok(
            &fx,
            &q,
            &["--target", t, "add", "fx:sdlc/code-review", "--yes"],
        );
        assert!(generated_entries(&q, t).is_empty());
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_removed_on_deactivate_and_foreign_left() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        // A foreign skill and a foreign look-alike next to the brief.
        fx.plant_file(&p.join(dest).join("house-style/SKILL.md"), b"mine\n");
        fx.plant_file(&p.join(dest).join("other-brief/SKILL.md"), b"not unam's\n");
        let (code, out, _) = fx.run(&p, &["--target", t, "deactivate", "--yes"]);
        assert_eq!(code, 0, "{out}");
        assert!(
            out.contains("Removed from future discovery."),
            "SES-2 caveat for a brief-only removal too: {out}"
        );
        assert!(!brief_dir(&p, dest, "hermes-only").exists());
        assert!(generated_entries(&p, t).is_empty());
        assert!(p.join(dest).join("house-style/SKILL.md").is_file());
        assert!(p.join(dest).join("other-brief/SKILL.md").is_file());
        // The moved-aside copy is gone once the apply is final.
        assert!(
            all_files_under(&p.join(".unam").join(t).join("trash")).is_empty(),
            "{:?}",
            all_files_under(&p.join(".unam").join(t).join("trash"))
        );
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_edited_is_kept_and_released() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        let file = brief_dir(&p, dest, "hermes-only").join("SKILL.md");
        fs::write(&file, b"my own notes\n").unwrap();
        // Still wanted: the edited brief never blocks the plan. It is left
        // alone, kept in the receipt, and named in a plan warning.
        let (code, v) = fx.json_raw(
            &p,
            &["--target", t, "add", "hs:ops/deploy-checklist", "--yes"],
        );
        assert_eq!(code, 0, "{v}");
        assert_eq!(fs::read(&file).unwrap(), b"my own notes\n");
        let w = v["data"]["plan"]["warnings"].as_array().unwrap();
        assert!(
            w.iter().any(|x| x["code"] == "brief-modified"
                && x["message"].as_str().unwrap().contains("hermes-only-brief")),
            "{v}"
        );
        assert_eq!(generated_entries(&p, t).len(), 1, "receipt entry kept");
        // No longer wanted: kept (skipped as modified) and released.
        let (code, v) = fx.json_raw(&p, &["--target", t, "deactivate", "--yes"]);
        assert_eq!(code, 7, "{v}");
        let r = v["data"]["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["op"] == "remove_generated")
            .unwrap();
        assert_eq!(r["result"], "skipped");
        assert_eq!(r["code"], "modified");
        assert_eq!(fs::read(&file).unwrap(), b"my own notes\n");
        assert!(
            generated_entries(&p, t).is_empty(),
            "released from the receipt"
        );
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_foreign_name_refuses() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        fx.plant_file(
            &p.join(dest).join("hermes-only-brief/SKILL.md"),
            b"foreign\n",
        );
        let start = snapshot_project(&p);
        let (code, v) = fx.json_raw(
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        assert_eq!(code, 6, "{v}");
        assert_eq!(v["data"]["plan"]["conflicts"][0]["code"], "foreign-at-dest");
        assert!(
            v["data"]["plan"]["conflicts"][0]["message"]
                .as_str()
                .unwrap()
                .contains(&format!("{dest}/hermes-only-brief")),
            "{v}"
        );
        assert_project_unchanged(&start, &p);
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_changed_instruction_rewrites() {
    let fx = Fx::new();
    for (t, dest) in TARGETS {
        fx.write_bundle_with("chg", &["code-review"], Some("First version."), None);
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "chg", "--replace", "--yes"],
        );
        let skill_inode = ino(&p.join(dest).join("code-review"));
        let old = generated_entries(&p, t);
        fx.write_bundle_with("chg", &["code-review"], Some("Second version."), None);
        let v = ok(
            &fx,
            &p,
            &["--target", t, "add", "hs:ops/deploy-checklist", "--yes"],
        );
        let o = ops(&v);
        let rm = o
            .iter()
            .position(|x| x == "remove_generated")
            .unwrap_or_else(|| panic!("{o:?}"));
        let wr = o
            .iter()
            .position(|x| x == "write_generated")
            .unwrap_or_else(|| panic!("{o:?}"));
        assert!(rm < wr, "{o:?}");
        let text = fs::read_to_string(brief_dir(&p, dest, "chg").join("SKILL.md")).unwrap();
        assert!(text.ends_with("Second version.\n"), "{text}");
        let new = generated_entries(&p, t);
        assert_eq!(new.len(), 1);
        assert_ne!(new[0]["content_hash"], old[0]["content_hash"]);
        assert_eq!(
            new[0]["content_hash"],
            Hash::of_bytes(text.as_bytes()).to_string()
        );
        assert_eq!(ino(&p.join(dest).join("code-review")), skill_inode);
        assert!(all_files_under(&p.join(".unam").join(t).join("trash")).is_empty());
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_never_reported_foreign() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        // A look-alike that unam did not write IS foreign: the control.
        fx.plant_file(&p.join(dest).join("stray-brief/SKILL.md"), b"x\n");
        let (code, s) = fx.json_raw(&p, &["--target", t, "status"]);
        assert_eq!(code, 0, "{s}");
        let states = record_states(&s);
        assert!(
            !states.iter().any(|(n, _)| n == "hermes-only-brief"),
            "{states:?}"
        );
        assert!(
            states.contains(&("stray-brief".to_string(), "foreign".to_string())),
            "{states:?}"
        );
        let (code, r) = fx.json_raw(&p, &["--target", t, "reconcile"]);
        assert_eq!(code, 0, "{r}");
        let foreign: Vec<&str> = r["data"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["kind"] == "foreign")
            .map(|f| f["name"].as_str().unwrap())
            .collect();
        assert_eq!(foreign, vec!["stray-brief"]);
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_missing_is_reported_and_repaired() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        let dir = brief_dir(&p, dest, "hermes-only");
        let want = fs::read(dir.join("SKILL.md")).unwrap();
        let clean = ok(&fx, &p, &["--target", t, "status"]);
        assert!(
            !warning_codes(&clean)
                .iter()
                .any(|c| c.starts_with("brief-"))
        );
        // Deleted by hand.
        fs::remove_dir_all(&dir).unwrap();
        let s = ok(&fx, &p, &["--target", t, "status"]);
        assert!(
            warning_codes(&s).contains(&"brief-missing".to_string()),
            "{s}"
        );
        let r = ok(&fx, &p, &["--target", t, "reconcile"]);
        let kinds: Vec<&str> = r["data"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["kind"].as_str().unwrap())
            .collect();
        assert!(kinds.contains(&"brief-missing"), "{kinds:?}");
        // Repair writes it again through the journaled executor.
        let v = ok(&fx, &p, &["--target", t, "reconcile", "--repair", "--yes"]);
        assert_eq!(ops(&v), vec!["write_generated"], "{v}");
        assert_eq!(fs::read(dir.join("SKILL.md")).unwrap(), want);
        let s = ok(&fx, &p, &["--target", t, "status"]);
        assert!(
            !warning_codes(&s).iter().any(|c| c.starts_with("brief-")),
            "{s}"
        );
        // Edited by hand: reported, never repaired over.
        fs::write(dir.join("SKILL.md"), b"changed\n").unwrap();
        let s = ok(&fx, &p, &["--target", t, "status"]);
        assert!(
            warning_codes(&s).contains(&"brief-modified".to_string()),
            "{s}"
        );
        let (code, v) = fx.json_raw(&p, &["--target", t, "reconcile", "--repair", "--yes"]);
        assert_eq!(code, 0, "{v}");
        assert!(ops(&v).is_empty(), "{v}");
        assert_eq!(fs::read(dir.join("SKILL.md")).unwrap(), b"changed\n");
        zero(&fx, &p, t);
    }
    fx.done();
}

// ---- interruption: injected failure at every sampled mutating call ----

struct BriefScenario {
    s: Scenario,
    target: TargetId,
}

fn refs(r: &[&str]) -> Vec<String> {
    r.iter().map(|s| s.to_string()).collect()
}

const MATRIX_BUNDLE: &str = "mx";

/// D: an empty project; `use mx --replace` writes the skills and a brief.
fn scenario_create(fx: &Fx, target: TargetId, name: &str) -> BriefScenario {
    fx.write_bundle_with(
        MATRIX_BUNDLE,
        &["code-review", "pytest-fixtures"],
        Some("Matrix instruction."),
        None,
    );
    let project = fx.project(name);
    BriefScenario {
        s: Scenario {
            project,
            edit: Edit::UseReplace(MATRIX_BUNDLE.into()),
            command: "use",
            copy_default: false,
            target,
        },
        target,
    }
}

/// E: the bundle applied (skills and brief), then `deactivate`.
fn scenario_deactivate(fx: &Fx, target: TargetId, name: &str) -> BriefScenario {
    let sc = scenario_create(fx, target, name);
    core_apply_for(
        &fx.env,
        &sc.s.project,
        target,
        Edit::UseReplace(MATRIX_BUNDLE.into()),
        false,
    );
    BriefScenario {
        s: Scenario {
            edit: Edit::Deactivate,
            command: "deactivate",
            ..sc.s
        },
        target,
    }
}

/// F: the bundle applied, its instruction then changed, then `add`: the
/// brief is replaced (a move-aside remove, then a write).
fn scenario_replace(fx: &Fx, target: TargetId, name: &str) -> BriefScenario {
    let sc = scenario_create(fx, target, name);
    core_apply_for(
        &fx.env,
        &sc.s.project,
        target,
        Edit::UseReplace(MATRIX_BUNDLE.into()),
        false,
    );
    fx.write_bundle_with(
        MATRIX_BUNDLE,
        &["code-review", "pytest-fixtures"],
        Some("A changed matrix instruction."),
        None,
    );
    BriefScenario {
        s: Scenario {
            edit: Edit::Add(refs(&["hs:ops/deploy-checklist"])),
            command: "add",
            ..sc.s
        },
        target,
    }
}

/// Dest listing (file bytes and link texts) and the receipt's essence.
type Essence = (Vec<(String, String, Vec<u8>)>, Vec<String>);

fn essence(project: &Path, target: TargetId) -> Essence {
    let dest = match target {
        TargetId::ClaudeCode => ".claude/skills",
        TargetId::AgentsNeutral => ".agents/skills",
    };
    let files = snapshot_project(project)
        .0
        .into_iter()
        .filter(|(r, ..)| r == dest || r.starts_with(&format!("{dest}/")))
        .map(|(r, k, _, b)| (r, k, b))
        .collect();
    let mut receipt: Vec<String> = match fs::read(
        project
            .join(".unam")
            .join(target.as_str())
            .join("applied.json"),
    ) {
        Ok(b) => serde_json::from_slice::<Value>(&b).unwrap()["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                format!(
                    "{} {} {}",
                    e["kind"],
                    e["name"],
                    e.get("content_hash")
                        .or(e.get("tree_hash"))
                        .unwrap_or(&Value::Null)
                )
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    receipt.sort();
    (files, receipt)
}

fn run_brief_matrix(
    build: fn(&Fx, TargetId, &str) -> BriefScenario,
    label: &str,
    target: TargetId,
) {
    quiet_crashes();
    let fx = Fx::new();
    fx.write_bundle_with("swap", &["hs:ops/deploy-checklist"], None, None);
    let full = std::env::var("UNAM_FULL_FAULT_MATRIX").is_ok_and(|v| v == "1");
    let clean = build(&fx, target, &format!("{label}-clean"));
    let counter = FailingFs::new(FailMode::None);
    let out = clean
        .s
        .run(&fx.env, &counter, &StepClock::new())
        .map_err(|f| f.error)
        .unwrap();
    assert!(matches!(out, Outcome::Applied(..)), "{label}");
    let total = counter.total();
    assert!(total >= 4, "{label}: {total} calls");
    let calls = counter.calls();
    assert!(
        calls
            .iter()
            .any(|(_, m, _)| *m == "write_generated" || *m == "remove_generated"),
        "{label}: the scenario never reaches a brief call: {calls:?}"
    );
    let clean_state = essence(&clean.s.project, target);
    let mut runs = 0;
    for n in 1..=total {
        if !full && n != 1 && n != total && n % 3 != 0 {
            continue;
        }
        for mode in FailMode::all_at(n) {
            for recovery in ["resume", "rollback"] {
                runs += 1;
                let ctx = format!("{label} {target} {mode:?} {recovery}");
                let b = build(&fx, target, &format!("{label}-{n}-{runs}"));
                let start = snapshot_project(&b.s.project);
                let start_essence = essence(&b.s.project, target);
                let ff = FailingFs::new(mode);
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    b.s.run(&fx.env, &ff, &StepClock::new())
                }));
                assert_ordering_invariant_for(&b.s.project, b.target, &ctx);
                let c = b.s.ctx(&fx.env);
                let clock = StepClock::new();
                let journal =
                    b.s.project
                        .join(".unam")
                        .join(target.as_str())
                        .join("journal.jsonl");
                if !journal.exists() {
                    if essence(&b.s.project, target) == clean_state {
                        // The apply finished before the injection.
                        assert_zero_divergence_for(&c, b.target, &ctx);
                        continue;
                    }
                    // Before the `begin` record: nothing applied; a re-run
                    // reaches the clean end state.
                    assert_eq!(essence(&b.s.project, target), start_essence, "{ctx}");
                    b.s.run(&fx.env, &RealFs, &StepClock::new())
                        .map_err(|f| f.error)
                        .unwrap();
                    assert_eq!(essence(&b.s.project, target), clean_state, "{ctx}: re-run");
                    continue;
                }
                let req = c.recover(&clock);
                if recovery == "resume" {
                    let out = change::resume(&req, &RealFs, &mut |_| Decision::Proceed)
                        .map_err(|f| f.error)
                        .unwrap();
                    assert!(matches!(out, RecoverOutcome::Recovered(..)), "{ctx}");
                    assert_eq!(essence(&b.s.project, target), clean_state, "{ctx}");
                    assert!(!journal.exists(), "{ctx}");
                    assert_zero_divergence_for(&c, b.target, &ctx);
                    // The brief file and its receipt entry agree.
                    assert!(
                        all_files_under(
                            &b.s.project
                                .join(".unam")
                                .join(target.as_str())
                                .join("trash")
                        )
                        .is_empty(),
                        "{ctx}: trash left"
                    );
                } else {
                    let out = change::rollback(&req, &RealFs, &mut |_| Decision::Proceed)
                        .map_err(|f| f.error)
                        .unwrap();
                    let RecoverOutcome::Recovered(report, _) = out else {
                        panic!("{ctx}")
                    };
                    assert!(
                        report
                            .warnings
                            .iter()
                            .all(|w| !w.code.starts_with("rollback-")),
                        "{ctx}: {:?}",
                        report.warnings
                    );
                    assert_project_unchanged(&start, &b.s.project);
                    assert_zero_divergence_for(&c, b.target, &ctx);
                }
            }
        }
    }
    assert!(runs >= 24, "{label}: {runs} matrix runs");
    println!("brief fault matrix {label} {target}: {total} calls, {runs} runs");
    fx.done();
}

#[test]
fn brief_resume_refuses_journal_content_that_does_not_match_its_hash() {
    quiet_crashes();
    let fx = Fx::new();
    let sc = scenario_create(&fx, TargetId::AgentsNeutral, "tamper");
    // The journal `begin` record is written, then the first step fails to start.
    let ff = FailingFs::new(FailMode::FailBefore(3));
    assert!(sc.s.run(&fx.env, &ff, &StepClock::new()).is_err());
    let journal = sc.s.project.join(".unam/agents-neutral/journal.jsonl");
    let text = fs::read_to_string(&journal).unwrap();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let mut begin: Value = serde_json::from_str(&lines[0]).unwrap();
    let mut tampered = 0;
    for step in begin["plan"].as_array_mut().unwrap() {
        if step["op"] == "write_generated" {
            let c = step["content"].as_str().unwrap().to_string();
            step["content"] = Value::String(format!("{c}TAMPER"));
            tampered += 1;
        }
    }
    assert_eq!(tampered, 1);
    lines[0] = begin.to_string();
    fs::write(&journal, format!("{}\n", lines.join("\n"))).unwrap();
    let c = sc.s.ctx(&fx.env);
    let clock = StepClock::new();
    let out = change::resume(&c.recover(&clock), &RealFs, &mut |_| Decision::Proceed)
        .map_err(|f| f.error)
        .unwrap();
    let RecoverOutcome::Recovered(report, _) = out else {
        panic!("not recovered")
    };
    let w = report
        .results
        .iter()
        .find(|r| r.op == "write_generated")
        .unwrap();
    match &w.outcome {
        unam_core::plan::StepOutcome::Failed { code, .. } => assert_eq!(code, "journal-invalid"),
        other => panic!("{other:?}"),
    }
    // Nothing was written for the tampered step; the other steps ran.
    assert!(!brief_dir(&sc.s.project, ".agents/skills", MATRIX_BUNDLE).exists());
    assert!(sc.s.project.join(".agents/skills/code-review").exists());
    assert!(generated_entries(&sc.s.project, "agents-neutral").is_empty());
    fx.done();
}

type Build = fn(&Fx, TargetId, &str) -> BriefScenario;

fn all_matrices(target: TargetId) {
    let all: [(&str, Build); 3] = [
        ("create", scenario_create),
        ("deactivate", scenario_deactivate),
        ("replace", scenario_replace),
    ];
    for (label, build) in all {
        run_brief_matrix(build, label, target);
    }
}

#[test]
fn brief_interruption_resume_and_rollback() {
    all_matrices(TargetId::AgentsNeutral);
}

#[test]
fn brief_interruption_resume_and_rollback_claude_code() {
    all_matrices(TargetId::ClaudeCode);
}

/// EVL fix cycle 1 regression: a replaced brief interrupted at the seventh
/// mutating call (the old brief's move-aside), failing before or after it,
/// then resumed, must end with the new brief, its receipt row, and no trash
/// (the strided matrix run does not reach this index).
#[test]
fn brief_replace_interrupted_at_call_seven_resumes_to_clean_state() {
    quiet_crashes();
    let fx = Fx::new();
    fx.write_bundle_with("swap", &["hs:ops/deploy-checklist"], None, None);
    let target = TargetId::ClaudeCode;
    let clean = scenario_replace(&fx, target, "r7-clean");
    let counter = FailingFs::new(FailMode::None);
    clean
        .s
        .run(&fx.env, &counter, &StepClock::new())
        .map_err(|f| f.error)
        .unwrap();
    // Call 7 is the replaced brief's remove_generated: a failure there leaves
    // the old brief (before) or the old brief moved aside with the new brief
    // already written (after).
    assert_eq!(counter.calls()[6].1, "remove_generated");
    let clean_state = essence(&clean.s.project, target);
    for (i, mode) in [FailMode::FailBefore(7), FailMode::FailAfter(7)]
        .into_iter()
        .enumerate()
    {
        let ctx = format!("replace {mode:?} resume");
        let b = scenario_replace(&fx, target, &format!("r7-run-{i}"));
        let ff = FailingFs::new(mode);
        let _ = catch_unwind(AssertUnwindSafe(|| {
            b.s.run(&fx.env, &ff, &StepClock::new())
        }));
        let c = b.s.ctx(&fx.env);
        let clock = StepClock::new();
        let req = c.recover(&clock);
        let out = change::resume(&req, &RealFs, &mut |_| Decision::Proceed)
            .map_err(|f| f.error)
            .unwrap();
        assert!(matches!(out, RecoverOutcome::Recovered(..)), "{ctx}");
        assert_eq!(essence(&b.s.project, target), clean_state, "{ctx}");
        assert!(
            all_files_under(
                &b.s.project
                    .join(".unam")
                    .join(target.as_str())
                    .join("trash")
            )
            .is_empty(),
            "{ctx}: trash left"
        );
        assert_zero_divergence_for(&c, b.target, &ctx);
    }
    fx.done();
}

// ---- review fixes R1 to R4 ----

#[test]
fn brief_with_ds_store_is_not_a_conflict_and_is_removed_whole() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        let dir = brief_dir(&p, dest, "hermes-only");
        fs::write(dir.join(".DS_Store"), b"finder noise").unwrap();
        // Still wanted: no conflict, no brief step, the noise file stays.
        let v = ok(
            &fx,
            &p,
            &["--target", t, "add", "hs:ops/deploy-checklist", "--yes"],
        );
        assert_eq!(ops(&v), vec!["add"], "{v}");
        assert!(dir.join(".DS_Store").exists());
        let s = ok(&fx, &p, &["--target", t, "status"]);
        assert!(
            !warning_codes(&s).iter().any(|c| c.starts_with("brief-")),
            "{s}"
        );
        // Deactivated: the whole directory goes, noise included.
        let v = ok(&fx, &p, &["--target", t, "deactivate", "--yes"]);
        assert!(ops(&v).contains(&"remove_generated".to_string()), "{v}");
        assert!(!dir.exists(), "brief directory removed whole");
        assert!(generated_entries(&p, t).is_empty());
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn brief_equal_to_the_desired_text_is_adopted_and_the_receipt_hash_refreshed() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        let dir = brief_dir(&p, dest, "hermes-only");
        let want = fs::read(dir.join("SKILL.md")).unwrap();
        // Make the receipt hash stale; the disk already holds the desired text.
        let receipt = p.join(".unam").join(t).join("applied.json");
        let real = generated_entries(&p, t)[0]["content_hash"]
            .as_str()
            .unwrap()
            .to_string();
        let stale = format!("{}0", &real[..real.len() - 1]);
        let stale = if stale == real {
            format!("{}1", &real[..real.len() - 1])
        } else {
            stale
        };
        let text = fs::read_to_string(&receipt).unwrap();
        fs::write(&receipt, text.replace(&real, &stale)).unwrap();
        let v = ok(
            &fx,
            &p,
            &["--target", t, "add", "hs:ops/deploy-checklist", "--yes"],
        );
        assert!(
            !v["data"]["plan"]["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .any(|w| w["code"] == "brief-modified"),
            "{v}"
        );
        assert_eq!(fs::read(dir.join("SKILL.md")).unwrap(), want);
        assert_eq!(generated_entries(&p, t)[0]["content_hash"], real);
        zero(&fx, &p, t);
    }
    fx.done();
}

#[test]
fn unwanted_brief_already_missing_is_dropped_silently() {
    let fx = Fx::new();
    copy_fixture_bundle(&fx.env, "hermes-only.yaml");
    for (t, dest) in TARGETS {
        let p = fx.project(&format!("p-{t}"));
        ok(
            &fx,
            &p,
            &["--target", t, "use", "hermes-only", "--replace", "--yes"],
        );
        fs::remove_dir_all(brief_dir(&p, dest, "hermes-only")).unwrap();
        let v = ok(&fx, &p, &["--target", t, "deactivate", "--yes"]);
        assert!(!ops(&v).contains(&"remove_generated".to_string()), "{v}");
        assert!(generated_entries(&p, t).is_empty(), "receipt let go");
        zero(&fx, &p, t);
    }
    fx.done();
}
