//! Unit tests of `fsops` (M3 plan, the hostile matrix). Tempdirs only.

use super::*;
use std::fs;

fn tmp() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

fn canon(p: &Path) -> PathBuf {
    fs::canonicalize(p).unwrap()
}

fn write(p: &Path, text: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A skill directory `<lib>/<rel>` with nested files; returns its path.
fn skill(lib: &Path, rel: &str) -> PathBuf {
    let d = lib.join(rel);
    write(&d.join("SKILL.md"), "---\nname: s\ndescription: d\n---\n");
    write(&d.join("references/a.md"), "a\n");
    write(&d.join("scripts/run.sh"), "#!/bin/sh\necho hi\n");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(d.join("scripts/run.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    }
    d
}

fn hash_of(p: &Path) -> Hash {
    tree_hash(p, SKILL_SKIP_NAMES).unwrap()
}

fn src(p: &Path) -> SourcePath {
    SourcePath::new(canon(p))
}

fn root_in(project: &Path) -> DestPath {
    DestPath::dest_root(project, ".claude/skills").unwrap()
}

fn copy_expect(src: &SourcePath, h: &Hash) -> OwnedExpectation {
    OwnedExpectation {
        strategy: Strategy::Copy,
        source: src.as_path().to_string_lossy().into_owned(),
        tree_hash: Some(h.clone()),
    }
}

fn link_expect(src: &SourcePath) -> OwnedExpectation {
    OwnedExpectation {
        strategy: Strategy::Symlink,
        source: src.as_path().to_string_lossy().into_owned(),
        tree_hash: None,
    }
}

// ---- typed paths ----

#[test]
fn join_child_accepts_and_rejects_names() {
    let t = tmp();
    let root = root_in(t.path());
    let c = root.join_child("code-review").unwrap();
    assert_eq!(c.path(), t.path().join(".claude/skills/code-review"));
    assert_eq!(c.rel(), ".claude/skills/code-review");
    assert_eq!(c.name(), Some("code-review"));
    let long = "a".repeat(256);
    for bad in [
        "..",
        ".",
        "a/b",
        "a\0b",
        "a\nb",
        long.as_str(),
        "   ",
        ".unam-tmp-x",
        "/abs",
        "",
        "a\\b",
    ] {
        assert!(root.join_child(bad).is_err(), "{bad:?}");
    }
    assert!(root.join_child(&"a".repeat(255)).is_ok());
    assert!(c.join_child("x").is_err(), "a child has no children");
}

#[test]
fn dest_root_rejects_bad_rel_and_ancestors_list_prefixes() {
    let t = tmp();
    for bad in ["/abs", "a/../b", "", "a//b", "./a", "a/b/c/d/e/f/g/h/i"] {
        assert!(DestPath::dest_root(t.path(), bad).is_err(), "{bad:?}");
    }
    let root = root_in(t.path());
    let anc = root.ancestors();
    let rels: Vec<&str> = anc.iter().map(|(r, _)| r.as_str()).collect();
    assert_eq!(rels, vec![".claude", ".claude/skills"]);
    assert_eq!(anc[0].1.path(), t.path().join(".claude"));
    assert_eq!(anc[1].1.path(), t.path().join(".claude/skills"));
    assert!(anc[1].1.join_child("x").is_err());
    assert_eq!(
        dest_prefixes(".claude/skills"),
        vec![".claude".to_string(), ".claude/skills".to_string()]
    );
    assert!(dest_prefixes("../x").is_empty());
}

#[test]
fn validate_dest_name_rules() {
    assert!(validate_dest_name("code-review").is_ok());
    assert!(validate_dest_name("caf\u{e9}").is_ok());
    assert!(validate_dest_name("a\u{7f}b").is_err());
    assert!(validate_dest_name("a\u{1b}b").is_err());
}

// ---- write_atomic, lock file ----

#[test]
fn creates_parents_overwrites_and_leaves_no_temp() {
    let dir = tmp();
    let target = ToolPath::new(dir.path().join("a/b/c.txt"));
    write_atomic(&target, b"one").unwrap();
    assert_eq!(fs::read(target.path()).unwrap(), b"one");
    write_atomic(&target, b"two").unwrap();
    assert_eq!(fs::read(target.path()).unwrap(), b"two");
    let names: Vec<_> = fs::read_dir(dir.path().join("a/b"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![std::ffi::OsString::from("c.txt")]);
}

#[test]
fn fails_when_parent_is_a_file() {
    let dir = tmp();
    fs::write(dir.path().join("f"), b"x").unwrap();
    let err = write_atomic(&ToolPath::new(dir.path().join("f/x.json")), b"y").unwrap_err();
    assert_eq!(err.code(), "io-error");
}

#[test]
fn write_atomic_fsyncs_the_parent_directory() {
    let dir = tmp();
    let p = StatePath::new(dir.path().join("s/x.json"));
    write_atomic(&p, b"{}").unwrap();
    assert!(sync_dir(&dir.path().join("s")).is_ok());
}

#[test]
fn open_lock_file_creates_parents_and_does_not_truncate() {
    let t = tmp();
    let p = StatePath::new(t.path().join(".unam/lock"));
    drop(open_lock_file(&p).unwrap());
    assert!(p.path().is_file());
    fs::write(p.path(), b"content").unwrap();
    let f = open_lock_file(&p).unwrap();
    verify_lock_identity(&f, &p).unwrap();
    assert_eq!(fs::read(p.path()).unwrap(), b"content");
}

#[cfg(unix)]
#[test]
fn open_lock_file_refuses_symlinks_and_wrong_kinds() {
    let t = tmp();
    let victim = t.path().join("victim.txt");
    fs::write(&victim, b"precious").unwrap();
    let outside = t.path().join("outside");
    fs::create_dir_all(&outside).unwrap();

    // Symlink to an existing file.
    let a = t.path().join("a/.unam");
    fs::create_dir_all(&a).unwrap();
    std::os::unix::fs::symlink(&victim, a.join("lock")).unwrap();
    let e = open_lock_file(&StatePath::new(a.join("lock"))).unwrap_err();
    assert_eq!(e.code(), "state-dir-invalid");
    assert_eq!(fs::read(&victim).unwrap(), b"precious");

    // Dangling symlink: the outside path is not created.
    let b = t.path().join("b/.unam");
    fs::create_dir_all(&b).unwrap();
    std::os::unix::fs::symlink(outside.join("new-file"), b.join("lock")).unwrap();
    let e = open_lock_file(&StatePath::new(b.join("lock"))).unwrap_err();
    assert_eq!(e.code(), "state-dir-invalid");
    assert!(!outside.join("new-file").exists());

    // A directory at the lock path.
    let c = t.path().join("c/.unam/lock");
    fs::create_dir_all(&c).unwrap();
    let e = open_lock_file(&StatePath::new(c)).unwrap_err();
    assert_eq!(e.code(), "state-dir-invalid");

    // A lock path under a symlinked `.unam`.
    let d = t.path().join("d");
    fs::create_dir_all(&d).unwrap();
    std::os::unix::fs::symlink(&outside, d.join(".unam")).unwrap();
    let e = open_lock_file(&StatePath::new(d.join(".unam/lock"))).unwrap_err();
    assert_eq!(e.code(), "state-dir-invalid");
    assert_eq!(fs::read_dir(&outside).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn verify_lock_identity_refuses_a_replaced_file() {
    let t = tmp();
    let p = StatePath::new(t.path().join(".unam/lock"));
    let f = open_lock_file(&p).unwrap();
    verify_lock_identity(&f, &p).unwrap();
    let other = t.path().join(".unam/other");
    fs::write(&other, b"x").unwrap();
    fs::rename(&other, p.path()).unwrap();
    let e = verify_lock_identity(&f, &p).unwrap_err();
    assert_eq!(e.code(), "state-dir-invalid");
}

// ---- structural guards ----

/// Production text of a source file: everything before the first
/// `#[cfg(test)]`.
fn production_text(text: &str) -> &str {
    match text.find("#[cfg(test)]") {
        Some(i) => &text[..i],
        None => text,
    }
}

/// `(fn name, parameter list text, body start line)` for each `fn`.
fn functions(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("fn ") {
        let before_ok = i == 0
            || !rest[..i]
                .chars()
                .last()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
        let after = &rest[i + 3..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if before_ok && !name.is_empty() {
            let tail = &after[name.len()..];
            let tail = match tail.find('(') {
                Some(open)
                    if tail[..open].trim().is_empty() || tail[..open].trim().starts_with('<') =>
                {
                    &tail[open..]
                }
                _ => {
                    rest = &rest[i + 3..];
                    continue;
                }
            };
            let mut depth = 0;
            let mut end = 0;
            for (k, ch) in tail.char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = k;
                            break;
                        }
                    }
                    _ => {}
                }
            }
            out.push((name, tail[..=end].to_string()));
        }
        rest = &rest[i + 3..];
    }
    out
}

#[test]
fn source_path_never_reaches_a_write_function() {
    let text = production_text(include_str!("../fsops.rs"));
    let impls: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("impl Writable for"))
        .collect();
    assert_eq!(impls.len(), 3, "{impls:?}");
    assert!(impls.iter().all(|l| !l.contains("SourcePath")));
    for (name, params) in functions(text) {
        if params.contains("SourcePath") {
            assert!(
                ["symlink_dir", "copy_tree", "restore_owned"].contains(&name.as_str()),
                "{name} takes a SourcePath"
            );
        }
    }
    for line in text.lines() {
        assert!(
            !(line.contains("SourcePath") && line.contains("write_atomic")),
            "{line}"
        );
    }
    // `SourcePath::as_path` (crate-visible) is used only to read the
    // source in the three functions above.
    let uses = text.matches(".as_path()").count();
    assert_eq!(uses, 4, "as_path calls in fsops");
}

#[test]
fn remove_dir_all_is_confined() {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits: Vec<(PathBuf, usize)> = Vec::new();
    let mut stack = vec![src.clone()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x == "rs") {
                // A file that is entirely a test module has no production text.
                if p.file_name().is_some_and(|n| n == "tests.rs") {
                    continue;
                }
                let text = fs::read_to_string(&p).unwrap();
                let prod = production_text(&text);
                for (i, line) in prod.lines().enumerate() {
                    if line.contains("remove_dir_all") {
                        hits.push((p.clone(), i));
                    }
                }
            }
        }
    }
    assert_eq!(hits.len(), 1, "{hits:?}");
    let (file, line) = &hits[0];
    assert!(file.ends_with("fsops.rs"), "{}", file.display());
    let text = fs::read_to_string(file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.contains("fn remove_own_temp_dir("))
        .unwrap();
    let end = start
        + lines[start..]
            .iter()
            .position(|l| l.starts_with('}'))
            .unwrap();
    assert!(
        *line > start && *line < end,
        "line {line} not in {start}..{end}"
    );
}

