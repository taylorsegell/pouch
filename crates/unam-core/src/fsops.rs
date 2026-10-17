//! Filesystem operations (spec 4.9; M3 plan P2, P3, P30).
//!
//! No function here writes through a `SourcePath`: every write takes one of
//! the sealed `Writable` path types (`DestPath`, `StatePath`, `ToolPath`).
//! Deletions are limited to what `remove_owned` proves is tool-owned, empty
//! directories, our own state files, and our own unique temp directories.
//!
//! ```compile_fail
//! use unam_core::fsops::write_atomic;
//! use unam_core::source::SourcePath;
//! let p = SourcePath::new(std::path::PathBuf::from("/nonexistent/x"));
//! write_atomic(&p, b"x").unwrap();
//! ```
//!
//! The same snippet with a `StatePath` compiles:
//!
//! ```
//! use unam_core::fsops::{StatePath, write_atomic};
//! let dir = tempfile::tempdir().unwrap();
//! let p = StatePath::new(dir.path().join("x"));
//! write_atomic(&p, b"x").unwrap();
//! ```

use crate::catalog::SKILL_SKIP_NAMES;
use crate::config::Strategy;
use crate::error::{Error, Result};
use crate::hash::{Hash, tree_hash};
use crate::source::SourcePath;
use std::fs::File;
use std::io::{ErrorKind, Write};
use std::path::{Component, Path, PathBuf};

mod sealed {
    pub trait Sealed {}
}

/// A path that write functions accept. Sealed: implemented for exactly
/// `DestPath`, `StatePath`, and `ToolPath`.
pub trait Writable: sealed::Sealed {
    fn write_target(&self) -> &Path;
}

/// A path under the user-scope unam directories (config, cache, data,
/// bundles, editor temp copies).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolPath(PathBuf);

