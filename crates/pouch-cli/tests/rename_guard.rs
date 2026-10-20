//! Rename guard: the project name is `pouch`. No source, manifest, snapshot or
//! fixture under `crates/` may carry the old name, and the environment
//! variables are `POUCH_` only. This file builds its needles at runtime so it
//! never contains the forbidden text, and it skips itself.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

use std::fs;
use std::path::{Path, PathBuf};

const SELF_NAME: &str = "rename_guard.rs";

/// English-word survivors: (file path suffix, line substring, reason).
/// Each entry needs a one-line reason. Empty means none are allowed.
const ENGLISH_WORD_ALLOWLIST: &[(&str, &str, &str)] = &[];

/// Every environment variable of the tool, named in the rename plan.
const POUCH_VARS: [&str; 11] = [
    "POUCH_CONFIG_DIR",
    "POUCH_BUNDLES_DIR",
    "POUCH_CACHE_DIR",
    "POUCH_LOG",
    "POUCH_FULL_FAULT_MATRIX",
    "POUCH_REAL_LIBRARY",
    "POUCH_ENTERPRISE_SKILLS_DIRS",
    "POUCH_PERF_CHILD",
    "POUCH_TUI_PANIC_CHILD",
    "POUCH_TEST_LIBS",
    "POUCH_TEST_UNSET_VAR",
];

fn crates_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf()
}

/// Every readable text file under `crates/`, skipping build output and this
/// file. Binary files (not valid UTF-8) are skipped.
fn corpus() -> Vec<(PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if path.is_dir() {
                if name != "target" {
                    walk(&path, out);
                }
            } else if name != SELF_NAME
                && let Ok(text) = fs::read_to_string(&path)
            {
                out.push((path, text));
            }
        }
    }
    let mut out = Vec::new();
    walk(&crates_root(), &mut out);
    out
}

#[test]
fn no_old_name_in_crates() {
    let old = ["un", "am"].concat();
    let word = ["un", "ambiguous"].concat();
    let mut hits = Vec::new();
    for (path, text) in corpus() {
        let shown = path.display().to_string();
        for (i, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            let mut from = 0;
            while let Some(pos) = lower[from..].find(&old) {
                let at = from + pos;
                let rest = &lower[at + old.len()..];
                if rest.starts_with(&word[old.len()..]) {
                    let allowed = ENGLISH_WORD_ALLOWLIST
                        .iter()
                        .any(|(suffix, frag, _)| shown.ends_with(suffix) && line.contains(frag));
                    if !allowed {
                        hits.push(format!("{shown}:{}: {}", i + 1, line.trim()));
                    }
                } else {
                    hits.push(format!("{shown}:{}: {}", i + 1, line.trim()));
                }
                from = at + old.len();
            }
        }
    }
    assert!(
        hits.is_empty(),
        "old name found in crates ({} hits):\n{}",
        hits.len(),
        hits.join("\n")
    );
}

#[test]
fn env_vars_are_pouch_only() {
    let old_prefix = ["UN", "AM_"].concat();
    let corpus = corpus();
    let mut stale = Vec::new();
    for (path, text) in &corpus {
        for (i, line) in text.lines().enumerate() {
            if line.contains(&old_prefix) {
                stale.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "old environment variable prefix found:\n{}",
        stale.join("\n")
    );
    for var in POUCH_VARS {
        assert!(
            corpus.iter().any(|(_, text)| text.contains(var)),
            "{var} appears nowhere under crates/"
        );
    }
}
