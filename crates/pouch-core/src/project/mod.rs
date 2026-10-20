//! Projects (spec 4.5, 2.3): root detection, the selection file, and the
//! project index.

pub mod index;
pub mod selection;

pub use selection::{
    Selection, SelectionSave, TargetId, TargetSelection, load_selection, save_selection,
    selection_path,
};

use crate::error::{Error, Result};
use std::path::{Path, PathBuf};

/// How a project root was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootDetection {
    /// The nearest ancestor-or-self holding a `.git` entry.
    Git,
    /// No `.git` found: the working directory itself.
    Explicit,
}

/// A detected project root (canonical path).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRoot {
    pub path: PathBuf,
    pub how: RootDetection,
}

/// Find the project root for `cwd` (M2 plan P8): canonicalize `cwd` (and
/// `ceiling`), walk up checking for a `.git` entry (file, directory, or
/// symlink), and return the nearest ancestor-or-self that has one; none found
/// gives the canonical `cwd` as `Explicit`. `ceiling` (`None` in the product)
/// stops the walk after examining the ceiling directory.
pub fn detect_root(cwd: &Path, ceiling: Option<&Path>) -> Result<ProjectRoot> {
    let start = std::fs::canonicalize(cwd).map_err(|e| {
        Error::io(
            format!("cannot resolve working directory {}", cwd.display()),
            e,
        )
    })?;
    let ceiling = ceiling.map(|c| std::fs::canonicalize(c).unwrap_or_else(|_| c.to_path_buf()));
    let mut dir: &Path = &start;
    loop {
        if std::fs::symlink_metadata(dir.join(".git")).is_ok() {
            return Ok(ProjectRoot {
                path: dir.to_path_buf(),
                how: RootDetection::Git,
            });
        }
        if ceiling.as_deref() == Some(dir) {
            break;
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    Ok(ProjectRoot {
        path: start,
        how: RootDetection::Explicit,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn canon(p: &Path) -> PathBuf {
        fs::canonicalize(p).unwrap()
    }

    #[test]
    fn git_directory_file_and_symlink() {
        let t = tempfile::tempdir().unwrap();
        let ceil = t.path();
        let a = t.path().join("a");
        fs::create_dir_all(a.join(".git")).unwrap();
        let r = detect_root(&a, Some(ceil)).unwrap();
        assert_eq!(
            r,
            ProjectRoot {
                path: canon(&a),
                how: RootDetection::Git
            }
        );

        let b = t.path().join("b");
        fs::create_dir_all(&b).unwrap();
        fs::write(b.join(".git"), "gitdir: elsewhere\n").unwrap();
        assert_eq!(detect_root(&b, Some(ceil)).unwrap().how, RootDetection::Git);

        #[cfg(unix)]
        {
            let c = t.path().join("c");
            fs::create_dir_all(&c).unwrap();
            std::os::unix::fs::symlink("/definitely/missing", c.join(".git")).unwrap();
            let r = detect_root(&c, Some(ceil)).unwrap();
            assert_eq!(r.path, canon(&c));
            assert_eq!(r.how, RootDetection::Git);
        }
    }

    #[test]
    fn nearest_wins_and_subdirectory_start() {
        let t = tempfile::tempdir().unwrap();
        let outer = t.path().join("outer");
        let inner = outer.join("inner");
        let deep = inner.join("src/deep");
        fs::create_dir_all(outer.join(".git")).unwrap();
        fs::create_dir_all(inner.join(".git")).unwrap();
        fs::create_dir_all(&deep).unwrap();
        let r = detect_root(&deep, Some(t.path())).unwrap();
        assert_eq!(r.path, canon(&inner));
        let other = outer.join("other/x");
        fs::create_dir_all(&other).unwrap();
        assert_eq!(
            detect_root(&other, Some(t.path())).unwrap().path,
            canon(&outer)
        );
    }

    #[test]
    fn none_found_is_explicit_cwd() {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("plain/sub");
        fs::create_dir_all(&d).unwrap();
        let r = detect_root(&d, Some(t.path())).unwrap();
        assert_eq!(
            r,
            ProjectRoot {
                path: canon(&d),
                how: RootDetection::Explicit
            }
        );
    }

    #[test]
    fn ceiling_is_examined_but_not_passed() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join(".git")).unwrap();
        let ceil = t.path().join("ceil");
        let d = ceil.join("x");
        fs::create_dir_all(&d).unwrap();
        // The marker above the ceiling is never consulted.
        let r = detect_root(&d, Some(&ceil)).unwrap();
        assert_eq!(r.how, RootDetection::Explicit);
        // A marker in the ceiling itself is found.
        fs::create_dir_all(ceil.join(".git")).unwrap();
        let r = detect_root(&d, Some(&ceil)).unwrap();
        assert_eq!(
            r,
            ProjectRoot {
                path: canon(&ceil),
                how: RootDetection::Git
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn ceiling_given_as_symlink_is_canonicalized() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join(".git")).unwrap();
        let real = t.path().join("real");
        let d = real.join("x");
        fs::create_dir_all(&d).unwrap();
        let link = t.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let r = detect_root(&link.join("x"), Some(&link)).unwrap();
        assert_eq!(r.how, RootDetection::Explicit);
        assert_eq!(r.path, canon(&d));
    }

    #[test]
    fn missing_cwd_is_an_io_error() {
        let t = tempfile::tempdir().unwrap();
        let e = detect_root(&t.path().join("nope"), Some(t.path())).unwrap_err();
        assert_eq!(e.code(), "io-error");
    }
}
