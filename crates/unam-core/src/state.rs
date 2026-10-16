//! Project state under `<root>/.unam/` (M3 plan P4, P5, P6, P16, P18):
//! the clock and operation ids, the per-target state directory, the layout
//! check, the project lock, the receipt (`applied.json`), history entries,
//! and the write-ahead journal (`journal.jsonl`).
//!
//! The receipt and the journal are untrusted input: both are validated as a
//! whole before use, and nothing is created or deleted from an invalid one.

use crate::config::Strategy;
use crate::error::{Error, Result};
use crate::fsops::{FsOps, StatePath, dest_prefixes, validate_dest_name};
use crate::hash::Hash;
use crate::project::{TargetId, TargetSelection};
use crate::resolve::Provenance;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::io::{ErrorKind, Read, Seek, Write};
use std::path::{Path, PathBuf};

// ---- clock and operation ids ----

/// Time and entropy source (tests use a fixed or stepping clock).
pub trait Clock {
    fn now_rfc3339(&self) -> String;
    fn now_millis(&self) -> u64;
    fn entropy(&self) -> u32;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_rfc3339(&self) -> String {
        jiff::Timestamp::now().to_string()
    }

    fn now_millis(&self) -> u64 {
        u64::try_from(jiff::Timestamp::now().as_millisecond()).unwrap_or(0)
    }

    fn entropy(&self) -> u32 {
        use std::hash::{BuildHasher, Hasher};
        let mut h = std::hash::RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u32(std::process::id());
        (h.finish() & 0xffff_ffff) as u32
    }
}

/// `{unix_millis:013}-{entropy:08x}`.
pub fn new_op_id(clock: &dyn Clock) -> String {
    format!("{:013}-{:08x}", clock.now_millis(), clock.entropy())
}

/// True when `id` matches `^[0-9]{13}-[0-9a-f]{8}$`.
pub fn valid_op_id(id: &str) -> bool {
    let b = id.as_bytes();
    b.len() == 22
        && b[..13].iter().all(u8::is_ascii_digit)
        && b[13] == b'-'
        && b[14..]
            .iter()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c))
}

fn invalid_op_id(path: &Path, op_id: &str) -> Error {
    Error::JournalInvalid {
        path: path.to_path_buf(),
        reason: format!("invalid operation id {op_id:?}"),
    }
}

// ---- state directory and layout ----

/// `<root>/.unam`.
pub fn unam_dir(root: &Path) -> StatePath {
    StatePath::new(root.join(".unam"))
}

/// `<root>/.unam/lock`.
pub fn lock_path(root: &Path) -> StatePath {
    StatePath::new(root.join(".unam").join("lock"))
}

/// The per-target state directory `<root>/.unam/<target>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDir {
    pub root: PathBuf,
    pub target: TargetId,
}

impl StateDir {
    pub fn new(root: &Path, target: TargetId) -> Self {
        StateDir {
            root: root.to_path_buf(),
            target,
        }
    }

    pub fn dir(&self) -> StatePath {
        StatePath::new(self.root.join(".unam").join(self.target.as_str()))
    }

    pub fn applied(&self) -> StatePath {
        self.dir().join("applied.json")
    }

    pub fn journal(&self) -> StatePath {
        self.dir().join("journal.jsonl")
    }

    pub fn history_dir(&self) -> StatePath {
        self.dir().join("history")
    }

    pub fn trash_root(&self) -> StatePath {
        self.dir().join("trash")
    }

    pub fn history_file(&self, op_id: &str) -> Result<StatePath> {
        if !valid_op_id(op_id) {
            return Err(invalid_op_id(self.history_dir().path(), op_id));
        }
        Ok(self.history_dir().join(&format!("{op_id}.json")))
    }

    pub fn trash_dir(&self, op_id: &str) -> Result<StatePath> {
        if !valid_op_id(op_id) {
            return Err(invalid_op_id(self.trash_root().path(), op_id));
        }
        Ok(self.trash_root().join(op_id))
    }
}

