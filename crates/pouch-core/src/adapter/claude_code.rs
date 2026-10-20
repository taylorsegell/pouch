//! The `claude-code` adapter (spec 4.7; M3 plan P14). Default strategy
//! symlink per the M0 Outcome (`spike/symlink-discovery.md`).

use super::{
    Adapter, ApplyContext, Detection, Evidence, EvidenceSource, EvidenceStatus, Followup, Refresh,
    RefreshNote, ScanLocation, Scope,
};
use crate::config::Strategy;
use crate::paths::Dirs;
use crate::project::TargetId;
use std::path::Path;

const DOCS: &str = "https://code.claude.com/docs/en/skills";
const SPIKE: &str = "spike/symlink-discovery.md";

/// The `claude-code` adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeCode;

static CONSUMERS: &[&str] = &[
    "claude-code",
    "cursor (compat read)",
    "windsurf-devin (when Claude config reading is enabled)",
];

static STRATEGIES: &[Strategy] = &[Strategy::Symlink, Strategy::Copy];

static EVIDENCE: &[Evidence] = &[
    Evidence {
        claim_key: "destination",
        claim: "Project skills live in .claude/skills/<name>/SKILL.md",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:user",
        claim: "Personal skills live in ~/.claude/skills",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:project",
        claim: "Project skills live in .claude/skills",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:enterprise",
        claim: "The enterprise location outranks personal and project scope; the platform paths used are an assumption",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Assumption,
    },
    Evidence {
        claim_key: "scan:nested",
        claim: "Nested .claude/skills directories are discovered (not enumerated in v1)",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "precedence",
        claim: "Enterprise beats personal beats project when names collide",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "strategy:symlink",
        claim: "A symlinked skill directory is discovered at start and mid-session with no reload (Claude Code 2.1.285, 2026-10-05)",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "strategy:copy",
        claim: "A copied skill directory is discovered like any project skill",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:none",
        claim: "Existing skill directories are watched; no restart or reload is needed (except in bare mode)",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:command",
        claim: "Run /reload-skills once when the skills directory did not exist at startup",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:claude-code",
        claim: "Claude Code reads .claude/skills",
        source: EvidenceSource::Url(DOCS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:cursor",
        claim: "Cursor reads .claude/skills for compatibility",
        source: EvidenceSource::Url("https://cursor.com/docs/skills"),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:windsurf-devin",
        claim: "Windsurf and Devin read .claude/skills when Claude config reading is enabled",
        source: EvidenceSource::Url("https://docs.devin.ai/desktop/cascade/skills"),
        status: EvidenceStatus::Verified,
    },
];

impl Adapter for ClaudeCode {
    fn id(&self) -> TargetId {
        TargetId::ClaudeCode
    }

    fn consumers(&self) -> &'static [&'static str] {
        CONSUMERS
    }

    fn detect(&self, root: &Path, dirs: &Dirs) -> Detection {
        let mut reasons = Vec::new();
        if std::fs::symlink_metadata(root.join(".claude")).is_ok() {
            reasons.push("<root>/.claude exists".to_string());
        }
        if let Ok(home) = dirs.home()
            && std::fs::symlink_metadata(home.join(".claude")).is_ok()
        {
            reasons.push("~/.claude exists".to_string());
        }
        if reasons.is_empty() {
            Detection::Absent
        } else {
            Detection::Present { reasons }
        }
    }

    fn destination(&self) -> &'static str {
        ".claude/skills"
    }

    fn scan_locations(&self, root: &Path, dirs: &Dirs) -> Vec<ScanLocation> {
        let mut out: Vec<ScanLocation> = dirs
            .enterprise_skill_dirs
            .iter()
            .map(|p| ScanLocation {
                path: p.clone(),
                scope: Scope::Enterprise,
                rank: 0,
            })
            .collect();
        if let Ok(home) = dirs.home() {
            out.push(ScanLocation {
                path: home.join(".claude").join("skills"),
                scope: Scope::User,
                rank: 1,
            });
        }
        out.push(ScanLocation {
            path: root.join(".claude").join("skills"),
            scope: Scope::Project,
            rank: 2,
        });
        out
    }

    fn strategies(&self) -> &'static [Strategy] {
        STRATEGIES
    }

    fn default_strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn refresh(&self, ctx: ApplyContext) -> Refresh {
        if ctx.dest_root_existed {
            Refresh::None
        } else {
            Refresh::Command("/reload-skills".to_string())
        }
    }

    fn refresh_note(&self, ctx: ApplyContext) -> RefreshNote {
        match self.refresh(ctx) {
            Refresh::Command(c) => RefreshNote {
                kind: "command",
                command: Some(c),
                text: format!(
                    "this apply created the skills directory; run /reload-skills in a running Claude Code session so it scans it (evidence: {DOCS})"
                ),
                evidence: DOCS.to_string(),
            },
            _ => RefreshNote {
                kind: "none",
                command: None,
                text: format!(
                    "no restart or reload required (Claude Code watches existing skill directories; evidence: {DOCS})"
                ),
                evidence: DOCS.to_string(),
            },
        }
    }

    fn followups(&self, _root: &Path, _dirs: &Dirs) -> Vec<Followup> {
        Vec::new()
    }

    fn evidence(&self) -> &'static [Evidence] {
        EVIDENCE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dirs(root: &Path, enterprise: &str) -> Dirs {
        let mut m = std::collections::HashMap::new();
        m.insert("HOME".to_string(), root.join("home").into_os_string());
        m.insert(
            "POUCH_CONFIG_DIR".to_string(),
            root.join("config").into_os_string(),
        );
        m.insert(
            "POUCH_CACHE_DIR".to_string(),
            root.join("cache").into_os_string(),
        );
        m.insert(
            "XDG_DATA_HOME".to_string(),
            root.join("data").into_os_string(),
        );
        m.insert(
            crate::paths::ENTERPRISE_SKILLS_VAR.to_string(),
            enterprise.into(),
        );
        Dirs::resolve(crate::paths::Env::from_map(m)).unwrap()
    }

    #[test]
    fn claude_code_strategies_match_m0_outcome() {
        let a = ClaudeCode;
        assert_eq!(a.strategies(), &[Strategy::Symlink, Strategy::Copy]);
        assert_eq!(a.default_strategy(), Strategy::Symlink);
        assert!(
            a.evidence()
                .iter()
                .any(|e| e.claim_key == "strategy:symlink"
                    && e.source == EvidenceSource::Spike("spike/symlink-discovery.md")
                    && e.status == EvidenceStatus::Verified)
        );
    }

    #[test]
    fn claude_code_destination_and_consumers() {
        let a = ClaudeCode;
        assert_eq!(a.destination(), ".claude/skills");
        assert_eq!(a.id(), TargetId::ClaudeCode);
        assert_eq!(
            a.consumers(),
            &[
                "claude-code",
                "cursor (compat read)",
                "windsurf-devin (when Claude config reading is enabled)"
            ]
        );
        assert!(
            a.followups(Path::new("/p"), &dirs(Path::new("/t"), "/e"))
                .is_empty()
        );
    }

    #[test]
    fn claude_code_detect() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path(), "/e");
        let root = t.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(ClaudeCode.detect(&root, &d), Detection::Absent);
        std::fs::create_dir_all(t.path().join("home/.claude")).unwrap();
        assert_eq!(
            ClaudeCode.detect(&root, &d),
            Detection::Present {
                reasons: vec!["~/.claude exists".into()]
            }
        );
        std::fs::create_dir_all(root.join(".claude")).unwrap();
        assert_eq!(
            ClaudeCode.detect(&root, &d),
            Detection::Present {
                reasons: vec!["<root>/.claude exists".into(), "~/.claude exists".into()]
            }
        );
    }

    #[test]
    fn claude_code_scan_locations_order_and_ranks() {
        let d = dirs(Path::new("/t"), "/e1:/e2");
        let locs = ClaudeCode.scan_locations(Path::new("/p"), &d);
        let got: Vec<(PathBuf, Scope, u8)> = locs
            .into_iter()
            .map(|l| (l.path, l.scope, l.rank))
            .collect();
        assert_eq!(
            got,
            vec![
                (PathBuf::from("/e1"), Scope::Enterprise, 0),
                (PathBuf::from("/e2"), Scope::Enterprise, 0),
                (PathBuf::from("/t/home/.claude/skills"), Scope::User, 1),
                (PathBuf::from("/p/.claude/skills"), Scope::Project, 2),
            ]
        );
    }

    #[test]
    fn claude_code_refresh_none_when_dir_existed_and_reload_when_created() {
        assert_eq!(
            ClaudeCode.refresh(ApplyContext {
                dest_root_existed: true
            }),
            Refresh::None
        );
        assert_eq!(
            ClaudeCode.refresh(ApplyContext {
                dest_root_existed: false
            }),
            Refresh::Command("/reload-skills".into())
        );
    }

    #[test]
    fn refresh_note_texts_cite_the_evidence_url() {
        let none = ClaudeCode.refresh_note(ApplyContext {
            dest_root_existed: true,
        });
        assert_eq!(none.kind, "none");
        assert!(none.text.starts_with("no restart or reload required"));
        assert!(none.text.contains("code.claude.com/docs/en/skills"));
        assert_eq!(none.evidence, DOCS);
        let cmd = ClaudeCode.refresh_note(ApplyContext {
            dest_root_existed: false,
        });
        assert_eq!(cmd.kind, "command");
        assert_eq!(cmd.command.as_deref(), Some("/reload-skills"));
        assert!(cmd.text.contains("/reload-skills"));
        assert!(cmd.text.contains("code.claude.com/docs/en/skills"));
    }
}