impl ToolPath {
    pub fn new(path: PathBuf) -> Self {
        ToolPath(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// A path under `<project>/.unam/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatePath(PathBuf);

impl StatePath {
    pub fn new(path: PathBuf) -> Self {
        StatePath(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    /// A child of this state path (the caller validates `name`).
    pub fn join(&self, name: &str) -> StatePath {
        StatePath(self.0.join(name))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DestKind {
    Root,
    Child,
    Ancestor,
}

/// A path inside an adapter destination: the destination root, one of its
/// project-relative ancestors, or a single validated child of the root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestPath {
    path: PathBuf,
    rel: String,
    kind: DestKind,
}

impl DestPath {
    /// The destination root `<project_root>/<rel>`. `rel` must be relative,
    /// non-empty, and made of at most 8 normal components.
    pub fn dest_root(project_root: &Path, rel: &str) -> std::result::Result<DestPath, String> {
        let parts = rel_components(rel)?;
        let mut path = project_root.to_path_buf();
        for p in &parts {
            path.push(p);
        }
        Ok(DestPath {
            path,
            rel: parts.join("/"),
            kind: DestKind::Root,
        })
    }

    /// The child `<root>/<name>` after `validate_dest_name`. Only a root has
    /// children; the error string is the reason.
    pub fn join_child(&self, name: &str) -> std::result::Result<DestPath, String> {
        if self.kind != DestKind::Root {
            return Err("only a destination root has children".to_string());
        }
        validate_dest_name(name)?;
        let mut parts = Path::new(name).components();
        match (parts.next(), parts.next()) {
            (Some(Component::Normal(_)), None) => {}
            _ => return Err("not a single path component".to_string()),
        }
        Ok(DestPath {
            path: self.path.join(name),
            rel: format!("{}/{name}", self.rel),
            kind: DestKind::Child,
        })
    }

    /// Each project-relative directory prefix of the root, shallowest first
    /// (for `.claude/skills`: `.claude`, then `.claude/skills`), with its
    /// typed path. Empty for a path that is not a root.
    pub fn ancestors(&self) -> Vec<(String, DestPath)> {
        if self.kind != DestKind::Root {
            return Vec::new();
        }
        let prefixes = dest_prefixes(&self.rel);
        let depth = prefixes.len();
        let mut base = self.path.clone();
        for _ in 0..depth {
            base.pop();
        }
        prefixes
            .into_iter()
            .map(|p| {
                let path = base.join(&p);
                (
                    p.clone(),
                    DestPath {
                        path,
                        rel: p,
                        kind: DestKind::Ancestor,
                    },
                )
            })
            .collect()
    }

    /// The absolute path (read access).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The project-relative text (`.claude/skills/code-review`).
    pub fn rel(&self) -> &str {
        &self.rel
    }

    /// The child name, for a child.
    pub fn name(&self) -> Option<&str> {
        if self.kind == DestKind::Child {
            self.rel.rsplit('/').next()
        } else {
            None
        }
    }
}

impl sealed::Sealed for DestPath {}
impl Writable for DestPath {
    fn write_target(&self) -> &Path {
        &self.path
    }
}
impl sealed::Sealed for StatePath {}
impl Writable for StatePath {
    fn write_target(&self) -> &Path {
        &self.0
    }
}
impl sealed::Sealed for ToolPath {}
impl Writable for ToolPath {
    fn write_target(&self) -> &Path {
        &self.0
    }
}

fn rel_components(rel: &str) -> std::result::Result<Vec<String>, String> {
    if rel.is_empty() {
        return Err("empty destination".to_string());
    }
    if Path::new(rel).is_absolute() || rel.starts_with('/') {
        return Err("destination must be relative".to_string());
    }
    let parts: Vec<String> = rel.split('/').map(str::to_string).collect();
    if parts.len() > 8 {
        return Err("destination is deeper than 8 components".to_string());
    }
    for p in &parts {
        if p.is_empty() || p == "." || p == ".." || p.contains('\\') || p.contains('\0') {
            return Err(format!("invalid destination component {p:?}"));
        }
    }
    Ok(parts)
}

/// The project-relative directory prefixes of `rel`, shallowest first,
/// without touching the filesystem (`.claude/skills` gives `.claude` and
/// `.claude/skills`). An invalid `rel` gives an empty list.
pub fn dest_prefixes(rel: &str) -> Vec<String> {
    let Ok(parts) = rel_components(rel) else {
        return Vec::new();
    };
    (1..=parts.len()).map(|n| parts[..n].join("/")).collect()
}

/// Maximum destination name length in bytes.
pub const MAX_DEST_NAME_BYTES: usize = 255;

/// Prefix of our own temp directories inside a destination root.
pub const TEMP_PREFIX: &str = ".unam-tmp-";

/// Validate a destination name (plan P3): non-empty, not whitespace-only,
/// at most 255 bytes, not `.` or `..`, no `/`, `\`, NUL, or other control
/// character, and not starting with `.unam-tmp-`.
pub fn validate_dest_name(name: &str) -> std::result::Result<(), String> {
    if name.is_empty() {
        return Err("empty name".to_string());
    }
    if name.trim().is_empty() {
        return Err("whitespace-only name".to_string());
    }
    if name.len() > MAX_DEST_NAME_BYTES {
        return Err("longer than 255 bytes".to_string());
    }
    if name == "." || name == ".." {
        return Err(format!("`{name}` is not allowed"));
    }
    if name.contains('/') || name.contains('\\') {
        return Err("path separator in name".to_string());
    }
    if name.chars().any(char::is_control) {
        return Err("control character in name".to_string());
    }
    if name.starts_with(TEMP_PREFIX) {
        return Err("reserved `.unam-tmp-` prefix".to_string());
    }
    Ok(())
}

fn io_err(context: String) -> impl FnOnce(std::io::Error) -> Error {
    move |e| Error::io(context, e)
}

/// Fsync a directory; `Unsupported` and `InvalidInput` are ignored.
fn sync_dir(dir: &Path) -> Result<()> {
    let r = File::open(dir).and_then(|f| f.sync_all());
    match r {
        Ok(()) => Ok(()),
        Err(e) if matches!(e.kind(), ErrorKind::Unsupported | ErrorKind::InvalidInput) => Ok(()),
        Err(e) => Err(Error::io(format!("cannot sync {}", dir.display()), e)),
    }
}

fn parent_of(path: &Path) -> &Path {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

fn write_atomic_raw(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = parent_of(path);
    std::fs::create_dir_all(parent).map_err(io_err(format!(
        "cannot create directory {}",
        parent.display()
    )))?;
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(io_err(format!(
        "cannot create temp file in {}",
        parent.display()
    )))?;
    tmp.write_all(bytes)
        .map_err(io_err(format!("cannot write {}", path.display())))?;
    tmp.as_file()
        .sync_all()
        .map_err(io_err(format!("cannot sync {}", path.display())))?;
    tmp.persist(path)
        .map_err(|e| Error::io(format!("cannot replace {}", path.display()), e.error))?;
    sync_dir(parent)
}

/// Write `bytes` atomically: parent directories are created, the bytes go
/// to a temp file in the same directory, which is fsynced and renamed over
/// the target; then the parent directory is fsynced. An interruption leaves
/// the old or the new file.
pub fn write_atomic<P: Writable>(p: &P, bytes: &[u8]) -> Result<()> {
    write_atomic_raw(p.write_target(), bytes)
}

/// Open the lock file read-write without truncating it (plan P5, G1). A
/// symlink (dangling or not) or any entry that is not a regular file at the
/// lock path is `StateDirInvalid` and nothing is created or opened.
pub fn open_lock_file<P: Writable>(p: &P) -> Result<File> {
    let path = p.write_target();
    match std::fs::symlink_metadata(path) {
        Ok(m) => {
            if m.file_type().is_symlink() || !m.file_type().is_file() {
                return Err(Error::StateDirInvalid {
                    path: path.to_path_buf(),
                    reason: "the lock path is a symlink or not a regular file".to_string(),
                });
            }
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(Error::io(format!("cannot inspect {}", path.display()), e)),
    }
    let parent = parent_of(path);
    if let Ok(m) = std::fs::symlink_metadata(parent)
        && !m.file_type().is_dir()
    {
        return Err(Error::StateDirInvalid {
            path: parent.to_path_buf(),
            reason: "the state directory is a symlink or not a directory".to_string(),
        });
    }
    std::fs::create_dir_all(parent).map_err(io_err(format!(
        "cannot create directory {}",
        parent.display()
    )))?;
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(io_err(format!("cannot open {}", path.display())))
}

/// The open lock file and the entry at the lock path must be the same
/// regular file (device and inode), else `StateDirInvalid`.
pub fn verify_lock_identity(file: &File, p: &StatePath) -> Result<()> {
    let changed = || Error::StateDirInvalid {
        path: p.path().to_path_buf(),
        reason: "lock file changed while opening".to_string(),
    };
    let on_disk = std::fs::symlink_metadata(p.path()).map_err(|_| changed())?;
    if !on_disk.file_type().is_file() {
        return Err(changed());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let open = file
            .metadata()
            .map_err(io_err(format!("cannot stat {}", p.path().display())))?;
        if open.dev() != on_disk.dev() || open.ino() != on_disk.ino() {
            return Err(changed());
        }
    }
    #[cfg(not(unix))]
    {
        let _ = file;
    }
    Ok(())
}

/// What `remove_owned` expects to find (decision n).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedExpectation {
    pub strategy: Strategy,
    /// The absolute source path text (the symlink text for `symlink`).
    pub source: String,
    /// Required for `copy`.
    pub tree_hash: Option<Hash>,
}

/// Why a step was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    Modified,
    Foreign,
    Missing,
    CrossDevice,
    DestOccupied,
    SourceChanged,
}

impl SkipReason {
    pub fn code(self) -> &'static str {
        match self {
            SkipReason::Modified => "modified",
            SkipReason::Foreign => "foreign",
            SkipReason::Missing => "missing",
            SkipReason::CrossDevice => "cross-device",
            SkipReason::DestOccupied => "dest-occupied",
            SkipReason::SourceChanged => "source-changed",
        }
    }

    pub fn parse(code: &str) -> Option<SkipReason> {
        [
            SkipReason::Modified,
            SkipReason::Foreign,
            SkipReason::Missing,
            SkipReason::CrossDevice,
            SkipReason::DestOccupied,
            SkipReason::SourceChanged,
        ]
        .into_iter()
        .find(|r| r.code() == code)
    }
}

/// Result of `remove_owned`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveOutcome {
    Removed,
    RemovedWithResidue { trash: PathBuf },
    Skipped(SkipReason),
}

/// Result of `restore_owned`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestoreOutcome {
    Restored,
    NotRestored(String),
}

/// What sits at a destination path (lstat only; never followed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservedKind {
    Missing,
    Symlink { target: String, resolves: bool },
    Dir { tree_hash: Option<Hash> },
    File,
    Other,
}

#[cfg(unix)]
fn link_text(path: &Path) -> std::io::Result<String> {
    use std::os::unix::ffi::OsStrExt;
    let target = std::fs::read_link(path)?;
    let bytes = target.as_os_str().as_bytes().to_vec();
    Ok(match String::from_utf8(bytes) {
        Ok(s) => s,
        // Never equal to an expected path text (which cannot hold NUL).
        Err(e) => format!("\0invalid-utf8:{}", String::from_utf8_lossy(e.as_bytes())),
    })
}

#[cfg(not(unix))]
fn link_text(path: &Path) -> std::io::Result<String> {
    Ok(std::fs::read_link(path)?.to_string_lossy().into_owned())
}

fn observe_path(path: &Path, want_hash: bool) -> Result<ObservedKind> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(ObservedKind::Missing),
        Err(e) => return Err(Error::io(format!("cannot inspect {}", path.display()), e)),
    };
    let ft = meta.file_type();
    if ft.is_symlink() {
        let target =
            link_text(path).map_err(io_err(format!("cannot read link {}", path.display())))?;
        return Ok(ObservedKind::Symlink {
            target,
            resolves: std::fs::metadata(path).is_ok(),
        });
    }
    if ft.is_dir() {
        let tree_hash = if want_hash {
            Some(tree_hash(path, SKILL_SKIP_NAMES)?)
        } else {
            None
        };
        return Ok(ObservedKind::Dir { tree_hash });
    }
    if ft.is_file() {
        return Ok(ObservedKind::File);
    }
    Ok(ObservedKind::Other)
}

