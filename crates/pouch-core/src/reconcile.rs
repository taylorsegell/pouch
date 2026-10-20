//! Reconcile (spec 4.12; M4 plan B3): `verify` plus the divergences grouped
//! by cause and the repair plan. Reporting is read-only. The repair itself
//! is `change::run` with `Edit::Reconcile`, so it inherits the lock, the
//! journal, the history, and `resume` and `rollback`.

use crate::catalog::SkillId;
use crate::change::{ChangeRequest, Edit, prepare};
use crate::error::Error;
use crate::plan::Plan;
use crate::state::Clock;
use crate::verify::{EntryState, Reason, Record, VerifyReport, VerifyRequest, verify};
use serde_json::{Value, json};

/// Why the destination diverges from the selection (M4-6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Desired, but there is no receipt entry.
    NotApplied,
    HandDeleted,
    HandModified,
    SourceMoved,
    SourceUnreachable,
    SourceSkillMissing,
    /// Receipted, but no longer desired.
    Deselected,
}

impl Cause {
    pub fn as_str(self) -> &'static str {
        match self {
            Cause::NotApplied => "not-applied",
            Cause::HandDeleted => "hand-deleted",
            Cause::HandModified => "hand-modified",
            Cause::SourceMoved => "source-moved",
            Cause::SourceUnreachable => "source-unreachable",
            Cause::SourceSkillMissing => "source-skill-missing",
            Cause::Deselected => "deselected",
        }
    }
}

/// One divergence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub cause: Cause,
    pub id: Option<String>,
    pub name: String,
    pub dest: String,
    pub detail: String,
}

impl Divergence {
    pub fn to_json(&self) -> Value {
        json!({
            "cause": self.cause.as_str(),
            "id": self.id,
            "name": self.name,
            "dest": self.dest,
            "detail": self.detail,
        })
    }
}

/// An informational finding that is not a divergence (exit 0).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// `shadowed`, `foreign`, `brief-missing`, or `brief-modified`.
    pub kind: &'static str,
    pub name: String,
    pub dest: String,
    pub detail: String,
}

impl Finding {
    pub fn to_json(&self) -> Value {
        json!({
            "kind": self.kind,
            "name": self.name,
            "dest": self.dest,
            "detail": self.detail,
        })
    }
}

/// The reconcile report.
#[derive(Debug)]
pub struct ReconcileReport {
    pub verify: VerifyReport,
    pub divergences: Vec<Divergence>,
    pub findings: Vec<Finding>,
    /// The repair plan (what `--repair` would do); `None` when it could not
    /// be computed.
    pub plan: Option<Plan>,
    /// Why `--repair` cannot run now (conflicts or an unresolved reference).
    pub repair_blocked: Option<String>,
}

fn orphan_cause(req: &VerifyRequest<'_>, rec: &Record, reason: Reason) -> Cause {
    match reason {
        Reason::Missing => Cause::HandDeleted,
        Reason::Modified => Cause::HandModified,
        Reason::SourceSkillMissing => Cause::SourceSkillMissing,
        Reason::BrokenLink => {
            let id = rec.id.as_deref().and_then(|t| SkillId::parse(t).ok());
            let Some(id) = id else {
                return Cause::HandModified;
            };
            if !req.catalog.source(&id.source).is_some_and(|s| s.reachable) {
                return Cause::SourceUnreachable;
            }
            if req.catalog.skill(&id).is_none() {
                return Cause::SourceSkillMissing;
            }
            let live = req
                .catalog
                .abs_path(&id)
                .map(|p| p.to_string_lossy().into_owned());
            let linked = std::fs::read_link(req.root.path.join(&rec.dest))
                .ok()
                .map(|p| p.to_string_lossy().into_owned());
            if live.is_some() && linked != live {
                Cause::SourceMoved
            } else {
                Cause::HandModified
            }
        }
    }
}