// ---- symlink_dir ----

#[cfg(unix)]
#[test]
fn symlink_dir_creates_absolute_link_and_never_overwrites() {
    let t = tmp();
    let lib = t.path().join("lib");
    let s = src(&skill(&lib, "cat/one"));
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let dst = root.join_child("one").unwrap();
    RealFs.symlink_dir(&s, &dst).unwrap();
    assert_eq!(fs::read_link(dst.path()).unwrap(), s.as_path());
    assert!(dst.path().join("SKILL.md").is_file());
    // An existing link, file, or directory is never overwritten.
    assert!(RealFs.symlink_dir(&s, &dst).is_err());
    let f = root.join_child("file").unwrap();
    fs::write(f.path(), b"mine").unwrap();
    assert!(RealFs.symlink_dir(&s, &f).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"mine");
    let d = root.join_child("dir").unwrap();
    fs::create_dir_all(d.path()).unwrap();
    assert!(RealFs.symlink_dir(&s, &d).is_err());
    assert!(fs::symlink_metadata(d.path()).unwrap().is_dir());
    // A relative source is refused.
    let rel = SourcePath::new(PathBuf::from("lib/cat/one"));
    let r = root.join_child("rel").unwrap();
    assert!(RealFs.symlink_dir(&rel, &r).is_err());
    assert!(fs::symlink_metadata(r.path()).is_err());
}

