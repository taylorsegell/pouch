//! Observation of a destination (M3 plan P8; minimal form): the root
//! state, each named entry, and shadow candidates in higher-precedence scan
//! locations. Shared by `plan::diff` (through `Observed`) and by
//! `fsops::remove_owned` (through `observe_one`). Milestone M4 promotes it
//! into `verify`.

pub use crate::fsops::{ObservedKind, observe_one};

use crate::adapter::{ScanLocation, Scope};
use crate::error::{Error, Result};
use crate::fsops::{DestPath, dest_prefixes, validate_dest_name};
use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

/// State of the destination root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestRootState {
    /// Absent: the project-relative prefixes still to create, in order.
    Missing {
        to_create: Vec<String>,
    },
    Dir,
    NotDir,
    Outside {
        resolved: String,
    },
}

/// A same-named skill in a higher-precedence scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shadow {
    pub name: String,
    pub by: PathBuf,
    pub scope: Scope,
}

/// What `observe` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observed {
    pub dest_root: DestRootState,
    pub entries: BTreeMap<String, ObservedKind>,
    pub shadows: Vec<Shadow>,
}

impl Observed {
    /// The observation of `name` (`Missing` when not observed).
    pub fn kind(&self, name: &str) -> ObservedKind {
        self.entries
            .get(name)
            .cloned()
            .unwrap_or(ObservedKind::Missing)
    }
}

fn canonical(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path)
        .map_err(|e| Error::io(format!("cannot resolve {}", path.display()), e))
}

/// Classify the destination root `<project_root>/<rel>` (P3): every
/// existing prefix must resolve inside the canonical project root and be a
/// directory; a symlink whose resolution stays inside is allowed.
pub fn classify_dest_root(project_root: &Path, rel: &str) -> Result<DestRootState> {
    let canon_root = canonical(project_root)?;
    let prefixes = dest_prefixes(rel);
    if prefixes.is_empty() {
        return Ok(DestRootState::NotDir);
    }
    for (i, prefix) in prefixes.iter().enumerate() {
        let p = project_root.join(prefix);
        let meta = match std::fs::symlink_metadata(&p) {
            Ok(m) => m,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Ok(DestRootState::Missing {
                    to_create: prefixes[i..].to_vec(),
                });
            }
            Err(e) => return Err(Error::io(format!("cannot inspect {}", p.display()), e)),
        };
        if meta.file_type().is_symlink() {
            let resolved = match std::fs::canonicalize(&p) {
                Ok(r) => r,
                Err(_) => return Ok(DestRootState::NotDir),
            };
            if !resolved.starts_with(&canon_root) {
                return Ok(DestRootState::Outside {
                    resolved: resolved.to_string_lossy().into_owned(),
                });
            }
            if !resolved.is_dir() {
                return Ok(DestRootState::NotDir);
            }
        } else if !meta.file_type().is_dir() {
            return Ok(DestRootState::NotDir);
        }
    }
    // Belt and braces: the whole root resolves inside the project.
    let resolved = canonical(&project_root.join(rel))?;
    if !resolved.starts_with(&canon_root) {
        return Ok(DestRootState::Outside {
            resolved: resolved.to_string_lossy().into_owned(),
        });
    }
    Ok(DestRootState::Dir)
}

