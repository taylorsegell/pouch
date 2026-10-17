//! Adapters (spec 4.7; M3 plan P14): where a target's skills go, how they
//! are discovered, which strategies work, and what to tell the user after
//! an apply. Every claim carries evidence (PRD S9).
//!
//! Milestone M3 registered `claude-code`; milestone M5 adds
//! `agents-neutral` without changing this trait.

pub mod agents_neutral;
pub mod claude_code;

use crate::config::Strategy;
use crate::paths::Dirs;
use crate::project::TargetId;
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Whether a target is in use for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Detection {
    Present { reasons: Vec<String> },
    Absent,
    Unknown { reason: String },
}

impl Detection {
    pub fn is_present(&self) -> bool {
        matches!(self, Detection::Present { .. })
    }
}

/// Discovery scope of a scan location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Enterprise,
    User,
    Project,
    Nested,
    /// A parent directory's skills directory, read by agents that walk
    /// upward to the repository root (`agents-neutral`).
    Ancestor,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Enterprise => "enterprise",
            Scope::User => "user",
            Scope::Project => "project",
            Scope::Nested => "nested",
            Scope::Ancestor => "ancestor",
        }
    }
}

/// A directory the agent reads skills from; lower `rank` wins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanLocation {
    pub path: PathBuf,
    pub scope: Scope,
    pub rank: u8,
}

/// What the agent needs after an apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refresh {
    None,
    Command(String),
    Restart,
    Unknown,
}

/// A suggested follow-up command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Followup {
    pub command: String,
    pub reason: String,
}

/// Where a claim's evidence lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    Url(&'static str),
    Spike(&'static str),
}

/// Whether a claim was verified or is an assumption.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceStatus {
    Verified,
    Assumption,
}

/// One adapter claim with its evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Evidence {
    pub claim_key: &'static str,
    pub claim: &'static str,
    pub source: EvidenceSource,
    pub status: EvidenceStatus,
}

/// Facts about the apply that decide the refresh note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyContext {
    /// The destination root existed before the apply.
    pub dest_root_existed: bool,
}

/// The SES-1 line data (spec 5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RefreshNote {
    pub kind: &'static str,
    pub command: Option<String>,
    pub text: String,
    pub evidence: String,
}

/// A target adapter (object safe).
pub trait Adapter: Send + Sync {
    fn id(&self) -> TargetId;
    /// Agents that read this destination.
    fn consumers(&self) -> &'static [&'static str];
    fn detect(&self, root: &Path, dirs: &Dirs) -> Detection;
    /// Project-relative destination directory.
    fn destination(&self) -> &'static str;
    fn scan_locations(&self, root: &Path, dirs: &Dirs) -> Vec<ScanLocation>;
    fn strategies(&self) -> &'static [Strategy];
    fn default_strategy(&self) -> Strategy;
    fn refresh(&self, ctx: ApplyContext) -> Refresh;
    fn refresh_note(&self, ctx: ApplyContext) -> RefreshNote;
    fn followups(&self, root: &Path, dirs: &Dirs) -> Vec<Followup>;
    fn evidence(&self) -> &'static [Evidence];
}

/// Every registered adapter. The order decides the default target when no
/// adapter is detected (M3 decision 44; M5-3): `claude-code` first.
pub fn adapters() -> Vec<&'static dyn Adapter> {
    vec![&claude_code::ClaudeCode, &agents_neutral::AgentsNeutral]
}

/// The adapter of a target, when one is registered.
pub fn adapter_for(target: TargetId) -> Option<&'static dyn Adapter> {
    adapters().into_iter().find(|a| a.id() == target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap()
            .to_path_buf()
    }

    fn has_evidence(a: &dyn Adapter, key: &str) -> bool {
        a.evidence().iter().any(|e| {
            e.claim_key == key
                && match e.source {
                    EvidenceSource::Url(u) => u.starts_with("https://"),
                    EvidenceSource::Spike(p) => repo_root().join(p).is_file(),
                }
        })
    }

    #[test]
    fn adapter_claims_have_evidence() {
        let t = tempfile::tempdir().unwrap();
        let dirs = crate::project::index::tests_support::dirs_in(t.path());
        for a in adapters() {
            // Required keys generalize over every adapter (M5-13): the
            // destination, the precedence rule when two or more locations
            // compete, and one claim per strategy, scan location, and
            // consumer; a refresh claim is checked separately below.
            let mut keys: Vec<String> = vec!["destination".into()];
            let locations = a.scan_locations(t.path(), &dirs);
            if locations.len() >= 2 {
                keys.push("precedence".into());
            }
            for s in a.strategies() {
                let name = match s {
                    Strategy::Symlink => "symlink",
                    Strategy::Copy => "copy",
                };
                keys.push(format!("strategy:{name}"));
            }
            for loc in &locations {
                keys.push(format!("scan:{}", loc.scope.as_str()));
            }
            for c in a.consumers() {
                let first = c.split_whitespace().next().unwrap();
                keys.push(format!("consumers:{first}"));
            }
            for k in &keys {
                assert!(has_evidence(a, k), "{}: no evidence for {k}", a.id());
            }
            let refresh_keys: Vec<&str> = a
                .evidence()
                .iter()
                .map(|e| e.claim_key)
                .filter(|k| k.starts_with("refresh:"))
                .collect();
            assert!(!refresh_keys.is_empty(), "{}: no refresh claim", a.id());
            for k in refresh_keys {
                assert!(has_evidence(a, k), "{}: no evidence for {k}", a.id());
            }
            for e in a.evidence() {
                // Every entry, Assumption included, carries a usable source.
                assert!(has_evidence(a, e.claim_key), "{}", e.claim_key);
                assert!(!e.claim.is_empty());
            }
        }
    }

    #[test]
    fn adapter_for_registry() {
        assert_eq!(
            adapter_for(TargetId::ClaudeCode).map(|a| a.id()),
            Some(TargetId::ClaudeCode)
        );
        assert_eq!(
            adapter_for(TargetId::AgentsNeutral).map(|a| a.id()),
            Some(TargetId::AgentsNeutral)
        );
        let ids: Vec<TargetId> = adapters().iter().map(|a| a.id()).collect();
        assert_eq!(ids, vec![TargetId::ClaudeCode, TargetId::AgentsNeutral]);
    }
}