/// The file a bundle brief directory holds (M5-8).
pub const BRIEF_FILE: &str = "SKILL.md";

/// What sits at a bundle-brief destination (lstat only; never followed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BriefObserved {
    Missing,
    /// A real directory holding exactly one regular file, `SKILL.md`; the
    /// hash of its bytes.
    Brief {
        hash: Hash,
    },
    /// Anything else (`directory` for a directory of another shape).
    Other {
        found: &'static str,
    },
}

fn brief_at(path: &Path) -> Result<BriefObserved> {
    match observe_path(path, false)? {
        ObservedKind::Missing => Ok(BriefObserved::Missing),
        ObservedKind::Symlink { .. } => Ok(BriefObserved::Other { found: "symlink" }),
        ObservedKind::File => Ok(BriefObserved::Other { found: "file" }),
        ObservedKind::Other => Ok(BriefObserved::Other { found: "other" }),
        ObservedKind::Dir { .. } => {
            let rd = std::fs::read_dir(path)
                .map_err(io_err(format!("cannot read {}", path.display())))?;
            let mut names: Vec<std::ffi::OsString> = Vec::new();
            for e in rd {
                let e = e.map_err(io_err(format!("cannot read {}", path.display())))?;
                let name = e.file_name();
                if SKILL_SKIP_NAMES.contains(&name.to_string_lossy().as_ref()) {
                    continue;
                }
                names.push(name);
                if names.len() > 1 {
                    break;
                }
            }
            let file = path.join(BRIEF_FILE);
            let shaped = names.len() == 1
                && names[0] == BRIEF_FILE
                && std::fs::symlink_metadata(&file)
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false);
            if !shaped {
                return Ok(BriefObserved::Other { found: "directory" });
            }
            let bytes =
                std::fs::read(&file).map_err(io_err(format!("cannot read {}", file.display())))?;
            Ok(BriefObserved::Brief {
                hash: Hash::of_bytes(&bytes),
            })
        }
    }
}

