//! Source walking and per-skill scanning (spec 4.3; plan D3, D4, P3, P4).
//!
//! Two passes. Discovery walks the source with `ignore::WalkBuilder` (hidden
//! skipped, `.gitignore` honored inside git trees, `node_modules` and `.git`
//! pruned, skill directories never descended) and finds skill directories.
//! The per-skill pass walks each skill directory exactly once and reuses
//! those entries for the fingerprint, the supporting list, and the tree hash.

use super::frontmatter::parse_skill_md;
use super::{RelPath, Skill, SkillId};
use crate::error::Diagnostic;
use crate::hash::{FileEntry, hash_entries, rel_string, walk_files};
use crate::source::SourceAlias;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

/// Names skipped inside a skill directory (plan D3).
pub const SKILL_SKIP_NAMES: &[&str] = &[".git", ".DS_Store"];

/// Walk limits. `git_global` controls whether the global gitignore is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanLimits {
    pub max_depth: usize,
    pub max_dirs: usize,
    pub git_global: bool,
}

impl ScanLimits {
    /// Product defaults (spec 4.3): depth 8, 5,000 directories.
    pub fn product(git_global: bool) -> Self {
        ScanLimits {
            max_depth: 8,
            max_dirs: 5000,
            git_global,
        }
    }
}

/// Stat fingerprint of one file in a skill directory (plan P15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FingerprintEntry {
    pub path: String,
    pub size: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
}

impl From<&FileEntry> for FingerprintEntry {
    fn from(e: &FileEntry) -> Self {
        FingerprintEntry {
            path: e.rel.clone(),
            size: e.size,
            mtime_secs: e.mtime_secs,
            mtime_nanos: e.mtime_nanos,
        }
    }
}

/// A loaded skill plus its fingerprint (what the cache stores).
#[derive(Debug, Clone, PartialEq)]
pub struct ScannedSkill {
    pub skill: Skill,
    pub fingerprint: Vec<FingerprintEntry>,
}

/// Result of scanning one source.
#[derive(Debug, Clone, Default)]
pub struct SourceScan {
    /// Loaded skills sorted by ID.
    pub skills: Vec<ScannedSkill>,
    /// Catalog-level diagnostics (skipped skills and source-level notices).
    pub diagnostics: Vec<Diagnostic>,
    /// Loaded skills that were parsed (not reused from `prior`).
    pub reparsed: usize,
    /// Prior skills no longer loaded.
    pub dropped: usize,
}

/// True when `dir` contains a regular file named exactly `SKILL.md` (P3):
/// cheap stat first, then the directory listing confirms the exact case.
fn has_skill_md(dir: &Path) -> bool {
    match std::fs::symlink_metadata(dir.join("SKILL.md")) {
        Ok(m) if m.file_type().is_file() => {}
        _ => return false,
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        return false;
    };
    rd.flatten()
        .any(|e| e.file_name() == "SKILL.md" && e.file_type().map(|t| t.is_file()).unwrap_or(false))
}

fn walk_error_parts(err: &ignore::Error) -> (Option<PathBuf>, String) {
    match err {
        ignore::Error::WithPath { path, err } => (Some(path.clone()), err.to_string()),
        ignore::Error::WithDepth { err, .. } => walk_error_parts(err),
        ignore::Error::WithLineNumber { err, .. } => walk_error_parts(err),
        other => (None, other.to_string()),
    }
}

/// Scan one source. `prior` maps relative skill paths to cached entries.
pub fn scan_source(
    alias: &SourceAlias,
    root: &Path,
    prior: &HashMap<String, ScannedSkill>,
    limits: &ScanLimits,
) -> SourceScan {
    let mut diagnostics = Vec::new();
    let skill_dirs = discover(root, limits, &mut diagnostics);

    let mut skills: Vec<ScannedSkill> = Vec::new();
    let mut reparsed = 0;
    for (rel, dir) in &skill_dirs {
        match scan_skill(alias, rel, dir, prior.get(rel)) {
            Ok((scanned, reused)) => {
                if !reused {
                    reparsed += 1;
                }
                skills.push(scanned);
            }
            Err(d) => diagnostics.push(d.with_path(rel.clone())),
        }
    }
    skills.sort_by(|a, b| a.skill.id.cmp(&b.skill.id));

    let loaded: BTreeSet<&str> = skills
        .iter()
        .map(|s| s.skill.id.rel_path.as_str())
        .collect();
    let dropped = prior
        .keys()
        .filter(|k| !loaded.contains(k.as_str()))
        .count();

    let mut by_name: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for s in &skills {
        by_name
            .entry(s.skill.name.as_str())
            .or_default()
            .push(s.skill.id.to_string());
    }
    for (name, ids) in by_name {
        if ids.len() > 1 {
            diagnostics.push(Diagnostic::info(
                "duplicate-name",
                format!(
                    "name `{name}` is used by {} skills: {}",
                    ids.len(),
                    ids.join(", ")
                ),
            ));
        }
    }

    SourceScan {
        skills,
        diagnostics,
        reparsed,
        dropped,
    }
}