fn detail_of(cause: Cause, rec: &Record) -> String {
    let id = rec.id.as_deref().unwrap_or("");
    match cause {
        Cause::NotApplied => format!("{id} is selected but not applied"),
        Cause::HandDeleted => format!("{} is gone from the destination ({id})", rec.dest),
        Cause::HandModified => format!("{} was changed after pouch placed it ({id})", rec.dest),
        Cause::SourceMoved => format!(
            "{} links to a path that no longer resolves; the source now lives at {} ({id})",
            rec.dest,
            rec.source_path.as_deref().unwrap_or("?")
        ),
        Cause::SourceUnreachable => format!("the source of {id} is unreachable"),
        Cause::SourceSkillMissing => {
            let mut s = format!("{id} is no longer in its source");
            if !rec.candidates.is_empty() {
                s.push_str(&format!("; closest: {}", rec.candidates.join(", ")));
            }
            s
        }
        Cause::Deselected => format!("{id} is applied but no longer selected"),
    }
}

/// Build the report: classify (spec 4.11), group the divergences, and
/// compute the repair plan with `prepare` (no write).
pub fn report(req: &VerifyRequest<'_>, clock: &dyn Clock) -> crate::error::Result<ReconcileReport> {
    let verify_report = verify(req)?;
    let mut divergences: Vec<Divergence> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    for rec in &verify_report.records {
        let cause = match rec.state {
            EntryState::Selected => Some(Cause::NotApplied),
            EntryState::Orphaned => rec.reason.map(|r| orphan_cause(req, rec, r)),
            EntryState::Applied | EntryState::Active | EntryState::Shadowed => rec
                .id
                .as_ref()
                .filter(|id| !verify_report.desired_ids.contains(id))
                .map(|_| Cause::Deselected),
            EntryState::Foreign => None,
        };
        if let Some(cause) = cause {
            divergences.push(Divergence {
                cause,
                id: rec.id.clone(),
                name: rec.name.clone(),
                dest: rec.dest.clone(),
                detail: detail_of(cause, rec),
            });
        }
        match rec.state {
            EntryState::Shadowed => findings.push(Finding {
                kind: "shadowed",
                name: rec.name.clone(),
                dest: rec.dest.clone(),
                detail: format!(
                    "shadowed by {}",
                    rec.shadowed_by
                        .as_deref()
                        .unwrap_or("a higher-precedence scope")
                ),
            }),
            EntryState::Foreign => findings.push(Finding {
                kind: "foreign",
                name: rec.name.clone(),
                dest: rec.dest.clone(),
                detail: "not managed by pouch; never touched".to_string(),
            }),
            _ => {}
        }
    }
    for d in &verify_report.diagnostics {
        let kind = match d.code.as_str() {
            "brief-missing" => "brief-missing",
            "brief-modified" => "brief-modified",
            _ => continue,
        };
        let dest = d.path.clone().unwrap_or_default();
        findings.push(Finding {
            kind,
            name: dest.rsplit('/').next().unwrap_or("").to_string(),
            dest,
            detail: d.message.clone(),
        });
    }
    let change_req = ChangeRequest {
        dirs: req.dirs,
        config: req.config,
        catalog: req.catalog,
        bundles: req.bundles,
        root: req.root,
        target: req.target,
        edit: Edit::Reconcile,
        command: "reconcile",
        clock,
    };
    let (plan, repair_blocked) = match prepare(&change_req) {
        Ok(p) => {
            let blocked = p.plan.is_blocked().then(|| {
                Error::Conflicts {
                    count: p.plan.conflicts.len(),
                }
                .to_string()
            });
            (Some(p.plan), blocked)
        }
        Err(f) => (None, Some(f.error.to_string())),
    };
    Ok(ReconcileReport {
        verify: verify_report,
        divergences,
        findings,
        plan,
        repair_blocked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::change::Outcome;
    use crate::verify::fixture::*;

    fn causes(r: &ReconcileReport) -> Vec<(&'static str, String)> {
        r.divergences
            .iter()
            .map(|d| (d.cause.as_str(), d.name.clone()))
            .collect()
    }

    #[cfg(unix)]
    #[test]
    fn hand_deleted_is_reported_and_repair_plan_re_adds() {
        let f = fx();
        f.add(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]);
        let clean = report(&f.request(), &f.clock).unwrap();
        assert!(clean.divergences.is_empty());
        assert!(clean.plan.as_ref().unwrap().steps.is_empty());
        std::fs::remove_file(f.dest("code-review")).unwrap();
        let r = report(&f.request(), &f.clock).unwrap();
        assert_eq!(
            causes(&r),
            vec![("hand-deleted", "code-review".to_string())]
        );
        let plan = r.plan.as_ref().unwrap();
        let ops: Vec<&str> = plan.steps.iter().map(|s| s.op()).collect();
        assert_eq!(ops, vec!["add"]);
        assert!(r.repair_blocked.is_none());
        let out = f.run(Edit::Reconcile);
        assert!(matches!(out, Outcome::Applied(..)));
        let after = report(&f.request(), &f.clock).unwrap();
        assert!(after.divergences.is_empty());
        // A second repair is a no-op.
        assert!(matches!(f.run(Edit::Reconcile), Outcome::NoChanges(_)));
    }

    #[cfg(unix)]
    #[test]
    fn each_cause_is_named() {
        let mut f = fx();
        f.add(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]);
        // modified: a hand-made link in place of the owned one.
        std::fs::remove_file(f.dest("pytest-fixtures")).unwrap();
        std::os::unix::fs::symlink(f.lib.join("sdlc/code-review"), f.dest("pytest-fixtures"))
            .unwrap();
        // not-applied through a hand-edited selection.
        write(
            &crate::project::selection_path(&f.root.path),
            "version: 1\ntargets:\n  claude-code:\n    add:\n      - fx:sdlc/code-review\n      - fx:testing/pytest-fixtures\n      - fx:data/schema-migration\n",
        );
        // A foreign directory is a finding, not a divergence.
        write(&f.dest("house-style/SKILL.md"), "x");
        f.reload();
        let r = report(&f.request(), &f.clock).unwrap();
        assert_eq!(
            causes(&r),
            vec![
                ("hand-modified", "pytest-fixtures".to_string()),
                ("not-applied", "schema-migration".to_string()),
            ]
        );
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.findings[0].kind, "foreign");
        // The modified entry blocks a repair; nothing is deleted.
        assert!(r.repair_blocked.is_some());
    }

    #[cfg(unix)]
    #[test]
    fn a_moved_source_is_repointed_by_the_repair() {
        let mut f = fx();
        f.add(&["fx:sdlc/code-review"]);
        let new_lib = f.base.join("moved-lib");
        std::fs::rename(&f.lib, &new_lib).unwrap();
        f.config.sources[0].path = new_lib.to_string_lossy().into_owned();
        f.reload();
        let r = report(&f.request(), &f.clock).unwrap();
        assert_eq!(
            causes(&r),
            vec![("source-moved", "code-review".to_string())]
        );
        let ops: Vec<&str> = r
            .plan
            .as_ref()
            .unwrap()
            .steps
            .iter()
            .map(|s| s.op())
            .collect();
        assert_eq!(ops, vec!["remove", "add"]);
        assert!(matches!(f.run(Edit::Reconcile), Outcome::Applied(..)));
        let after = f.verify();
        assert_eq!(after.records[0].state, EntryState::Active);
        assert_eq!(
            std::fs::read_link(f.dest("code-review")).unwrap(),
            new_lib.join("sdlc/code-review")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreachable_and_renamed_sources_are_named_and_block_the_repair() {
        let mut f = fx();
        f.add(&["fx:sdlc/code-review"]);
        std::fs::rename(
            f.lib.join("sdlc/code-review"),
            f.lib.join("sdlc/code-reviewer"),
        )
        .unwrap();
        f.reload();
        let r = report(&f.request(), &f.clock).unwrap();
        assert_eq!(
            causes(&r),
            vec![("source-skill-missing", "code-review".to_string())]
        );
        assert!(r.divergences[0].detail.contains("fx:sdlc/code-reviewer"));
        assert!(r.plan.is_none());
        assert!(
            r.repair_blocked
                .as_deref()
                .unwrap()
                .contains("no skill matches")
        );
        let g = {
            let mut g = fx();
            g.add(&["fx:sdlc/code-review"]);
            std::fs::remove_dir_all(&g.lib).unwrap();
            g.reload();
            g
        };
        let r = report(&g.request(), &g.clock).unwrap();
        assert_eq!(
            causes(&r),
            vec![("source-unreachable", "code-review".to_string())]
        );
    }
}
