//! Acceptance criteria for the `agents-neutral` adapter (milestone M5:
//! AC-20, AC-24 and AC-29 for the second adapter, AC-42, AC-43) plus the
//! warnings and target rules around it (M5-3, M5-7). Every test runs through
//! the shared fixture `Fx` (temp HOME, so `~/.hermes` and `~/.agents` are
//! fixtures; never this repository) and ends with `fx.done()`.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::faults::assert_zero_divergence_in;
use common::*;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const CONSUMERS_LINE: &str = "Visible to: codex, cursor, windsurf-devin, hermes.";
const AN: &str = "agents-neutral";
const CC: &str = "claude-code";

fn an_dest(p: &Path) -> PathBuf {
    p.join(".agents/skills")
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

fn plan_warnings<'a>(v: &'a Value, code: &str) -> Vec<&'a Value> {
    diags_with_code(&v["data"]["plan"]["warnings"], code)
}

/// `~/.hermes` in the test HOME (a fixture directory; no agent CLI exists
/// or is run).
fn fixture_hermes_home(fx: &Fx) {
    fs::create_dir_all(fx.env.path("home").join(".hermes")).unwrap();
}

#[test]
fn ac_20_two_targets_hold_and_report_different_selections() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    // Each destination holds its own set.
    assert_eq!(children(&p.join(".claude/skills")), vec!["code-review"]);
    assert_eq!(children(&an_dest(&p)), vec!["pytest-fixtures"]);
    // Two receipts, one per target.
    for t in [CC, AN] {
        assert!(
            p.join(".pouch").join(t).join("applied.json").is_file(),
            "{t}"
        );
    }
    // The selection file holds both targets with different selections.
    let sel = fs::read_to_string(p.join(".pouch/selection.yaml")).unwrap();
    assert!(
        sel.contains("claude-code:") && sel.contains("agents-neutral:"),
        "{sel}"
    );
    // `status` reports each target's own selection.
    let (code, cc) = fx.json_raw(&p, &["--target", CC, "status"]);
    assert_eq!(code, 0, "{cc}");
    let (code, an) = fx.json_raw(&p, &["--target", AN, "status"]);
    assert_eq!(code, 0, "{an}");
    assert_eq!(cc["data"]["target"], CC);
    assert_eq!(an["data"]["target"], AN);
    assert_eq!(cc["data"]["destination"], ".claude/skills");
    assert_eq!(an["data"]["destination"], ".agents/skills");
    assert_eq!(
        record_states(&cc),
        vec![("code-review".to_string(), "active".to_string())]
    );
    assert_eq!(
        record_states(&an),
        vec![("pytest-fixtures".to_string(), "active".to_string())]
    );
    // With two targets and no flag the command is ambiguous (M5-3).
    let (code, v) = fx.json_raw(&p, &["status"]);
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["errors"][0]["code"], "target-ambiguous");
    // Changing one target leaves the other untouched.
    let cc_before = read_receipt(&p, CC);
    ok(&fx, &p, &["--target", AN, "deactivate", "--yes"]);
    assert_eq!(read_receipt(&p, CC), cc_before);
    assert_eq!(children(&p.join(".claude/skills")), vec!["code-review"]);
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::ClaudeCode,
        "ac_20 claude-code",
    );
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "ac_20 agents-neutral",
    );
    fx.done();
}

