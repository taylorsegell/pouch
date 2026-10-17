// Integration helpers may unwrap: allow-unwrap-in-tests does not cover
// non-test helper functions in a shared module.
#![allow(clippy::unwrap_used)]
// Each test binary uses a subset of these helpers.
#![allow(dead_code)]

pub mod faults;

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
    "enterprise",
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
            ("UNAM_ENTERPRISE_SKILLS_DIRS", self.path("enterprise")),
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

/// Copy `hermes-only.yaml` into the bundles dir with the per-bundle brief
/// opt-out appended (`materialize_instruction: false`). The suites that
/// activate this bundle for `claude-code` use it so the default-on bundle
/// brief (M5-9) does not add a `hermes-only-brief` entry to their plans and
/// snapshots (EX1). The fixture file itself stays byte-identical.
pub fn copy_fixture_bundle_no_brief(env: &TestEnv) -> PathBuf {
    let dest = env.path("bundles").join("hermes-only.yaml");
    fs::create_dir_all(env.path("bundles")).unwrap();
    let mut text =
        fs::read_to_string(fixtures_dir().join("bundles").join("hermes-only.yaml")).unwrap();
    text.push_str("metadata:\n  unambiguous:\n    materialize_instruction: false\n");
    fs::write(&dest, text).unwrap();
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

// ---- milestone M3 helpers (plan Test Harness section) ----

/// A planted foreign fixture: every file's bytes at planting time.
#[derive(Debug, Clone)]
pub struct ForeignFixture {
    pub root: PathBuf,
    pub files: Vec<(PathBuf, Vec<u8>)>,
    pub links: Vec<(PathBuf, PathBuf)>,
}

/// The shared M3 fixture: libraries `fx` and `hs` registered, bundle
/// `hermes-only` copied, source snapshots taken. Every mutating test ends
/// with `fx.done()` (PRD S3 and S4).
pub struct Fx {
    pub env: TestEnv,
    pub lib: PathBuf,
    pub lib2: PathBuf,
    pub before: SourceSnapshot,
    pub before2: SourceSnapshot,
    foreign: std::cell::RefCell<Vec<ForeignFixture>>,
}

impl Fx {
    pub fn new() -> Fx {
        Fx::new_with(|_| {})
    }

    /// Build the fixture; `mutate` edits the fresh `fx` library before the
    /// snapshot and the registration.
    pub fn new_with(mutate: impl FnOnce(&Path)) -> Fx {
        let env = TestEnv::new();
        let lib = build_library(&env);
        mutate(&lib);
        let lib2 = build_second_library(&env);
        let before = snapshot_source(&lib);
        let before2 = snapshot_source(&lib2);
        for (alias, l) in [("fx", &lib), ("hs", &lib2)] {
            let (code, v) = run_json(&env, &["source", "add", alias, l.to_str().unwrap()]);
            assert_eq!(code, 0, "{v}");
        }
        copy_fixture_bundle_no_brief(&env);
        Fx {
            env,
            lib,
            lib2,
            before,
            before2,
            foreign: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// A project directory under `work/` with a `.git` marker.
    pub fn project(&self, name: &str) -> PathBuf {
        let p = self.env.path("work").join(name);
        fs::create_dir_all(&p).unwrap();
        git_marker(&p);
        p
    }

    /// The canonical path of a project.
    pub fn canon(&self, p: &Path) -> PathBuf {
        fs::canonicalize(p).unwrap()
    }

    /// Plant a foreign skill directory `.claude/skills/<name>/` (SKILL.md
    /// plus a second file) and remember its bytes.
    pub fn plant_foreign(&self, project: &Path, name: &str) -> ForeignFixture {
        let root = project.join(".claude/skills").join(name);
        write_file(
            &root,
            "SKILL.md",
            skill_md(name, "Not managed by unam.").as_bytes(),
        );
        write_file(&root, "notes.md", b"house rules\n");
        let f = ForeignFixture {
            files: vec![
                (
                    root.join("SKILL.md"),
                    fs::read(root.join("SKILL.md")).unwrap(),
                ),
                (
                    root.join("notes.md"),
                    fs::read(root.join("notes.md")).unwrap(),
                ),
            ],
            links: vec![],
            root,
        };
        self.foreign.borrow_mut().push(f.clone());
        f
    }

    /// Remember an arbitrary foreign file (its current bytes).
    pub fn plant_file(&self, path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
        self.foreign.borrow_mut().push(ForeignFixture {
            root: path.to_path_buf(),
            files: vec![(path.to_path_buf(), bytes.to_vec())],
            links: vec![],
        });
    }

    /// Remember a foreign symlink (its target text).
    #[cfg(unix)]
    pub fn plant_link(&self, link: &Path, target: &Path) {
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, link).unwrap();
        self.foreign.borrow_mut().push(ForeignFixture {
            root: link.to_path_buf(),
            files: vec![],
            links: vec![(link.to_path_buf(), target.to_path_buf())],
        });
    }

    /// Run `unam` in `project`.
    pub fn run(&self, project: &Path, args: &[&str]) -> (i32, String, String) {
        run_in(&self.env, project, args)
    }

    /// Run `unam --json` in `project`; stdout parsed, temp roots, op ids,
    /// and timestamps normalized.
    pub fn json(&self, project: &Path, args: &[&str]) -> (i32, Value) {
        let (code, v) = self.json_raw(project, args);
        let text = normalize_volatile(&self.env.normalize(&v.to_string()));
        (code, serde_json::from_str(&text).unwrap())
    }

    /// Run `unam --json` in `project` without normalization.
    pub fn json_raw(&self, project: &Path, args: &[&str]) -> (i32, Value) {
        let mut full = vec!["--json"];
        full.extend_from_slice(args);
        let (code, stdout, stderr) = run_in(&self.env, project, &full);
        let v: Value = serde_json::from_str(&stdout).unwrap_or_else(|e| {
            panic!("stdout is not one JSON object ({e}): {stdout:?}\nstderr: {stderr}")
        });
        (code, v)
    }

    /// Write bundle `<name>.yaml` with these skill references.
    pub fn write_bundle(&self, name: &str, skills: &[&str]) {
        let mut text = format!("name: {name}\nskills:\n");
        for s in skills {
            text.push_str(&format!("  - \"{s}\"\n"));
        }
        fs::write(self.env.path("bundles").join(format!("{name}.yaml")), text).unwrap();
    }

    /// Write bundle `<name>.yaml` with these skills, an optional
    /// `instruction`, and an optional `materialize_instruction` value.
    pub fn write_bundle_with(
        &self,
        name: &str,
        skills: &[&str],
        instruction: Option<&str>,
        materialize: Option<bool>,
    ) {
        let mut text = format!("name: {name}\nskills:\n");
        for s in skills {
            text.push_str(&format!("  - \"{s}\"\n"));
        }
        if let Some(i) = instruction {
            text.push_str("instruction: |\n");
            for line in i.lines() {
                text.push_str(&format!("  {line}\n"));
            }
        }
        if let Some(m) = materialize {
            text.push_str(&format!(
                "metadata:\n  unambiguous:\n    materialize_instruction: {m}\n"
            ));
        }
        fs::write(self.env.path("bundles").join(format!("{name}.yaml")), text).unwrap();
    }

    /// Write the project's selection file.
    pub fn write_selection(&self, project: &Path, text: &str) {
        write_file(project, ".unam/selection.yaml", text.as_bytes());
    }

    /// `unam status` in `project`, normalized.
    pub fn status(&self, project: &Path) -> (i32, Value) {
        self.json(project, &["status"])
    }

    /// Plant a user-scope skill (under the test `HOME`) with this name: it
    /// shadows a project skill of the same name.
    pub fn plant_user_skill(&self, name: &str) -> PathBuf {
        let dir = self.env.path("home").join(".claude/skills").join(name);
        write_file(
            &dir,
            "SKILL.md",
            skill_md(name, "A user-scope skill.").as_bytes(),
        );
        dir
    }

    /// The two invariants: every source snapshot equals the start and every
    /// planted foreign fixture is byte-identical.
    pub fn done(&self) {
        self.done_at(&self.lib);
    }

    /// `done` for a library that was moved (the deliberate change of AC-37):
    /// the `fx` source now lives at `lib` and must equal the start snapshot.
    pub fn done_at(&self, lib: &Path) {
        self.done_with(lib, &self.before);
    }

    /// `done` after a deliberate edit of the `fx` library (AC-36, AC-39):
    /// the caller retook `snapshot` right after the edit.
    pub fn done_with(&self, lib: &Path, snapshot: &SourceSnapshot) {
        assert_source_unchanged(snapshot, lib);
        assert_source_unchanged(&self.before2, &self.lib2);
        for f in self.foreign.borrow().iter() {
            for (p, bytes) in &f.files {
                let meta = fs::symlink_metadata(p)
                    .unwrap_or_else(|e| panic!("foreign file {} is gone: {e}", p.display()));
                assert!(meta.is_file(), "foreign {} changed kind", p.display());
                assert_eq!(
                    &fs::read(p).unwrap(),
                    bytes,
                    "foreign {} changed",
                    p.display()
                );
            }
            for (l, t) in &f.links {
                assert_eq!(
                    &fs::read_link(l).unwrap(),
                    t,
                    "foreign link {}",
                    l.display()
                );
            }
        }
    }
}

/// The `data.records` of a `status` or `reconcile` JSON result as
/// `(name, state)` pairs.
pub fn record_states(v: &Value) -> Vec<(String, String)> {
    v["data"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["name"].as_str().unwrap().to_string(),
                r["state"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The record named `name` in a `status` or `reconcile` JSON result.
pub fn record<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["data"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap_or_else(|| panic!("no record {name} in {v}"))
}

/// One project entry: (relative path, kind, mode, bytes or link text).
pub type ProjectEntry = (String, String, u32, Vec<u8>);

/// A kind, size, and bytes snapshot of a project tree (mtime and permission
/// bits excluded: a restored file is rewritten atomically as a new file). `.unam/lock` is ignored, and `.unam` itself when it
/// then holds nothing else.
#[derive(Debug, PartialEq, Eq)]
pub struct ProjectSnapshot(pub Vec<ProjectEntry>);

fn mode_of(meta: &fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode()
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0
    }
}

pub fn snapshot_project(root: &Path) -> ProjectSnapshot {
    let mut out: Vec<ProjectEntry> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if rel == ".unam/lock" {
                continue;
            }
            let meta = fs::symlink_metadata(&p).unwrap();
            let ft = meta.file_type();
            if ft.is_symlink() {
                let t = fs::read_link(&p).unwrap();
                out.push((
                    rel,
                    "link".into(),
                    0,
                    t.to_string_lossy().as_bytes().to_vec(),
                ));
            } else if ft.is_dir() {
                out.push((rel, "dir".into(), 0, Vec::new()));
                stack.push(p);
            } else {
                out.push((rel, "file".into(), 0, fs::read(&p).unwrap()));
            }
        }
    }
    // `.unam` holding only the lock counts as absent.
    let only_lock = out.iter().all(|(r, _, _, _)| !r.starts_with(".unam/"));
    if only_lock {
        out.retain(|(r, _, _, _)| r != ".unam");
    }
    out.sort();
    ProjectSnapshot(out)
}

/// Assert the project tree equals `before`, with a readable diff.
pub fn assert_project_unchanged(before: &ProjectSnapshot, root: &Path) {
    let after = snapshot_project(root);
    if *before != after {
        let names = |s: &ProjectSnapshot| -> Vec<String> {
            s.0.iter()
                .map(|(r, k, m, b)| format!("{r} {k} {m:o} {}", String::from_utf8_lossy(b)))
                .collect()
        };
        let (b, a) = (names(before), names(&after));
        let mut diff = String::new();
        for x in &b {
            if !a.contains(x) {
                diff.push_str(&format!("- {x}\n"));
            }
        }
        for x in &a {
            if !b.contains(x) {
                diff.push_str(&format!("+ {x}\n"));
            }
        }
        panic!("project tree changed:\n{diff}");
    }
}

/// The receipt `.unam/<target>/applied.json` as JSON.
pub fn read_receipt(project: &Path, target: &str) -> Value {
    let p = project.join(".unam").join(target).join("applied.json");
    serde_json::from_slice(&fs::read(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .unwrap()
}

/// Each skill entry of the receipt re-serialized compactly.
pub fn receipt_entry_texts(project: &Path, target: &str) -> Vec<String> {
    read_receipt(project, target)["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "skill")
        .map(|e| e.to_string())
        .collect()
}

/// The inode of the entry itself (never followed).
#[cfg(unix)]
pub fn ino(path: &Path) -> u64 {
    use std::os::unix::fs::MetadataExt;
    fs::symlink_metadata(path).unwrap().ino()
}

fn is_hex_lower(b: u8) -> bool {
    b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
}

/// Replace operation ids (`\d{13}-[0-9a-f]{8}`) with `[OP_ID]` and RFC 3339
/// timestamps with `[TIME]` (manual scanning; no regex crate).
pub fn normalize_volatile(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    let digits = |from: usize, n: usize| {
        from + n <= b.len() && b[from..from + n].iter().all(u8::is_ascii_digit)
    };
    while i < b.len() {
        let boundary = i == 0 || !b[i - 1].is_ascii_alphanumeric();
        // Operation id.
        if boundary
            && digits(i, 13)
            && i + 22 <= b.len()
            && b[i + 13] == b'-'
            && b[i + 14..i + 22].iter().all(|c| is_hex_lower(*c))
            && (i + 22 == b.len() || !b[i + 22].is_ascii_alphanumeric())
        {
            out.extend_from_slice(b"[OP_ID]");
            i += 22;
            continue;
        }
        // RFC 3339: YYYY-MM-DDTHH:MM:SS(.fraction)?(Z|+HH:MM)
        if boundary
            && digits(i, 4)
            && i + 19 <= b.len()
            && b[i + 4] == b'-'
            && digits(i + 5, 2)
            && b[i + 7] == b'-'
            && digits(i + 8, 2)
            && b[i + 10] == b'T'
            && digits(i + 11, 2)
            && b[i + 13] == b':'
            && digits(i + 14, 2)
            && b[i + 16] == b':'
            && digits(i + 17, 2)
        {
            let mut j = i + 19;
            if j < b.len() && b[j] == b'.' {
                j += 1;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
            }
            if j < b.len() && b[j] == b'Z' {
                j += 1;
            } else if j + 6 <= b.len()
                && (b[j] == b'+' || b[j] == b'-')
                && digits(j + 1, 2)
                && b[j + 3] == b':'
                && digits(j + 4, 2)
            {
                j += 6;
            }
            out.extend_from_slice(b"[TIME]");
            i = j;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap()
}

/// Counts `unam` invocations (PRD S2).
#[derive(Default)]
pub struct Invocations(std::cell::Cell<u32>);

impl Invocations {
    pub fn run_json(&self, fx: &Fx, project: &Path, args: &[&str]) -> (i32, Value) {
        self.0.set(self.0.get() + 1);
        fx.json_raw(project, args)
    }

    pub fn count(&self) -> u32 {
        self.0.get()
    }
}
