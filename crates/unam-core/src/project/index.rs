//! The project index `<data dir>/projects.json` (M2 plan D4, P9): the list
//! of project roots that have written a selection, and the reference
//! scanners used by `source rm` and `bundle rm`.
//!
//! Schema: `{"version": 1, "projects": [string...]}`, canonical absolute root
//! strings, sorted and deduplicated.

use crate::bundle::slugify;
use crate::error::{Diagnostic, Error, Result};
use crate::fsops::write_atomic;
use crate::paths::Dirs;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// How the index file was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexState {
    Absent,
    Ok,
    Corrupt,
    Unreadable,
}

/// Result of `read_index`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexRead {
    pub projects: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub state: IndexState,
}

#[derive(Deserialize, Serialize)]
struct IndexFile {
    version: u32,
    projects: Vec<String>,
}

/// Read the index. Absent gives an empty list; a parse or IO failure gives
/// an empty list plus a `project-index-unreadable` warning.
pub fn read_index(dirs: &Dirs) -> IndexRead {
    let index_path = dirs.project_index_file();
    let text = match std::fs::read_to_string(&index_path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return IndexRead {
                projects: Vec::new(),
                diagnostics: Vec::new(),
                state: IndexState::Absent,
            };
        }
        Err(e) => {
            return IndexRead {
                projects: Vec::new(),
                diagnostics: vec![Diagnostic::warning(
                    "project-index-unreadable",
                    format!("cannot read project index {}: {e}", index_path.display()),
                )],
                state: IndexState::Unreadable,
            };
        }
    };
    match serde_json::from_str::<IndexFile>(&text) {
        Ok(index) => IndexRead {
            projects: index.projects,
            diagnostics: Vec::new(),
            state: IndexState::Ok,
        },
        Err(e) => IndexRead {
            projects: Vec::new(),
            diagnostics: vec![Diagnostic::warning(
                "project-index-unreadable",
                format!("cannot parse project index {}: {e}", index_path.display()),
            )],
            state: IndexState::Corrupt,
        },
    }
}

/// Add `root` (canonicalized when possible) to the index and write it
/// atomically. A corrupt index is rebuilt from this root alone (warning); an
/// index that cannot be read for another IO reason is not written (warning).
pub fn register(dirs: &Dirs, root: &Path) -> Result<Vec<Diagnostic>> {
    let read = read_index(dirs);
    if read.state == IndexState::Unreadable {
        return Ok(read.diagnostics);
    }
    let canonical = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut projects = read.projects;
    projects.push(canonical.to_string_lossy().into_owned());
    projects.sort();
    projects.dedup();
    let file = IndexFile {
        version: 1,
        projects,
    };
    let mut text = serde_json::to_string_pretty(&file).map_err(|e| {
        Error::io(
            "cannot serialize the project index",
            std::io::Error::other(e),
        )
    })?;
    text.push('\n');
    write_atomic(&dirs.project_index_file(), text.as_bytes())?;
    Ok(read.diagnostics)
}

/// Lenient view of a selection file for reference scanning: any YAML that
/// parses counts (a selection without `version` still counts).
fn scan_selection(root: &str, wants: impl Fn(&serde_norway::Value) -> bool) -> bool {
    let sel = Path::new(root).join(".unam").join("selection.yaml");
    let Ok(text) = std::fs::read_to_string(&sel) else {
        return false;
    };
    let Ok(value) = serde_norway::from_str::<serde_norway::Value>(&text) else {
        return false;
    };
    let Some(targets) = value.get("targets").and_then(|v| v.as_mapping()) else {
        return false;
    };
    targets.iter().any(|(_, target)| wants(target))
}

fn strings_under<'a>(target: &'a serde_norway::Value, key: &str) -> Vec<&'a str> {
    target
        .get(key)
        .and_then(|v| v.as_sequence())
        .map(|seq| seq.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default()
}

fn referencing(
    dirs: &Dirs,
    wants: impl Fn(&serde_norway::Value) -> bool,
) -> (Vec<String>, Vec<Diagnostic>) {
    let read = read_index(dirs);
    let mut out: Vec<String> = read
        .projects
        .into_iter()
        .filter(|root| scan_selection(root, &wants))
        .collect();
    out.sort();
    out.dedup();
    (out, read.diagnostics)
}

/// Project roots whose selection references source `alias`: a string under
/// `targets.*.add` or `targets.*.remove` that starts with `<alias>:`.
pub fn projects_referencing_source(dirs: &Dirs, alias: &str) -> (Vec<String>, Vec<Diagnostic>) {
    let prefix = format!("{alias}:");
    referencing(dirs, |target| {
        ["add", "remove"].iter().any(|key| {
            strings_under(target, key)
                .iter()
                .any(|s| s.starts_with(&prefix))
        })
    })
}

/// Project roots whose selection references bundle `slug`: a string under
/// `targets.*.bundles` whose slug equals `slug`.
pub fn projects_referencing_bundle(dirs: &Dirs, slug: &str) -> (Vec<String>, Vec<Diagnostic>) {
    referencing(dirs, |target| {
        strings_under(target, "bundles")
            .iter()
            .any(|s| slugify(s) == slug)
    })
}

#[cfg(test)]
pub(crate) mod tests_support {
    use crate::paths::{Dirs, Env};
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::path::Path;

