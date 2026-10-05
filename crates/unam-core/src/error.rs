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
            | Error::Io { .. } => 1,
            Error::InvalidAlias { .. }
            | Error::InvalidEnvPath { .. }
            | Error::AliasExists { .. }
            | Error::SourcePathInvalid { .. }
            | Error::SourcePathAlreadyRegistered { .. }
            | Error::UnknownSourceFilter { .. }
            | Error::InvalidRef { .. } => 2,
            Error::SourceReferenced { .. } => 3,
            Error::SourceNotFound { .. } | Error::NotFound { .. } => 4,
            Error::Ambiguous { .. } => 5,
        }
    }

    /// Remedy text for user-fixable variants.
    pub fn remedy(&self) -> Option<&'static str> {
        match self {
            Error::ConfigRead { .. } | Error::Io { .. } => None,
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
        ];
        assert_eq!(table.len(), 17);
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
