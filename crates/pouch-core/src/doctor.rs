//! Doctor (spec 5; M4 plan D1): diagnose the environment for one project and
//! target. Read-only except for one probe: a `tempfile` directory created
//! next to the destination root, holding a probe symlink, and removed before
//! `run` returns, so the project is byte-identical afterwards. That probe is
//! the one place core creates files outside `FsOps`; it writes nothing the
//! user keeps.

use crate::adapter::{Detection, EvidenceSource, EvidenceStatus, adapter_for, adapters};
use crate::catalog::Catalog;
use crate::error::{Error, REMEDY_SOURCE_UNREACHABLE, Result};
use crate::observe::{DestRootState, classify_dest_root};
use crate::paths::Dirs;
use crate::project::{ProjectRoot, TargetId};
use crate::state::{Receipt, StateDir};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Result of one check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Ok,
    Fail,
    Info,
}

impl CheckStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Fail => "fail",
            CheckStatus::Info => "info",
        }
    }
}

/// One check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub id: &'static str,
    pub status: CheckStatus,
    pub message: String,
    pub remedy: Option<String>,
}

impl Check {
    fn new(id: &'static str, status: CheckStatus, message: impl Into<String>) -> Self {
        Check {
            id,
            status,
            message: message.into(),
            remedy: None,
        }
    }

    fn with_remedy(mut self, remedy: impl Into<String>) -> Self {
        self.remedy = Some(remedy.into());
        self
    }

    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "status": self.status.as_str(),
            "message": self.message,
            "remedy": self.remedy,
        })
    }
}

/// One adapter claim with its evidence (`doctor --evidence`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRow {
    pub adapter: String,
    pub claim_key: String,
    pub claim: String,
    /// `url` or `spike`.
    pub source_kind: &'static str,
    pub source: String,
    /// `verified` or `assumption`.
    pub status: &'static str,
}

impl EvidenceRow {
    pub fn to_json(&self) -> Value {
        json!({
            "adapter": self.adapter,
            "claim_key": self.claim_key,
            "claim": self.claim,
            "source_kind": self.source_kind,
            "source": self.source,
            "status": self.status,
        })
    }
}

/// The doctor report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    pub target: TargetId,
    pub dest_rel: String,
    pub checks: Vec<Check>,
    pub evidence: Option<Vec<EvidenceRow>>,
}

impl DoctorReport {
    pub fn failed(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count()
    }
}

/// Input of `run`.
pub struct DoctorRequest<'a> {
    pub dirs: &'a Dirs,
    pub catalog: &'a Catalog,
    pub root: &'a ProjectRoot,
    pub target: TargetId,
    pub evidence: bool,
}

#[cfg(unix)]
fn make_link(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(not(unix))]
fn make_link(_target: &Path, _link: &Path) -> std::io::Result<()> {
    Err(std::io::Error::from(std::io::ErrorKind::Unsupported))
}

/// Probe `base`: create a temp directory in it (writability), then a
/// symlink to a directory inside that (symlink capability). The temp
/// directory is removed on return.
fn probe(
    base: &Path,
) -> (
    std::result::Result<(), String>,
    std::result::Result<(), String>,
) {
    let dir = match tempfile::Builder::new()
        .prefix(".pouch-doctor-")
        .tempdir_in(base)
    {
        Ok(d) => d,
        Err(e) => return (Err(e.to_string()), Err("not probed".to_string())),
    };
    let target = dir.path().join("target");
    let link = dir.path().join("link");
    let link_result = std::fs::create_dir(&target)
        .and_then(|()| make_link(&target, &link))
        .and_then(|()| {
            if link.is_dir() {
                Ok(())
            } else {
                Err(std::io::Error::other("the probe link does not resolve"))
            }
        })
        .map_err(|e| e.to_string());
    (Ok(()), link_result)
}