    /// `Dirs` rooted in a tempdir (never the process environment).
    pub fn dirs_in(root: &Path) -> Dirs {
        let mut m: HashMap<String, OsString> = HashMap::new();
        m.insert("HOME".into(), root.join("home").into());
        m.insert("UNAM_CONFIG_DIR".into(), root.join("config").into());
        m.insert("UNAM_CACHE_DIR".into(), root.join("cache").into());
        m.insert("UNAM_BUNDLES_DIR".into(), root.join("bundles").into());
        m.insert("XDG_DATA_HOME".into(), root.join("data").into());
        Dirs::resolve(Env::from_map(m)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::dirs_in;
    use super::*;

    fn write_index(d: &Dirs, text: &str) {
        std::fs::create_dir_all(&d.data).unwrap();
        std::fs::write(d.project_index_file(), text).unwrap();
    }

    fn write_selection(root: &Path, text: &str) {
        std::fs::create_dir_all(root.join(".unam")).unwrap();
        std::fs::write(root.join(".unam/selection.yaml"), text).unwrap();
    }

    #[test]
    fn projects_referencing_source_cases() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        // Absent index.
        assert_eq!(projects_referencing_source(&d, "fx"), (vec![], vec![]));
        // Corrupt index.
        write_index(&d, "{not json");
        let (p, diags) = projects_referencing_source(&d, "fx");
        assert!(p.is_empty());
        assert_eq!(diags[0].code, "project-index-unreadable");
        // Referenced, unreferenced, and missing selection.
        let p1 = t.path().join("p1");
        let p2 = t.path().join("p2");
        let p3 = t.path().join("p3");
        write_selection(
            &p1,
            "version: 1\ntargets:\n  claude-code:\n    add: [\"fx:sdlc/code-review\"]\n",
        );
        write_selection(
            &p2,
            "version: 1\ntargets:\n  claude-code:\n    add: [\"other:x\"]\n    remove: [\"fxx:y\"]\n",
        );
        let idx = serde_json::json!({"version": 1, "projects": [p1, p2, p3]}).to_string();
        write_index(&d, &idx);
        let (p, diags) = projects_referencing_source(&d, "fx");
        assert_eq!(p, vec![p1.to_string_lossy().into_owned()]);
        assert!(diags.is_empty());
        write_selection(&p2, "targets:\n  t:\n    remove:\n      - fx:a\n");
        let (p, _) = projects_referencing_source(&d, "fx");
        assert_eq!(p.len(), 2);
    }

    #[test]
    fn projects_referencing_bundle_cases() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        assert_eq!(
            projects_referencing_bundle(&d, "backend-dev"),
            (vec![], vec![])
        );
        let p1 = t.path().join("p1");
        let p2 = t.path().join("p2");
        let p3 = t.path().join("p3");
        write_selection(
            &p1,
            "version: 1\ntargets:\n  claude-code:\n    bundles: [\"Backend Dev\"]\n",
        );
        write_selection(
            &p2,
            "targets:\n  agents-neutral:\n    bundles:\n      - backend-dev\n",
        );
        write_selection(
            &p3,
            "targets:\n  claude-code:\n    bundles: [other]\n    add: [backend-dev]\n",
        );
        let idx = serde_json::json!({"version": 1, "projects": [p2, p1, p3, p1]}).to_string();
        write_index(&d, &idx);
        let (p, diags) = projects_referencing_bundle(&d, "backend-dev");
        let mut expected = vec![
            p1.to_string_lossy().into_owned(),
            p2.to_string_lossy().into_owned(),
        ];
        expected.sort();
        assert_eq!(p, expected);
        assert!(diags.is_empty());
    }

    fn read_projects(d: &Dirs) -> Vec<String> {
        let v: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(d.project_index_file()).unwrap())
                .unwrap();
        assert_eq!(v["version"], 1);
        v["projects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p.as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn register_cases() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        let ca = std::fs::canonicalize(&a)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let cb = std::fs::canonicalize(&b)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        // Creates the data dir and the file.
        assert!(!d.data.exists());
        assert!(register(&d, &b).unwrap().is_empty());
        assert_eq!(read_projects(&d), vec![cb.clone()]);
        // Idempotent; two roots sorted and deduplicated.
        register(&d, &b).unwrap();
        register(&d, &a).unwrap();
        register(&d, &a).unwrap();
        assert_eq!(read_projects(&d), vec![ca.clone(), cb.clone()]);
        assert_eq!(read_index(&d).state, IndexState::Ok);
        // Corrupt index rebuilt from this root alone, with a warning.
        write_index(&d, "{not json");
        let diags = register(&d, &a).unwrap();
        assert_eq!(diags[0].code, "project-index-unreadable");
        assert_eq!(read_projects(&d), vec![ca.clone()]);
    }

    #[cfg(unix)]
    #[test]
    fn register_does_not_write_an_unreadable_index() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        let d = dirs_in(t.path());
        write_index(&d, "{\"version\": 1, \"projects\": []}");
        let p = d.project_index_file();
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read(&p).is_ok() {
            println!("SKIP-ROOT: running as root; chmod 000 does not block reads");
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
            return;
        }
        let diags = register(&d, t.path()).unwrap();
        assert_eq!(diags[0].code, "project-index-unreadable");
        assert_eq!(read_index(&d).state, IndexState::Unreadable);
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            std::fs::read_to_string(&p).unwrap(),
            "{\"version\": 1, \"projects\": []}"
        );
    }
}