/// `observe_brief` for an arbitrary path (a trash copy); read-only.
pub fn observe_brief_at(path: &Path) -> Result<BriefObserved> {
    brief_at(path)
}

/// Observe a bundle-brief destination: absent, a directory holding exactly
/// `SKILL.md` (with the hash of its bytes), or something else.
pub fn observe_brief(dst: &DestPath) -> Result<BriefObserved> {
    brief_at(dst.path())
}

/// Observe one destination path with `symlink_metadata` (never followed);
/// the tree hash (with `SKILL_SKIP_NAMES`) only when `want_hash` and the
/// entry is a real directory.
pub fn observe_one(dst: &DestPath, want_hash: bool) -> Result<ObservedKind> {
    observe_path(dst.path(), want_hash)
}

/// The filesystem seam (plan P2). `RealFs` is the only production
/// implementation; tests wrap it to fail or crash at any mutating call.
pub trait FsOps {
    fn write_state(&self, p: &StatePath, bytes: &[u8]) -> Result<()>;
    fn append_state(&self, p: &StatePath, bytes: &[u8]) -> Result<()>;
    fn remove_state(&self, p: &StatePath) -> Result<()>;
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool>;
    fn create_dir(&self, dir: &DestPath) -> Result<bool>;
    fn rmdir_if_empty(&self, dir: &DestPath) -> Result<bool>;
    fn symlink_dir(&self, src: &SourcePath, dst: &DestPath) -> Result<()>;
    fn copy_tree(&self, src: &SourcePath, dst: &DestPath, tag: &str, expect: &Hash)
    -> Result<Hash>;
    fn remove_owned(
        &self,
        dst: &DestPath,
        expect: &OwnedExpectation,
        trash: &StatePath,
    ) -> Result<RemoveOutcome>;
    fn restore_owned(
        &self,
        dst: &DestPath,
        expect: &OwnedExpectation,
        trash: &StatePath,
        source: Option<&SourcePath>,
        tag: &str,
    ) -> Result<RestoreOutcome>;
    /// Create the bundle-brief directory `dst` holding only `SKILL.md` with
    /// `content`, atomically (a temp directory renamed into place). `dst`
    /// must be absent.
    fn write_generated(&self, dst: &DestPath, content: &[u8], tag: &str) -> Result<()>;
    /// Move the bundle-brief directory `dst` into `trash` when it still is a
    /// directory holding exactly `SKILL.md` whose bytes hash to `expect`;
    /// otherwise skip and leave it.
    fn remove_generated(
        &self,
        dst: &DestPath,
        expect: &Hash,
        trash: &StatePath,
    ) -> Result<RemoveOutcome>;
}

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealFs;

impl FsOps for RealFs {
    fn write_state(&self, p: &StatePath, bytes: &[u8]) -> Result<()> {
        write_atomic(p, bytes)
    }

    fn append_state(&self, p: &StatePath, bytes: &[u8]) -> Result<()> {
        let path = p.path();
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(io_err(format!("cannot open {}", path.display())))?;
        f.write_all(bytes)
            .map_err(io_err(format!("cannot append to {}", path.display())))?;
        f.sync_all()
            .map_err(io_err(format!("cannot sync {}", path.display())))
    }

