//! `docs/adapters.md` carries every adapter claim with its source (milestone
//! M7, PRD S9): the rows `unam doctor --evidence` prints must each have a row
//! in the matching `## Adapter evidence: the <adapter> claims` section.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn adapters_doc() -> String {
    fs::read_to_string(repo_root().join("docs").join("adapters.md")).unwrap()
}

/// The lines of the section `## Adapter evidence: the <adapter> claims`
/// (heading excluded), up to the next `## ` heading.
fn adapter_section<'a>(doc: &'a str, adapter: &str) -> Vec<&'a str> {
    let heading = format!("## Adapter evidence: the {adapter} claims");
    let mut lines = doc.lines();
    if !lines.any(|l| l.trim_end() == heading) {
        return Vec::new();
    }
    lines.take_while(|l| !l.starts_with("## ")).collect()
}

/// True when the adapter's own section has one line that holds the
/// backticked claim key and the source. A bare substring search of the whole
/// file would be vacuous (`destination` occurs throughout).
fn has_evidence_row(doc: &str, adapter: &str, claim_key: &str, source: &str) -> bool {
    let key = format!("`{claim_key}`");
    adapter_section(doc, adapter)
        .iter()
        .any(|l| l.starts_with('|') && l.contains(&key) && l.contains(source))
}

/// Move the evidence row of `claim_key` out of the `from` section into the
/// `to` section, right after that section's table header separator.
fn move_row(doc: &str, claim_key: &str, from: &str, to: &str) -> String {
    let key = format!("`{claim_key}`");
    let from_heading = format!("## Adapter evidence: the {from} claims");
    let to_heading = format!("## Adapter evidence: the {to} claims");
    let lines: Vec<&str> = doc.lines().collect();
    let from_at = lines.iter().position(|l| *l == from_heading).unwrap();
    let row_at = (from_at..lines.len())
        .find(|&i| lines[i].starts_with('|') && lines[i].contains(&key))
        .unwrap();
    let row = lines[row_at];
    let mut out: Vec<&str> = lines.clone();
    out.remove(row_at);
    let to_at = out.iter().position(|l| *l == to_heading).unwrap();
    let sep_at = (to_at..out.len())
        .find(|&i| out[i].starts_with("|---"))
        .unwrap();
    out.insert(sep_at + 1, row);
    out.join("\n")
}

#[test]
fn docs_adapters_cover_every_doctor_evidence_claim() {
    let doc = adapters_doc();
    let fx = Fx::new();
    let p = fx.project("p");
    // `doctor --evidence` in a temp project, never this repository.
    let (code, v) = fx.json_raw(&p, &["doctor", "--evidence"]);
    assert_eq!(code, 0, "{v}");
    let rows = v["data"]["evidence"].as_array().unwrap();
    let mut missing = Vec::new();
    for r in rows {
        let adapter = r["adapter"].as_str().unwrap();
        let key = r["claim_key"].as_str().unwrap();
        let source = r["source"].as_str().unwrap();
        if !has_evidence_row(&doc, adapter, key, source) {
            missing.push(format!("{adapter} {key} {source}"));
        }
    }
    // Not vacuous: the real evidence list has dozens of rows.
    assert!(rows.len() >= 20, "{} rows", rows.len());
    assert!(
        missing.is_empty(),
        "no row in docs/adapters.md: {missing:#?}"
    );

    // The two M2 sections are kept, and the M0 outcome names the CLI.
    for h in [
        "## Bundle slugify parity with Hermes",
        "## Bundle loader: differences from Hermes",
    ] {
        assert!(doc.lines().any(|l| l == h), "missing heading {h}");
    }
    let m0: Vec<&str> = {
        let mut it = doc.lines();
        assert!(it.any(|l| l == "## Adapter evidence: the M0 outcome"));
        it.take_while(|l| !l.starts_with("## ")).collect()
    };
    let m0 = m0.join("\n");
    assert!(m0.contains("`cursor-agent` CLI"), "{m0}");
    assert!(m0.contains("Cursor editor"), "{m0}");
    fx.done();
}

#[test]
fn docs_adapters_check_detects_missing_row() {
    let doc = adapters_doc();
    let key = "scan:ancestor";
    let source = "https://learn.chatgpt.com/docs/build-skills.md";
    // The real doc passes, and the section is not empty.
    assert!(has_evidence_row(&doc, "agents-neutral", key, source));
    assert!(adapter_section(&doc, "agents-neutral").len() > 10);

    // Row removed: fails.
    let without: String = doc
        .lines()
        .filter(|l| !(l.starts_with('|') && l.contains(&format!("`{key}`"))))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!has_evidence_row(&without, "agents-neutral", key, source));

    // Row under the wrong adapter: fails for the right one and holds for the
    // wrong one only, which is why the section matters.
    let moved = move_row(&doc, key, "agents-neutral", "claude-code");
    assert!(!has_evidence_row(&moved, "agents-neutral", key, source));
    assert!(has_evidence_row(&moved, "claude-code", key, source));

    // A wrong source on the same key fails.
    assert!(!has_evidence_row(
        &doc,
        "agents-neutral",
        key,
        "https://example.invalid/none"
    ));
    // A heading that does not exist yields no rows.
    assert!(!has_evidence_row(&doc, "nonexistent", key, source));

    // At least 20 real rows are covered (13 claude-code + 16 agents-neutral).
    let rows = |a: &str| {
        adapter_section(&doc, a)
            .iter()
            .filter(|l| l.starts_with('|') && l.contains('`'))
            .count()
    };
    assert!(rows("claude-code") + rows("agents-neutral") >= 20);
}