/// The nearest existing directory at or above the destination root.
fn probe_base(root: &Path, rel: &str, state: &DestRootState) -> Option<PathBuf> {
    match state {
        DestRootState::Dir => Some(root.join(rel)),
        DestRootState::Missing { to_create } => to_create
            .first()
            .and_then(|first| root.join(first).parent().map(Path::to_path_buf)),
        DestRootState::NotDir | DestRootState::Outside { .. } => None,
    }
}

fn dest_checks(req: &DoctorRequest<'_>, rel: &str) -> Vec<Check> {
    let root = &req.root.path;
    let state = match classify_dest_root(root, rel) {
        Ok(s) => s,
        Err(e) => {
            return vec![
                Check::new(
                    "destination-writable",
                    CheckStatus::Fail,
                    format!("cannot inspect {rel}: {e}"),
                ),
                Check::new("symlink-capability", CheckStatus::Info, "not probed"),
            ];
        }
    };
    let invalid = match &state {
        DestRootState::Outside { resolved } => Some(format!(
            "{rel} resolves to {resolved}, outside the project; not probed"
        )),
        DestRootState::NotDir => Some(format!("{rel} exists and is not a directory; not probed")),
        _ => None,
    };
    if let Some(message) = invalid {
        return vec![
            Check::new("destination-writable", CheckStatus::Fail, message)
                .with_remedy("Remove or fix the destination root, then re-run `pouch doctor`."),
            Check::new("symlink-capability", CheckStatus::Info, "not probed"),
        ];
    }
    let Some(base) = probe_base(root, rel, &state) else {
        return vec![
            Check::new(
                "destination-writable",
                CheckStatus::Fail,
                format!("no directory to probe for {rel}"),
            ),
            Check::new("symlink-capability", CheckStatus::Info, "not probed"),
        ];
    };
    let (writable, symlink) = probe(&base);
    let writable_check = match writable {
        Ok(()) => Check::new(
            "destination-writable",
            CheckStatus::Ok,
            format!("{} accepts new entries", base.display()),
        ),
        Err(e) => Check::new(
            "destination-writable",
            CheckStatus::Fail,
            format!("cannot create entries in {}: {e}", base.display()),
        )
        .with_remedy("Fix the permissions of that directory."),
    };
    let symlink_check = match symlink {
        Ok(()) => Check::new(
            "symlink-capability",
            CheckStatus::Ok,
            "directory symbolic links can be created and followed here",
        ),
        Err(e) if e == "not probed" => Check::new("symlink-capability", CheckStatus::Info, e),
        Err(e) => Check::new(
            "symlink-capability",
            CheckStatus::Fail,
            format!("cannot create a directory symbolic link: {e}"),
        )
        .with_remedy(
            "Set `strategy: copy` for the target in `.pouch/selection.yaml`, or `defaults.strategy = \"copy\"` in config.toml.",
        ),
    };
    vec![writable_check, symlink_check]
}

fn evidence_rows() -> Vec<EvidenceRow> {
    let mut rows = Vec::new();
    for a in adapters() {
        for e in a.evidence() {
            let (source_kind, source) = match e.source {
                EvidenceSource::Url(u) => ("url", u),
                EvidenceSource::Spike(p) => ("spike", p),
            };
            rows.push(EvidenceRow {
                adapter: a.id().as_str().to_string(),
                claim_key: e.claim_key.to_string(),
                claim: e.claim.to_string(),
                source_kind,
                source: source.to_string(),
                status: match e.status {
                    EvidenceStatus::Verified => "verified",
                    EvidenceStatus::Assumption => "assumption",
                },
            });
        }
    }
    rows
}