    fn remove_state(&self, p: &StatePath) -> Result<()> {
        let path = p.path();
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(format!("cannot remove {}", path.display()), e)),
        }
        sync_dir(parent_of(path))
    }

    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
        let path = p.path();
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_dir() => {}
            Ok(_) => return Ok(false),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(Error::io(format!("cannot inspect {}", path.display()), e)),
        }
        match std::fs::remove_dir(path) {
            Ok(()) => Ok(true),
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty) => {
                Ok(false)
            }
            Err(e) => Err(Error::io(format!("cannot remove {}", path.display()), e)),
        }
    }

    fn create_dir(&self, dir: &DestPath) -> Result<bool> {
        let path = dir.path();
        match std::fs::create_dir(path) {
            Ok(()) => {
                let _ = sync_dir(parent_of(path));
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                match std::fs::symlink_metadata(path) {
                    Ok(m) if m.file_type().is_dir() => Ok(false),
                    _ => Err(Error::io(
                        format!("cannot create directory {}", path.display()),
                        e,
                    )),
                }
            }
            Err(e) => Err(Error::io(
                format!("cannot create directory {}", path.display()),
                e,
            )),
        }
    }

    fn rmdir_if_empty(&self, dir: &DestPath) -> Result<bool> {
        let path = dir.path();
        match std::fs::symlink_metadata(path) {
            Ok(m) if m.file_type().is_dir() => {}
            Ok(_) => return Ok(false),
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(Error::io(format!("cannot inspect {}", path.display()), e)),
        }
        match std::fs::remove_dir(path) {
            Ok(()) => {
                let _ = sync_dir(parent_of(path));
                Ok(true)
            }
            Err(e) if matches!(e.kind(), ErrorKind::NotFound | ErrorKind::DirectoryNotEmpty) => {
                Ok(false)
            }
            Err(e) => Err(Error::io(format!("cannot remove {}", path.display()), e)),
        }
    }

    fn symlink_dir(&self, src: &SourcePath, dst: &DestPath) -> Result<()> {
        make_symlink(src.as_path(), dst.path())
    }

    fn write_generated(&self, dst: &DestPath, content: &[u8], tag: &str) -> Result<()> {
        write_generated_at(dst.path(), content, tag)
    }

    fn remove_generated(
        &self,
        dst: &DestPath,
        expect: &Hash,
        trash: &StatePath,
    ) -> Result<RemoveOutcome> {
        remove_generated_at(dst.path(), expect, trash.path())
    }

    fn copy_tree(
        &self,
        src: &SourcePath,
        dst: &DestPath,
        tag: &str,
        expect: &Hash,
    ) -> Result<Hash> {
        copy_skill(src.as_path(), dst.path(), tag, expect)
    }

    fn remove_owned(
        &self,
        dst: &DestPath,
        expect: &OwnedExpectation,
        trash: &StatePath,
    ) -> Result<RemoveOutcome> {
        remove_owned_at(dst.path(), expect, trash.path(), &|| {})
    }

    fn restore_owned(
        &self,
        dst: &DestPath,
        expect: &OwnedExpectation,
        trash: &StatePath,
        source: Option<&SourcePath>,
        tag: &str,
    ) -> Result<RestoreOutcome> {
        let path = dst.path();
        if observe_path(path, false)? != ObservedKind::Missing {
            return Ok(RestoreOutcome::NotRestored("dest-occupied".to_string()));
        }
        match expect.strategy {
            Strategy::Copy => {
                let Some(want) = &expect.tree_hash else {
                    return Ok(RestoreOutcome::NotRestored(
                        "content-not-recoverable".to_string(),
                    ));
                };
                let name = file_name_text(path)?;
                let item = trash.path().join(&name);
                if let ObservedKind::Dir { tree_hash: Some(h) } = observe_path(&item, true)?
                    && h == *want
                {
                    std::fs::rename(&item, path)
                        .map_err(io_err(format!("cannot restore {}", path.display())))?;
                    let _ = sync_dir(parent_of(path));
                    let _ = std::fs::remove_dir(trash.path());
                    return Ok(RestoreOutcome::Restored);
                }
                match source {
                    Some(s) => {
                        copy_skill(s.as_path(), path, tag, want)?;
                        Ok(RestoreOutcome::Restored)
                    }
                    None => Ok(RestoreOutcome::NotRestored(
                        "content-not-recoverable".to_string(),
                    )),
                }
            }
            Strategy::Symlink => match source {
                Some(s) => {
                    make_symlink(s.as_path(), path)?;
                    Ok(RestoreOutcome::Restored)
                }
                None => Ok(RestoreOutcome::NotRestored(
                    "source-unavailable".to_string(),
                )),
            },
        }
    }
}

fn write_generated_at(dst: &Path, content: &[u8], tag: &str) -> Result<()> {
    if brief_at(dst)? != BriefObserved::Missing {
        return Err(Error::io(
            format!("destination {} already exists", dst.display()),
            std::io::Error::from(ErrorKind::AlreadyExists),
        ));
    }
    let name = file_name_text(dst)?;
    let temp = parent_of(dst).join(format!("{TEMP_PREFIX}{tag}-{name}"));
    if observe_path(&temp, false)? != ObservedKind::Missing {
        return Err(Error::io(
            format!("temp path {} already exists", temp.display()),
            std::io::Error::from(ErrorKind::AlreadyExists),
        ));
    }
    let result = (|| -> Result<()> {
        std::fs::create_dir(&temp).map_err(io_err(format!("cannot create {}", temp.display())))?;
        let file = temp.join(BRIEF_FILE);
        let mut f =
            File::create(&file).map_err(io_err(format!("cannot create {}", file.display())))?;
        f.write_all(content)
            .map_err(io_err(format!("cannot write {}", file.display())))?;
        f.sync_all()
            .map_err(io_err(format!("cannot sync {}", file.display())))?;
        sync_dir(&temp)?;
        if observe_path(dst, false)? != ObservedKind::Missing {
            return Err(Error::io(
                format!("destination {} appeared during the write", dst.display()),
                std::io::Error::from(ErrorKind::AlreadyExists),
            ));
        }
        std::fs::rename(&temp, dst).map_err(io_err(format!(
            "cannot move the brief to {}",
            dst.display()
        )))?;
        Ok(())
    })();
    match result {
        Ok(()) => {
            let _ = sync_dir(parent_of(dst));
            Ok(())
        }
        Err(e) => {
            let _ = remove_own_temp_dir(&temp);
            Err(e)
        }
    }
}

