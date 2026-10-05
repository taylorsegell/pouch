// Integration helpers may unwrap: allow-unwrap-in-tests does not cover
// non-test helper functions in a shared module.
#![allow(clippy::unwrap_used)]
// Each test binary uses a subset of these helpers.
#![allow(dead_code)]

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// An isolated environment: every unam directory, HOME, and the XDG bases
/// point into one tempdir, and the working directory is a tempdir too.
pub struct TestEnv {
    pub tmp: TempDir,
    pub root: PathBuf,
    pub canonical_root: PathBuf,
}

const SUBDIRS: &[&str] = &[
    "home",
    "xdg-config",
    "xdg-cache",
    "xdg-data",
    "config",
    "bundles",
    "cache",
    "data",
    "cwd",
    "work",
];

impl TestEnv {
    pub fn new() -> Self {
        let tmp = tempfile::Builder::new()
            .prefix("unam-test-")
            .tempdir()
            .unwrap();
        let root = tmp.path().to_path_buf();
        let canonical_root = fs::canonicalize(&root).unwrap();
        // Isolation rule (plan Safety): under the system temp dir, not under
        // the crate or repository.
        let temp = std::env::temp_dir();
        let temp_canon = fs::canonicalize(&temp).unwrap_or(temp.clone());
        assert!(
            root.starts_with(&temp) || canonical_root.starts_with(&temp_canon),
            "test root {} is not under the system temp dir",
            root.display()
        );
        let manifest = fs::canonicalize(env!("CARGO_MANIFEST_DIR")).unwrap();
        let repo = manifest
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf();
        assert!(!canonical_root.starts_with(&manifest));
        assert!(!canonical_root.starts_with(&repo));
        for d in SUBDIRS {
            fs::create_dir_all(root.join(d)).unwrap();
        }
        TestEnv {
            tmp,
            root,
            canonical_root,
        }
    }

    pub fn path(&self, sub: &str) -> PathBuf {
        self.root.join(sub)
    }

    /// The isolating environment variables applied to every command.
    pub fn vars(&self) -> Vec<(&'static str, PathBuf)> {
        vec![
            ("HOME", self.path("home")),
            ("XDG_CONFIG_HOME", self.path("xdg-config")),
            ("XDG_CACHE_HOME", self.path("xdg-cache")),
            ("XDG_DATA_HOME", self.path("xdg-data")),
            ("UNAM_CONFIG_DIR", self.path("config")),
            ("UNAM_BUNDLES_DIR", self.path("bundles")),
            ("UNAM_CACHE_DIR", self.path("cache")),
        ]
    }

    /// A `unam` command with a cleared, isolated environment.
    pub fn cmd(&self) -> Command {
        let mut c = Command::new(assert_cmd::cargo::cargo_bin!("unam"));
        c.env_clear();
        for (k, v) in self.vars() {
            c.env(k, v);
        }
        c.current_dir(self.path("cwd"));
        c
    }

    pub fn config_file(&self) -> PathBuf {
        self.path("config").join("config.toml")
    }

    pub fn cache_file(&self, alias: &str) -> PathBuf {
        self.path("cache")
            .join("catalog")
            .join(format!("{alias}.json"))
    }

    pub fn project_index(&self) -> PathBuf {
        self.path("xdg-data").join("unam").join("projects.json")
    }

    /// Replace the temp root (as created and canonicalized) with `[TMP]`.
    pub fn normalize(&self, text: &str) -> String {
        let canon = self.canonical_root.to_string_lossy().into_owned();
        let raw = self.root.to_string_lossy().into_owned();
        let mut out = text.replace(&canon, "[TMP]");
        if raw != canon {
            out = out.replace(&raw, "[TMP]");
        }
        out
    }

    /// Run `unam` with args; returns (exit code, stdout, stderr).
    pub fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = self.cmd().args(args).output().unwrap();
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

/// Run `unam --json <args>` and parse stdout as one JSON object.
pub fn run_json(env: &TestEnv, args: &[&str]) -> (i32, Value) {
    let mut full = vec!["--json"];
    full.extend_from_slice(args);
    let (code, stdout, stderr) = env.run(&full);
    let v: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
        panic!("stdout is not JSON ({e}): {stdout:?}\nstderr: {stderr}");
    });
    (code, v)
}