/// Run the checks (spec 5 `doctor`).
pub fn run(req: &DoctorRequest<'_>) -> Result<DoctorReport> {
    let adapter = adapter_for(req.target).ok_or_else(|| Error::TargetUnsupported {
        target: req.target.as_str().to_string(),
    })?;
    let root = &req.root.path;
    let rel = adapter.destination();
    let mut checks = dest_checks(req, rel);

    checks.push(Check::new(
        "adapter-detection",
        CheckStatus::Info,
        match adapter.detect(root, req.dirs) {
            Detection::Present { reasons } => {
                format!("{} detected ({})", req.target, reasons.join("; "))
            }
            Detection::Absent => {
                format!(
                    "{} not detected; {rel} will be created on the first apply",
                    req.target
                )
            }
            Detection::Unknown { reason } => format!("{} detection unknown: {reason}", req.target),
        },
    ));

    if req.catalog.sources.is_empty() {
        checks.push(Check::new(
            "source-reachability",
            CheckStatus::Info,
            "no sources registered",
        ));
    }
    for s in &req.catalog.sources {
        checks.push(if s.reachable {
            Check::new(
                "source-reachability",
                CheckStatus::Ok,
                format!(
                    "source `{}` at {} is reachable",
                    s.alias,
                    s.expanded.display()
                ),
            )
        } else {
            Check::new(
                "source-reachability",
                CheckStatus::Fail,
                format!(
                    "source `{}` at {} is unreachable",
                    s.alias,
                    s.expanded.display()
                ),
            )
            .with_remedy(REMEDY_SOURCE_UNREACHABLE)
        });
    }

    let state = StateDir::new(root, req.target);
    checks.push(match Receipt::load(&state, root, req.target) {
        Err(e) => Check::new("receipt-integrity", CheckStatus::Fail, e.to_string())
            .with_remedy("Fix or move aside the receipt `.pouch/<target>/applied.json`."),
        Ok(r) => {
            let bad: Vec<String> = r
                .skill_entries()
                .filter(|e| !e.consistent(rel))
                .map(|e| e.skill_id.clone())
                .collect();
            if bad.is_empty() {
                Check::new(
                    "receipt-integrity",
                    CheckStatus::Ok,
                    format!(
                        "{} receipt entry(ies), all consistent",
                        r.skill_entries().count()
                    ),
                )
            } else {
                Check::new(
                    "receipt-integrity",
                    CheckStatus::Fail,
                    format!(
                        "{} receipt entry(ies) do not match the destination: {}",
                        bad.len(),
                        bad.join(", ")
                    ),
                )
                .with_remedy("Inspect `.pouch/<target>/applied.json` by hand; nothing was changed.")
            }
        }
    });

    checks.push(
        if std::fs::symlink_metadata(state.journal().path()).is_ok() {
            Check::new(
                "journal-presence",
                CheckStatus::Fail,
                "an interrupted apply left a journal",
            )
            .with_remedy("Run `pouch resume` to finish it, or `pouch rollback` to undo it.")
        } else {
            Check::new("journal-presence", CheckStatus::Ok, "no interrupted apply")
        },
    );

    Ok(DoctorReport {
        target: req.target,
        dest_rel: rel.to_string(),
        checks,
        evidence: req.evidence.then(evidence_rows),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::fixture::*;

    fn report(f: &Fx, evidence: bool) -> DoctorReport {
        run(&DoctorRequest {
            dirs: &f.dirs,
            catalog: &f.catalog,
            root: &f.root,
            target: TargetId::ClaudeCode,
            evidence,
        })
        .unwrap()
    }

    fn status_of(r: &DoctorReport, id: &str) -> Vec<CheckStatus> {
        r.checks
            .iter()
            .filter(|c| c.id == id)
            .map(|c| c.status)
            .collect()
    }

    fn listing(root: &Path) -> Vec<String> {
        let mut out = Vec::new();
        let mut stack = vec![root.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap() {
                let p = e.unwrap().path();
                out.push(p.strip_prefix(root).unwrap().to_string_lossy().into_owned());
                if p.is_dir() && !p.is_symlink() {
                    stack.push(p);
                }
            }
        }
        out.sort();
        out
    }

    #[cfg(unix)]
    #[test]
    fn a_clean_project_passes_every_check_and_is_left_untouched() {
        let f = fx();
        f.add(&["fx:sdlc/code-review"]);
        let before = listing(&f.root.path);
        let r = report(&f, false);
        assert_eq!(r.failed(), 0, "{:?}", r.checks);
        for id in [
            "symlink-capability",
            "destination-writable",
            "source-reachability",
            "receipt-integrity",
            "journal-presence",
        ] {
            assert_eq!(status_of(&r, id), vec![CheckStatus::Ok], "{id}");
        }
        assert_eq!(status_of(&r, "adapter-detection"), vec![CheckStatus::Info]);
        assert!(r.evidence.is_none());
        assert_eq!(listing(&f.root.path), before);
        // A project with no destination root yet is probed one level up.
        let g = fx();
        let before = listing(&g.root.path);
        let r = report(&g, false);
        assert_eq!(r.failed(), 0);
        assert_eq!(listing(&g.root.path), before);
        assert!(!g.root.path.join(".claude").exists());
    }

    #[cfg(unix)]
    #[test]
    fn unreachable_source_and_leftover_journal_fail() {
        let mut f = fx();
        f.add(&["fx:sdlc/code-review"]);
        write(&f.root.path.join(".pouch/claude-code/journal.jsonl"), "");
        std::fs::remove_dir_all(&f.lib).unwrap();
        f.reload();
        let r = report(&f, false);
        assert_eq!(
            status_of(&r, "source-reachability"),
            vec![CheckStatus::Fail]
        );
        assert_eq!(status_of(&r, "journal-presence"), vec![CheckStatus::Fail]);
        assert_eq!(r.failed(), 2);
    }

    #[cfg(unix)]
    #[test]
    fn an_invalid_receipt_and_a_destination_outside_the_project_fail() {
        let f = fx();
        write(
            &f.root.path.join(".pouch/claude-code/applied.json"),
            "{not json",
        );
        let r = report(&f, false);
        assert_eq!(status_of(&r, "receipt-integrity"), vec![CheckStatus::Fail]);
        let g = fx();
        let outside = g.base.join("outside");
        std::fs::create_dir_all(outside.join("skills")).unwrap();
        std::os::unix::fs::symlink(&outside, g.root.path.join(".claude")).unwrap();
        let r = report(&g, false);
        assert_eq!(
            status_of(&r, "destination-writable"),
            vec![CheckStatus::Fail]
        );
        assert_eq!(status_of(&r, "symlink-capability"), vec![CheckStatus::Info]);
        assert_eq!(listing(&outside), vec!["skills"]);
    }

    #[test]
    fn listing_has_every_adapter_claim_with_its_source() {
        let f = fx();
        let r = report(&f, true);
        let rows = r.evidence.unwrap();
        let expected: usize = adapters().iter().map(|a| a.evidence().len()).sum();
        assert_eq!(rows.len(), expected);
        assert!(
            rows.iter()
                .all(|e| !e.source.is_empty() && !e.claim.is_empty())
        );
        assert!(
            rows.iter()
                .any(|e| e.claim_key == "destination" && e.source_kind == "url")
        );
        assert!(
            rows.iter()
                .any(|e| e.claim_key == "strategy:symlink" && e.source_kind == "spike")
        );
    }

    #[cfg(unix)]
    #[test]
    fn an_unwritable_destination_fails() {
        use std::os::unix::fs::PermissionsExt;
        let f = fx();
        std::fs::set_permissions(&f.root.path, std::fs::Permissions::from_mode(0o555)).unwrap();
        let writable = std::fs::create_dir(f.root.path.join("probe-dir")).is_ok();
        if writable {
            println!("SKIP-ROOT: running as root; chmod 555 does not block writes");
            std::fs::remove_dir(f.root.path.join("probe-dir")).unwrap();
        } else {
            let r = report(&f, false);
            assert_eq!(
                status_of(&r, "destination-writable"),
                vec![CheckStatus::Fail]
            );
        }
        std::fs::set_permissions(&f.root.path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