fn remove_generated_at(dst: &Path, expect: &Hash, trash: &Path) -> Result<RemoveOutcome> {
    match brief_at(dst)? {
        BriefObserved::Missing => Ok(RemoveOutcome::Skipped(SkipReason::Missing)),
        BriefObserved::Other { found: "directory" } => {
            Ok(RemoveOutcome::Skipped(SkipReason::Modified))
        }
        BriefObserved::Other { .. } => Ok(RemoveOutcome::Skipped(SkipReason::Foreign)),
        BriefObserved::Brief { hash } if hash != *expect => {
            Ok(RemoveOutcome::Skipped(SkipReason::Modified))
        }
        BriefObserved::Brief { .. } => {
            // Skip-listed entries are ignored when observing. A `.git` is
            // never deleted (the brief is then Modified); a `.DS_Store` is
            // editor/OS noise and goes with the brief.
            if std::fs::symlink_metadata(dst.join(".git")).is_ok() {
                return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
            }
            let noise = dst.join(".DS_Store");
            match std::fs::symlink_metadata(&noise) {
                Ok(m) if m.file_type().is_file() => std::fs::remove_file(&noise)
                    .map_err(io_err(format!("cannot remove {}", noise.display())))?,
                Ok(_) => return Ok(RemoveOutcome::Skipped(SkipReason::Modified)),
                Err(e) if e.kind() == ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(Error::io(format!("cannot inspect {}", noise.display()), e));
                }
            }
            std::fs::create_dir_all(trash)
                .map_err(io_err(format!("cannot create {}", trash.display())))?;
            let item = trash.join(file_name_text(dst)?);
            if observe_path(&item, false)? != ObservedKind::Missing {
                return Err(Error::io(
                    format!("trash path {} already exists", item.display()),
                    std::io::Error::from(ErrorKind::AlreadyExists),
                ));
            }
            match std::fs::rename(dst, &item) {
                Ok(()) => {}
                Err(e) if rename_aside_skip(e.kind()).is_some() => {
                    let _ = std::fs::remove_dir(trash);
                    return Ok(RemoveOutcome::Skipped(SkipReason::CrossDevice));
                }
                Err(e) => {
                    return Err(Error::io(format!("cannot move {} aside", dst.display()), e));
                }
            }
            match brief_at(&item)? {
                BriefObserved::Brief { hash } if hash == *expect => {}
                _ => {
                    if observe_path(dst, false)? == ObservedKind::Missing {
                        std::fs::rename(&item, dst)
                            .map_err(io_err(format!("cannot move {} back", dst.display())))?;
                        let _ = std::fs::remove_dir(trash);
                    }
                    return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
                }
            }
            let _ = sync_dir(parent_of(dst));
            Ok(RemoveOutcome::Removed)
        }
    }
}

fn file_name_text(path: &Path) -> Result<String> {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| {
            Error::io(
                format!("no file name in {}", path.display()),
                std::io::Error::from(ErrorKind::InvalidInput),
            )
        })
}

