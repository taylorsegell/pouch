//! The `agents-neutral` adapter (spec 4.7; M5 plan M5-1, M5-4 to M5-6):
//! the shared `.agents/skills` directory that Codex, Cursor, Windsurf and
//! Devin, and Hermes read. Default strategy symlink per the M0 Outcome
//! (`spike/symlink-discovery.md`). Every claim carries evidence (PRD S9).

use super::{
    Adapter, ApplyContext, Detection, Evidence, EvidenceSource, EvidenceStatus, Followup, Refresh,
    RefreshNote, ScanLocation, Scope,
};
use crate::config::Strategy;
use crate::observe::ancestors_to_git_root;
use crate::paths::Dirs;
use crate::project::TargetId;
use std::path::Path;

const CODEX: &str = "https://learn.chatgpt.com/docs/build-skills.md";
const CURSOR: &str = "https://cursor.com/docs/skills";
const AGENT_SKILLS: &str = "https://agentskills.io/client-implementation/adding-skills-support.md";
const DEVIN: &str = "https://docs.devin.ai/desktop/cascade/skills";
const HERMES: &str = "https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md";
const SPIKE: &str = "spike/symlink-discovery.md";

/// The `agents-neutral` adapter.
#[derive(Debug, Clone, Copy, Default)]
pub struct AgentsNeutral;

static CONSUMERS: &[&str] = &["codex", "cursor", "windsurf-devin", "hermes"];

static STRATEGIES: &[Strategy] = &[Strategy::Symlink, Strategy::Copy];

static EVIDENCE: &[Evidence] = &[
    Evidence {
        claim_key: "destination",
        claim: "Project skills live in .agents/skills/<name>/SKILL.md (Codex and Cursor document it)",
        source: EvidenceSource::Url(CODEX),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:project",
        claim: "<project>/.agents/skills is read by Codex and Cursor",
        source: EvidenceSource::Url(CURSOR),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:ancestor",
        claim: "Codex reads every .agents/skills from the working directory up to the repository root",
        source: EvidenceSource::Url(CODEX),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "scan:user",
        claim: "~/.agents/skills is read as the user scope",
        source: EvidenceSource::Url(CODEX),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "precedence",
        claim: "A project skill beats a user skill of the same name; the order between ancestor directories is not documented (an assumption, so a same-name skill there is a warning, not a shadow)",
        source: EvidenceSource::Url(AGENT_SKILLS),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "strategy:symlink",
        claim: "A symlinked skill directory in .agents/skills was discovered by Codex 0.160.0, the cursor-agent CLI 2026.10.01, and Hermes v0.21.5 after trust (run 2026-10-05)",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "strategy:copy",
        claim: "A copied skill directory in .agents/skills is discovered like any project skill",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:unknown",
        claim: "Whether a running agent notices a skill added to .agents/skills differs per agent, so pouch reports it per consumer and never claims a refresh is unnecessary",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:codex",
        claim: "Codex detects skill changes automatically; restart it if the skill does not appear",
        source: EvidenceSource::Url(CODEX),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:cursor",
        claim: "Cursor does not document refresh; the cursor-agent CLI showed a mid-session addition only after a restart (the Cursor editor was not tested)",
        source: EvidenceSource::Spike(SPIKE),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "refresh:windsurf-devin",
        claim: "Windsurf and Devin do not document refresh (Windsurf was not tested); this is an assumption from the absence of documentation",
        source: EvidenceSource::Url(DEVIN),
        status: EvidenceStatus::Assumption,
    },
    Evidence {
        claim_key: "refresh:hermes",
        claim: "Hermes lists project skills only after `hermes skills trust` is run once per repository, and needed a restart for a mid-session addition",
        source: EvidenceSource::Url(HERMES),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:codex",
        claim: "Codex reads .agents/skills",
        source: EvidenceSource::Url(CODEX),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:cursor",
        claim: "Cursor reads .agents/skills (documented; the cursor-agent CLI spike is supporting evidence, the editor is untested)",
        source: EvidenceSource::Url(CURSOR),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:windsurf-devin",
        claim: "Windsurf and Devin read .agents/skills and ~/.agents/skills",
        source: EvidenceSource::Url(DEVIN),
        status: EvidenceStatus::Verified,
    },
    Evidence {
        claim_key: "consumers:hermes",
        claim: "Hermes reads .agents/skills once the repository is trusted",
        source: EvidenceSource::Url(HERMES),
        status: EvidenceStatus::Verified,
    },
];