/// Discovery pass: returns skill directories keyed by relative path.
fn discover(
    root: &Path,
    limits: &ScanLimits,
    diagnostics: &mut Vec<Diagnostic>,
) -> BTreeMap<String, PathBuf> {
    let filter_root = root.to_path_buf();
    let walker = ignore::WalkBuilder::new(root)
        .max_depth(Some(limits.max_depth))
        .follow_links(false)
        .git_global(limits.git_global)
        .sort_by_file_path(|a, b| a.cmp(b))
        .filter_entry(move |entry| {
            let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
            if !is_dir {
                return true;
            }
            let name = entry.file_name();
            if name == "node_modules" || name == ".git" {
                return false;
            }
            match entry.path().parent() {
                Some(parent) if parent != filter_root => !has_skill_md(parent),
                _ => true,
            }
        })
        .build();

    let mut skill_dirs = BTreeMap::new();
    let mut dir_count = 0usize;
    for item in walker {
        let entry = match item {
            Ok(e) => e,
            Err(err) => {
                let (path, msg) = walk_error_parts(&err);
                let mut d = Diagnostic::warning("walk-error", format!("cannot read: {msg}"));
                if let Some(rel) = path.as_deref().and_then(|p| rel_string(root, p)) {
                    d = d.with_path(rel);
                }
                diagnostics.push(d);
                continue;
            }
        };
        if entry.depth() == 0 {
            continue;
        }
        let Some(ft) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if ft.is_dir() {
            dir_count += 1;
            if dir_count > limits.max_dirs {
                diagnostics.push(Diagnostic::warning(
                    "walk-capped",
                    format!(
                        "directory limit of {} reached; results are partial",
                        limits.max_dirs
                    ),
                ));
                break;
            }
            continue;
        }
        let is_skill_md = entry.file_name() == "SKILL.md";
        let parent = path.parent().unwrap_or(root);
        if ft.is_file() && is_skill_md {
            if parent == root {
                diagnostics.push(
                    Diagnostic::warning(
                        "root-skill-skipped",
                        "a SKILL.md directly in the source root is ignored; skills need a directory",
                    )
                    .with_path("SKILL.md"),
                );
            } else if let Some(rel) = rel_string(root, parent) {
                skill_dirs.insert(rel, parent.to_path_buf());
            }
        } else if ft.is_symlink() {
            if is_skill_md {
                let rel = rel_string(root, parent).unwrap_or_default();
                diagnostics.push(
                    Diagnostic::warning(
                        "symlinked-skill-md-skipped",
                        "SKILL.md is a symlink; symlinks are not followed",
                    )
                    .with_path(rel),
                );
            } else if std::fs::metadata(path).is_ok_and(|m| m.is_dir()) && has_skill_md(path) {
                let rel = rel_string(root, path).unwrap_or_default();
                diagnostics.push(
                    Diagnostic::warning(
                        "symlinked-skill-skipped",
                        "skill directory is a symlink; symlinks are not followed",
                    )
                    .with_path(rel),
                );
            }
        }
    }
    skill_dirs
}