fn layout_error(path: &Path, reason: &str) -> Error {
    Error::StateDirInvalid {
        path: path.to_path_buf(),
        reason: reason.to_string(),
    }
}

fn check_kind(path: &Path, want_dir: bool) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(m) => {
            let ft = m.file_type();
            if ft.is_symlink() {
                return Err(layout_error(path, "is a symlink"));
            }
            if want_dir && !ft.is_dir() {
                return Err(layout_error(path, "is not a directory"));
            }
            if !want_dir && !ft.is_file() {
                return Err(layout_error(path, "is not a regular file"));
            }
            Ok(())
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(format!("cannot inspect {}", path.display()), e)),
    }
}

/// Refuse a symlinked or wrong-kind state entry before anything is opened,
/// created, or written (G1). Absent paths are fine.
pub fn check_state_layout(root: &Path, target: Option<&str>) -> Result<()> {
    let unam = root.join(".unam");
    check_kind(&unam, true)?;
    check_kind(&unam.join("lock"), false)?;
    if let Some(t) = target {
        let d = unam.join(t);
        check_kind(&d, true)?;
        check_kind(&d.join("history"), true)?;
        check_kind(&d.join("trash"), true)?;
        check_kind(&d.join("applied.json"), false)?;
        check_kind(&d.join("journal.jsonl"), false)?;
    }
    Ok(())
}

// ---- lock ----

/// The project lock: held for the whole command, released on drop. The
/// lock file is never deleted.
#[derive(Debug)]
pub struct ProjectLock {
    _file: std::fs::File,
}

fn read_lock_pid(path: &Path) -> Option<u32> {
    let text = std::fs::read_to_string(path).ok()?;
    let v: Value = serde_json::from_str(text.trim()).ok()?;
    v.get("pid")
        .and_then(Value::as_u64)
        .and_then(|p| u32::try_from(p).ok())
}

impl ProjectLock {
    /// Take the project lock (P5, G1): layout check, open without
    /// truncation, `try_lock`, identity check, then write one JSON line.
    pub fn acquire(root: &Path, command: &str, clock: &dyn Clock) -> Result<ProjectLock> {
        check_state_layout(root, None)?;
        let path = lock_path(root);
        let mut file = crate::fsops::open_lock_file(&path)?;
        match file.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => {
                return Err(Error::Locked {
                    pid: read_lock_pid(path.path()),
                });
            }
            Err(std::fs::TryLockError::Error(e)) => {
                return Err(Error::io(
                    format!("cannot lock {}", path.path().display()),
                    e,
                ));
            }
        }
        crate::fsops::verify_lock_identity(&file, &path)?;
        let line = json!({
            "pid": std::process::id(),
            "started_at": clock.now_rfc3339(),
            "command": command,
        })
        .to_string();
        let ctx = format!("cannot write {}", path.path().display());
        file.set_len(0)
            .and_then(|()| file.rewind())
            .and_then(|()| file.write_all(format!("{line}\n").as_bytes()))
            .and_then(|()| file.sync_all())
            .map_err(|e| Error::io(ctx, e))?;
        Ok(ProjectLock { _file: file })
    }
}

/// Read the lock line (diagnostics only).
pub fn read_lock_line(root: &Path) -> Option<String> {
    let mut s = String::new();
    std::fs::File::open(lock_path(root).path())
        .ok()?
        .read_to_string(&mut s)
        .ok()?;
    Some(s)
}

// ---- selection hash ----

/// Blake3 of the compact canonical JSON of a target selection (P6).
pub fn selection_hash(strategy: Strategy, t: &TargetSelection) -> Hash {
    let s = match strategy {
        Strategy::Symlink => "symlink",
        Strategy::Copy => "copy",
    };
    let v = json!({
        "strategy": s,
        "bundles": t.bundles,
        "add": t.add,
        "remove": t.remove,
    });
    Hash::of_bytes(v.to_string().as_bytes())
}

