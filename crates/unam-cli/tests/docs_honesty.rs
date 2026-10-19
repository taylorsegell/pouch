//! Honesty checks on the public documents (milestone M7, D-HONEST; PRD G12,
//! SES-1, SES-2): the README carries the required caveats and leads with the
//! primary scenario, and no document makes an unverified claim.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

/// Marker phrases the README must contain, literally.
const REQUIRED: [&str; 4] = [
    "Linux: tests pass in CI on Ubuntu x86_64",
    "Cursor editor and Windsurf are not tested",
    "no telemetry",
    "no network",
];

/// Phrases no line of any public document may contain (compared lower case).
const FORBIDDEN: [&str; 3] = [
    "Linux is verified",
    "Windsurf is supported",
    "Cursor editor is supported",
];

fn missing_markers(text: &str) -> Vec<&'static str> {
    REQUIRED
        .iter()
        .filter(|m| !text.contains(**m))
        .copied()
        .collect()
}

/// `(line number, phrase)` for every forbidden phrase on a line.
fn forbidden_hits(text: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let low = line.to_lowercase();
        for f in FORBIDDEN {
            if low.contains(&f.to_lowercase()) {
                out.push((i + 1, f));
            }
        }
    }
    out
}

/// The text of the first fenced block after the first `## ` heading.
fn first_example(text: &str) -> Option<String> {
    let mut lines = text.lines().skip_while(|l| !l.starts_with("## "));
    lines.next()?;
    let mut lines = lines.skip_while(|l| !l.trim_start().starts_with("```"));
    lines.next()?;
    let body: Vec<&str> = lines
        .take_while(|l| !l.trim_start().starts_with("```"))
        .collect();
    Some(body.join("\n"))
}

/// The primary scenario: a bare `unam` line (an optional `#` comment is
/// allowed) and the word `space` for choosing skills in the picker.
fn is_primary_scenario(example: &str) -> bool {
    let bare = example
        .lines()
        .any(|l| l.split('#').next().unwrap_or("").trim() == "unam");
    bare && example.contains("space")
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap()
}

#[test]
fn docs_honesty_readme_carries_required_caveats() {
    let readme = read("README.md");
    assert!(!readme.is_empty());
    let missing = missing_markers(&readme);
    assert!(missing.is_empty(), "README lacks: {missing:?}");
    let example = first_example(&readme).expect("README has a fenced example after a ## heading");
    assert!(
        is_primary_scenario(&example),
        "the first example is not the primary scenario: {example}"
    );
}

#[test]
fn docs_honesty_no_forbidden_claims() {
    let files = [
        "README.md",
        "docs/adapters.md",
        "docs/json.md",
        "CHANGELOG.md",
    ];
    let mut read_count = 0;
    let mut hits = Vec::new();
    for f in files {
        let text = read(f);
        assert!(!text.trim().is_empty(), "{f} is empty");
        read_count += 1;
        for (line, phrase) in forbidden_hits(&text) {
            hits.push(format!("{f}:{line}: {phrase}"));
        }
    }
    assert_eq!(read_count, 4);
    assert!(hits.is_empty(), "{hits:#?}");
}

#[test]
fn docs_honesty_check_detects_violations() {
    let good = format!(
        "## Start\n\n```\nunam   # then press space\n```\n\n{}\n",
        REQUIRED.join("\n")
    );
    assert!(missing_markers(&good).is_empty());
    assert!(forbidden_hits(&good).is_empty());
    assert!(is_primary_scenario(&first_example(&good).unwrap()));

    // Each marker, removed in turn, is reported.
    for m in REQUIRED {
        let bad = good.replace(m, "");
        assert_eq!(missing_markers(&bad), vec![m], "{m}");
    }
    // Each forbidden phrase is caught, in any case, with its line number.
    for f in FORBIDDEN {
        let bad = format!("{good}\nSure: {} here.\n", f.to_uppercase());
        let hits = forbidden_hits(&bad);
        assert_eq!(hits.len(), 1, "{f}: {hits:?}");
        assert_eq!(hits[0].1, f);
    }
    // The primary scenario needs a bare `unam` line and the word `space`.
    assert!(!is_primary_scenario("unam add x\nspace"));
    assert!(!is_primary_scenario("unam"));
    assert!(first_example("# Title\n```\nunam\n```\n").is_none());
}