/// Diagnostics in a JSON array with the given code.
pub fn diags_with_code<'a>(arr: &'a Value, code: &str) -> Vec<&'a Value> {
    arr.as_array()
        .map(|a| a.iter().filter(|d| d["code"] == code).collect())
        .unwrap_or_default()
}

/// IDs from a `search` JSON result.
pub fn result_ids(v: &Value) -> Vec<String> {
    v["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_str().unwrap().to_string())
        .collect()
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("library")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let ft = entry.file_type().unwrap();
        let dest = to.join(entry.file_name());
        if ft.is_dir() {
            copy_tree(&entry.path(), &dest);
        } else if ft.is_file() {
            fs::copy(entry.path(), &dest).unwrap();
        }
    }
}

pub fn write_file(root: &Path, rel: &str, content: &[u8]) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, content).unwrap();
}

pub fn skill_md(name: &str, description: &str) -> String {
    format!("---\nname: {name}\ndescription: {description}\n---\n\nBody.\n")
}

/// Copy the static fixture library into `work/lib` and add the generated
/// pieces (Fixture Contract). Returns the library path.
pub fn build_library(env: &TestEnv) -> PathBuf {
    let lib = env.path("work").join("lib");
    copy_tree(&fixture_root(), &lib);
    write_file(
        &lib,
        "bom-crlf/windows-skill/SKILL.md",
        "\u{feff}---\r\nname: windows-skill\r\ndescription: A skill saved with a BOM and CRLF line endings.\r\n---\r\n\r\nBody.\r\n"
            .as_bytes(),
    );
    for rel in [
        "node_modules/pkg/SKILL.md",
        "vendor/node_modules/deep/SKILL.md",
        "vendor/some-repo/.git/SKILL.md",
        "sdlc/.hidden-skill/SKILL.md",
    ] {
        write_file(
            &lib,
            rel,
            skill_md("hidden-review", "Must never be discovered; review.").as_bytes(),
        );
    }
    write_file(
        &lib,
        "case/lowercase/skill.md",
        skill_md("lowercase", "Lowercase file name; not a skill.").as_bytes(),
    );
    #[cfg(unix)]
    {
        fs::create_dir_all(lib.join("linked")).unwrap();
        std::os::unix::fs::symlink("../sdlc/code-review", lib.join("linked/code-review-link"))
            .unwrap();
    }
    lib
}

/// `build_library` plus a root `.git/` holding a `SKILL.md` and a config.
pub fn build_library_with_root_git(env: &TestEnv) -> PathBuf {
    let lib = build_library(env);
    write_file(
        &lib,
        ".git/hooks/SKILL.md",
        skill_md("git-hook", "Must never be discovered.").as_bytes(),
    );
    write_file(&lib, ".git/config", b"[core]\n\tbare = false\n");
    lib
}

/// One stat entry: (relative path, kind, size, mtime in nanoseconds).
pub type StatEntry = (String, String, u64, i128);

#[derive(Debug, PartialEq, Eq)]
pub struct SourceSnapshot {
    pub tree_hash: String,
    pub entries: Vec<StatEntry>,
}

fn collect_entries(root: &Path, dir: &Path, out: &mut Vec<StatEntry>) {
    let mut children: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap()).collect();
    children.sort_by_key(|e| e.file_name());
    for entry in children {
        let path = entry.path();
        let meta = fs::symlink_metadata(&path).unwrap();
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let ft = meta.file_type();
        let kind = if ft.is_symlink() {
            "symlink"
        } else if ft.is_dir() {
            "dir"
        } else {
            "file"
        };
        let mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_nanos() as i128)
            .unwrap_or(0);
        let size = if ft.is_dir() { 0 } else { meta.len() };
        out.push((rel, kind.to_string(), size, mtime));
        if ft.is_dir() {
            collect_entries(root, &path, out);
        }
    }
}