// ---- receipt ----

/// One skill entry of a receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillEntry {
    pub skill_id: String,
    pub name: String,
    pub dest: String,
    pub strategy: Strategy,
    pub source_path: String,
    pub tree_hash: String,
    pub git_commit: Value,
    pub pin: String,
    pub provenance: Vec<Provenance>,
    pub applied_at: String,
    /// Unknown fields, preserved on write.
    pub extra: Map<String, Value>,
}

impl SkillEntry {
    /// The P6 consistency rule: `dest` is exactly `<dest_rel>/<name>` and
    /// `name` passes `validate_dest_name`.
    pub fn consistent(&self, dest_rel: &str) -> bool {
        validate_dest_name(&self.name).is_ok() && self.dest == format!("{dest_rel}/{}", self.name)
    }

    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("kind".into(), json!("skill"));
        m.insert("skill_id".into(), json!(self.skill_id));
        m.insert("name".into(), json!(self.name));
        m.insert("dest".into(), json!(self.dest));
        m.insert("strategy".into(), json!(strategy_text(self.strategy)));
        m.insert("source_path".into(), json!(self.source_path));
        m.insert("tree_hash".into(), json!(self.tree_hash));
        m.insert("git_commit".into(), self.git_commit.clone());
        m.insert("pin".into(), json!(self.pin));
        m.insert(
            "provenance".into(),
            serde_json::to_value(&self.provenance).unwrap_or(Value::Array(vec![])),
        );
        m.insert("applied_at".into(), json!(self.applied_at));
        for (k, v) in &self.extra {
            m.insert(k.clone(), v.clone());
        }
        Value::Object(m)
    }

    fn from_value(v: &Value) -> std::result::Result<SkillEntry, String> {
        let o = v.as_object().ok_or("an entry is not an object")?;
        let s = |k: &str| -> std::result::Result<String, String> {
            o.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("skill entry field `{k}` is missing or not a string"))
        };
        let strategy = match s("strategy")?.as_str() {
            "symlink" => Strategy::Symlink,
            "copy" => Strategy::Copy,
            other => return Err(format!("unknown strategy {other:?}")),
        };
        let provenance: Vec<Provenance> = serde_json::from_value(
            o.get("provenance")
                .cloned()
                .ok_or("skill entry field `provenance` is missing")?,
        )
        .map_err(|e| format!("invalid provenance: {e}"))?;
        let git_commit = o
            .get("git_commit")
            .cloned()
            .ok_or("skill entry field `git_commit` is missing")?;
        const KNOWN: &[&str] = &[
            "kind",
            "skill_id",
            "name",
            "dest",
            "strategy",
            "source_path",
            "tree_hash",
            "git_commit",
            "pin",
            "provenance",
            "applied_at",
        ];
        let extra = o
            .iter()
            .filter(|(k, _)| !KNOWN.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(SkillEntry {
            skill_id: s("skill_id")?,
            name: s("name")?,
            dest: s("dest")?,
            strategy,
            source_path: s("source_path")?,
            tree_hash: s("tree_hash")?,
            git_commit,
            pin: s("pin")?,
            provenance,
            applied_at: s("applied_at")?,
            extra,
        })
    }
}

/// `"symlink"` or `"copy"`.
pub fn strategy_text(s: Strategy) -> &'static str {
    match s {
        Strategy::Symlink => "symlink",
        Strategy::Copy => "copy",
    }
}

/// A receipt entry: a skill, or anything else (carried verbatim).
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    Skill(Box<SkillEntry>),
    Other(Value),
}

impl Entry {
    pub fn to_value(&self) -> Value {
        match self {
            Entry::Skill(s) => s.to_value(),
            Entry::Other(v) => v.clone(),
        }
    }
}

