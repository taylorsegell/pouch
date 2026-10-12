//! `unam doctor` (milestone M4, spec 5): the six checks, exit 9 on any
//! failure, `--evidence`, and the guarantee that the project is left
//! byte-identical.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;
use std::fs;

fn check_status(v: &Value, id: &str) -> Vec<String> {
    v["data"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["id"] == id)
        .map(|c| c["status"].as_str().unwrap().to_string())
        .collect()
}

#[cfg(unix)]
#[test]
fn doctor_clean_project_passes_every_check_and_leaves_the_project_untouched() {
    let fx = Fx::new();
    let p = fx.project("p");
    // Before any apply: no destination root yet, and none is created.
    let start = snapshot_project(&p);
    let (code, v) = fx.json_raw(&p, &["doctor"]);
    assert_eq!(code, 0, "{v}");
    assert_project_unchanged(&start, &p);
    assert!(!p.join(".claude").exists() && !p.join(".unam").exists());

    let (code, v) = fx.json_raw(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{v}");
    let start = snapshot_project(&p);
    let (code, v) = fx.json_raw(&p, &["doctor"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    for id in [
        "symlink-capability",
        "destination-writable",
        "receipt-integrity",
        "journal-presence",
    ] {
        assert_eq!(check_status(&v, id), vec!["ok"], "{id}");
    }
    assert_eq!(check_status(&v, "source-reachability"), vec!["ok", "ok"]);
    assert_eq!(check_status(&v, "adapter-detection"), vec!["info"]);
    assert!(v["data"]["evidence"].is_null());
    assert_project_unchanged(&start, &p);
    let (code, out, _) = fx.run(&p, &["doctor"]);
    assert_eq!(code, 0);
    for want in ["RESULT", "symlink-capability", "journal-presence"] {
        assert!(out.contains(want), "{want}: {out}");
    }
    fx.done();
}

#[test]
fn doctor_unreachable_source_fails_with_exit_9() {
    let fx = Fx::new();
    let p = fx.project("p");
    let scratch = fx.env.path("work/scratch-lib");
    write_file(
        &scratch,
        "one/SKILL.md",
        skill_md("one", "Goes away.").as_bytes(),
    );
    let (code, v) = fx.json_raw(&p, &["source", "add", "scratch", scratch.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    fs::remove_dir_all(&scratch).unwrap();
    let (code, v) = fx.json_raw(&p, &["doctor"]);
    assert_eq!(code, 9, "{v}");
    assert_eq!(v["ok"], false);
    assert_eq!(v["errors"][0]["code"], "doctor-failed");
    let statuses = check_status(&v, "source-reachability");
    assert_eq!(statuses.iter().filter(|s| *s == "fail").count(), 1);
    assert_eq!(statuses.iter().filter(|s| *s == "ok").count(), 2);
    let (code, out, err) = fx.run(&p, &["doctor"]);
    assert_eq!(code, 9);
    assert!(out.contains("fail") && out.contains("unreachable"), "{out}");
    assert!(err.contains("error:"), "{err}");
    fx.done();
}

#[test]
fn doctor_leftover_journal_fails_with_exit_9() {
    let fx = Fx::new();
    let p = fx.project("p");
    write_file(&p, ".unam/claude-code/journal.jsonl", b"");
    let (code, v) = fx.json_raw(&p, &["doctor"]);
    assert_eq!(code, 9, "{v}");
    assert_eq!(check_status(&v, "journal-presence"), vec!["fail"]);
    assert_eq!(check_status(&v, "receipt-integrity"), vec!["ok"]);
    fx.done();
}

#[test]
fn doctor_evidence_lists_every_adapter_claim_with_its_source() {
    let fx = Fx::new();
    let p = fx.project("p");
    let (code, v) = fx.json_raw(&p, &["doctor", "--evidence"]);
    assert_eq!(code, 0, "{v}");
    let rows = v["data"]["evidence"].as_array().unwrap();
    let mut expected: Vec<(String, String)> = Vec::new();
    for a in unam_core::adapter::adapters() {
        for e in a.evidence() {
            let source = match e.source {
                unam_core::adapter::EvidenceSource::Url(u)
                | unam_core::adapter::EvidenceSource::Spike(u) => u,
            };
            expected.push((e.claim_key.to_string(), source.to_string()));
        }
    }
    assert!(!expected.is_empty());
    let mut got: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            (
                r["claim_key"].as_str().unwrap().to_string(),
                r["source"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    got.sort();
    expected.sort();
    assert_eq!(got, expected);
    assert!(
        rows.iter()
            .any(|r| r["source"].as_str().unwrap().starts_with("https://"))
    );
    let (code, out, _) = fx.run(&p, &["doctor", "--evidence"]);
    assert_eq!(code, 0);
    for (key, source) in expected {
        assert!(out.contains(&key) && out.contains(&source), "{key}: {out}");
    }
    fx.done();
}

#[cfg(unix)]
#[test]
fn doctor_unwritable_destination_fails_with_exit_9() {
    use std::os::unix::fs::PermissionsExt;
    let fx = Fx::new();
    let p = fx.project("p");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::create_dir(p.join("probe-dir")).is_ok() {
        // Hybrid row: the precondition is a non-root user.
        println!("SKIP-ROOT: running as root; chmod 555 does not block writes");
        fs::remove_dir(p.join("probe-dir")).unwrap();
    } else {
        let (code, v) = fx.json_raw(&p, &["doctor"]);
        assert_eq!(code, 9, "{v}");
        assert_eq!(check_status(&v, "destination-writable"), vec!["fail"]);
    }
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    fx.done();
}
