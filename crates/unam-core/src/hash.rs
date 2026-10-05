//! Tree hashing (spec 4.3, plan P5).
//!
//! Canonical listing: regular files and symlinks only (directories are never
//! listed), sorted bytewise by forward-slash relative path.
//! File line: `path NUL size NUL blake3hex(content) LF`.
//! Symlink line: `path NUL L<target byte length> NUL blake3hex(target bytes) LF`.
//! Symlinks are never followed.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// A content hash in the form `blake3:<hex>`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Hash(String);

impl Hash {
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Hash(format!("blake3:{}", blake3::hash(bytes).to_hex()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Kind of a listed entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    File,
    Symlink,
}

/// One listed entry under a root, with its stat data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub rel: String,
    pub kind: FileKind,
    pub size: u64,
    pub mtime_secs: i64,
    pub mtime_nanos: u32,
}

/// Split a modification time into seconds and nanoseconds since the epoch.
pub fn mtime_parts(meta: &std::fs::Metadata) -> (i64, u32) {
    match meta.modified() {
        Ok(t) => match t.duration_since(UNIX_EPOCH) {
            Ok(d) => (
                i64::try_from(d.as_secs()).unwrap_or(i64::MAX),
                d.subsec_nanos(),
            ),
            Err(e) => {
                let d = e.duration();
                let secs = i64::try_from(d.as_secs()).unwrap_or(i64::MAX);
                if d.subsec_nanos() == 0 {
                    (-secs, 0)
                } else {
                    (-secs - 1, 1_000_000_000 - d.subsec_nanos())
                }
            }
        },
        Err(_) => (0, 0),
    }
}

fn walk_error(root: &Path, e: ignore::Error) -> Error {
    let ctx = format!("cannot walk {}", root.display());
    match e.into_io_error() {
        Some(io) => Error::io(ctx, io),
        None => Error::io(ctx, std::io::Error::other("walk error")),
    }
}

/// Forward-slash relative path of `path` under `root`.
pub fn rel_string(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

/// List regular files and symlinks under `root`, pruning any entry whose file
/// name is in `skip_names`, sorted bytewise by relative path.
pub fn walk_files(root: &Path, skip_names: &[&str]) -> Result<Vec<FileEntry>> {
    let skip: Vec<String> = skip_names.iter().map(|s| s.to_string()).collect();
    let walker = ignore::WalkBuilder::new(root)
        .standard_filters(false)
        .hidden(false)
        .follow_links(false)
        .filter_entry(move |e| {
            let name = e.file_name().to_string_lossy();
            !skip.iter().any(|s| *s == name)
        })
        .build();
    let mut out = Vec::new();
    for item in walker {
        let entry = item.map_err(|e| walk_error(root, e))?;
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        let meta = std::fs::symlink_metadata(path)
            .map_err(|e| Error::io(format!("cannot stat {}", path.display()), e))?;
        let ft = meta.file_type();
        let kind = if ft.is_symlink() {
            FileKind::Symlink
        } else if ft.is_file() {
            FileKind::File
        } else {
            continue;
        };
        let Some(rel) = rel_string(root, path) else {
            continue;
        };
        let (mtime_secs, mtime_nanos) = mtime_parts(&meta);
        out.push(FileEntry {
            rel,
            kind,
            size: meta.len(),
            mtime_secs,
            mtime_nanos,
        });
    }
    out.sort_by(|a, b| a.rel.as_bytes().cmp(b.rel.as_bytes()));
    Ok(out)
}

#[cfg(unix)]
fn link_target_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::os::unix::ffi::OsStrExt;
    Ok(std::fs::read_link(path)?.as_os_str().as_bytes().to_vec())
}

#[cfg(not(unix))]
fn link_target_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    Ok(std::fs::read_link(path)?
        .to_string_lossy()
        .into_owned()
        .into_bytes())
}

/// Build the canonical listing for `entries` (relative to `root`) and hash it.
pub fn hash_entries(root: &Path, entries: &[FileEntry]) -> Result<Hash> {
    let mut sorted: Vec<&FileEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.rel.as_bytes().cmp(b.rel.as_bytes()));
    let mut listing = Vec::new();
    for e in sorted {
        let path = root.join(&e.rel);
        listing.extend_from_slice(e.rel.as_bytes());
        listing.push(0);
        match e.kind {
            FileKind::File => {
                let content = std::fs::read(&path)
                    .map_err(|err| Error::io(format!("cannot read {}", path.display()), err))?;
                listing.extend_from_slice(content.len().to_string().as_bytes());
                listing.push(0);
                listing.extend_from_slice(blake3::hash(&content).to_hex().as_bytes());
            }
            FileKind::Symlink => {
                let target = link_target_bytes(&path).map_err(|err| {
                    Error::io(format!("cannot read link {}", path.display()), err)
                })?;
                listing.push(b'L');
                listing.extend_from_slice(target.len().to_string().as_bytes());
                listing.push(0);
                listing.extend_from_slice(blake3::hash(&target).to_hex().as_bytes());
            }
        }
        listing.push(b'\n');
    }
    Ok(Hash::of_bytes(&listing))
}