/// The receipt `applied.json` (spec 2.4).
#[derive(Debug, Clone, PartialEq)]
pub struct Receipt {
    pub version: u64,
    pub project_root: String,
    pub target: String,
    pub applied_at: String,
    pub selection_hash: String,
    pub last_op_id: Option<String>,
    pub created_dirs: Vec<String>,
    pub entries: Vec<Entry>,
    /// Unknown top-level fields, preserved on write.
    pub extra: Map<String, Value>,
}

impl Receipt {
    pub fn empty(root: &Path, target: TargetId) -> Receipt {
        Receipt {
            version: 1,
            project_root: root.to_string_lossy().into_owned(),
            target: target.as_str().to_string(),
            applied_at: String::new(),
            selection_hash: String::new(),
            last_op_id: None,
            created_dirs: Vec::new(),
            entries: Vec::new(),
            extra: Map::new(),
        }
    }

    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("version".into(), json!(self.version));
        m.insert("project_root".into(), json!(self.project_root));
        m.insert("target".into(), json!(self.target));
        m.insert("applied_at".into(), json!(self.applied_at));
        m.insert("selection_hash".into(), json!(self.selection_hash));
        m.insert("last_op_id".into(), json!(self.last_op_id));
        m.insert("created_dirs".into(), json!(self.created_dirs));
        m.insert(
            "entries".into(),
            Value::Array(self.entries.iter().map(Entry::to_value).collect()),
        );
        for (k, v) in &self.extra {
            m.insert(k.clone(), v.clone());
        }
        Value::Object(m)
    }

    /// Pretty JSON (2-space indent) with a trailing newline.
    pub fn to_text(&self) -> String {
        let mut s = serde_json::to_string_pretty(&self.to_value()).unwrap_or_default();
        s.push('\n');
        s
    }

    /// Parse a receipt value; `path` names the file in errors.
    pub fn from_value(v: &Value, path: &Path) -> Result<Receipt> {
        let invalid = |message: String| Error::ReceiptInvalid {
            path: path.to_path_buf(),
            message,
        };
        let o = v
            .as_object()
            .ok_or_else(|| invalid("the receipt is not a JSON object".into()))?;
        let version = o
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| invalid("missing `version`".into()))?;
        if version != 1 {
            return Err(Error::ReceiptUnsupportedVersion {
                path: path.to_path_buf(),
                version,
            });
        }
        let s = |k: &str| -> Result<String> {
            o.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| invalid(format!("field `{k}` is missing or not a string")))
        };
        let last_op_id = match o.get("last_op_id") {
            None | Some(Value::Null) => None,
            Some(Value::String(x)) => Some(x.clone()),
            Some(_) => return Err(invalid("field `last_op_id` is not a string".into())),
        };
        let created_dirs = match o.get("created_dirs") {
            None => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .map(|x| {
                    x.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| invalid("`created_dirs` holds a non-string".into()))
                })
                .collect::<Result<Vec<_>>>()?,
            Some(_) => return Err(invalid("field `created_dirs` is not an array".into())),
        };
        let raw_entries = o
            .get("entries")
            .and_then(Value::as_array)
            .ok_or_else(|| invalid("field `entries` is missing or not an array".into()))?;
        let mut entries = Vec::new();
        for e in raw_entries {
            if e.get("kind").and_then(Value::as_str) == Some("skill") {
                entries.push(Entry::Skill(Box::new(
                    SkillEntry::from_value(e).map_err(invalid)?,
                )));
            } else {
                entries.push(Entry::Other(e.clone()));
            }
        }
        const KNOWN: &[&str] = &[
            "version",
            "project_root",
            "target",
            "applied_at",
            "selection_hash",
            "last_op_id",
            "created_dirs",
            "entries",
        ];
        let extra = o
            .iter()
            .filter(|(k, _)| !KNOWN.contains(&k.as_str()))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        Ok(Receipt {
            version,
            project_root: s("project_root")?,
            target: s("target")?,
            applied_at: s("applied_at")?,
            selection_hash: s("selection_hash")?,
            last_op_id,
            created_dirs,
            entries,
            extra,
        })
    }

    /// Parse receipt text (an empty or malformed file is `ReceiptInvalid`).
    pub fn parse(text: &str, path: &Path) -> Result<Receipt> {
        let v: Value = serde_json::from_str(text).map_err(|e| Error::ReceiptInvalid {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        Receipt::from_value(&v, path)
    }

    /// Load the receipt of `state` (absent gives `empty`); a receipt of
    /// another target is `ReceiptInvalid` (G10).
    pub fn load(state: &StateDir, root: &Path, target: TargetId) -> Result<Receipt> {
        let path = state.applied();
        let text = match std::fs::read_to_string(path.path()) {
            Ok(t) => t,
            Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Receipt::empty(root, target)),
            Err(e) => {
                return Err(Error::io(
                    format!("cannot read {}", path.path().display()),
                    e,
                ));
            }
        };
        let r = Receipt::parse(&text, path.path())?;
        if r.target != target.as_str() {
            return Err(Error::ReceiptInvalid {
                path: path.path().to_path_buf(),
                message: format!(
                    "the receipt is for target {:?}, not {}",
                    r.target,
                    target.as_str()
                ),
            });
        }
        Ok(r)
    }

    /// True when the receipt was written for another project root.
    pub fn root_moved(&self, root: &Path) -> bool {
        !self.project_root.is_empty() && self.project_root != root.to_string_lossy()
    }

    pub fn skill_entries(&self) -> impl Iterator<Item = &SkillEntry> {
        self.entries.iter().filter_map(|e| match e {
            Entry::Skill(s) => Some(s.as_ref()),
            Entry::Other(_) => None,
        })
    }
}