// ---- copy_tree ----

fn listing(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            out.push(p.strip_prefix(dir).unwrap().to_string_lossy().into_owned());
            if fs::symlink_metadata(&p).unwrap().is_dir() {
                stack.push(p);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn copy_tree_copies_nested_files_and_hash_matches() {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    write(&sdir.join(".git/config"), "x");
    write(&sdir.join(".DS_Store"), "x");
    let h = hash_of(&sdir);
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let dst = root.join_child("one").unwrap();
    let got = RealFs
        .copy_tree(&src(&sdir), &dst, "0000000000001-0000abcd", &h)
        .unwrap();
    assert_eq!(got, h);
    assert_eq!(hash_of(dst.path()), h);
    assert!(dst.path().join("references/a.md").is_file());
    assert!(!dst.path().join(".git").exists());
    assert!(!dst.path().join(".DS_Store").exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dst.path().join("scripts/run.sh"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }
    // No temp left behind: the root holds exactly the copy.
    let names: Vec<String> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, vec!["one".to_string()]);
    assert!(listing(dst.path()).contains(&"references/a.md".to_string()));
    assert!(fs::read_dir(root.path()).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(TEMP_PREFIX)
    }));
}

#[cfg(unix)]
#[test]
fn copy_tree_recreates_an_intra_skill_link() {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    std::os::unix::fs::symlink("../references/a.md", sdir.join("scripts/a-link")).unwrap();
    let h = hash_of(&sdir);
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let dst = root.join_child("one").unwrap();
    RealFs.copy_tree(&src(&sdir), &dst, "tag", &h).unwrap();
    assert_eq!(
        fs::read_link(dst.path().join("scripts/a-link")).unwrap(),
        PathBuf::from("../references/a.md")
    );
}