/// `walk_files` plus `hash_entries`.
pub fn tree_hash(root: &Path, skip_names: &[&str]) -> Result<Hash> {
    let entries = walk_files(root, skip_names)?;
    hash_entries(root, &entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;

    fn write(root: &Path, rel: &str, content: &[u8]) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }

    #[test]
    fn hash_format() {
        let h = Hash::of_bytes(b"x");
        assert!(h.as_str().starts_with("blake3:"));
        assert_eq!(h.as_str().len(), 7 + 64);
    }

    #[test]
    fn deterministic_across_creation_order() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        write(a.path(), "x/1.md", b"one");
        write(a.path(), "y.md", b"two");
        write(b.path(), "y.md", b"two");
        write(b.path(), "x/1.md", b"one");
        assert_eq!(
            tree_hash(a.path(), &[]).unwrap(),
            tree_hash(b.path(), &[]).unwrap()
        );
        assert_eq!(
            tree_hash(a.path(), &[]).unwrap(),
            tree_hash(a.path(), &[]).unwrap()
        );
    }

    #[test]
    fn bytewise_sort_pin() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a/b", b"1");
        write(d.path(), "a-b", b"2");
        let rels: Vec<String> = walk_files(d.path(), &[])
            .unwrap()
            .into_iter()
            .map(|e| e.rel)
            .collect();
        assert_eq!(rels, vec!["a-b".to_string(), "a/b".to_string()]);
    }

    #[test]
    fn changes_on_content_size_and_rename() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "f.md", b"abc");
        let h0 = tree_hash(d.path(), &[]).unwrap();
        write(d.path(), "f.md", b"abd");
        let h1 = tree_hash(d.path(), &[]).unwrap();
        assert_ne!(h0, h1);
        write(d.path(), "f.md", b"abcd");
        let h2 = tree_hash(d.path(), &[]).unwrap();
        assert_ne!(h1, h2);
        fs::rename(d.path().join("f.md"), d.path().join("g.md")).unwrap();
        let h3 = tree_hash(d.path(), &[]).unwrap();
        assert_ne!(h2, h3);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_hashed_by_target_and_not_followed() {
        use std::os::unix::fs::symlink;
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "real.md", b"content");
        symlink("real.md", d.path().join("link")).unwrap();
        let entries = walk_files(d.path(), &[]).unwrap();
        let link = entries.iter().find(|e| e.rel == "link").unwrap();
        assert_eq!(link.kind, FileKind::Symlink);
        let h0 = tree_hash(d.path(), &[]).unwrap();
        // Changing the target content does not change the hash of the link line,
        // but the target file itself changes, so compare with a dangling link.
        let e = tempfile::tempdir().unwrap();
        symlink("/nonexistent/target", e.path().join("dangling")).unwrap();
        let hd = tree_hash(e.path(), &[]).unwrap();
        fs::remove_file(e.path().join("dangling")).unwrap();
        symlink("/other/target", e.path().join("dangling")).unwrap();
        let hd2 = tree_hash(e.path(), &[]).unwrap();
        assert_ne!(hd, hd2);
        // A symlink and a file with equal bytes differ.
        let f = tempfile::tempdir().unwrap();
        write(f.path(), "x", b"/other/target");
        let g = tempfile::tempdir().unwrap();
        symlink("/other/target", g.path().join("x")).unwrap();
        assert_ne!(
            tree_hash(f.path(), &[]).unwrap(),
            tree_hash(g.path(), &[]).unwrap()
        );
        // Symlinked directory is not followed.
        let s = tempfile::tempdir().unwrap();
        write(s.path(), "dir/inner.md", b"x");
        symlink("dir", s.path().join("dirlink")).unwrap();
        let rels: Vec<String> = walk_files(s.path(), &[])
            .unwrap()
            .into_iter()
            .map(|e| e.rel)
            .collect();
        assert_eq!(
            rels,
            vec!["dir/inner.md".to_string(), "dirlink".to_string()]
        );
        assert!(h0.as_str().starts_with("blake3:"));
    }

    #[test]
    fn skip_names_prune_directories() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.md", b"a");
        let h0 = tree_hash(d.path(), &[".git"]).unwrap();
        write(d.path(), ".git/config", b"x");
        write(d.path(), ".DS_Store", b"x");
        assert_eq!(tree_hash(d.path(), &[".git", ".DS_Store"]).unwrap(), h0);
        assert_ne!(tree_hash(d.path(), &[]).unwrap(), h0);
    }

    #[test]
    fn empty_directory_does_not_change_hash() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "a.md", b"a");
        let h0 = tree_hash(d.path(), &[]).unwrap();
        fs::create_dir_all(d.path().join("empty/nested")).unwrap();
        assert_eq!(tree_hash(d.path(), &[]).unwrap(), h0);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]
        #[test]
        fn same_contents_same_hash_regardless_of_order(
            files in proptest::collection::btree_map("(sub/)?[a-d]{1,4}\\.md", proptest::collection::vec(any::<u8>(), 0..16), 1..8)
        ) {
            let a = tempfile::tempdir().unwrap();
            let b = tempfile::tempdir().unwrap();
            for (k, v) in &files {
                write(a.path(), k, v);
            }
            for (k, v) in files.iter().rev() {
                write(b.path(), k, v);
            }
            prop_assert_eq!(tree_hash(a.path(), &[]).unwrap(), tree_hash(b.path(), &[]).unwrap());
        }
    }
}