#[cfg(unix)]
fn make_symlink(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_absolute() {
        return Err(Error::io(
            format!("symlink source {} is not absolute", src.display()),
            std::io::Error::from(ErrorKind::InvalidInput),
        ));
    }
    match std::os::unix::fs::symlink(src, dst) {
        Ok(()) => {
            let _ = sync_dir(parent_of(dst));
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::Unsupported => Err(Error::SymlinkUnsupported {
            dest: dst.to_path_buf(),
            source: e,
        }),
        Err(e) => Err(Error::io(
            format!("cannot create link {}", dst.display()),
            e,
        )),
    }
}

#[cfg(not(unix))]
fn make_symlink(_src: &Path, dst: &Path) -> Result<()> {
    Err(Error::SymlinkUnsupported {
        dest: dst.to_path_buf(),
        source: std::io::Error::from(ErrorKind::Unsupported),
    })
}

/// One entry of a source skill listing (relative path, kind).
enum SrcEntry {
    Dir(String),
    File(String),
    Link(String, PathBuf),
}

fn refuse(path: &Path, reason: &str) -> Error {
    Error::CopyRefused {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

/// True when `target` (a link at `link_rel` inside the skill) stays inside
/// the skill root by lexical resolution.
fn link_stays_inside(link_rel: &str, target: &Path) -> bool {
    if target.is_absolute() {
        return false;
    }
    let mut stack: Vec<String> = link_rel.split('/').map(str::to_string).collect();
    stack.pop();
    for c in target.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if stack.pop().is_none() {
                    return false;
                }
            }
            Component::Normal(n) => stack.push(n.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn list_source(root: &Path, canonical_root: &Path) -> Result<Vec<SrcEntry>> {
    let mut out = Vec::new();
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, rel)) = stack.pop() {
        let rd =
            std::fs::read_dir(&dir).map_err(io_err(format!("cannot read {}", dir.display())))?;
        let mut children: Vec<_> = rd
            .collect::<std::io::Result<Vec<_>>>()
            .map_err(io_err(format!("cannot read {}", dir.display())))?;
        children.sort_by_key(|e| e.file_name());
        for entry in children {
            let name = entry.file_name().to_string_lossy().into_owned();
            if SKILL_SKIP_NAMES.contains(&name.as_str()) {
                continue;
            }
            let path = entry.path();
            let child_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let meta = std::fs::symlink_metadata(&path)
                .map_err(io_err(format!("cannot inspect {}", path.display())))?;
            let ft = meta.file_type();
            if ft.is_symlink() {
                let target = std::fs::read_link(&path)
                    .map_err(io_err(format!("cannot read link {}", path.display())))?;
                if !link_stays_inside(&child_rel, &target) {
                    return Err(refuse(&path, "link leaves the skill directory"));
                }
                if let Ok(resolved) = std::fs::canonicalize(&path)
                    && !resolved.starts_with(canonical_root)
                {
                    return Err(refuse(&path, "link leaves the skill directory"));
                }
                out.push(SrcEntry::Link(child_rel, target));
            } else if ft.is_dir() {
                out.push(SrcEntry::Dir(child_rel.clone()));
                stack.push((path, child_rel));
            } else if ft.is_file() {
                out.push(SrcEntry::File(child_rel));
            } else {
                return Err(refuse(&path, "special file"));
            }
        }
    }
    Ok(out)
}

fn copy_into(src: &Path, temp: &Path, entries: &[SrcEntry]) -> Result<()> {
    std::fs::create_dir(temp).map_err(io_err(format!("cannot create {}", temp.display())))?;
    for e in entries {
        if let SrcEntry::Dir(rel) = e {
            let d = temp.join(rel);
            std::fs::create_dir_all(&d)
                .map_err(io_err(format!("cannot create {}", d.display())))?;
        }
    }
    for e in entries {
        match e {
            SrcEntry::File(rel) => {
                let from = src.join(rel);
                let to = temp.join(rel);
                std::fs::copy(&from, &to)
                    .map_err(io_err(format!("cannot copy {}", from.display())))?;
                File::open(&to)
                    .and_then(|f| f.sync_all())
                    .map_err(io_err(format!("cannot sync {}", to.display())))?;
            }
            SrcEntry::Link(rel, target) => {
                let to = temp.join(rel);
                #[cfg(unix)]
                std::os::unix::fs::symlink(target, &to)
                    .map_err(io_err(format!("cannot create link {}", to.display())))?;
                #[cfg(not(unix))]
                {
                    let _ = target;
                    return Err(Error::SymlinkUnsupported {
                        dest: to,
                        source: std::io::Error::from(ErrorKind::Unsupported),
                    });
                }
            }
            SrcEntry::Dir(_) => {}
        }
    }
    // Directory permission bits last, deepest first, so read-only source
    // directories do not block their own children.
    let mut dirs: Vec<&String> = entries
        .iter()
        .filter_map(|e| match e {
            SrcEntry::Dir(rel) => Some(rel),
            _ => None,
        })
        .collect();
    dirs.sort_by_key(|r| std::cmp::Reverse(r.matches('/').count()));
    for rel in dirs {
        let from = src.join(rel);
        let perms = std::fs::metadata(&from)
            .map_err(io_err(format!("cannot stat {}", from.display())))?
            .permissions();
        let to = temp.join(rel);
        std::fs::set_permissions(&to, perms).map_err(io_err(format!(
            "cannot set permissions of {}",
            to.display()
        )))?;
    }
    Ok(())
}

fn copy_skill(src: &Path, dst: &Path, tag: &str, expect: &Hash) -> Result<Hash> {
    if observe_path(dst, false)? != ObservedKind::Missing {
        return Err(Error::io(
            format!("destination {} already exists", dst.display()),
            std::io::Error::from(ErrorKind::AlreadyExists),
        ));
    }
    let name = file_name_text(dst)?;
    let temp = parent_of(dst).join(format!("{TEMP_PREFIX}{tag}-{name}"));
    if observe_path(&temp, false)? != ObservedKind::Missing {
        return Err(Error::io(
            format!("temp path {} already exists", temp.display()),
            std::io::Error::from(ErrorKind::AlreadyExists),
        ));
    }
    let canonical_root =
        std::fs::canonicalize(src).map_err(io_err(format!("cannot resolve {}", src.display())))?;
    let entries = list_source(src, &canonical_root)?;
    let result = (|| -> Result<Hash> {
        copy_into(src, &temp, &entries)?;
        let got = tree_hash(&temp, SKILL_SKIP_NAMES)?;
        if got != *expect {
            return Err(Error::SourceChanged {
                id: src.to_string_lossy().into_owned(),
                reason: "the copied tree hash differs from the catalog hash".to_string(),
            });
        }
        if observe_path(dst, false)? != ObservedKind::Missing {
            return Err(Error::io(
                format!("destination {} appeared during the copy", dst.display()),
                std::io::Error::from(ErrorKind::AlreadyExists),
            ));
        }
        std::fs::rename(&temp, dst)
            .map_err(io_err(format!("cannot move the copy to {}", dst.display())))?;
        Ok(got)
    })();
    match result {
        Ok(h) => {
            let _ = sync_dir(parent_of(dst));
            Ok(h)
        }
        Err(e) => {
            let _ = remove_own_temp_dir(&temp);
            Err(e)
        }
    }
}

/// Remove our own temp directory `.unam-tmp-*` after lstat shows a real
/// directory (plan P30: the single recursive removal in production code).
fn remove_own_temp_dir(path: &Path) -> Result<bool> {
    let ours = path
        .file_name()
        .map(|n| n.to_string_lossy().starts_with(TEMP_PREFIX))
        .unwrap_or(false);
    if !ours {
        return Ok(false);
    }
    match std::fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_dir() => {}
        Ok(_) => return Ok(false),
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(Error::io(format!("cannot inspect {}", path.display()), e)),
    }
    std::fs::remove_dir_all(path).map_err(io_err(format!("cannot remove {}", path.display())))?;
    Ok(true)
}

/// Remove stale `.unam-tmp-<op_id>-*` directories of one operation in the
/// destination root (resume and rollback). Only real directories with our
/// own name pattern are removed; returns how many were removed.
pub fn remove_stale_temp_dirs(dest_root: &DestPath, op_id: &str) -> Result<usize> {
    let prefix = format!("{TEMP_PREFIX}{op_id}-");
    let rd = match std::fs::read_dir(dest_root.path()) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => {
            return Err(Error::io(
                format!("cannot read {}", dest_root.path().display()),
                e,
            ));
        }
    };
    let mut n = 0;
    for entry in rd {
        let entry = entry.map_err(io_err(format!(
            "cannot read {}",
            dest_root.path().display()
        )))?;
        if entry.file_name().to_string_lossy().starts_with(&prefix)
            && remove_own_temp_dir(&entry.path())?
        {
            n += 1;
        }
    }
    Ok(n)
}

