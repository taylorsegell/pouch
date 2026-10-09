//! Error model (spec 7): one `Error` enum with stable codes, exit codes, and
//! remedy strings, plus the `Diagnostic` shape used for non-fatal findings.

use serde::Serialize;
use std::path::PathBuf;

/// Severity of a diagnostic. `Error` means the item was skipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A non-fatal finding. `code` strings are a stable contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub path: Option<String>,
    pub remedy: Option<String>,
}

impl Diagnostic {
    pub fn new(code: &str, severity: Severity, message: impl Into<String>) -> Self {
        Diagnostic {
            code: code.to_string(),
            severity,
            message: message.into(),
            path: None,
            remedy: None,
        }
    }

    pub fn info(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Info, message)
    }

    pub fn warning(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Warning, message)
    }

    pub fn error(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, Severity::Error, message)
    }

    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn with_remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }
}

/// Remedy for an unreachable source (spec 7, verbatim).
pub const REMEDY_SOURCE_UNREACHABLE: &str =
    "Mount the volume or run `unam source relocate <alias> <path>`.";

/// Which destination-root problem `Error::DestRootInvalid` reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestRootProblem {
    /// The root resolves outside the canonical project root.
    Outside,
    /// The root exists and is not a directory.
    NotDirectory,
}

fn pid_text(pid: &Option<u32>) -> String {
    match pid {
        Some(p) => format!("process {p}"),
        None => "another process".to_string(),
    }
}