/// Per-skill pass for one directory. Returns the skill and whether it was reused.
fn scan_skill(
    alias: &SourceAlias,
    rel: &str,
    dir: &Path,
    prior: Option<&ScannedSkill>,
) -> Result<(ScannedSkill, bool), Diagnostic> {
    let io_err = |msg: String| Diagnostic::error("skill-io-error", msg);
    let rel_path = RelPath::parse(rel).map_err(|r| io_err(format!("invalid skill path: {r}")))?;
    let entries = walk_files(dir, SKILL_SKIP_NAMES).map_err(|e| io_err(e.to_string()))?;
    let fingerprint: Vec<FingerprintEntry> = entries.iter().map(FingerprintEntry::from).collect();
    if let Some(p) = prior
        && p.fingerprint == fingerprint
    {
        return Ok((p.clone(), true));
    }
    let bytes = std::fs::read(dir.join("SKILL.md"))
        .map_err(|e| io_err(format!("cannot read SKILL.md: {e}")))?;
    let parsed = parse_skill_md(&bytes, rel_path.file_name())?;
    let tree_hash = hash_entries(dir, &entries).map_err(|e| io_err(e.to_string()))?;
    let supporting: Vec<String> = entries
        .iter()
        .filter(|e| e.rel != "SKILL.md")
        .map(|e| e.rel.clone())
        .collect();
    let category = rel_path.parent_str().to_string();
    let skill = Skill {
        id: SkillId {
            source: alias.clone(),
            rel_path,
        },
        name: parsed.name,
        description: parsed.description,
        category,
        frontmatter: parsed.frontmatter,
        supporting,
        tree_hash,
        diagnostics: parsed.diagnostics,
    };
    Ok((ScannedSkill { skill, fingerprint }, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn limits() -> ScanLimits {
        ScanLimits {
            max_depth: 8,
            max_dirs: 5000,
            git_global: false,
        }
    }

    fn alias() -> SourceAlias {
        SourceAlias::parse("fx").unwrap()
    }

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    fn skill(root: &Path, rel_dir: &str, name: &str) {
        write(
            root,
            &format!("{rel_dir}/SKILL.md"),
            &format!("---\nname: {name}\ndescription: About {name}.\n---\n"),
        );
    }

    fn scan(root: &Path) -> SourceScan {
        scan_source(&alias(), root, &HashMap::new(), &limits())
    }

    fn ids(s: &SourceScan) -> Vec<String> {
        s.skills.iter().map(|k| k.skill.id.to_string()).collect()
    }

    fn codes(s: &SourceScan) -> Vec<&str> {
        s.diagnostics.iter().map(|d| d.code.as_str()).collect()
    }

    fn prior_of(s: &SourceScan) -> HashMap<String, ScannedSkill> {
        s.skills
            .iter()
            .map(|k| (k.skill.id.rel_path.to_string(), k.clone()))
            .collect()
    }

    #[test]
    fn nesting_and_category() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "top", "top");
        skill(t.path(), "a/b/c/deep", "deep");
        let s = scan(t.path());
        assert_eq!(ids(&s), vec!["fx:a/b/c/deep", "fx:top"]);
        assert_eq!(s.skills[0].skill.category, "a/b/c");
        assert_eq!(s.skills[1].skill.category, "");
        assert!(s.diagnostics.is_empty());
    }

    #[test]
    fn depth_boundary() {
        let t = tempfile::tempdir().unwrap();
        // Skill dir at depth 7 (SKILL.md at 8) is found; at depth 8 it is not.
        skill(t.path(), "1/2/3/4/5/6/seven", "seven");
        skill(t.path(), "1/2/3/4/5/6/7/eight", "eight");
        assert_eq!(ids(&scan(t.path())), vec!["fx:1/2/3/4/5/6/seven"]);
    }

    #[test]
    fn git_node_modules_hidden_skipped() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "ok", "ok");
        skill(t.path(), "node_modules/pkg", "pkg");
        skill(t.path(), "vendor/node_modules/deep", "deep");
        skill(t.path(), "vendor/repo/.git/x", "x");
        skill(t.path(), ".hidden/h", "h");
        skill(t.path(), "cat/.hidden-skill", "hs");
        assert_eq!(ids(&scan(t.path())), vec!["fx:ok"]);
    }

    #[test]
    fn nested_skill_not_discovered_but_listed_as_supporting() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "s/outer", "outer");
        skill(t.path(), "s/outer/nested", "nested");
        write(t.path(), "s/outer/references/r.md", "ref");
        write(t.path(), "s/outer/.DS_Store", "x");
        let s = scan(t.path());
        assert_eq!(ids(&s), vec!["fx:s/outer"]);
        assert_eq!(
            s.skills[0].skill.supporting,
            vec!["nested/SKILL.md".to_string(), "references/r.md".to_string()]
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_inside_skill_hashed_by_target() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "s", "s");
        std::os::unix::fs::symlink("/nowhere", t.path().join("s/link")).unwrap();
        let h1 = scan(t.path()).skills[0].skill.tree_hash.clone();
        fs::remove_file(t.path().join("s/link")).unwrap();
        std::os::unix::fs::symlink("/elsewhere", t.path().join("s/link")).unwrap();
        let s2 = scan(t.path());
        assert_ne!(h1, s2.skills[0].skill.tree_hash);
        assert!(s2.skills[0].skill.supporting.contains(&"link".to_string()));
    }

    #[test]
    fn lowercase_skill_md_is_not_a_skill() {
        let t = tempfile::tempdir().unwrap();
        write(
            t.path(),
            "case/lower/skill.md",
            "---\nname: lower\ndescription: x\n---\n",
        );
        write(
            t.path(),
            "case/other/skill.md",
            "---\ndescription: x\n---\n",
        );
        write(t.path(), "case/other/notes.md", "n");
        let s = scan(t.path());
        assert!(s.skills.is_empty());
        assert!(s.diagnostics.is_empty());
    }

    #[test]
    fn cap_returns_partial_results() {
        let t = tempfile::tempdir().unwrap();
        for i in 0..20 {
            skill(t.path(), &format!("c{i:02}"), &format!("c{i:02}"));
        }
        let lim = ScanLimits {
            max_dirs: 10,
            ..limits()
        };
        let s = scan_source(&alias(), t.path(), &HashMap::new(), &lim);
        assert_eq!(s.skills.len(), 10);
        assert_eq!(codes(&s), vec!["walk-capped"]);
        let lim = ScanLimits {
            max_dirs: 20,
            ..limits()
        };
        let s = scan_source(&alias(), t.path(), &HashMap::new(), &lim);
        assert_eq!(s.skills.len(), 20);
        assert!(s.diagnostics.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_skill_dir_and_skill_md() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "real/s", "s");
        fs::create_dir_all(t.path().join("linked")).unwrap();
        std::os::unix::fs::symlink("../real/s", t.path().join("linked/s-link")).unwrap();
        fs::create_dir_all(t.path().join("md/x")).unwrap();
        std::os::unix::fs::symlink("../../real/s/SKILL.md", t.path().join("md/x/SKILL.md"))
            .unwrap();
        let s = scan(t.path());
        assert_eq!(ids(&s), vec!["fx:real/s"]);
        let mut c = codes(&s);
        c.sort();
        assert_eq!(
            c,
            vec!["symlinked-skill-md-skipped", "symlinked-skill-skipped"]
        );
        let link = s
            .diagnostics
            .iter()
            .find(|d| d.code == "symlinked-skill-skipped")
            .unwrap();
        assert_eq!(link.path.as_deref(), Some("linked/s-link"));
        let md = s
            .diagnostics
            .iter()
            .find(|d| d.code == "symlinked-skill-md-skipped")
            .unwrap();
        assert_eq!(md.path.as_deref(), Some("md/x"));
    }

    #[test]
    fn root_skill_ignored_children_scanned() {
        let t = tempfile::tempdir().unwrap();
        write(t.path(), "SKILL.md", "---\ndescription: root\n---\n");
        skill(t.path(), "child", "child");
        let s = scan(t.path());
        assert_eq!(ids(&s), vec!["fx:child"]);
        assert_eq!(codes(&s), vec!["root-skill-skipped"]);
    }

    #[test]
    fn deterministic_order() {
        let t = tempfile::tempdir().unwrap();
        for n in ["zz", "aa", "mm/x", "b"] {
            skill(t.path(), n, n.rsplit('/').next().unwrap());
        }
        let a = ids(&scan(t.path()));
        let b = ids(&scan(t.path()));
        assert_eq!(a, b);
        assert_eq!(a, vec!["fx:aa", "fx:b", "fx:mm/x", "fx:zz"]);
    }

    #[test]
    fn broken_and_no_description_skipped() {
        let t = tempfile::tempdir().unwrap();
        write(
            t.path(),
            "broken/bad-yaml/SKILL.md",
            "---\ndescription: [unclosed\n---\n",
        );
        write(
            t.path(),
            "broken/no-desc/SKILL.md",
            "---\nname: no-desc\n---\n",
        );
        skill(t.path(), "good", "good");
        let s = scan(t.path());
        assert_eq!(ids(&s), vec!["fx:good"]);
        assert_eq!(codes(&s), vec!["unparseable-yaml", "missing-description"]);
        assert_eq!(s.diagnostics[0].path.as_deref(), Some("broken/bad-yaml"));
        assert_eq!(s.diagnostics[1].path.as_deref(), Some("broken/no-desc"));
    }

    #[test]
    fn duplicate_name_info() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "a/error-handling", "error-handling");
        skill(t.path(), "b/error-handling", "error-handling");
        let s = scan(t.path());
        assert_eq!(codes(&s), vec!["duplicate-name"]);
        let d = &s.diagnostics[0];
        assert!(d.message.contains("fx:a/error-handling"));
        assert!(d.message.contains("fx:b/error-handling"));
    }

    #[test]
    fn fingerprint_reuse_and_reparse() {
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "a", "a");
        skill(t.path(), "b", "b");
        let first = scan(t.path());
        assert_eq!(first.reparsed, 2);
        let prior = prior_of(&first);
        let second = scan_source(&alias(), t.path(), &prior, &limits());
        assert_eq!(second.reparsed, 0);
        assert_eq!(second.dropped, 0);
        assert_eq!(second.skills, first.skills);
        // Poisoned prior: unchanged fingerprint means the prior value is returned untouched.
        let mut poisoned = prior.clone();
        poisoned.get_mut("a").unwrap().skill.description = "MARKER".into();
        let third = scan_source(&alias(), t.path(), &poisoned, &limits());
        assert_eq!(third.skills[0].skill.description, "MARKER");
        // Same size, new mtime: re-parsed.
        let p = t.path().join("a/SKILL.md");
        fs::write(&p, "---\nname: a\ndescription: About X.\n---\n").unwrap();
        let f = fs::File::options().write(true).open(&p).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000))
            .unwrap();
        let fourth = scan_source(&alias(), t.path(), &poisoned, &limits());
        assert_eq!(fourth.reparsed, 1);
        assert_eq!(fourth.skills[0].skill.description, "About X.");
        // Size change: re-parsed.
        fs::write(t.path().join("b/extra.md"), "x").unwrap();
        let fifth = scan_source(&alias(), t.path(), &prior_of(&fourth), &limits());
        assert_eq!(fifth.reparsed, 1);
        // Dropped skill counted.
        fs::remove_dir_all(t.path().join("b")).unwrap();
        let sixth = scan_source(&alias(), t.path(), &prior_of(&fifth), &limits());
        assert_eq!(sixth.dropped, 1);
        assert_eq!(sixth.skills.len(), 1);
    }

    #[cfg(unix)]
    fn permissions_enforced(path: &Path) -> bool {
        fs::read(path).is_err()
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_skill_file_gives_skill_io_error() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "locked", "locked");
        skill(t.path(), "open", "open");
        let p = t.path().join("locked/SKILL.md");
        fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
        if !permissions_enforced(&p) {
            fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
            println!("SKIPPED (Hybrid): file permissions are not enforced for this user");
            return;
        }
        let s = scan(t.path());
        fs::set_permissions(&p, fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(ids(&s), vec!["fx:open"]);
        assert_eq!(codes(&s), vec!["skill-io-error"]);
        assert_eq!(s.diagnostics[0].path.as_deref(), Some("locked"));
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_directory_gives_walk_error() {
        use std::os::unix::fs::PermissionsExt;
        let t = tempfile::tempdir().unwrap();
        skill(t.path(), "closed/s", "s");
        skill(t.path(), "open/s", "s2");
        let d = t.path().join("closed");
        fs::set_permissions(&d, fs::Permissions::from_mode(0o000)).unwrap();
        if fs::read_dir(&d).is_ok() {
            fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
            println!("SKIPPED (Hybrid): directory permissions are not enforced for this user");
            return;
        }
        let s = scan(t.path());
        fs::set_permissions(&d, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(ids(&s), vec!["fx:open/s"]);
        assert_eq!(codes(&s), vec!["walk-error"]);
        assert_eq!(s.diagnostics[0].path.as_deref(), Some("closed"));
    }
}