#[test]
fn ac_42_nested_project_warns_and_names_the_ancestor() {
    let fx = Fx::new();
    let repo = fx.project("repo");
    // The ancestor holds two skills.
    for name in ["house-a", "house-b"] {
        write_file(
            &repo,
            &format!(".agents/skills/{name}/SKILL.md"),
            skill_md(name, "An ancestor skill.").as_bytes(),
        );
    }
    let sub = repo.join("packages/web");
    fs::create_dir_all(&sub).unwrap();
    let anc = fx.canon(&repo).join(".agents/skills");
    let args = |extra: &[&'static str]| -> Vec<String> {
        let mut v = vec![
            "--project".to_string(),
            sub.to_string_lossy().into_owned(),
            "--target".to_string(),
            AN.to_string(),
        ];
        v.extend(extra.iter().map(|s| s.to_string()));
        v
    };
    let a = args(&["add", "fx:sdlc/code-review", "--yes"]);
    let refs: Vec<&str> = a.iter().map(String::as_str).collect();
    let v = ok(&fx, &repo, &refs);
    let w = plan_warnings(&v, "ancestor-skills");
    assert_eq!(w.len(), 1, "{v}");
    let msg = w[0]["message"].as_str().unwrap();
    assert!(msg.contains(&anc.to_string_lossy().into_owned()), "{msg}");
    assert!(msg.contains("contributes 2 skill(s)"), "{msg}");
    assert!(msg.contains("codex walks upward"), "{msg}");
    assert_eq!(w[0]["path"], anc.to_string_lossy().as_ref());
    // The human plan shows it too.
    let a = args(&["add", "fx:testing/pytest-fixtures"]);
    let refs: Vec<&str> = a.iter().map(String::as_str).collect();
    let (_, out, _) = fx.run(&repo, &refs);
    assert!(out.contains("ancestor-skills"), "{out}");
    assert!(out.contains(&anc.to_string_lossy().into_owned()), "{out}");
    // A project that is the Git root has no ancestors, so no warning.
    let root_args = [
        "--project",
        repo.to_str().unwrap(),
        "--target",
        AN,
        "add",
        "fx:data/schema-migration",
        "--yes",
    ];
    let v = ok(&fx, &repo, &root_args);
    assert!(plan_warnings(&v, "ancestor-skills").is_empty(), "{v}");
    // An ancestor directory without any skill is no warning either.
    let repo2 = fx.project("repo2");
    fs::create_dir_all(repo2.join(".agents/skills")).unwrap();
    let sub2 = repo2.join("x");
    fs::create_dir_all(&sub2).unwrap();
    let v = ok(
        &fx,
        &repo2,
        &[
            "--project",
            sub2.to_str().unwrap(),
            "--target",
            AN,
            "add",
            "fx:sdlc/code-review",
            "--yes",
        ],
    );
    assert!(plan_warnings(&v, "ancestor-skills").is_empty(), "{v}");
    assert_zero_divergence_in(
        &fx.env,
        &sub,
        pouch_core::project::TargetId::AgentsNeutral,
        "ac_42",
    );
    fx.done();
}