#[cfg(unix)]
#[test]
fn copy_tree_refuses_escaping_links() {
    let t = tmp();
    let lib = t.path().join("lib");
    let outside = t.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    let cases: Vec<(&str, PathBuf)> = vec![
        ("abs", PathBuf::from("/etc")),
        ("dotdot", PathBuf::from("../../other")),
        ("chain", PathBuf::from("hop")),
    ];
    for (name, target) in cases {
        let sdir = skill(&lib, &format!("cat/{name}"));
        if name == "chain" {
            // `hop` stays inside lexically but resolves outside.
            std::os::unix::fs::symlink(&outside, sdir.join("hop")).ok();
        }
        std::os::unix::fs::symlink(&target, sdir.join("leak")).unwrap();
        let root = root_in(&t.path().join(format!("p-{name}")));
        fs::create_dir_all(root.path()).unwrap();
        let dst = root.join_child(name).unwrap();
        let e = RealFs
            .copy_tree(&src(&sdir), &dst, "tag", &hash_of(&sdir))
            .unwrap_err();
        assert_eq!(e.code(), "copy-refused", "{name}");
        assert!(fs::symlink_metadata(dst.path()).is_err(), "{name}");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0, "{name}");
    }
}

#[cfg(unix)]
#[test]
fn copy_tree_refuses_a_fifo() {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    let ok = std::process::Command::new("mkfifo")
        .arg(sdir.join("pipe"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        println!("self-skip: mkfifo is not available (named residual)");
        return;
    }
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let dst = root.join_child("one").unwrap();
    let e = RealFs
        .copy_tree(&src(&sdir), &dst, "tag", &hash_of(&sdir))
        .unwrap_err();
    assert_eq!(e.code(), "copy-refused");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn copy_tree_refuses_an_existing_destination_and_a_stale_temp() {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    let h = hash_of(&sdir);
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let f = root.join_child("f").unwrap();
    fs::write(f.path(), b"mine").unwrap();
    assert!(RealFs.copy_tree(&src(&sdir), &f, "tag", &h).is_err());
    assert_eq!(fs::read(f.path()).unwrap(), b"mine");
    let d = root.join_child("d").unwrap();
    fs::create_dir_all(d.path()).unwrap();
    assert!(RealFs.copy_tree(&src(&sdir), &d, "tag", &h).is_err());
    assert_eq!(fs::read_dir(d.path()).unwrap().count(), 0);
    let l = root.join_child("l").unwrap();
    std::os::unix::fs::symlink("/nonexistent", l.path()).unwrap();
    assert!(RealFs.copy_tree(&src(&sdir), &l, "tag", &h).is_err());
    // A stale temp path is refused and kept.
    let n = root.join_child("n").unwrap();
    let stale = root.path().join(".unam-tmp-tag-n");
    fs::create_dir_all(&stale).unwrap();
    assert!(RealFs.copy_tree(&src(&sdir), &n, "tag", &h).is_err());
    assert!(stale.is_dir());
    assert!(fs::symlink_metadata(n.path()).is_err());
    let names: Vec<String> = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), 4, "{names:?}");
}