/// The refresh sentence, one clause per consumer (spec 4.7).
fn refresh_text() -> String {
    format!(
        "refresh is not documented for this shared directory. Codex: detects changes automatically; restart it if the skill does not appear. Cursor: not documented (the cursor-agent CLI needed a restart in testing). Windsurf and Devin: not documented. Hermes: run `hermes skills trust` once per repository, and restart it for skills added mid-session (evidence: {CODEX}, {HERMES}, {SPIKE})"
    )
}

/// `text` shell-quoted only when it holds characters a shell would split or
/// expand.
fn shell_word(text: &str) -> String {
    let plain = !text.is_empty()
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+' | ':' | '@')
        });
    if plain {
        text.to_string()
    } else {
        format!("'{}'", text.replace('\'', "'\\''"))
    }
}

impl Adapter for AgentsNeutral {
    fn id(&self) -> TargetId {
        TargetId::AgentsNeutral
    }

    fn consumers(&self) -> &'static [&'static str] {
        CONSUMERS
    }

    fn detect(&self, root: &Path, dirs: &Dirs) -> Detection {
        let mut reasons = Vec::new();
        if std::fs::symlink_metadata(root.join(".agents")).is_ok() {
            reasons.push("<root>/.agents exists".to_string());
        }
        if let Ok(home) = dirs.home() {
            for (rel, label) in [
                (".codex", "~/.codex exists"),
                (".cursor", "~/.cursor exists"),
                (".codeium", "~/.codeium exists"),
                (".config/devin", "~/.config/devin exists"),
                (".hermes", "~/.hermes exists"),
            ] {
                if std::fs::symlink_metadata(home.join(rel)).is_ok() {
                    reasons.push(label.to_string());
                }
            }
        }
        if reasons.is_empty() {
            Detection::Absent
        } else {
            Detection::Present { reasons }
        }
    }

    fn destination(&self) -> &'static str {
        ".agents/skills"
    }

    fn scan_locations(&self, root: &Path, dirs: &Dirs) -> Vec<ScanLocation> {
        let mut out = vec![ScanLocation {
            path: root.join(".agents").join("skills"),
            scope: Scope::Project,
            rank: 0,
        }];
        for dir in ancestors_to_git_root(root) {
            out.push(ScanLocation {
                path: dir.join(".agents").join("skills"),
                scope: Scope::Ancestor,
                rank: 1,
            });
        }
        if let Ok(home) = dirs.home() {
            out.push(ScanLocation {
                path: home.join(".agents").join("skills"),
                scope: Scope::User,
                rank: 2,
            });
        }
        out
    }

    fn strategies(&self) -> &'static [Strategy] {
        STRATEGIES
    }

    fn default_strategy(&self) -> Strategy {
        Strategy::Symlink
    }

    fn refresh(&self, _ctx: ApplyContext) -> Refresh {
        Refresh::Unknown
    }

    fn refresh_note(&self, _ctx: ApplyContext) -> RefreshNote {
        RefreshNote {
            kind: "unknown",
            command: None,
            text: refresh_text(),
            evidence: format!("{CODEX} {HERMES} {SPIKE}"),
        }
    }

    fn followups(&self, root: &Path, dirs: &Dirs) -> Vec<Followup> {
        let hermes = dirs
            .home()
            .is_ok_and(|h| std::fs::symlink_metadata(h.join(".hermes")).is_ok());
        if !hermes {
            return Vec::new();
        }
        vec![Followup {
            command: format!("hermes skills trust {}", shell_word(&root.to_string_lossy())),
            reason: "Hermes lists project skills only after the repository is trusted; pouch prints this and never runs it".to_string(),
        }]
    }

    fn evidence(&self) -> &'static [Evidence] {
        EVIDENCE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dirs(root: &Path) -> Dirs {
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
        Dirs::resolve(crate::paths::Env::from_map(m)).unwrap()
    }

    #[test]
    fn identity_consumers_and_strategy() {
        let a = AgentsNeutral;
        assert_eq!(a.id(), TargetId::AgentsNeutral);
        assert_eq!(a.destination(), ".agents/skills");
        assert_eq!(
            a.consumers(),
            &["codex", "cursor", "windsurf-devin", "hermes"]
        );
        assert_eq!(a.strategies(), &[Strategy::Symlink, Strategy::Copy]);
        assert_eq!(a.default_strategy(), Strategy::Symlink);
        assert!(
            a.evidence()
                .iter()
                .any(|e| e.claim_key == "strategy:symlink"
                    && e.source == EvidenceSource::Spike(SPIKE)
                    && e.status == EvidenceStatus::Verified)
        );
    }

    #[test]
    fn every_consumer_has_a_refresh_claim() {
        for c in AgentsNeutral.consumers() {
            let key = format!("refresh:{c}");
            assert!(
                AgentsNeutral.evidence().iter().any(|e| e.claim_key == key),
                "no {key}"
            );
        }
        assert!(
            AgentsNeutral
                .evidence()
                .iter()
                .any(|e| e.claim_key == "refresh:unknown")
        );
    }

    #[test]
    fn detect_rows() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path());
        let root = t.path().join("proj");
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(AgentsNeutral.detect(&root, &d), Detection::Absent);
        std::fs::create_dir_all(root.join(".agents")).unwrap();
        assert_eq!(
            AgentsNeutral.detect(&root, &d),
            Detection::Present {
                reasons: vec!["<root>/.agents exists".into()]
            }
        );
        std::fs::remove_dir_all(root.join(".agents")).unwrap();
        for (rel, label) in [
            (".codex", "~/.codex exists"),
            (".cursor", "~/.cursor exists"),
            (".codeium", "~/.codeium exists"),
            (".config/devin", "~/.config/devin exists"),
            (".hermes", "~/.hermes exists"),
        ] {
            let p = t.path().join("home").join(rel);
            std::fs::create_dir_all(&p).unwrap();
            assert_eq!(
                AgentsNeutral.detect(&root, &d),
                Detection::Present {
                    reasons: vec![label.into()]
                },
                "{rel}"
            );
            std::fs::remove_dir_all(&p).unwrap();
        }
        // Claude's own directory does not count.
        std::fs::create_dir_all(t.path().join("home/.claude")).unwrap();
        assert_eq!(AgentsNeutral.detect(&root, &d), Detection::Absent);
    }

    fn rows(a: &AgentsNeutral, root: &Path, d: &Dirs) -> Vec<(PathBuf, Scope, u8)> {
        a.scan_locations(root, d)
            .into_iter()
            .map(|l| (l.path, l.scope, l.rank))
            .collect()
    }

    #[test]
    fn scan_locations_git_root_has_no_ancestors() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path());
        let root = t.path().join("repo");
        std::fs::create_dir_all(root.join(".git")).unwrap();
        assert_eq!(
            rows(&AgentsNeutral, &root, &d),
            vec![
                (root.join(".agents/skills"), Scope::Project, 0),
                (t.path().join("home/.agents/skills"), Scope::User, 2),
            ]
        );
    }

    #[test]
    fn scan_locations_below_the_git_root_list_each_ancestor_nearest_first() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path());
        let repo = t.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        let root = repo.join("packages/web");
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(
            rows(&AgentsNeutral, &root, &d),
            vec![
                (root.join(".agents/skills"), Scope::Project, 0),
                (repo.join("packages/.agents/skills"), Scope::Ancestor, 1),
                (repo.join(".agents/skills"), Scope::Ancestor, 1),
                (t.path().join("home/.agents/skills"), Scope::User, 2),
            ]
        );
    }

    #[test]
    fn scan_locations_without_any_git_root_have_no_ancestors() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path());
        let root = t.path().join("a/b/c");
        std::fs::create_dir_all(&root).unwrap();
        let got = rows(&AgentsNeutral, &root, &d);
        assert_eq!(got.len(), 2, "{got:?}");
        assert!(got.iter().all(|(_, s, _)| *s != Scope::Ancestor));
    }

    #[test]
    fn refresh_is_unknown_and_never_claims_none() {
        let ctx = ApplyContext {
            dest_root_existed: true,
        };
        assert_eq!(AgentsNeutral.refresh(ctx), Refresh::Unknown);
        let n = AgentsNeutral.refresh_note(ctx);
        assert_eq!(n.kind, "unknown");
        assert!(n.command.is_none());
        assert!(n.text.contains("not documented"));
        for clause in ["Codex", "Cursor", "Windsurf and Devin", "Hermes"] {
            assert!(n.text.contains(clause), "{clause}: {}", n.text);
        }
        assert!(n.text.contains("hermes skills trust"));
        assert!(!n.text.contains("no restart or reload required"));
        assert!(n.evidence.starts_with("https://"));
    }

    #[test]
    fn followup_only_when_hermes_home_exists_and_is_text_only() {
        let t = tempfile::tempdir().unwrap();
        let d = dirs(t.path());
        let root = Path::new("/p/my proj");
        assert!(AgentsNeutral.followups(root, &d).is_empty());
        std::fs::create_dir_all(t.path().join("home/.hermes")).unwrap();
        let f = AgentsNeutral.followups(root, &d);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].command, "hermes skills trust '/p/my proj'");
        let plain = AgentsNeutral.followups(Path::new("/p/proj"), &d);
        assert_eq!(plain[0].command, "hermes skills trust /p/proj");
        assert_eq!(shell_word("a'b"), "'a'\\''b'");
    }
}
