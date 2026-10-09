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
    let mut shadows = Vec::new();
    for name in names {
        if validate_dest_name(name).is_err() {
            continue;
        }
        for loc in scan_locations {
            if !matches!(loc.scope, Scope::Enterprise | Scope::User) {
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