// ---- history ----

/// The selection file as it was before a change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionBefore {
    pub existed: bool,
    pub text: String,
}

/// The selection text after a change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionAfter {
    pub text: String,
}

/// One history entry (P16).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub version: u32,
    pub op_id: String,
    pub command: String,
    pub target: String,
    pub started_at: String,
    pub finished_at: String,
    pub outcome: String,
    pub selection_before: SelectionBefore,
    pub selection_after: SelectionAfter,
    pub receipt_before: Value,
    pub receipt_after: Value,
}

/// Write `history/<op_id>.json` through `fs`.
pub fn write_history(fs: &dyn FsOps, state: &StateDir, entry: &HistoryEntry) -> Result<()> {
    let path = state.history_file(&entry.op_id)?;
    let mut text = serde_json::to_string_pretty(entry)
        .map_err(|e| Error::io("cannot serialize history", std::io::Error::other(e)))?;
    text.push('\n');
    fs.write_state(&path, text.as_bytes())
}

/// Keep the `keep` newest history files (names matching `<op_id>.json`);
/// other files and directories are never touched. Returns how many were
/// removed.
pub fn prune_history(fs: &dyn FsOps, state: &StateDir, keep: usize) -> Result<usize> {
    let dir = state.history_dir();
    let rd = match std::fs::read_dir(dir.path()) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(0),
        Err(e) => {
            return Err(Error::io(
                format!("cannot read {}", dir.path().display()),
                e,
            ));
        }
    };
    let mut ids: Vec<String> = Vec::new();
    for entry in rd {
        let entry = entry.map_err(|e| Error::io("cannot read history", e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let is_file = std::fs::symlink_metadata(entry.path())
            .map(|m| m.file_type().is_file())
            .unwrap_or(false);
        if valid_op_id(id) && is_file {
            ids.push(id.to_string());
        }
    }
    ids.sort();
    let excess = ids.len().saturating_sub(keep);
    for id in ids.iter().take(excess) {
        fs.remove_state(&state.history_file(id)?)?;
    }
    Ok(excess)
}

/// The newest history entry of `state` (op IDs sort by time), or `None`
/// when there is none. Only files named `<op_id>.json` count. An entry that
/// cannot be read or parsed is `StateDirInvalid`.
pub fn read_latest_history(state: &StateDir) -> Result<Option<HistoryEntry>> {
    let dir = state.history_dir();
    let rd = match std::fs::read_dir(dir.path()) {
        Ok(rd) => rd,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(Error::io(
                format!("cannot read {}", dir.path().display()),
                e,
            ));
        }
    };
    let mut newest: Option<String> = None;
    for entry in rd {
        let entry = entry.map_err(|e| Error::io("cannot read history", e))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let is_file = std::fs::symlink_metadata(entry.path())
            .map(|m| m.file_type().is_file())
            .unwrap_or(false);
        if valid_op_id(id) && is_file && newest.as_deref().is_none_or(|n| id > n) {
            newest = Some(id.to_string());
        }
    }
    let Some(id) = newest else {
        return Ok(None);
    };
    let path = state.history_file(&id)?;
    let invalid = |reason: String| Error::StateDirInvalid {
        path: path.path().to_path_buf(),
        reason,
    };
    let bytes = std::fs::read(path.path()).map_err(|e| invalid(e.to_string()))?;
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| invalid(format!("invalid history entry: {e}")))
}