/// Observe the destination: the root state, each name in `names` that
/// passes `validate_dest_name` (hashing those in `hash_names`), and shadow
/// candidates in enterprise and user scan locations.
pub fn observe(
    root: &Path,
    rel: &str,
    names: &BTreeSet<String>,
    hash_names: &BTreeSet<String>,
    scan_locations: &[ScanLocation],
) -> Result<Observed> {
    let dest_root = classify_dest_root(root, rel)?;
    let mut entries = BTreeMap::new();
    if dest_root == DestRootState::Dir {
        let droot = DestPath::dest_root(root, rel).map_err(|reason| Error::DestRootInvalid {
            problem: crate::error::DestRootProblem::NotDirectory,
            dest_root: rel.to_string(),
            detail: reason,
        })?;
        for name in names {
            if validate_dest_name(name).is_err() {
                continue;
            }
            let Ok(child) = droot.join_child(name) else {
                continue;
            };
            entries.insert(
                name.clone(),
                observe_one(&child, hash_names.contains(name))?,
            );
        }
    }
    // A location shadows only when it outranks the Project location (lower
    // rank wins; M5-2). With no Project location nothing shadows. For
    // `claude-code` the enterprise and user locations rank 0 and 1 against
    // the project's 2; for `agents-neutral` the user location ranks 2 against
    // the project's 0, so it never shadows, and ancestors are warned about
    // elsewhere, never shadows.
    let project_rank = scan_locations
        .iter()
        .find(|l| l.scope == Scope::Project)
        .map(|l| l.rank);
    let mut shadows = Vec::new();
    for name in names {
        if validate_dest_name(name).is_err() {
            continue;
        }
        let Some(project_rank) = project_rank else {
            continue;
        };
        for loc in scan_locations {
            if !matches!(loc.scope, Scope::Enterprise | Scope::User) || loc.rank >= project_rank {
                continue;
            }
            let by = loc.path.join(name);
            if by.join("SKILL.md").is_file() {
                shadows.push(Shadow {
                    name: name.clone(),
                    by,
                    scope: loc.scope,
                });
            }
        }
    }
    Ok(Observed {
        dest_root,
        entries,
        shadows,
    })
}

/// The parent directories of `root`, nearest first, up to and including the
/// nearest one that holds a `.git` entry (the same test as
/// `project::detect_root`). Empty when `root` itself holds `.git` or when no
/// `.git` is found above it: the walk never runs on to the filesystem root
/// (M5-1).
pub fn ancestors_to_git_root(root: &Path) -> Vec<PathBuf> {
    if std::fs::symlink_metadata(root.join(".git")).is_ok() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut dir = root.parent();
    while let Some(d) = dir {
        out.push(d.to_path_buf());
        if std::fs::symlink_metadata(d.join(".git")).is_ok() {
            return out;
        }
        dir = d.parent();
    }
    Vec::new()
}

