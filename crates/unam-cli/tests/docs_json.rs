//! `docs/json.md` documents every `--json` snapshot (milestone M7, spec
//! line 648): each JSON snapshot has a marker `<!-- snap: NAME -->`, and every
//! top-level key of its `data` object appears in backticks in the marker's
//! section. The envelope keys are checked once in the `## Envelope` section.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const ENVELOPE_KEYS: [&str; 5] = ["ok", "command", "data", "warnings", "errors"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn snapshot_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("snapshots")
}

/// The JSON body of an insta snapshot file (the header between the two
/// `---` lines removed); `None` when the body is not JSON.
fn snapshot_json(text: &str) -> Option<Value> {
    let mut parts = text.splitn(3, "---\n");
    parts.next()?;
    parts.next()?;
    serde_json::from_str(parts.next()?).ok()
}

/// Every marker name in `doc`.
fn marker_names(doc: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in doc.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("<!-- snap:")
            && let Some(name) = rest.strip_suffix("-->")
        {
            out.push(name.trim().to_string());
        }
    }
    out
}

/// The section holding line `idx`: from the nearest preceding `## ` heading
/// to the next `## ` heading.
fn section_at(lines: &[&str], idx: usize) -> Option<String> {
    let start = (0..=idx).rev().find(|&i| lines[i].starts_with("## "))?;
    let end = (idx + 1..lines.len())
        .find(|&i| lines[i].starts_with("## "))
        .unwrap_or(lines.len());
    Some(lines[start..end].join("\n"))
}

/// The section of the marker `name`, if the marker exists under a heading.
fn marker_section(doc: &str, name: &str) -> Option<String> {
    let lines: Vec<&str> = doc.lines().collect();
    let want = format!("<!-- snap: {name} -->");
    let idx = lines.iter().position(|l| l.trim() == want)?;
    section_at(&lines, idx)
}

/// The section whose heading is exactly `## Envelope`.
fn envelope_section(doc: &str) -> Option<String> {
    let lines: Vec<&str> = doc.lines().collect();
    let idx = lines.iter().position(|l| l.trim() == "## Envelope")?;
    section_at(&lines, idx)
}

/// Problems for one snapshot: a missing marker, or each `data` key that is
/// not in backticks in the marker's section. Empty means documented.
fn problems(doc: &str, name: &str, snapshot: &Value) -> Vec<String> {
    let Some(section) = marker_section(doc, name) else {
        return vec![format!("{name}: no marker or no section")];
    };
    let mut out = Vec::new();
    if let Some(data) = snapshot.get("data").and_then(Value::as_object) {
        for key in data.keys() {
            if !section.contains(&format!("`{key}`")) {
                out.push(format!("{name}: data key `{key}` not documented"));
            }
        }
    }
    out
}

fn envelope_problems(doc: &str, snapshot: &Value) -> Vec<String> {
    let Some(section) = envelope_section(doc) else {
        return vec!["no `## Envelope` section".to_string()];
    };
    let mut out = Vec::new();
    let keys: Vec<String> = snapshot
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    for key in keys.iter().map(String::as_str).chain(ENVELOPE_KEYS) {
        if !section.contains(&format!("`{key}`")) {
            out.push(format!("envelope key `{key}` not documented"));
        }
    }
    out
}

/// `(name, parsed JSON)` for every JSON snapshot file, plus the count of
/// snapshot files read (JSON or not).
fn json_snapshots() -> (Vec<(String, Value)>, usize) {
    let mut files: Vec<PathBuf> = fs::read_dir(snapshot_dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("snapshots__") && n.ends_with(".snap"))
        })
        .collect();
    files.sort();
    let total = files.len();
    let mut out = Vec::new();
    for f in files {
        let stem = f.file_stem().unwrap().to_str().unwrap().to_string();
        let name = stem.strip_prefix("snapshots__").unwrap().to_string();
        if let Some(v) = snapshot_json(&fs::read_to_string(&f).unwrap()) {
            out.push((name, v));
        }
    }
    (out, total)
}

fn json_doc() -> String {
    fs::read_to_string(repo_root().join("docs").join("json.md")).unwrap()
}

#[test]
fn docs_json_every_snapshot_is_documented() {
    let doc = json_doc();
    let (snaps, total) = json_snapshots();
    // Not vacuous: the snapshot directory is read and most files are JSON.
    assert!(
        total >= 30 && snaps.len() >= 30,
        "{total} files, {}",
        snaps.len()
    );
    let mut all = Vec::new();
    for (name, v) in &snaps {
        all.extend(problems(&doc, name, v));
    }
    assert!(all.is_empty(), "{all:#?}");
    let envelope = envelope_problems(&doc, &snaps[0].1);
    assert!(envelope.is_empty(), "{envelope:#?}");
}

#[test]
fn docs_json_every_marker_has_a_snapshot() {
    let doc = json_doc();
    let names = marker_names(&doc);
    // Control: an empty or unmarked document cannot pass.
    assert!(names.len() >= 15, "{} markers", names.len());
    for n in &names {
        let f = snapshot_dir().join(format!("snapshots__{n}.snap"));
        assert!(f.is_file(), "marker {n} has no snapshot {}", f.display());
    }
    let mut sorted = names.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "duplicate marker names");
}

#[test]
fn docs_json_check_detects_missing_key() {
    let doc = "\
## Envelope

`ok` `command` `data` `warnings` `errors`

## Thing

<!-- snap: thing -->

`a` and `b`

## Other

`c`
";
    let snap: Value = serde_json::json!({
        "ok": true, "command": "x", "data": {"a": 1, "b": 2},
        "warnings": [], "errors": []
    });
    assert!(problems(doc, "thing", &snap).is_empty());
    assert!(envelope_problems(doc, &snap).is_empty());

    // A key that only the next section documents does not count.
    let more: Value = serde_json::json!({"data": {"a": 1, "c": 3}});
    let p = problems(doc, "thing", &more);
    assert_eq!(p.len(), 1, "{p:?}");
    assert!(p[0].contains("`c`"));

    // A missing marker is a problem, and so is a missing envelope key.
    assert_eq!(problems(doc, "absent", &snap).len(), 1);
    let no_env = "## Envelope\n\n`ok` `data`\n";
    assert!(!envelope_problems(no_env, &snap).is_empty());
    assert!(!envelope_problems("## Thing\n", &snap).is_empty());

    // The snapshot header is stripped and a text body is not JSON.
    let file = "---\nsource: x\nexpression: v\n---\n{\n  \"ok\": true\n}\n";
    assert_eq!(snapshot_json(file).unwrap()["ok"], true);
    assert!(snapshot_json("---\nsource: x\n---\nUsage: unam\n").is_none());
}
