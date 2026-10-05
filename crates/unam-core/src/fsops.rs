//! Filesystem operations. Milestone M1 has only `write_atomic`; typed paths,
//! symlink, and copy operations arrive in milestone M3 (spec 4.9).

use crate::error::{Error, Result};
use std::io::Write;
use std::path::Path;

/// Write `bytes` to `path` atomically: parent directories are created, the
/// bytes go to a temp file in the same directory, which is fsynced and then
/// renamed over the target. An interruption leaves the old or the new file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)
        .map_err(|e| Error::io(format!("cannot create directory {}", parent.display()), e))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| {
        Error::io(
            format!("cannot create temp file in {}", parent.display()),
            e,
        )
    })?;
    tmp.write_all(bytes)
        .map_err(|e| Error::io(format!("cannot write {}", path.display()), e))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| Error::io(format!("cannot sync {}", path.display()), e))?;
    tmp.persist(path)
        .map_err(|e| Error::io(format!("cannot replace {}", path.display()), e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_parents_overwrites_and_leaves_no_temp() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("a/b/c.txt");
        write_atomic(&target, b"one").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"one");
        write_atomic(&target, b"two").unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), b"two");
        let names: Vec<_> = std::fs::read_dir(dir.path().join("a/b"))
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, vec![std::ffi::OsString::from("c.txt")]);
    }

    #[test]
    fn fails_when_parent_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("f"), b"x").unwrap();
        let err = write_atomic(&dir.path().join("f/x.json"), b"y").unwrap_err();
        assert_eq!(err.code(), "io-error");
    }
}