/// The foreign entries of the destination root (M4-3): non-hidden
/// directories, and symlinks that resolve to directories, whose name is not
/// in `known`. Plain files and dot-names (`.DS_Store`, `.pouch-tmp-*`) are
/// ignored. Empty unless the destination root is a plain directory. Sorted
/// by name.
pub fn list_foreign(
    root: &Path,
    rel: &str,
    known: &BTreeSet<String>,
) -> Result<Vec<(String, ObservedKind)>> {
    if classify_dest_root(root, rel)? != DestRootState::Dir {
        return Ok(Vec::new());
    }
    let droot = DestPath::dest_root(root, rel).map_err(|reason| Error::DestRootInvalid {
        problem: crate::error::DestRootProblem::NotDirectory,
        dest_root: rel.to_string(),
        detail: reason,
    })?;
    let dir = root.join(rel);
    let mut names: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(&dir)
        .map_err(|e| Error::io(format!("cannot read {}", dir.display()), e))?
    {
        let entry = entry.map_err(|e| Error::io(format!("cannot read {}", dir.display()), e))?;
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if !name.starts_with('.') && !known.contains(&name) {
            names.push(name);
        }
    }
    names.sort();
    let mut out = Vec::new();
    for name in names {
        let Ok(child) = droot.join_child(&name) else {
            continue;
        };
        let kind = observe_one(&child, false)?;
        let is_dir = match &kind {
            ObservedKind::Dir { .. } => true,
            ObservedKind::Symlink { .. } => child.path().is_dir(),
            _ => false,
        };
        if is_dir {
            out.push((name, kind));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const REL: &str = ".claude/skills";

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| s.to_string()).collect()
    }

    #[cfg(unix)]
    #[test]
    fn each_kind_is_observed() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let d = root.join(REL);
        fs::create_dir_all(d.join("dir")).unwrap();
        fs::write(d.join("dir/SKILL.md"), b"x").unwrap();
        fs::write(d.join("file"), b"x").unwrap();
        std::os::unix::fs::symlink(root, d.join("link")).unwrap();
        std::os::unix::fs::symlink("/nonexistent/x", d.join("dangling")).unwrap();
        let fifo_ok = std::process::Command::new("mkfifo")
            .arg(d.join("fifo"))
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let names = set(&["dir", "file", "link", "dangling", "none", "fifo", ".."]);
        let o = observe(root, REL, &names, &set(&["dir"]), &[]).unwrap();
        assert_eq!(o.dest_root, DestRootState::Dir);
        assert!(matches!(
            o.kind("dir"),
            ObservedKind::Dir { tree_hash: Some(_) }
        ));
        assert_eq!(o.kind("file"), ObservedKind::File);
        assert!(matches!(
            o.kind("link"),
            ObservedKind::Symlink { resolves: true, .. }
        ));
        assert!(matches!(
            o.kind("dangling"),
            ObservedKind::Symlink {
                resolves: false,
                ..
            }
        ));
        assert_eq!(o.kind("none"), ObservedKind::Missing);
        if fifo_ok {
            assert_eq!(o.kind("fifo"), ObservedKind::Other);
        } else {
            println!("self-skip: mkfifo is not available (named residual)");
        }
        // Names failing validation are skipped.
        assert!(!o.entries.contains_key(".."));
        // A directory observed without a hash request has none.
        let o = observe(root, REL, &set(&["dir"]), &set(&[]), &[]).unwrap();
        assert_eq!(o.kind("dir"), ObservedKind::Dir { tree_hash: None });
    }

    #[cfg(unix)]
    #[test]
    fn classify_dest_root_rows() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("proj");
        fs::create_dir_all(&p).unwrap();
        assert_eq!(
            classify_dest_root(&p, REL).unwrap(),
            DestRootState::Missing {
                to_create: vec![".claude".into(), ".claude/skills".into()]
            }
        );
        fs::create_dir_all(p.join(".claude")).unwrap();
        assert_eq!(
            classify_dest_root(&p, REL).unwrap(),
            DestRootState::Missing {
                to_create: vec![".claude/skills".into()]
            }
        );
        fs::create_dir_all(p.join(REL)).unwrap();
        assert_eq!(classify_dest_root(&p, REL).unwrap(), DestRootState::Dir);
        // A file at the root.
        fs::remove_dir(p.join(REL)).unwrap();
        fs::write(p.join(REL), b"x").unwrap();
        assert_eq!(classify_dest_root(&p, REL).unwrap(), DestRootState::NotDir);
        fs::remove_file(p.join(REL)).unwrap();
        // A symlink inside the project.
        fs::create_dir_all(p.join("elsewhere/skills")).unwrap();
        fs::remove_dir(p.join(".claude")).unwrap();
        std::os::unix::fs::symlink(p.join("elsewhere"), p.join(".claude")).unwrap();
        assert_eq!(classify_dest_root(&p, REL).unwrap(), DestRootState::Dir);
        // A symlink outside the project.
        fs::remove_file(p.join(".claude")).unwrap();
        let outside = t.path().join("outside");
        fs::create_dir_all(outside.join("skills")).unwrap();
        std::os::unix::fs::symlink(&outside, p.join(".claude")).unwrap();
        assert!(matches!(
            classify_dest_root(&p, REL).unwrap(),
            DestRootState::Outside { .. }
        ));
    }

    #[cfg(unix)]
    #[test]
    fn foreign_listing_rules() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path();
        let d = root.join(REL);
        fs::create_dir_all(d.join("mine")).unwrap();
        fs::create_dir_all(d.join("theirs")).unwrap();
        fs::create_dir_all(d.join(".pouch-tmp-x")).unwrap();
        fs::write(d.join("README.md"), b"x").unwrap();
        fs::write(d.join(".DS_Store"), b"x").unwrap();
        fs::create_dir_all(root.join("elsewhere")).unwrap();
        std::os::unix::fs::symlink(root.join("elsewhere"), d.join("linked")).unwrap();
        std::os::unix::fs::symlink("/nonexistent/x", d.join("dangling")).unwrap();
        let got = list_foreign(root, REL, &set(&["mine"])).unwrap();
        let names: Vec<&str> = got.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["linked", "theirs"]);
        // No destination root: nothing to list.
        let empty = tempfile::tempdir().unwrap();
        assert!(
            list_foreign(empty.path(), REL, &set(&[]))
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn user_location_never_shadows_under_agents_neutral() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("proj");
        let rel = ".agents/skills";
        fs::create_dir_all(root.join(rel)).unwrap();
        let user = t.path().join("home/.agents/skills");
        let anc = t.path().join("above/.agents/skills");
        for base in [&user, &anc] {
            fs::create_dir_all(base.join("code-review")).unwrap();
            fs::write(base.join("code-review/SKILL.md"), b"x").unwrap();
        }
        let locs = vec![
            ScanLocation {
                path: root.join(rel),
                scope: Scope::Project,
                rank: 0,
            },
            ScanLocation {
                path: anc,
                scope: Scope::Ancestor,
                rank: 1,
            },
            ScanLocation {
                path: user,
                scope: Scope::User,
                rank: 2,
            },
        ];
        let o = observe(&root, rel, &set(&["code-review"]), &set(&[]), &locs).unwrap();
        assert!(o.shadows.is_empty(), "{:?}", o.shadows);
        // No Project location at all: nothing shadows.
        let o = observe(&root, rel, &set(&["code-review"]), &set(&[]), &locs[1..]).unwrap();
        assert!(o.shadows.is_empty());
    }

    #[test]
    fn ancestor_walk_rows() {
        let t = tempfile::tempdir().unwrap();
        let repo = t.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        // The Git root itself: none.
        assert!(ancestors_to_git_root(&repo).is_empty());
        // Below the Git root: nearest first, ending at the Git root.
        let sub = repo.join("a/b");
        fs::create_dir_all(&sub).unwrap();
        assert_eq!(
            ancestors_to_git_root(&sub),
            vec![repo.join("a"), repo.clone()]
        );
        // No Git root anywhere above: none.
        let loose = t.path().join("loose/x/y");
        fs::create_dir_all(&loose).unwrap();
        assert!(ancestors_to_git_root(&loose).is_empty());
        // A `.git` file counts, as in root detection.
        fs::write(t.path().join("loose/.git"), "gitdir: z\n").unwrap();
        assert_eq!(
            ancestors_to_git_root(&loose),
            vec![t.path().join("loose/x"), t.path().join("loose")]
        );
    }

    #[test]
    fn shadows_from_enterprise_and_user_only() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().join("proj");
        fs::create_dir_all(root.join(REL)).unwrap();
        let ent = t.path().join("ent");
        let user = t.path().join("home/.claude/skills");
        for base in [&ent, &user, &root.join(REL)] {
            fs::create_dir_all(base.join("code-review")).unwrap();
            fs::write(base.join("code-review/SKILL.md"), b"x").unwrap();
        }
        let locs = vec![
            ScanLocation {
                path: ent.clone(),
                scope: Scope::Enterprise,
                rank: 0,
            },
            ScanLocation {
                path: user.clone(),
                scope: Scope::User,
                rank: 1,
            },
            ScanLocation {
                path: root.join(REL),
                scope: Scope::Project,
                rank: 2,
            },
        ];
        let o = observe(
            &root,
            REL,
            &set(&["code-review", "other"]),
            &set(&[]),
            &locs,
        )
        .unwrap();
        let got: Vec<(String, Scope)> = o
            .shadows
            .iter()
            .map(|s| (s.name.clone(), s.scope))
            .collect();
        assert_eq!(
            got,
            vec![
                ("code-review".to_string(), Scope::Enterprise),
                ("code-review".to_string(), Scope::User)
            ]
        );
        assert_eq!(o.shadows[0].by, ent.join("code-review"));
    }
}