// ---- journal ----

/// What `remove_owned` expects, as recorded in the journal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalExpect {
    pub strategy: Strategy,
    pub source: String,
    #[serde(default)]
    pub tree_hash: Option<String>,
}

/// One plan step as recorded in the journal (G8c).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum JournalStep {
    Add {
        n: u32,
        id: String,
        name: String,
        strategy: Strategy,
        source: String,
        tree_hash: String,
        provenance: Vec<Provenance>,
    },
    Remove {
        n: u32,
        id: String,
        name: String,
        expect: JournalExpect,
    },
    DropOrphan {
        n: u32,
        id: String,
        name: String,
        reason: String,
    },
    UpdateProvenance {
        n: u32,
        id: String,
        name: String,
        provenance: Vec<Provenance>,
    },
    RemoveDir {
        n: u32,
        dir: String,
    },
}

impl JournalStep {
    pub fn n(&self) -> u32 {
        match self {
            JournalStep::Add { n, .. }
            | JournalStep::Remove { n, .. }
            | JournalStep::DropOrphan { n, .. }
            | JournalStep::UpdateProvenance { n, .. }
            | JournalStep::RemoveDir { n, .. } => *n,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            JournalStep::Add { name, .. }
            | JournalStep::Remove { name, .. }
            | JournalStep::DropOrphan { name, .. }
            | JournalStep::UpdateProvenance { name, .. } => Some(name),
            JournalStep::RemoveDir { .. } => None,
        }
    }
}

/// The `begin` record.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Begin {
    pub version: u32,
    pub op_id: String,
    pub command: String,
    pub target: String,
    pub project_root: String,
    pub started_at: String,
    pub dest_rel: String,
    pub dest_dirs_planned: Vec<String>,
    pub selection_before: SelectionBefore,
    pub selection_after_text: String,
    pub selection_after_hash: String,
    /// The P6 selection hash the receipt records (so `resume` can write the
    /// same receipt the interrupted apply would have written).
    #[serde(default)]
    pub selection_hash: String,
    pub receipt_before: Value,
    pub plan: Vec<JournalStep>,
}

/// How a journal ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerOutcome {
    Clean,
    Skips,
    Partial,
}

/// One journal record (`t` is the type).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Record {
    Begin(Box<Begin>),
    Started {
        n: u32,
    },
    Done {
        n: u32,
        how: String,
    },
    Failed {
        n: u32,
        code: String,
        message: String,
    },
    Skipped {
        n: u32,
        reason: String,
    },
    Dir {
        path: String,
    },
    DirSkipped {
        path: String,
    },
    DirRemoved {
        path: String,
    },
    Receipt {
        outcome: MarkerOutcome,
    },
}

impl Record {
    /// One compact JSON line with its newline.
    pub fn to_line(&self) -> String {
        let mut s = serde_json::to_string(self).unwrap_or_default();
        s.push('\n');
        s
    }
}