/// The single error type of `unam-core`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    // ---- config ----
    #[error("cannot read config file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {message}")]
    ConfigParse { path: PathBuf, message: String },
    #[error("config file {path} has version {version}; this unam supports version 1")]
    ConfigUnsupportedVersion { path: PathBuf, version: u32 },
    #[error("environment variable {name} is not set (in path {raw})")]
    ConfigUnsetVariable { name: String, raw: String },

    // ---- paths ----
    #[error("invalid alias {alias:?}")]
    InvalidAlias { alias: String },
    #[error("{name} must be an absolute path, got {value:?}")]
    InvalidEnvPath { name: String, value: String },
    #[error("cannot determine the home directory: HOME is not set")]
    NoHomeDir,

    // ---- source ----
    #[error("a source with alias {alias:?} already exists")]
    AliasExists { alias: String },
    #[error("invalid source path {path}: {reason}")]
    SourcePathInvalid { path: String, reason: String },
    #[error("{path} is already registered as source {alias:?}")]
    SourcePathAlreadyRegistered { path: String, alias: String },
    #[error("no source with alias {alias:?}")]
    SourceNotFound { alias: String },
    #[error("source {alias:?} is referenced by {} project(s): {}", projects.len(), projects.join(", "))]
    SourceReferenced {
        alias: String,
        projects: Vec<String>,
    },
    #[error("unknown source {alias:?}")]
    UnknownSourceFilter { alias: String },

    // ---- resolve ----
    #[error("invalid reference {reference:?}: {reason}")]
    InvalidRef { reference: String, reason: String },
    #[error("{message}")]
    NotFound {
        reference: String,
        message: String,
        candidates: Vec<String>,
    },
    #[error("reference {reference:?} is ambiguous: {} skills match", candidates.len())]
    Ambiguous {
        reference: String,
        candidates: Vec<String>,
    },

    // ---- bundle ----
    #[error("no bundle named {slug:?}")]
    BundleNotFound { slug: String },
    #[error("bundle {slug:?} already exists at {}", path.display())]
    BundleExists { slug: String, path: PathBuf },
    #[error("invalid bundle name {name:?}: {reason}")]
    InvalidBundleName { name: String, reason: String },
    #[error("invalid bundle {}: {message}", path.display())]
    BundleInvalid { path: PathBuf, message: String },
    #[error("cannot edit bundle {} in place: {reason}", path.display())]
    BundleEditUnsupported { path: PathBuf, reason: String },
    #[error("bundle edit of {} failed: {reason}", path.display())]
    BundleEditFailed { path: PathBuf, reason: String },
    #[error("bundle {bundle:?} has no entry matching {reference:?}")]
    SkillNotInBundle { bundle: String, reference: String },
    #[error("removing these entries would leave bundle {bundle:?} empty")]
    BundleWouldBeEmpty { bundle: String },
    #[error("the EDITOR environment variable is not set")]
    EditorNotSet,
    #[error("editor {command:?} failed: {detail}")]
    EditorFailed { command: String, detail: String },

    // ---- project ----
    #[error("cannot read selection {}: {source}", path.display())]
    SelectionRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid selection {}: {message}", path.display())]
    SelectionInvalid { path: PathBuf, message: String },
    #[error("selection {} has version {version}; this unam supports version 1", path.display())]
    SelectionUnsupportedVersion { path: PathBuf, version: u32 },
    #[error("no selection in project {}: {reason}", root.display())]
    NoSelection { root: PathBuf, reason: String },
    #[error("the selection has several targets: {}", targets.join(", "))]
    TargetAmbiguous { targets: Vec<String> },

    // ---- activation (milestone M3) ----
    #[error("{count} conflict(s) block this change")]
    Conflicts { count: usize },
    #[error("the apply did not finish cleanly: {failed} step(s) failed, {skipped} skipped")]
    ApplyFailed { failed: usize, skipped: usize },
    #[error("an apply of target {target} was interrupted (operation {op_id})")]
    Interrupted { target: String, op_id: String },
    #[error("the project is locked by {}", pid_text(pid))]
    Locked { pid: Option<u32> },
    #[error("this change needs confirmation and no terminal is available")]
    ConfirmationRequired,
    #[error("{reference:?} is not selected for this target")]
    NotSelected { reference: String },
    #[error("target {target} has no interrupted apply (no journal)")]
    NoJournal { target: String },
    #[error("target {target} is not supported yet")]
    TargetUnsupported { target: String },
    #[error("invalid target {value:?}")]
    InvalidTarget { value: String },
    #[error("invalid project path {path}: {reason}")]
    ProjectPathInvalid { path: String, reason: String },
    #[error("target {target} does not support strategy {strategy}")]
    StrategyUnsupported { target: String, strategy: String },
    #[error("cannot create a symbolic link at {}: {source}", dest.display())]
    SymlinkUnsupported {
        dest: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("copy refused at {}: {reason}", path.display())]
    CopyRefused { path: PathBuf, reason: String },
    #[error("source skill {id} changed: {reason}")]
    SourceChanged { id: String, reason: String },
    #[error("invalid journal {}: {reason}", path.display())]
    JournalInvalid { path: PathBuf, reason: String },
    #[error("invalid receipt {}: {message}", path.display())]
    ReceiptInvalid { path: PathBuf, message: String },
    #[error("receipt {} has version {version}; this unam supports version 1", path.display())]
    ReceiptUnsupportedVersion { path: PathBuf, version: u64 },
    #[error("invalid state entry {}: {reason}", path.display())]
    StateDirInvalid { path: PathBuf, reason: String },
    #[error("project {} overlaps the registered source {}", project.display(), source_root.display())]
    ProjectOverlapsSource {
        project: PathBuf,
        source_root: PathBuf,
    },
    #[error("destination root {dest_root} is invalid: {detail}")]
    DestRootInvalid {
        problem: DestRootProblem,
        dest_root: String,
        detail: String,
    },

    // ---- io ----
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
}

