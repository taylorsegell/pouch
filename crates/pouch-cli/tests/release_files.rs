//! Release files (milestone M7): licenses, package metadata, the rename to
//! `pouch` (binary `pouch`), the cargo-dist config, and the guards that
//! keep `publish = false` and unwinding panics.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const REPOSITORY: &str = "https://github.com/taylorsegell/pouch";
const TARGETS: [&str; 4] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn read(rel: &str) -> String {
    fs::read_to_string(repo_root().join(rel)).unwrap()
}

/// Lines of a TOML text with `#` comments cut off.
fn code_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Problems with the `cargo metadata` JSON: license, repository, publish,
/// names, bin target, dist flag.
fn metadata_problems(meta: &Value) -> Vec<String> {
    let mut out = Vec::new();
    let pkgs = meta["packages"].as_array().cloned().unwrap_or_default();
    if pkgs.len() != 2 {
        out.push(format!("{} packages, want 2", pkgs.len()));
    }
    let find = |n: &str| pkgs.iter().find(|p| p["name"] == n);
    for name in ["pouch", "pouch-core"] {
        let Some(p) = find(name) else {
            out.push(format!("package {name} missing"));
            continue;
        };
        if p["license"] != "MIT OR Apache-2.0" {
            out.push(format!("{name}: license {}", p["license"]));
        }
        if p["repository"] != REPOSITORY {
            out.push(format!("{name}: repository {}", p["repository"]));
        }
        if p["publish"] != serde_json::json!([]) {
            out.push(format!("{name}: publish {} (want [])", p["publish"]));
        }
    }
    match find("pouch") {
        Some(p) => {
            let has_bin = p["targets"].as_array().is_some_and(|ts| {
                ts.iter().any(|t| {
                    t["name"] == "pouch"
                        && t["kind"]
                            .as_array()
                            .is_some_and(|k| k.iter().any(|x| x == "bin"))
                })
            });
            if !has_bin {
                out.push("pouch has no bin target pouch".to_string());
            }
            if p["metadata"]["dist"]["dist"] != true {
                out.push("[package.metadata.dist] dist is not true".to_string());
            }
        }
        None => out.push("package pouch missing".to_string()),
    }
    out
}

/// Problems with `dist-workspace.toml` text.
fn dist_problems(text: &str) -> Vec<String> {
    let lines = code_lines(text);
    let mut out = Vec::new();
    for t in TARGETS {
        let quoted = format!("\"{t}\"");
        if !lines
            .iter()
            .any(|l| l.starts_with("targets") && l.contains(&quoted))
        {
            out.push(format!("target {t} missing"));
        }
    }
    if !lines
        .iter()
        .any(|l| l.starts_with("installers") && l.contains("\"homebrew\""))
    {
        out.push("homebrew installer missing".to_string());
    }
    if lines.iter().any(|l| l.starts_with("publish-jobs")) {
        out.push("publish-jobs is set".to_string());
    }
    out
}

fn compact(line: &str) -> String {
    line.chars()
        .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
        .collect()
}

/// Problems with a `Cargo.toml` text: a `[profile.dist]` table with
/// `inherits = "release"`, and no panic abort.
fn manifest_problems(text: &str) -> Vec<String> {
    let lines = code_lines(text);
    let mut out = Vec::new();
    let at = lines.iter().position(|l| l == "[profile.dist]");
    match at {
        None => out.push("no [profile.dist] table".to_string()),
        Some(i) => {
            let body = lines[i + 1..].iter().take_while(|l| !l.starts_with('['));
            if !body.map(|l| compact(l)).any(|l| l == "inherits=release") {
                out.push("[profile.dist] does not inherit release".to_string());
            }
        }
    }
    if lines.iter().any(|l| compact(l).contains("panic=abort")) {
        out.push("panic = abort".to_string());
    }
    out
}

fn cargo_metadata() -> Value {
    let out = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--locked",
            "--format-version",
            "1",
        ])
        .current_dir(repo_root())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn release_files_license_and_metadata() {
    let mit = read("LICENSE-MIT");
    assert!(mit.contains("Copyright (c) 2026 Taylor Segell"), "{mit}");
    assert!(mit.contains("MIT License"), "{mit}");
    let apache = read("LICENSE-APACHE");
    assert!(apache.contains("Apache License"));
    assert!(apache.contains("Version 2.0"));
    assert!(apache.contains("END OF TERMS AND CONDITIONS"));
    let problems = metadata_problems(&cargo_metadata());
    assert!(problems.is_empty(), "{problems:#?}");
}