/// State of one step after reading the journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepState {
    Pending,
    Started,
    Done { how: String },
    Failed { code: String, message: String },
    Skipped { reason: String },
}

impl StepState {
    /// `Done` and `Skipped` are terminal.
    pub fn terminal(&self) -> bool {
        matches!(self, StepState::Done { .. } | StepState::Skipped { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            StepState::Pending => "pending",
            StepState::Started => "started",
            StepState::Done { .. } => "done",
            StepState::Failed { .. } => "failed",
            StepState::Skipped { .. } => "skipped",
        }
    }
}

/// A validated journal.
#[derive(Debug, Clone, PartialEq)]
pub struct Journal {
    pub begin: Begin,
    pub states: BTreeMap<u32, StepState>,
    /// `dir` records minus `dir_skipped`, in order.
    pub dirs_created: Vec<String>,
    pub dirs_removed: Vec<String>,
    pub marker: Option<MarkerOutcome>,
    pub torn_tail: bool,
    /// The file ends with a newline.
    pub ends_with_newline: bool,
    /// The complete valid record lines, each ending in a newline.
    pub valid_text: String,
    /// The parsed `receipt_before` (`None` when it was null).
    pub receipt_before: Option<Receipt>,
}

/// Result of `read_journal`.
#[derive(Debug, Clone, PartialEq)]
pub enum JournalRead {
    Absent,
    TornBegin,
    Present(Box<Journal>),
}

/// Read and validate the journal of `state` (the journal is untrusted).
pub fn read_journal(state: &StateDir, adapter_dest: &str) -> Result<JournalRead> {
    let path = state.journal();
    let bytes = match std::fs::read(path.path()) {
        Ok(b) => b,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(JournalRead::Absent),
        Err(e) => {
            return Err(Error::io(
                format!("cannot read {}", path.path().display()),
                e,
            ));
        }
    };
    let invalid = |reason: String| Error::JournalInvalid {
        path: path.path().to_path_buf(),
        reason,
    };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let ends_with_newline = text.ends_with('\n');
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut records: Vec<Record> = Vec::new();
    let mut valid_text = String::new();
    let mut torn_tail = false;
    for (i, raw) in lines.iter().enumerate() {
        let last = i + 1 == lines.len();
        let body = raw.strip_suffix('\n').unwrap_or(raw);
        let value: Value = match serde_json::from_str(body) {
            Ok(v) => v,
            Err(e) => {
                if last {
                    torn_tail = true;
                    break;
                }
                return Err(invalid(format!("line {} is not JSON: {e}", i + 1)));
            }
        };
        let rec: Record = serde_json::from_value(value)
            .map_err(|e| invalid(format!("line {} is not a journal record: {e}", i + 1)))?;
        valid_text.push_str(body);
        valid_text.push('\n');
        records.push(rec);
    }
    if records.is_empty() {
        return Ok(JournalRead::TornBegin);
    }
    let mut iter = records.into_iter();
    let Some(Record::Begin(begin)) = iter.next() else {
        return Err(invalid("a record comes before `begin`".into()));
    };
    let begin = *begin;
    validate_begin(&begin, state, adapter_dest).map_err(invalid)?;
    let receipt_before = match &begin.receipt_before {
        Value::Null => None,
        v => {
            let r = Receipt::from_value(v, path.path())
                .map_err(|e| invalid(format!("receipt_before: {e}")))?;
            if r.target != state.target.as_str() {
                return Err(invalid("receipt_before is for another target".into()));
            }
            if let Some(bad) = r.skill_entries().find(|e| !e.consistent(adapter_dest)) {
                return Err(invalid(format!(
                    "receipt_before entry {} has the destination {:?}",
                    bad.skill_id, bad.dest
                )));
            }
            Some(r)
        }
    };
    let count = u32::try_from(begin.plan.len()).unwrap_or(u32::MAX);
    let prefixes = dest_prefixes(&begin.dest_rel);
    let mut states: BTreeMap<u32, StepState> =
        (1..=count).map(|n| (n, StepState::Pending)).collect();
    let mut dirs: Vec<String> = Vec::new();
    let mut skipped_dirs: Vec<String> = Vec::new();
    let mut dirs_removed: Vec<String> = Vec::new();
    let mut marker = None;
    let check_n = |n: u32| -> Result<()> {
        if n == 0 || n > count {
            Err(invalid(format!("record names step {n} outside the plan")))
        } else {
            Ok(())
        }
    };
    let check_dir = |p: &str| -> Result<()> {
        if prefixes.iter().any(|x| x == p) {
            Ok(())
        } else {
            Err(invalid(format!(
                "directory {p:?} is not a destination prefix"
            )))
        }
    };
    for rec in iter {
        match rec {
            Record::Begin(_) => return Err(invalid("duplicate `begin`".into())),
            Record::Started { n } => {
                check_n(n)?;
                states.insert(n, StepState::Started);
            }
            Record::Done { n, how } => {
                check_n(n)?;
                states.insert(n, StepState::Done { how });
            }
            Record::Failed { n, code, message } => {
                check_n(n)?;
                states.insert(n, StepState::Failed { code, message });
            }
            Record::Skipped { n, reason } => {
                check_n(n)?;
                states.insert(n, StepState::Skipped { reason });
            }
            Record::Dir { path } => {
                check_dir(&path)?;
                if !dirs.contains(&path) {
                    dirs.push(path);
                }
            }
            Record::DirSkipped { path } => {
                check_dir(&path)?;
                skipped_dirs.push(path);
            }
            Record::DirRemoved { path } => {
                check_dir(&path)?;
                if !dirs_removed.contains(&path) {
                    dirs_removed.push(path);
                }
            }
            Record::Receipt { outcome } => marker = Some(outcome),
        }
    }
    let dirs_created = dirs
        .into_iter()
        .filter(|d| !skipped_dirs.contains(d))
        .collect();
    Ok(JournalRead::Present(Box::new(Journal {
        begin,
        states,
        dirs_created,
        dirs_removed,
        marker,
        torn_tail,
        ends_with_newline,
        valid_text,
        receipt_before,
    })))
}

fn validate_begin(
    b: &Begin,
    state: &StateDir,
    adapter_dest: &str,
) -> std::result::Result<(), String> {
    if b.version != 1 {
        return Err(format!("unsupported journal version {}", b.version));
    }
    if !valid_op_id(&b.op_id) {
        return Err(format!("invalid operation id {:?}", b.op_id));
    }
    if b.target != state.target.as_str() {
        return Err(format!("journal is for target {:?}", b.target));
    }
    if b.dest_rel != adapter_dest {
        return Err(format!(
            "journal destination {:?} is not {adapter_dest}",
            b.dest_rel
        ));
    }
    if b.project_root != state.root.to_string_lossy() {
        return Err(
            "journal project_root differs from this project; move the project back or roll back there"
                .to_string(),
        );
    }
    let prefixes = dest_prefixes(&b.dest_rel);
    for d in &b.dest_dirs_planned {
        if !prefixes.contains(d) {
            return Err(format!(
                "planned directory {d:?} is not a destination prefix"
            ));
        }
    }
    for (i, step) in b.plan.iter().enumerate() {
        let want = u32::try_from(i + 1).unwrap_or(u32::MAX);
        if step.n() != want {
            return Err(format!("plan step {} has number {}", i + 1, step.n()));
        }
        if let Some(name) = step.name() {
            validate_dest_name(name).map_err(|r| format!("step {want} name {name:?}: {r}"))?;
        }
        if let JournalStep::RemoveDir { dir, .. } = step
            && !prefixes.contains(dir)
        {
            return Err(format!(
                "step {want} removes {dir:?}, not a destination prefix"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