#[test]
fn copy_tree_source_changed_leaves_no_temp_and_uses_the_temp_name() {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    let stale_hash = Hash::of_bytes(b"catalog hash before the edit");
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let dst = root.join_child("one").unwrap();
    let e = RealFs
        .copy_tree(&src(&sdir), &dst, "0000000000001-0000abcd", &stale_hash)
        .unwrap_err();
    assert_eq!(e.code(), "source-changed");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    // The temp name pattern (a stale temp with exactly this name is refused).
    fs::create_dir_all(root.path().join(".unam-tmp-0000000000001-0000abcd-one")).unwrap();
    assert!(
        RealFs
            .copy_tree(&src(&sdir), &dst, "0000000000001-0000abcd", &hash_of(&sdir))
            .is_err()
    );
    // remove_stale_temp_dirs removes only this operation's temps.
    fs::create_dir_all(root.path().join(".unam-tmp-0000000000002-0000abcd-one")).unwrap();
    fs::write(
        root.path().join(".unam-tmp-0000000000001-0000abcd-file"),
        b"x",
    )
    .unwrap();
    assert_eq!(
        remove_stale_temp_dirs(&root, "0000000000001-0000abcd").unwrap(),
        1
    );
    assert!(
        root.path()
            .join(".unam-tmp-0000000000002-0000abcd-one")
            .is_dir()
    );
    assert!(
        root.path()
            .join(".unam-tmp-0000000000001-0000abcd-file")
            .is_file()
    );
}

// ---- remove_owned: symlink ----

#[cfg(unix)]
#[test]
fn remove_owned_symlink_rows() {
    let t = tmp();
    let lib = t.path().join("lib");
    let s = src(&skill(&lib, "cat/one"));
    let before = hash_of(s.as_path());
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let trash = StatePath::new(t.path().join(".unam/claude-code/trash/op"));
    let e = link_expect(&s);

    // Matching link removed; target untouched.
    let a = root.join_child("a").unwrap();
    RealFs.symlink_dir(&s, &a).unwrap();
    assert_eq!(
        RealFs.remove_owned(&a, &e, &trash).unwrap(),
        RemoveOutcome::Removed
    );
    assert!(fs::symlink_metadata(a.path()).is_err());
    assert_eq!(hash_of(s.as_path()), before);

    // Retargeted link: Modified, kept.
    let b = root.join_child("b").unwrap();
    std::os::unix::fs::symlink(&lib, b.path()).unwrap();
    assert_eq!(
        RealFs.remove_owned(&b, &e, &trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Modified)
    );
    assert!(fs::symlink_metadata(b.path()).is_ok());

    // A real directory where a link was expected: Foreign.
    let c = root.join_child("c").unwrap();
    fs::create_dir_all(c.path()).unwrap();
    assert_eq!(
        RealFs.remove_owned(&c, &e, &trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Foreign)
    );
    assert!(c.path().is_dir());

    // Missing.
    let d = root.join_child("d").unwrap();
    assert_eq!(
        RealFs.remove_owned(&d, &e, &trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Missing)
    );

    // Dangling but matching link is removed.
    let gone = t.path().join("gone-skill");
    let ge = OwnedExpectation {
        strategy: Strategy::Symlink,
        source: gone.to_string_lossy().into_owned(),
        tree_hash: None,
    };
    let g = root.join_child("g").unwrap();
    std::os::unix::fs::symlink(&gone, g.path()).unwrap();
    assert_eq!(
        RealFs.remove_owned(&g, &ge, &trash).unwrap(),
        RemoveOutcome::Removed
    );
    assert!(!trash.path().exists());
}

// ---- remove_owned: copy ----