impl Error {
    /// Convenience constructor for IO errors with context.
    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io {
            context: context.into(),
            source,
        }
    }

    /// Stable kebab-case code.
    pub fn code(&self) -> &'static str {
        match self {
            Error::ConfigRead { .. } => "config-read-failed",
            Error::ConfigParse { .. } => "config-invalid",
            Error::ConfigUnsupportedVersion { .. } => "config-version-unsupported",
            Error::ConfigUnsetVariable { .. } => "config-unset-variable",
            Error::InvalidAlias { .. } => "invalid-alias",
            Error::InvalidEnvPath { .. } => "invalid-env-path",
            Error::NoHomeDir => "no-home-dir",
            Error::AliasExists { .. } => "alias-exists",
            Error::SourcePathInvalid { .. } => "source-path-invalid",
            Error::SourcePathAlreadyRegistered { .. } => "source-path-registered",
            Error::SourceNotFound { .. } => "source-not-found",
            Error::SourceReferenced { .. } => "source-referenced",
            Error::UnknownSourceFilter { .. } => "unknown-source",
            Error::InvalidRef { .. } => "invalid-ref",
            Error::NotFound { .. } => "not-found",
            Error::Ambiguous { .. } => "ambiguous",
            Error::BundleNotFound { .. } => "bundle-not-found",
            Error::BundleExists { .. } => "bundle-exists",
            Error::InvalidBundleName { .. } => "invalid-bundle-name",
            Error::BundleInvalid { .. } => "bundle-invalid",
            Error::BundleEditUnsupported { .. } => "bundle-edit-unsupported",
            Error::BundleEditFailed { .. } => "bundle-edit-failed",
            Error::SkillNotInBundle { .. } => "skill-not-in-bundle",
            Error::BundleWouldBeEmpty { .. } => "bundle-would-be-empty",
            Error::EditorNotSet => "editor-not-set",
            Error::EditorFailed { .. } => "editor-failed",
            Error::SelectionRead { .. } => "selection-read-failed",
            Error::SelectionInvalid { .. } => "selection-invalid",
            Error::SelectionUnsupportedVersion { .. } => "selection-version-unsupported",
            Error::NoSelection { .. } => "no-selection",
            Error::TargetAmbiguous { .. } => "target-ambiguous",
            Error::Conflicts { .. } => "blocked-by-conflicts",
            Error::ApplyFailed { .. } => "apply-failed",
            Error::Interrupted { .. } => "interrupted",
            Error::Locked { .. } => "locked",
            Error::ConfirmationRequired => "confirmation-required",
            Error::NotSelected { .. } => "not-selected",
            Error::NoJournal { .. } => "no-journal",
            Error::TargetUnsupported { .. } => "target-unsupported",
            Error::InvalidTarget { .. } => "invalid-target",
            Error::ProjectPathInvalid { .. } => "project-path-invalid",
            Error::StrategyUnsupported { .. } => "strategy-unsupported",
            Error::SymlinkUnsupported { .. } => "symlink-unsupported",
            Error::CopyRefused { .. } => "copy-refused",
            Error::SourceChanged { .. } => "source-changed",
            Error::JournalInvalid { .. } => "journal-invalid",
            Error::ReceiptInvalid { .. } => "receipt-invalid",
            Error::ReceiptUnsupportedVersion { .. } => "receipt-version-unsupported",
            Error::StateDirInvalid { .. } => "state-dir-invalid",
            Error::ProjectOverlapsSource { .. } => "project-overlaps-source",
            Error::DestRootInvalid { problem, .. } => match problem {
                DestRootProblem::Outside => "dest-outside-project",
                DestRootProblem::NotDirectory => "dest-not-directory",
            },
            Error::Io { .. } => "io-error",
        }
    }

    /// Process exit code (spec 5).
    pub fn exit_code(&self) -> i32 {
        match self {
            Error::ConfigRead { .. }
            | Error::ConfigParse { .. }
            | Error::ConfigUnsupportedVersion { .. }
            | Error::ConfigUnsetVariable { .. }
            | Error::NoHomeDir
            | Error::BundleInvalid { .. }
            | Error::BundleEditFailed { .. }
            | Error::EditorFailed { .. }
            | Error::SelectionRead { .. }
            | Error::SelectionInvalid { .. }
            | Error::SelectionUnsupportedVersion { .. }
            | Error::SymlinkUnsupported { .. }
            | Error::CopyRefused { .. }
            | Error::SourceChanged { .. }
            | Error::JournalInvalid { .. }
            | Error::ReceiptInvalid { .. }
            | Error::ReceiptUnsupportedVersion { .. }
            | Error::StateDirInvalid { .. }
            | Error::Io { .. } => 1,
            Error::InvalidAlias { .. }
            | Error::InvalidEnvPath { .. }
            | Error::AliasExists { .. }
            | Error::SourcePathInvalid { .. }
            | Error::SourcePathAlreadyRegistered { .. }
            | Error::UnknownSourceFilter { .. }
            | Error::InvalidRef { .. }
            | Error::BundleExists { .. }
            | Error::InvalidBundleName { .. }
            | Error::BundleEditUnsupported { .. }
            | Error::BundleWouldBeEmpty { .. }
            | Error::EditorNotSet
            | Error::TargetAmbiguous { .. }
            | Error::ConfirmationRequired
            | Error::TargetUnsupported { .. }
            | Error::InvalidTarget { .. }
            | Error::ProjectPathInvalid { .. }
            | Error::StrategyUnsupported { .. }
            | Error::ProjectOverlapsSource { .. } => 2,
            Error::SourceReferenced { .. } => 3,
            Error::SourceNotFound { .. }
            | Error::NotFound { .. }
            | Error::BundleNotFound { .. }
            | Error::SkillNotInBundle { .. }
            | Error::NoSelection { .. }
            | Error::NotSelected { .. }
            | Error::NoJournal { .. } => 4,
            Error::Ambiguous { .. } => 5,
            Error::Conflicts { .. } | Error::DestRootInvalid { .. } => 6,
            Error::ApplyFailed { .. } => 7,
            Error::Interrupted { .. } => 10,
            Error::Locked { .. } => 11,
        }
    }

    /// Remedy text for user-fixable variants.
    pub fn remedy(&self) -> Option<&'static str> {
        match self {
            Error::ConfigRead { .. }
            | Error::Io { .. }
            | Error::EditorFailed { .. }
            | Error::SelectionRead { .. } => None,
            Error::ConfigParse { .. } => {
                Some("Fix the TOML syntax, or move the file aside to start fresh.")
            }
            Error::ConfigUnsupportedVersion { .. } => {
                Some("This config was written by a newer unam; upgrade unam.")
            }
            Error::ConfigUnsetVariable { .. } => {
                Some("Set the environment variable, or edit the source path in config.toml.")
            }
            Error::InvalidAlias { .. } => Some(
                "Use 1 to 32 characters: lowercase letters, digits, and hyphens, starting with a letter or digit.",
            ),
            Error::InvalidEnvPath { .. } => Some("Set it to an absolute path."),
            Error::NoHomeDir => Some(
                "Set HOME (or the relevant XDG_* variable, or UNAM_CONFIG_DIR and UNAM_CACHE_DIR).",
            ),
            Error::AliasExists { .. } => {
                Some("Pick another alias, or run `unam source rm <alias>` first.")
            }
            Error::SourcePathInvalid { .. } => Some("Pass an existing, readable directory."),
            Error::SourcePathAlreadyRegistered { .. } => {
                Some("That directory is already registered; run `unam source list`.")
            }
            Error::SourceNotFound { .. } | Error::UnknownSourceFilter { .. } => {
                Some("Run `unam source list` to see registered aliases.")
            }
            Error::SourceReferenced { .. } => Some(
                "Remove those references first, or re-run with `--force` (the selections keep unresolvable refs).",
            ),
            Error::InvalidRef { .. } => {
                Some("Use `alias:path/to/skill`, `alias:name`, or a bare name.")
            }
            Error::NotFound { .. } => Some(
                "Run `unam search <words>` to find the skill, or qualify the reference as `alias:path`.",
            ),
            Error::Ambiguous { .. } => {
                Some("Qualify the reference, for example `skillzone:sdlc/code-review`.")
            }
            Error::BundleNotFound { .. } => {
                Some("Run `unam bundle ls` to see the available bundles.")
            }
            Error::BundleExists { .. } => {
                Some("Pick another name, or re-run with `--force` to overwrite.")
            }
            Error::InvalidBundleName { .. } => Some(
                "Use a name with at least one letter or digit, no control characters, and at most 128 bytes once normalized.",
            ),
            Error::BundleInvalid { .. } => {
                Some("Fix the manifest: a non-blank name and a non-empty `skills` list.")
            }
            Error::BundleEditUnsupported { .. } => Some(
                "Edit the file by hand (for a symbolic link, edit its target), or re-run with `--rewrite` when the shape is the problem (rewrites the whole file and drops comments).",
            ),
            Error::BundleEditFailed { .. } => {
                Some("The file was not changed. Edit it by hand, or re-run with `--rewrite`.")
            }
            Error::SkillNotInBundle { .. } => {
                Some("Run `unam bundle show <bundle>` to see its entries.")
            }
            Error::BundleWouldBeEmpty { .. } => Some(
                "Add another skill first, or delete the bundle with `unam bundle rm <bundle>`.",
            ),
            Error::EditorNotSet => {
                Some("Set the EDITOR environment variable, for example `EDITOR=vi`.")
            }
            Error::SelectionInvalid { .. } => {
                Some("Fix the YAML in the selection file, or move it aside to start fresh.")
            }
            Error::SelectionUnsupportedVersion { .. } => {
                Some("This selection was written by a newer unam; upgrade unam.")
            }
            Error::NoSelection { .. } => Some(
                "Run `unam add <ref>` or `unam use <bundle> --replace` in this project, or write `.unam/selection.yaml` by hand.",
            ),
            Error::TargetAmbiguous { .. } => {
                Some("This selection has several targets; pass `--target <id>`.")
            }
            Error::Conflicts { .. } => Some(
                "Resolve each conflict (remove or rename the existing entry, or fix the name), then re-run. Nothing was changed.",
            ),
            Error::ApplyFailed { .. } => Some(
                "See the per-entry results. Run `unam resume` to retry failed steps, or `unam rollback` to undo them.",
            ),
            Error::Interrupted { .. } => {
                Some("Run `unam resume` to finish, or `unam rollback` to undo.")
            }
            Error::Locked { .. } => Some(
                "Another `unam` process is changing this project. Wait for it to finish, or close the open picker.",
            ),
            Error::ConfirmationRequired => Some(
                "Pass `--yes` to apply without a prompt; the plan is still printed. Nothing was changed.",
            ),
            Error::NotSelected { .. } => Some(
                "The skill is not in this project's selection for the target; nothing was changed.",
            ),
            Error::NoJournal { .. } => Some(
                "There is no interrupted apply to resume or roll back. Rolling back from history arrives in milestone M4.",
            ),
            Error::TargetUnsupported { .. } => {
                Some("Only `claude-code` is available until milestone M5.")
            }
            Error::InvalidTarget { .. } => Some("Use `claude-code` or `agents-neutral`."),
            Error::ProjectPathInvalid { .. } => Some("Pass an existing directory to `--project`."),
            Error::StrategyUnsupported { .. } => {
                Some("Use `symlink` or `copy` in the selection file or in `defaults.strategy`.")
            }
            Error::SymlinkUnsupported { .. } => Some(
                "Set `strategy: copy` for the target in `.unam/selection.yaml`, or `defaults.strategy = \"copy\"` in config.toml.",
            ),
            Error::CopyRefused { .. } => Some(
                "Fix the source skill: a copied skill may not contain a link that leaves the skill directory, or a special file.",
            ),
            Error::SourceChanged { .. } => {
                Some("The source skill changed after the plan was made; re-run the command.")
            }
            Error::JournalInvalid { .. } => Some(
                "The journal cannot be trusted. Inspect `.unam/<target>/journal.jsonl` and the destination by hand before removing it.",
            ),
            Error::ReceiptInvalid { .. } => Some(
                "Fix or move aside the receipt `.unam/<target>/applied.json`; nothing was changed.",
            ),
            Error::ReceiptUnsupportedVersion { .. } => {
                Some("This receipt was written by a newer unam; upgrade unam.")
            }
            Error::StateDirInvalid { .. } => Some(
                "A symlink or an entry of the wrong kind sits where unam keeps its state. Inspect `.unam/` by hand and remove or fix that entry; nothing was changed.",
            ),
            Error::ProjectOverlapsSource { .. } => Some(
                "The project directory is, lies inside, or contains a registered source. Run from another project or pass `--project <dir>`; nothing was written.",
            ),
            Error::DestRootInvalid { .. } => Some(
                "The destination root is a symlink that leaves the project or is not a directory. Remove or fix it and re-run; nothing was changed.",
            ),
        }
    }

    /// Candidate IDs for `not-found` and `ambiguous`.
    pub fn candidates(&self) -> Option<&[String]> {
        match self {
            Error::NotFound { candidates, .. } | Error::Ambiguous { candidates, .. } => {
                Some(candidates)
            }
            _ => None,
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    fn io() -> std::io::Error {
        std::io::Error::other("x")
    }

    #[test]
    fn every_variant_has_code_exit_and_remedy() {
        let p = PathBuf::from("/x");
        let s = String::from;
        let table: Vec<(Error, &str, i32, bool)> = vec![
            (
                Error::ConfigRead {
                    path: p.clone(),
                    source: io(),
                },
                "config-read-failed",
                1,
                false,
            ),
            (
                Error::ConfigParse {
                    path: p.clone(),
                    message: s("m"),
                },
                "config-invalid",
                1,
                true,
            ),
            (
                Error::ConfigUnsupportedVersion {
                    path: p.clone(),
                    version: 2,
                },
                "config-version-unsupported",
                1,
                true,
            ),
            (
                Error::ConfigUnsetVariable {
                    name: s("A"),
                    raw: s("$A"),
                },
                "config-unset-variable",
                1,
                true,
            ),
            (
                Error::InvalidAlias { alias: s("A") },
                "invalid-alias",
                2,
                true,
            ),
            (
                Error::InvalidEnvPath {
                    name: s("N"),
                    value: s("v"),
                },
                "invalid-env-path",
                2,
                true,
            ),
            (Error::NoHomeDir, "no-home-dir", 1, true),
            (
                Error::AliasExists { alias: s("a") },
                "alias-exists",
                2,
                true,
            ),
            (
                Error::SourcePathInvalid {
                    path: s("p"),
                    reason: s("r"),
                },
                "source-path-invalid",
                2,
                true,
            ),
            (
                Error::SourcePathAlreadyRegistered {
                    path: s("p"),
                    alias: s("a"),
                },
                "source-path-registered",
                2,
                true,
            ),
            (
                Error::SourceNotFound { alias: s("a") },
                "source-not-found",
                4,
                true,
            ),
            (
                Error::SourceReferenced {
                    alias: s("a"),
                    projects: vec![s("/p")],
                },
                "source-referenced",
                3,
                true,
            ),
            (
                Error::UnknownSourceFilter { alias: s("a") },
                "unknown-source",
                2,
                true,
            ),
            (
                Error::InvalidRef {
                    reference: s("r"),
                    reason: s("x"),
                },
                "invalid-ref",
                2,
                true,
            ),
            (
                Error::NotFound {
                    reference: s("r"),
                    message: s("m"),
                    candidates: vec![],
                },
                "not-found",
                4,
                true,
            ),
            (
                Error::Ambiguous {
                    reference: s("r"),
                    candidates: vec![s("a:x"), s("a:y")],
                },
                "ambiguous",
                5,
                true,
            ),
            (Error::io("ctx", io()), "io-error", 1, false),
            (
                Error::BundleNotFound { slug: s("b") },
                "bundle-not-found",
                4,
                true,
            ),
            (
                Error::BundleExists {
                    slug: s("b"),
                    path: p.clone(),
                },
                "bundle-exists",
                2,
                true,
            ),
            (
                Error::InvalidBundleName {
                    name: s("!"),
                    reason: s("r"),
                },
                "invalid-bundle-name",
                2,
                true,
            ),
            (
                Error::BundleInvalid {
                    path: p.clone(),
                    message: s("m"),
                },
                "bundle-invalid",
                1,
                true,
            ),
            (
                Error::BundleEditUnsupported {
                    path: p.clone(),
                    reason: s("r"),
                },
                "bundle-edit-unsupported",
                2,
                true,
            ),
            (
                Error::BundleEditFailed {
                    path: p.clone(),
                    reason: s("r"),
                },
                "bundle-edit-failed",
                1,
                true,
            ),
            (
                Error::SkillNotInBundle {
                    bundle: s("b"),
                    reference: s("r"),
                },
                "skill-not-in-bundle",
                4,
                true,
            ),
            (
                Error::BundleWouldBeEmpty { bundle: s("b") },
                "bundle-would-be-empty",
                2,
                true,
            ),
            (Error::EditorNotSet, "editor-not-set", 2, true),
            (
                Error::EditorFailed {
                    command: s("vi"),
                    detail: s("d"),
                },
                "editor-failed",
                1,
                false,
            ),
            (
                Error::SelectionRead {
                    path: p.clone(),
                    source: io(),
                },
                "selection-read-failed",
                1,
                false,
            ),
            (
                Error::SelectionInvalid {
                    path: p.clone(),
                    message: s("m"),
                },
                "selection-invalid",
                1,
                true,
            ),
            (
                Error::SelectionUnsupportedVersion {
                    path: p.clone(),
                    version: 2,
                },
                "selection-version-unsupported",
                1,
                true,
            ),
            (
                Error::NoSelection {
                    root: p.clone(),
                    reason: s("r"),
                },
                "no-selection",
                4,
                true,
            ),
            (
                Error::TargetAmbiguous {
                    targets: vec![s("a"), s("b")],
                },
                "target-ambiguous",
                2,
                true,
            ),
        ];
        let m3: Vec<(Error, &str, i32, bool)> = vec![
            (
                Error::Conflicts { count: 1 },
                "blocked-by-conflicts",
                6,
                true,
            ),
            (
                Error::ApplyFailed {
                    failed: 1,
                    skipped: 0,
                },
                "apply-failed",
                7,
                true,
            ),
            (
                Error::Interrupted {
                    target: s("claude-code"),
                    op_id: s("0000000000001-0000abcd"),
                },
                "interrupted",
                10,
                true,
            ),
            (Error::Locked { pid: Some(7) }, "locked", 11, true),
            (
                Error::ConfirmationRequired,
                "confirmation-required",
                2,
                true,
            ),
            (
                Error::NotSelected { reference: s("r") },
                "not-selected",
                4,
                true,
            ),
            (Error::NoJournal { target: s("t") }, "no-journal", 4, true),
            (
                Error::TargetUnsupported { target: s("t") },
                "target-unsupported",
                2,
                true,
            ),
            (
                Error::InvalidTarget { value: s("v") },
                "invalid-target",
                2,
                true,
            ),
            (
                Error::ProjectPathInvalid {
                    path: s("p"),
                    reason: s("r"),
                },
                "project-path-invalid",
                2,
                true,
            ),
            (
                Error::StrategyUnsupported {
                    target: s("t"),
                    strategy: s("copy"),
                },
                "strategy-unsupported",
                2,
                true,
            ),
            (
                Error::SymlinkUnsupported {
                    dest: p.clone(),
                    source: io(),
                },
                "symlink-unsupported",
                1,
                true,
            ),
            (
                Error::CopyRefused {
                    path: p.clone(),
                    reason: s("r"),
                },
                "copy-refused",
                1,
                true,
            ),
            (
                Error::SourceChanged {
                    id: s("fx:a/b"),
                    reason: s("r"),
                },
                "source-changed",
                1,
                true,
            ),
            (
                Error::JournalInvalid {
                    path: p.clone(),
                    reason: s("r"),
                },
                "journal-invalid",
                1,
                true,
            ),
            (
                Error::ReceiptInvalid {
                    path: p.clone(),
                    message: s("m"),
                },
                "receipt-invalid",
                1,
                true,
            ),
            (
                Error::ReceiptUnsupportedVersion {
                    path: p.clone(),
                    version: 2,
                },
                "receipt-version-unsupported",
                1,
                true,
            ),
            (
                Error::StateDirInvalid {
                    path: p.clone(),
                    reason: s("r"),
                },
                "state-dir-invalid",
                1,
                true,
            ),
            (
                Error::ProjectOverlapsSource {
                    project: p.clone(),
                    source_root: p.clone(),
                },
                "project-overlaps-source",
                2,
                true,
            ),
            (
                Error::DestRootInvalid {
                    problem: DestRootProblem::Outside,
                    dest_root: s(".claude/skills"),
                    detail: s("d"),
                },
                "dest-outside-project",
                6,
                true,
            ),
            (
                Error::DestRootInvalid {
                    problem: DestRootProblem::NotDirectory,
                    dest_root: s(".claude/skills"),
                    detail: s("d"),
                },
                "dest-not-directory",
                6,
                true,
            ),
        ];
        // Twenty new variants (DestRootInvalid twice, one per problem).
        assert_eq!(m3.len(), 21);
        let mut table = table;
        table.extend(m3);
        assert_eq!(table.len(), 53);
        for (err, code, exit, has_remedy) in table {
            assert_eq!(err.code(), code);
            assert_eq!(err.exit_code(), exit, "{code}");
            assert_eq!(err.remedy().is_some(), has_remedy, "{code}");
            if let Some(r) = err.remedy() {
                assert!(!r.is_empty());
            }
            assert!(!err.to_string().is_empty());
        }
    }

    #[test]
    fn reworded_remedies_name_the_m3_commands() {
        let e = Error::NoSelection {
            root: PathBuf::from("/p"),
            reason: "r".into(),
        };
        assert_eq!(
            e.remedy(),
            Some(
                "Run `unam add <ref>` or `unam use <bundle> --replace` in this project, or write `.unam/selection.yaml` by hand."
            )
        );
        let e = Error::TargetAmbiguous { targets: vec![] };
        assert_eq!(
            e.remedy(),
            Some("This selection has several targets; pass `--target <id>`.")
        );
        assert!(
            Error::Locked { pid: None }
                .to_string()
                .contains("another process")
        );
    }

    #[test]
    fn ambiguous_remedy_is_spec_text() {
        let e = Error::Ambiguous {
            reference: "x".into(),
            candidates: vec![],
        };
        assert_eq!(
            e.remedy(),
            Some("Qualify the reference, for example `skillzone:sdlc/code-review`.")
        );
    }

    #[test]
    fn severity_serializes_lowercase() {
        let d = Diagnostic::warning("c", "m").with_path("p");
        let v = serde_json::to_value(&d).unwrap();
        assert_eq!(v["severity"], "warning");
        assert_eq!(v["path"], "p");
        assert!(v["remedy"].is_null());
    }
}