/// Tree hash (everything, including `.git`) plus a stat snapshot of every entry.
pub fn snapshot_source(root: &Path) -> SourceSnapshot {
    let tree_hash = unam_core::hash::tree_hash(root, &[]).unwrap().to_string();
    let mut entries = Vec::new();
    collect_entries(root, root, &mut entries);
    SourceSnapshot { tree_hash, entries }
}

/// Assert the source tree is identical to `before`, with a readable diff.
pub fn assert_source_unchanged(before: &SourceSnapshot, root: &Path) {
    let after = snapshot_source(root);
    if *before != after {
        let mut diff = String::new();
        if before.tree_hash != after.tree_hash {
            diff.push_str(&format!(
                "tree hash: {} -> {}\n",
                before.tree_hash, after.tree_hash
            ));
        }
        for e in &before.entries {
            if !after.entries.contains(e) {
                diff.push_str(&format!("- {e:?}\n"));
            }
        }
        for e in &after.entries {
            if !before.entries.contains(e) {
                diff.push_str(&format!("+ {e:?}\n"));
            }
        }
        panic!("source tree changed:\n{diff}");
    }
}

// ---- milestone M2 helpers (plan P17) ----

/// An in-process `Dirs` built from the `TestEnv` variables (never the
/// process environment).
pub fn dirs_for(env: &TestEnv) -> unam_core::paths::Dirs {
    let map: std::collections::HashMap<String, std::ffi::OsString> = env
        .vars()
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_os_string()))
        .collect();
    unam_core::paths::Dirs::resolve(unam_core::paths::Env::from_map(map)).unwrap()
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// Copy the static second library (alias `hs` in tests) into `work/lib2`.
pub fn build_second_library(env: &TestEnv) -> PathBuf {
    let lib = env.path("work").join("lib2");
    copy_tree(&fixtures_dir().join("library2"), &lib);
    lib
}

/// Copy `tests/fixtures/bundles/<name>` into the bundles dir; returns the copy.
pub fn copy_fixture_bundle(env: &TestEnv, name: &str) -> PathBuf {
    let dest = env.path("bundles").join(name);
    fs::create_dir_all(env.path("bundles")).unwrap();
    fs::copy(fixtures_dir().join("bundles").join(name), &dest).unwrap();
    dest
}

/// Write the project index (string-list format) with the given roots.
pub fn write_project_index(env: &TestEnv, roots: &[&Path]) {
    let index = env.project_index();
    fs::create_dir_all(index.parent().unwrap()).unwrap();
    let projects: Vec<String> = roots
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect();
    fs::write(
        index,
        serde_json::json!({"version": 1, "projects": projects}).to_string(),
    )
    .unwrap();
}

/// Plant an empty `.git` directory: a project-root marker.
pub fn git_marker(dir: &Path) {
    fs::create_dir_all(dir.join(".git")).unwrap();
}

/// Like `TestEnv::cmd` with another working directory.
pub fn cmd_in(env: &TestEnv, dir: &Path) -> Command {
    let mut c = env.cmd();
    c.current_dir(dir);
    c
}

/// Run `unam` in `dir`; returns (exit code, stdout, stderr).
pub fn run_in(env: &TestEnv, dir: &Path, args: &[&str]) -> (i32, String, String) {
    let out = cmd_in(env, dir).args(args).output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// A `#!/bin/sh` editor script inside the `TestEnv` (mode 755). The script
/// receives the file to edit as its last argument (`$1` when called alone).
#[cfg(unix)]
pub fn write_scripted_editor(env: &TestEnv, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = env.path("work").join("editors");
    fs::create_dir_all(&dir).unwrap();
    let n = fs::read_dir(&dir).unwrap().count();
    let path = dir.join(format!("editor-{n}.sh"));
    fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    path
}

/// Lines of a file with their terminators preserved.
pub fn read_lines(path: &Path) -> Vec<String> {
    split_lines(&fs::read_to_string(path).unwrap())
}

/// Split text into lines keeping `\n` or `\r\n` on each line.
pub fn split_lines(text: &str) -> Vec<String> {
    text.split_inclusive('\n').map(str::to_string).collect()
}