#[test]
fn release_files_dist_config() {
    let dist = read("dist-workspace.toml");
    let problems = dist_problems(&dist);
    assert!(problems.is_empty(), "{problems:#?}");
    assert!(
        repo_root().join(".github/workflows/release.yml").is_file(),
        "release.yml not generated"
    );
    let root = read("Cargo.toml");
    let problems = manifest_problems(&root);
    assert!(problems.is_empty(), "{problems:#?}");
    for rel in [
        "crates/pouch-cli/Cargo.toml",
        "crates/pouch-core/Cargo.toml",
        "dist-workspace.toml",
    ] {
        assert!(
            !code_lines(&read(rel))
                .iter()
                .any(|l| compact(l).contains("panic=abort")),
            "{rel} sets panic abort"
        );
    }
}

#[test]
fn release_files_check_detects_bad_input() {
    // Good samples pass.
    let pkg = |name: &str, license: &str, publish: Value| {
        serde_json::json!({
            "name": name, "license": license, "repository": REPOSITORY,
            "publish": publish,
            "targets": [{"name": "pouch", "kind": ["bin"]}],
            "metadata": {"dist": {"dist": true}}
        })
    };
    let good = serde_json::json!({"packages": [
        pkg("pouch", "MIT OR Apache-2.0", serde_json::json!([])),
        pkg("pouch-core", "MIT OR Apache-2.0", serde_json::json!([])),
    ]});
    assert!(metadata_problems(&good).is_empty());
    // A wrong license, an unrestricted publish, a missing package.
    let bad_license = serde_json::json!({"packages": [
        pkg("pouch", "MIT", serde_json::json!([])),
        pkg("pouch-core", "MIT OR Apache-2.0", serde_json::json!([])),
    ]});
    assert_eq!(metadata_problems(&bad_license).len(), 1);
    let bad_publish = serde_json::json!({"packages": [
        pkg("pouch", "MIT OR Apache-2.0", Value::Null),
        pkg("pouch-core", "MIT OR Apache-2.0", serde_json::json!([])),
    ]});
    assert_eq!(metadata_problems(&bad_publish).len(), 1);
    let one = serde_json::json!({"packages": [
        pkg("pouch", "MIT OR Apache-2.0", serde_json::json!([])),
    ]});
    assert!(metadata_problems(&one).len() >= 2);

    // dist-workspace.toml: good passes; a missing target, a missing
    // installer, and a publish-jobs line each fail.
    let dist_good = "# c\n[dist]\ntargets = [\"aarch64-apple-darwin\", \"x86_64-apple-darwin\", \"x86_64-unknown-linux-gnu\", \"aarch64-unknown-linux-gnu\"]\ninstallers = [\"shell\", \"homebrew\"]\n";
    assert!(dist_problems(dist_good).is_empty());
    let no_target = dist_good.replace(", \"aarch64-unknown-linux-gnu\"", "");
    assert_eq!(dist_problems(&no_target).len(), 1);
    let no_brew = dist_good.replace(", \"homebrew\"", "");
    assert_eq!(dist_problems(&no_brew).len(), 1);
    let with_jobs = format!("{dist_good}publish-jobs = [\"homebrew\"]\n");
    assert_eq!(dist_problems(&with_jobs).len(), 1);
    // A commented-out publish-jobs line is not a setting.
    let commented = format!("{dist_good}# publish-jobs = [\"homebrew\"]\n");
    assert!(dist_problems(&commented).is_empty());

    // Cargo.toml: good passes; a missing table, a profile that does not
    // inherit release, and a panic abort line each fail.
    let manifest_good = "[profile.release]\nlto = true\n\n[profile.dist]\ninherits = \"release\"\n";
    assert!(manifest_problems(manifest_good).is_empty());
    assert_eq!(
        manifest_problems("[profile.release]\nlto = true\n").len(),
        1
    );
    assert_eq!(
        manifest_problems("[profile.dist]\ninherits = \"dev\"\n").len(),
        1
    );
    let aborting = format!("{manifest_good}\n[profile.release]\npanic = \"abort\"\n");
    assert_eq!(manifest_problems(&aborting).len(), 1);
}