struct CopyFx {
    _t: tempfile::TempDir,
    root: DestPath,
    trash: StatePath,
    s: SourcePath,
    h: Hash,
}

fn copy_fx() -> CopyFx {
    let t = tmp();
    let lib = t.path().join("lib");
    let sdir = skill(&lib, "cat/one");
    let h = hash_of(&sdir);
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let trash = StatePath::new(t.path().join(".unam/claude-code/trash/op"));
    CopyFx {
        root,
        trash,
        s: src(&sdir),
        h,
        _t: t,
    }
}

impl CopyFx {
    fn copy(&self, name: &str) -> DestPath {
        let d = self.root.join_child(name).unwrap();
        RealFs.copy_tree(&self.s, &d, "tag", &self.h).unwrap();
        d
    }
    fn expect(&self) -> OwnedExpectation {
        copy_expect(&self.s, &self.h)
    }
}

#[test]
fn remove_owned_copy_matching_is_removed_and_trash_left_absent() {
    let f = copy_fx();
    let d = f.copy("one");
    assert_eq!(
        RealFs.remove_owned(&d, &f.expect(), &f.trash).unwrap(),
        RemoveOutcome::Removed
    );
    assert!(fs::symlink_metadata(d.path()).is_err());
    assert!(!f.trash.path().exists());
}

#[test]
fn remove_owned_copy_edited_or_extra_file_is_modified_and_untouched() {
    let f = copy_fx();
    let d = f.copy("one");
    fs::write(d.path().join("references/a.md"), b"edited").unwrap();
    assert_eq!(
        RealFs.remove_owned(&d, &f.expect(), &f.trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Modified)
    );
    assert_eq!(
        fs::read(d.path().join("references/a.md")).unwrap(),
        b"edited"
    );
    let e = f.copy("two");
    fs::write(e.path().join("extra.md"), b"mine").unwrap();
    assert_eq!(
        RealFs.remove_owned(&e, &f.expect(), &f.trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Modified)
    );
    assert!(e.path().join("extra.md").is_file());
}