/// `remove_owned` with a hook that runs between the first check and the
/// rename-aside (tests use it to change the copy in that window).
fn remove_owned_at(
    dst: &Path,
    expect: &OwnedExpectation,
    trash: &Path,
    before_rename: &dyn Fn(),
) -> Result<RemoveOutcome> {
    let want_hash = expect.strategy == Strategy::Copy;
    let seen = observe_path(dst, want_hash)?;
    if seen == ObservedKind::Missing {
        return Ok(RemoveOutcome::Skipped(SkipReason::Missing));
    }
    match expect.strategy {
        Strategy::Symlink => {
            let ObservedKind::Symlink { target, .. } = &seen else {
                return Ok(RemoveOutcome::Skipped(SkipReason::Foreign));
            };
            if *target != expect.source {
                return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
            }
            match observe_path(dst, false)? {
                ObservedKind::Symlink { target, .. } if target == expect.source => {}
                _ => return Ok(RemoveOutcome::Skipped(SkipReason::Modified)),
            }
            std::fs::remove_file(dst)
                .map_err(io_err(format!("cannot remove link {}", dst.display())))?;
            let _ = sync_dir(parent_of(dst));
            Ok(RemoveOutcome::Removed)
        }
        Strategy::Copy => {
            let ObservedKind::Dir { tree_hash: got } = &seen else {
                return Ok(RemoveOutcome::Skipped(SkipReason::Foreign));
            };
            let Some(want) = &expect.tree_hash else {
                return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
            };
            if got.as_ref() != Some(want) {
                return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
            }
            std::fs::create_dir_all(trash)
                .map_err(io_err(format!("cannot create {}", trash.display())))?;
            let item = trash.join(file_name_text(dst)?);
            before_rename();
            match std::fs::rename(dst, &item) {
                Ok(()) => {}
                Err(e) if rename_aside_skip(e.kind()).is_some() => {
                    let _ = std::fs::remove_dir(trash);
                    return Ok(RemoveOutcome::Skipped(SkipReason::CrossDevice));
                }
                Err(e) => {
                    return Err(Error::io(format!("cannot move {} aside", dst.display()), e));
                }
            }
            let again = tree_hash(&item, SKILL_SKIP_NAMES)?;
            if again != *want {
                if observe_path(dst, false)? == ObservedKind::Missing {
                    std::fs::rename(&item, dst)
                        .map_err(io_err(format!("cannot move {} back", dst.display())))?;
                    let _ = std::fs::remove_dir(trash);
                }
                return Ok(RemoveOutcome::Skipped(SkipReason::Modified));
            }
            let residue = delete_manifest(&item)?;
            let _ = std::fs::remove_dir(trash);
            if residue {
                Ok(RemoveOutcome::RemovedWithResidue { trash: item })
            } else {
                Ok(RemoveOutcome::Removed)
            }
        }
    }
}

/// Delete what the hash covered inside a trash copy (G9): a pruned lstat
/// walk (`SKILL_SKIP_NAMES` are never descended into or listed), regular
/// files and symlinks unlinked, a `.DS_Store` unlinked only when it is a
/// regular file, then directories removed bottom-up. Returns true when
/// anything is left (residue).
fn delete_manifest(item: &Path) -> Result<bool> {
    let mut residue = false;
    let mut dirs: Vec<PathBuf> = vec![item.to_path_buf()];
    let mut stack: Vec<PathBuf> = vec![item.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd =
            std::fs::read_dir(&dir).map_err(io_err(format!("cannot read {}", dir.display())))?;
        for entry in rd {
            let entry = entry.map_err(io_err(format!("cannot read {}", dir.display())))?;
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let meta = std::fs::symlink_metadata(&path)
                .map_err(io_err(format!("cannot inspect {}", path.display())))?;
            let ft = meta.file_type();
            if SKILL_SKIP_NAMES.contains(&name.as_str()) {
                if name == ".DS_Store" && ft.is_file() {
                    std::fs::remove_file(&path)
                        .map_err(io_err(format!("cannot remove {}", path.display())))?;
                } else {
                    residue = true;
                }
                continue;
            }
            if ft.is_symlink() || ft.is_file() {
                std::fs::remove_file(&path)
                    .map_err(io_err(format!("cannot remove {}", path.display())))?;
            } else if ft.is_dir() {
                dirs.push(path.clone());
                stack.push(path);
            } else {
                residue = true;
            }
        }
    }
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    for d in dirs {
        match std::fs::remove_dir(&d) {
            Ok(()) => {}
            Err(e) if e.kind() == ErrorKind::DirectoryNotEmpty => residue = true,
            Err(e) => return Err(Error::io(format!("cannot remove {}", d.display()), e)),
        }
    }
    Ok(residue)
}

/// Map an IO error kind of the rename-aside to a skip reason (unit-tested
/// stand-in for a second filesystem).
pub fn rename_aside_skip(kind: ErrorKind) -> Option<SkipReason> {
    (kind == ErrorKind::CrossesDevices).then_some(SkipReason::CrossDevice)
}

#[cfg(test)]
mod tests;