#[test]
fn ac_43_every_plan_names_the_four_consumers() {
    let fx = Fx::new();
    let p = fx.project("p");
    let four = ["codex", "cursor", "windsurf-devin", "hermes"];
    // The add plan: human (refused without --yes, the plan still prints).
    let (code, out, _) = fx.run(&p, &["--target", AN, "add", "fx:sdlc/code-review"]);
    assert_ne!(code, 0);
    assert!(out.contains(CONSUMERS_LINE), "{out}");
    // The add plan in --json: visible_to.
    let (_, v) = fx.json_raw(&p, &["--target", AN, "add", "fx:sdlc/code-review"]);
    assert_eq!(v["data"]["visible_to"], serde_json::json!(four));
    // The applied result names them too.
    let (code, out, _) = fx.run(&p, &["--target", AN, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains(CONSUMERS_LINE), "{out}");
    let v = ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    assert_eq!(v["data"]["visible_to"], serde_json::json!(four));
    // The no-change plan.
    let (code, out, _) = fx.run(&p, &["--target", AN, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(out.contains("No changes"), "{out}");
    assert!(out.contains(CONSUMERS_LINE), "{out}");
    let v = ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    assert_eq!(v["data"]["status"], "no-changes");
    assert_eq!(v["data"]["visible_to"], serde_json::json!(four));
    // The deactivate plan.
    let (_, out, _) = fx.run(&p, &["--target", AN, "deactivate"]);
    assert!(out.contains(CONSUMERS_LINE), "{out}");
    let (_, v) = fx.json_raw(&p, &["--target", AN, "deactivate"]);
    assert_eq!(v["data"]["visible_to"], serde_json::json!(four));
    let v = ok(&fx, &p, &["--target", AN, "deactivate", "--yes"]);
    assert_eq!(v["data"]["visible_to"], serde_json::json!(four));
    // The claude-code plan keeps its own consumers and no shared sentence.
    let (_, out, _) = fx.run(&p, &["--target", CC, "add", "fx:sdlc/code-review"]);
    assert!(!out.contains(CONSUMERS_LINE), "{out}");
    fx.done();
}

#[test]
fn ac_24_agents_neutral_no_agent_config_file_is_created_or_modified() {
    let fx = Fx::new();
    let p = fx.project("p");
    fixture_hermes_home(&fx);
    fx.plant_file(&p.join("AGENTS.md"), b"# project rules\n");
    fx.plant_file(&p.join(".codex/config.toml"), b"model = \"x\"\n");
    fx.plant_file(&p.join(".cursor/rules/style.mdc"), b"---\n---\nrule\n");
    fx.plant_file(&p.join(".hermes/config.yaml"), b"trusted: []\n");
    let outside = |p: &Path| -> Vec<String> {
        snapshot_project(p)
            .0
            .into_iter()
            .map(|e| e.0)
            .filter(|r| {
                !(r.starts_with(".agents/skills")
                    || r.starts_with(".pouch")
                    || r.starts_with(".git"))
            })
            .collect()
    };
    let start = outside(&p);
    let (code, out, _) = fx.run(&p, &["--target", AN, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{out}");
    // Manual steps are printed, not performed.
    assert!(out.contains("hermes skills trust"), "{out}");
    assert!(out.contains("After apply"), "{out}");
    let mut got = outside(&p);
    got.retain(|r| r != ".agents" && r != ".agents/skills");
    assert_eq!(got, start);
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "ac_24",
    );
    ok(&fx, &p, &["--target", AN, "deactivate", "--yes"]);
    let mut got = outside(&p);
    got.retain(|r| r != ".agents" && r != ".agents/skills");
    assert_eq!(got, start);
    assert!(!p.join(".gitignore").exists());
    // The home directory's agent config is not touched either.
    assert!(
        fx.env
            .path("home")
            .join(".hermes")
            .read_dir()
            .unwrap()
            .next()
            .is_none()
    );
    fx.done();
}

#[test]
fn ac_29_agents_neutral_says_undocumented_and_prints_the_hermes_followup() {
    let fx = Fx::new();
    // No ~/.hermes: the refresh is stated as undocumented, no followup.
    let p = fx.project("p");
    let (code, out, _) = fx.run(&p, &["--target", AN, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("not documented"), "{out}");
    assert!(!out.contains("no restart or reload required"), "{out}");
    assert!(!out.contains("now active"), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("Then")), "{out}");
    let v = ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    assert_eq!(v["data"]["refresh"]["kind"], "unknown");
    assert!(v["data"]["refresh"]["command"].is_null());
    assert!(
        v["data"]["refresh"]["evidence"]
            .as_str()
            .unwrap()
            .starts_with("https://")
    );
    assert_eq!(v["data"]["followups"], serde_json::json!([]));
    // With ~/.hermes: the followup is printed (text only) and in --json.
    fixture_hermes_home(&fx);
    let q = fx.project("q");
    let (code, out, _) = fx.run(&q, &["--target", AN, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{out}");
    let canon = fx.canon(&q).to_string_lossy().into_owned();
    let then: Vec<&str> = out.lines().filter(|l| l.starts_with("Then")).collect();
    assert_eq!(then.len(), 1, "{out}");
    assert!(
        then[0].contains(&format!("hermes skills trust {canon}")),
        "{out}"
    );
    let r = fx.project("r");
    let v = ok(
        &fx,
        &r,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    let f = v["data"]["followups"].as_array().unwrap();
    assert_eq!(f.len(), 1, "{v}");
    assert!(
        f[0].as_str().unwrap().starts_with("hermes skills trust "),
        "{v}"
    );
    // The refresh note itself states each consumer.
    let text = v["data"]["refresh"]["text"].as_str().unwrap();
    for c in ["Codex", "Cursor", "Windsurf and Devin", "Hermes"] {
        assert!(text.contains(c), "{text}");
    }
    // claude-code output keeps its own wording and has no followup.
    let s = fx.project("s");
    let (code, out, _) = fx.run(&s, &["--target", CC, "add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0);
    assert!(!out.lines().any(|l| l.starts_with("Then")), "{out}");
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "ac_29",
    );
    fx.done();
}

#[test]
fn cross_adapter_duplicate_warns_once_per_skill() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    // Nothing is duplicated yet.
    let v = ok(
        &fx,
        &p,
        &["--target", CC, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    assert!(
        plan_warnings(&v, "cross-adapter-duplicate").is_empty(),
        "{v}"
    );
    // The same skill for the other target: one warning for that skill.
    let v = ok(
        &fx,
        &p,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    let w = plan_warnings(&v, "cross-adapter-duplicate");
    assert_eq!(w.len(), 1, "{v}");
    let msg = w[0]["message"].as_str().unwrap();
    assert!(msg.contains("code-review"), "{msg}");
    assert!(
        msg.contains("duplicate for cursor and windsurf-devin via compat read of .claude/skills"),
        "{msg}"
    );
    // Exit codes are unchanged (it is a warning), and the human plan shows it.
    let (code, out, _) = fx.run(
        &p,
        &["--target", AN, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("cross-adapter-duplicate"), "{out}");
    assert!(out.contains("pytest-fixtures"), "{out}");
    // An unreadable other receipt means no duplicate (EX11).
    let q = fx.project("q");
    ok(
        &fx,
        &q,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    fs::write(q.join(".pouch/agents-neutral/applied.json"), "not json").unwrap();
    let v = ok(
        &fx,
        &q,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    assert!(
        plan_warnings(&v, "cross-adapter-duplicate").is_empty(),
        "{v}"
    );
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "dup an",
    );
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::ClaudeCode,
        "dup cc",
    );
    fx.done();
}

#[test]
fn destination_policy_isolated_warns() {
    let fx = Fx::new();
    let cfg = fx.env.config_file();
    let text = fs::read_to_string(&cfg).unwrap();
    assert!(text.contains("destination_policy = \"shared\""), "{text}");
    fs::write(
        &cfg,
        text.replace(
            "destination_policy = \"shared\"",
            "destination_policy = \"isolated\"",
        ),
    )
    .unwrap();
    let p = fx.project("p");
    let v = ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    let w = plan_warnings(&v, "destination-policy-isolated");
    assert_eq!(w.len(), 1, "{v}");
    assert!(
        w[0]["message"]
            .as_str()
            .unwrap()
            .contains("not implemented in v1; using shared"),
        "{v}"
    );
    // The skill is still placed in the shared destination.
    assert_eq!(children(&an_dest(&p)), vec!["code-review"]);
    // claude-code is not affected by the policy.
    let q = fx.project("q");
    let v = ok(
        &fx,
        &q,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    assert!(
        plan_warnings(&v, "destination-policy-isolated").is_empty(),
        "{v}"
    );
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "isolated",
    );
    fx.done();
}

#[test]
fn bare_pouch_with_two_targets_exits_zero() {
    let fx = Fx::new();
    let p = fx.project("p");
    ok(
        &fx,
        &p,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:testing/pytest-fixtures", "--yes"],
    );
    let (code, out, err) = fx.run(&p, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("Usage"), "{out}");
    assert!(out.contains("more than one target"), "{out}");
    assert!(out.contains("--target"), "{out}");
    // With a single target the status table follows the usage text.
    let q = fx.project("q");
    ok(
        &fx,
        &q,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    let (code, out, _) = fx.run(&q, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("agents-neutral"), "{out}");
    assert!(!out.contains("more than one target"), "{out}");
    fx.done();
}

#[test]
fn agents_directory_alone_selects_agents_neutral_and_claude_code_stays_the_default() {
    let fx = Fx::new();
    let p = fx.project("p");
    fs::create_dir_all(p.join(".agents")).unwrap();
    let v = ok(&fx, &p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["target"], AN);
    assert_eq!(v["data"]["detection"], "present");
    assert_eq!(children(&an_dest(&p)), vec!["code-review"]);
    // Nothing detected defaults to claude-code (M3 decision 44).
    let q = fx.project("q");
    let v = ok(&fx, &q, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["target"], CC);
    // `.claude/` and `.agents/` together: the registry order wins.
    let r = fx.project("r");
    fs::create_dir_all(r.join(".claude")).unwrap();
    fs::create_dir_all(r.join(".agents")).unwrap();
    let v = ok(&fx, &r, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(v["data"]["target"], CC);
    fx.done();
}

#[test]
fn a_user_scope_skill_does_not_shadow_a_project_skill_under_agents_neutral() {
    let fx = Fx::new();
    let p = fx.project("p");
    // The same name in ~/.agents/skills (rank 2) and ~/.claude/skills.
    write_file(
        &fx.env.path("home"),
        ".agents/skills/code-review/SKILL.md",
        skill_md("code-review", "A user-scope skill.").as_bytes(),
    );
    fx.plant_user_skill("code-review");
    let v = ok(
        &fx,
        &p,
        &["--target", AN, "add", "fx:sdlc/code-review", "--yes"],
    );
    assert!(plan_warnings(&v, "shadowed").is_empty(), "{v}");
    let (_, s) = fx.json_raw(&p, &["--target", AN, "status"]);
    assert_eq!(
        record_states(&s),
        vec![("code-review".to_string(), "active".to_string())]
    );
    // Under claude-code the user location still shadows.
    let q = fx.project("q");
    let v = ok(
        &fx,
        &q,
        &["--target", CC, "add", "fx:sdlc/code-review", "--yes"],
    );
    assert_eq!(plan_warnings(&v, "shadowed").len(), 1, "{v}");
    assert_zero_divergence_in(
        &fx.env,
        &p,
        pouch_core::project::TargetId::AgentsNeutral,
        "user scope",
    );
    fx.done();
}