#[cfg(unix)]
#[test]
fn remove_owned_copy_symlink_where_directory_expected_is_foreign() {
    let f = copy_fx();
    let d = f.root.join_child("one").unwrap();
    std::os::unix::fs::symlink(f.s.as_path(), d.path()).unwrap();
    assert_eq!(
        RealFs.remove_owned(&d, &f.expect(), &f.trash).unwrap(),
        RemoveOutcome::Skipped(SkipReason::Foreign)
    );
    assert!(
        fs::symlink_metadata(d.path())
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn remove_owned_copy_git_directory_is_residue_in_trash() {
    let f = copy_fx();
    let d = f.copy("one");
    write(&d.path().join(".git/config"), "keep me");
    let out = RealFs.remove_owned(&d, &f.expect(), &f.trash).unwrap();
    let RemoveOutcome::RemovedWithResidue { trash } = out else {
        panic!("{out:?}");
    };
    assert!(fs::symlink_metadata(d.path()).is_err());
    assert_eq!(fs::read(trash.join(".git/config")).unwrap(), b"keep me");
    assert!(!trash.join("SKILL.md").exists());
}

#[cfg(unix)]
#[test]
fn remove_owned_copy_ds_store_rules() {
    // A regular `.DS_Store` file is unlinked.
    let f = copy_fx();
    let d = f.copy("one");
    fs::write(d.path().join(".DS_Store"), b"x").unwrap();
    fs::write(d.path().join("references/.DS_Store"), b"x").unwrap();
    assert_eq!(
        RealFs.remove_owned(&d, &f.expect(), &f.trash).unwrap(),
        RemoveOutcome::Removed
    );
    // A `.DS_Store` directory with content survives in trash.
    let e = f.copy("two");
    write(&e.path().join(".DS_Store/inner.txt"), "keep");
    let out = RealFs.remove_owned(&e, &f.expect(), &f.trash).unwrap();
    let RemoveOutcome::RemovedWithResidue { trash } = out else {
        panic!("{out:?}");
    };
    assert_eq!(
        fs::read(trash.join(".DS_Store/inner.txt")).unwrap(),
        b"keep"
    );
    // A `.DS_Store` symlink is residue too.
    let g = f.copy("three");
    std::os::unix::fs::symlink("/nonexistent", g.path().join(".DS_Store")).unwrap();
    let out = RealFs.remove_owned(&g, &f.expect(), &f.trash).unwrap();
    let RemoveOutcome::RemovedWithResidue { trash } = out else {
        panic!("{out:?}");
    };
    assert!(
        fs::symlink_metadata(trash.join(".DS_Store"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

#[test]
fn remove_owned_copy_changed_between_check_and_rename_moves_back() {
    let f = copy_fx();
    let d = f.copy("one");
    let target = d.path().join("references/a.md");
    let out = remove_owned_at(d.path(), &f.expect(), f.trash.path(), &|| {
        fs::write(&target, b"edited in the window").unwrap();
    })
    .unwrap();
    assert_eq!(out, RemoveOutcome::Skipped(SkipReason::Modified));
    assert_eq!(fs::read(&target).unwrap(), b"edited in the window");
    assert!(!f.trash.path().exists());
}

#[test]
fn cross_device_rename_maps_to_a_skip() {
    assert_eq!(
        rename_aside_skip(ErrorKind::CrossesDevices),
        Some(SkipReason::CrossDevice)
    );
    assert_eq!(rename_aside_skip(ErrorKind::PermissionDenied), None);
    assert_eq!(SkipReason::CrossDevice.code(), "cross-device");
    for r in [
        SkipReason::Modified,
        SkipReason::Foreign,
        SkipReason::Missing,
        SkipReason::CrossDevice,
        SkipReason::DestOccupied,
        SkipReason::SourceChanged,
    ] {
        assert_eq!(SkipReason::parse(r.code()), Some(r));
    }
}

// ---- rmdir_if_empty, create_dir ----

#[cfg(unix)]
#[test]
fn rmdir_if_empty_rows() {
    let t = tmp();
    let root = root_in(t.path());
    let anc = root.ancestors();
    fs::create_dir_all(root.path()).unwrap();
    // Non-empty parent kept, empty root removed.
    assert!(!RealFs.rmdir_if_empty(&anc[0].1).unwrap());
    assert!(RealFs.rmdir_if_empty(&anc[1].1).unwrap());
    assert!(RealFs.rmdir_if_empty(&anc[0].1).unwrap());
    assert!(!RealFs.rmdir_if_empty(&anc[0].1).unwrap());
    // A symlink is never followed or removed.
    let elsewhere = t.path().join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    std::os::unix::fs::symlink(&elsewhere, t.path().join(".claude")).unwrap();
    assert!(!RealFs.rmdir_if_empty(&anc[0].1).unwrap());
    assert!(elsewhere.is_dir());
    assert!(fs::symlink_metadata(t.path().join(".claude")).is_ok());
}

#[test]
fn create_dir_rows() {
    let t = tmp();
    let root = root_in(t.path());
    let anc = root.ancestors();
    assert!(RealFs.create_dir(&anc[0].1).unwrap());
    assert!(RealFs.create_dir(&anc[1].1).unwrap());
    assert!(!RealFs.create_dir(&anc[1].1).unwrap());
    fs::remove_dir(root.path()).unwrap();
    fs::write(root.path(), b"file").unwrap();
    assert!(RealFs.create_dir(&anc[1].1).is_err());
}

// ---- restore_owned ----

#[test]
fn restore_owned_from_trash_after_a_hash_check() {
    let f = copy_fx();
    let d = f.copy("one");
    // Move aside by hand as a removal would.
    fs::create_dir_all(f.trash.path()).unwrap();
    fs::rename(d.path(), f.trash.path().join("one")).unwrap();
    assert_eq!(
        RealFs
            .restore_owned(&d, &f.expect(), &f.trash, None, "tag")
            .unwrap(),
        RestoreOutcome::Restored
    );
    assert_eq!(hash_of(d.path()), f.h);
}

#[test]
fn restore_owned_refuses_a_changed_trash_copy_and_an_occupied_destination() {
    let f = copy_fx();
    let d = f.copy("one");
    fs::create_dir_all(f.trash.path()).unwrap();
    fs::rename(d.path(), f.trash.path().join("one")).unwrap();
    fs::write(f.trash.path().join("one/extra"), b"x").unwrap();
    assert_eq!(
        RealFs
            .restore_owned(&d, &f.expect(), &f.trash, None, "tag")
            .unwrap(),
        RestoreOutcome::NotRestored("content-not-recoverable".into())
    );
    assert!(fs::symlink_metadata(d.path()).is_err());
    // Recreated from the given source instead.
    assert_eq!(
        RealFs
            .restore_owned(&d, &f.expect(), &f.trash, Some(&f.s), "tag")
            .unwrap(),
        RestoreOutcome::Restored
    );
    // Occupied now.
    assert_eq!(
        RealFs
            .restore_owned(&d, &f.expect(), &f.trash, Some(&f.s), "tag")
            .unwrap(),
        RestoreOutcome::NotRestored("dest-occupied".into())
    );
}

#[cfg(unix)]
#[test]
fn restore_owned_symlink_from_source_or_not_at_all() {
    let f = copy_fx();
    let d = f.root.join_child("lnk").unwrap();
    let e = link_expect(&f.s);
    assert_eq!(
        RealFs.restore_owned(&d, &e, &f.trash, None, "tag").unwrap(),
        RestoreOutcome::NotRestored("source-unavailable".into())
    );
    assert_eq!(
        RealFs
            .restore_owned(&d, &e, &f.trash, Some(&f.s), "tag")
            .unwrap(),
        RestoreOutcome::Restored
    );
    assert_eq!(fs::read_link(d.path()).unwrap(), f.s.as_path());
}

// ---- state operations and observation ----

#[test]
fn state_operations() {
    let t = tmp();
    let p = StatePath::new(t.path().join("s/j.jsonl"));
    RealFs.write_state(&p, b"a\n").unwrap();
    RealFs.append_state(&p, b"b\n").unwrap();
    assert_eq!(fs::read(p.path()).unwrap(), b"a\nb\n");
    RealFs.remove_state(&p).unwrap();
    RealFs.remove_state(&p).unwrap();
    assert!(!p.path().exists());
    let dir = StatePath::new(t.path().join("s"));
    fs::write(t.path().join("s/keep"), b"x").unwrap();
    assert!(!RealFs.remove_state_dir_if_empty(&dir).unwrap());
    fs::remove_file(t.path().join("s/keep")).unwrap();
    assert!(RealFs.remove_state_dir_if_empty(&dir).unwrap());
    assert!(!RealFs.remove_state_dir_if_empty(&dir).unwrap());
}

#[cfg(unix)]
#[test]
fn observe_one_kinds() {
    let t = tmp();
    let root = root_in(t.path());
    fs::create_dir_all(root.path()).unwrap();
    let c = |n: &str| root.join_child(n).unwrap();
    assert_eq!(
        observe_one(&c("none"), false).unwrap(),
        ObservedKind::Missing
    );
    std::os::unix::fs::symlink(t.path(), c("ok").path()).unwrap();
    assert_eq!(
        observe_one(&c("ok"), false).unwrap(),
        ObservedKind::Symlink {
            target: t.path().to_string_lossy().into_owned(),
            resolves: true
        }
    );
    std::os::unix::fs::symlink("/nonexistent/x", c("dangling").path()).unwrap();
    assert!(matches!(
        observe_one(&c("dangling"), false).unwrap(),
        ObservedKind::Symlink {
            resolves: false,
            ..
        }
    ));
    fs::create_dir_all(c("dir").path()).unwrap();
    assert_eq!(
        observe_one(&c("dir"), false).unwrap(),
        ObservedKind::Dir { tree_hash: None }
    );
    assert!(matches!(
        observe_one(&c("dir"), true).unwrap(),
        ObservedKind::Dir { tree_hash: Some(_) }
    ));
    fs::write(c("file").path(), b"x").unwrap();
    assert_eq!(observe_one(&c("file"), true).unwrap(), ObservedKind::File);
}

#[test]
fn fs_ops_is_object_safe() {
    let fs_ops: &dyn FsOps = &RealFs;
    let t = tmp();
    fs_ops
        .write_state(&StatePath::new(t.path().join("x")), b"1")
        .unwrap();
}
