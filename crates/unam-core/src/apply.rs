//! The journaled apply (spec 4.10; M3 plan P9, P10, P16; G2 to G6):
//! `execute` runs a plan step by step behind a write-ahead journal and
//! never stops at a failed step; `inspect` classifies a leftover journal;
//! `resume` and `rollback` finish or undo an interrupted apply by observing
//! the disk (the journal is untrusted: every path is rebuilt from the
//! destination root plus a validated name, every removal re-verified).

pub use crate::plan::{StepOutcome, StepResult};

use crate::adapter::{Adapter, ApplyContext, RefreshNote};
use crate::catalog::{Catalog, SkillId};
use crate::error::{DestRootProblem, Diagnostic, Error, Result};
use crate::fsops::{
    DestPath, FsOps, ObservedKind, OwnedExpectation, RemoveOutcome, RestoreOutcome, SkipReason,
    StatePath, observe_one, remove_stale_temp_dirs,
};
use crate::hash::Hash;
use crate::observe::{DestRootState, classify_dest_root};
use crate::plan::{OrphanReason, Plan, ReceiptUpdate, Step, build_receipt, hash_from_text};
use crate::project::selection_path;
use crate::source::SourcePath;
use crate::state::{
    Begin, Clock, HistoryEntry, Journal, JournalExpect, JournalRead, JournalStep, MarkerOutcome,
    Receipt, Record, SelectionAfter, SelectionBefore, StateDir, StepState, prune_history,
    read_journal, write_history,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// History entries kept per target.
pub const HISTORY_KEEP: usize = 20;

/// Everything an apply needs.
pub struct ApplyCtx<'a> {
    /// The canonical project root.
    pub root: PathBuf,
    pub adapter: &'a dyn Adapter,
    pub state: StateDir,
    pub dest: DestPath,
    pub catalog: Option<&'a Catalog>,
    pub clock: &'a dyn Clock,
}

impl<'a> ApplyCtx<'a> {
    pub fn new(
        root: &Path,
        adapter: &'a dyn Adapter,
        catalog: Option<&'a Catalog>,
        clock: &'a dyn Clock,
    ) -> Result<ApplyCtx<'a>> {
        let dest = DestPath::dest_root(root, adapter.destination()).map_err(|detail| {
            Error::DestRootInvalid {
                problem: DestRootProblem::NotDirectory,
                dest_root: adapter.destination().to_string(),
                detail,
            }
        })?;
        Ok(ApplyCtx {
            root: root.to_path_buf(),
            adapter,
            state: StateDir::new(root, adapter.id()),
            dest,
            catalog,
            clock,
        })
    }

    fn root_text(&self) -> String {
        self.root.to_string_lossy().into_owned()
    }
}

/// The selection side of an apply (G11).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionPrelude {
    pub command: String,
    pub selection_before: SelectionBefore,
    pub selection_after_text: String,
    pub selection_after_hash: String,
}

/// Overall result of an apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyStatus {
    Applied,
    AppliedWithSkips,
    Partial,
}

/// What an apply (or a resume) did.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplyReport {
    pub op_id: String,
    pub command: String,
    pub results: Vec<StepResult>,
    pub status: ApplyStatus,
    pub receipt: Receipt,
    pub history_written: bool,
    pub warnings: Vec<Diagnostic>,
    pub new_dirs: Vec<String>,
    pub removed_dirs: Vec<String>,
    pub removal_applied: bool,
    pub refresh: RefreshNote,
}

/// What `resume` or `rollback` did.
#[derive(Debug, Clone, PartialEq)]
pub struct RecoverReport {
    /// `resume` or `rollback`.
    pub action: &'static str,
    pub op_id: String,
    pub command: String,
    pub results: Vec<StepResult>,
    pub status: ApplyStatus,
    /// The receipt after the recovery (`None` when rollback removed it).
    pub receipt: Option<Receipt>,
    pub warnings: Vec<Diagnostic>,
    pub removal_applied: bool,
    pub refresh: Option<RefreshNote>,
}

/// One step of a journal summary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepSummary {
    pub n: u32,
    pub op: &'static str,
    pub name: Option<String>,
    pub state: &'static str,
}

/// A leftover journal, summarized for exit 10.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalSummary {
    pub op_id: String,
    pub command: String,
    pub target: String,
    pub started_at: String,
    pub receipt_written: bool,
    pub steps: Vec<StepSummary>,
}

/// Result of `inspect`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inspection {
    Clear { notes: Vec<String> },
    Interrupted(JournalSummary),
}

// ---- journal conversions (G8c) ----

/// A plan step as a journal step.
pub fn journal_step(step: &Step) -> JournalStep {
    match step {
        Step::Add {
            n,
            id,
            name,
            strategy,
            source,
            tree_hash,
            provenance,
        } => JournalStep::Add {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            strategy: *strategy,
            source: source.to_string_lossy().into_owned(),
            tree_hash: tree_hash.to_string(),
            provenance: provenance.clone(),
        },
        Step::Remove {
            n,
            id,
            name,
            expect,
        } => JournalStep::Remove {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            expect: JournalExpect {
                strategy: expect.strategy,
                source: expect.source.clone(),
                tree_hash: expect.tree_hash.as_ref().map(Hash::to_string),
            },
        },
        Step::DropOrphan {
            n,
            id,
            name,
            reason,
        } => JournalStep::DropOrphan {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            reason: reason.as_str().to_string(),
        },
        Step::UpdateProvenance {
            n,
            id,
            name,
            provenance,
        } => JournalStep::UpdateProvenance {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            provenance: provenance.clone(),
        },
        Step::RemoveDir { n, dir } => JournalStep::RemoveDir {
            n: *n,
            dir: dir.clone(),
        },
    }
}

/// A journal step as a plan step.
pub fn step_from_journal(step: &JournalStep) -> Step {
    match step {
        JournalStep::Add {
            n,
            id,
            name,
            strategy,
            source,
            tree_hash,
            provenance,
        } => Step::Add {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            strategy: *strategy,
            source: PathBuf::from(source),
            tree_hash: hash_from_text(tree_hash),
            provenance: provenance.clone(),
        },
        JournalStep::Remove {
            n,
            id,
            name,
            expect,
        } => Step::Remove {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            expect: OwnedExpectation {
                strategy: expect.strategy,
                source: expect.source.clone(),
                tree_hash: expect.tree_hash.as_deref().map(hash_from_text),
            },
        },
        JournalStep::DropOrphan {
            n,
            id,
            name,
            reason,
        } => Step::DropOrphan {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            reason: OrphanReason::parse(reason).unwrap_or(OrphanReason::Missing),
        },
        JournalStep::UpdateProvenance {
            n,
            id,
            name,
            provenance,
        } => Step::UpdateProvenance {
            n: *n,
            id: id.clone(),
            name: name.clone(),
            provenance: provenance.clone(),
        },
        JournalStep::RemoveDir { n, dir } => Step::RemoveDir {
            n: *n,
            dir: dir.clone(),
        },
    }
}

// ---- helpers ----

/// Refuse a destination root that resolves outside the project or is not a
/// directory, immediately before the first filesystem operation (G2).
pub fn ensure_dest_root(ctx: &ApplyCtx<'_>) -> Result<()> {
    let rel = ctx.adapter.destination();
    match classify_dest_root(&ctx.root, rel)? {
        DestRootState::Outside { resolved } => Err(Error::DestRootInvalid {
            problem: DestRootProblem::Outside,
            dest_root: rel.to_string(),
            detail: format!("it resolves to {resolved}, outside the project"),
        }),
        DestRootState::NotDir => Err(Error::DestRootInvalid {
            problem: DestRootProblem::NotDirectory,
            dest_root: rel.to_string(),
            detail: "it exists and is not a directory".to_string(),
        }),
        DestRootState::Dir | DestRootState::Missing { .. } => Ok(()),
    }
}

fn load_receipt_opt(ctx: &ApplyCtx<'_>) -> Result<Option<Receipt>> {
    if std::fs::symlink_metadata(ctx.state.applied().path()).is_err() {
        return Ok(None);
    }
    Receipt::load(&ctx.state, &ctx.root, ctx.adapter.id()).map(Some)
}

fn failed_outcome(e: &Error) -> StepOutcome {
    StepOutcome::Failed {
        code: e.code().to_string(),
        message: e.to_string(),
    }
}

fn terminal_record(n: u32, outcome: &StepOutcome, how: &str) -> Record {
    match outcome {
        StepOutcome::Done => Record::Done {
            n,
            how: how.to_string(),
        },
        StepOutcome::Failed { code, message } => Record::Failed {
            n,
            code: code.clone(),
            message: message.clone(),
        },
        StepOutcome::Skipped(r) => Record::Skipped {
            n,
            reason: r.code().to_string(),
        },
    }
}

fn outcome_of_state(s: &StepState) -> Option<StepOutcome> {
    match s {
        StepState::Done { .. } => Some(StepOutcome::Done),
        StepState::Skipped { reason } => Some(StepOutcome::Skipped(
            SkipReason::parse(reason).unwrap_or(SkipReason::Modified),
        )),
        StepState::Failed { code, message } => Some(StepOutcome::Failed {
            code: code.clone(),
            message: message.clone(),
        }),
        StepState::Pending | StepState::Started => None,
    }
}

fn status_of(outcomes: &BTreeMap<u32, StepOutcome>) -> ApplyStatus {
    if outcomes
        .values()
        .any(|o| matches!(o, StepOutcome::Failed { .. }))
    {
        ApplyStatus::Partial
    } else if outcomes
        .values()
        .any(|o| matches!(o, StepOutcome::Skipped(_)))
    {
        ApplyStatus::AppliedWithSkips
    } else {
        ApplyStatus::Applied
    }
}

fn result_of(step: &Step, dest_rel: &str, outcome: StepOutcome) -> StepResult {
    StepResult {
        n: step.n(),
        op: step.op(),
        id: step.id().map(str::to_string),
        name: step.name().map(str::to_string),
        dest: step.name().map(|n| format!("{dest_rel}/{n}")),
        outcome,
    }
}

fn is_removal(step: &Step) -> bool {
    matches!(step, Step::Remove { .. } | Step::DropOrphan { .. })
}

/// Shared step runner of `execute` and `resume`.
struct Runner<'r, 'a> {
    ctx: &'r ApplyCtx<'a>,
    fs: &'r dyn FsOps,
    op_id: String,
    planned_dirs: Vec<String>,
    dirs_created: Vec<String>,
    dirs_removed: Vec<String>,
    new_dirs: Vec<String>,
    removed_dirs: Vec<String>,
    warnings: Vec<Diagnostic>,
}

impl Runner<'_, '_> {
    fn append(&self, rec: &Record) -> Result<()> {
        self.fs
            .append_state(&self.ctx.state.journal(), rec.to_line().as_bytes())
    }

    fn ancestor(&self, dir: &str) -> Option<DestPath> {
        self.ctx
            .dest
            .ancestors()
            .into_iter()
            .find(|(rel, _)| rel == dir)
            .map(|(_, p)| p)
    }

    /// Create each missing planned prefix write-ahead (G6c). `Ok(Err(e))`
    /// is an operation failure for the step; `Err` aborts (journal).
    fn ensure_dirs(&mut self) -> Result<std::result::Result<(), Error>> {
        for dir in self.planned_dirs.clone() {
            let Some(p) = self.ancestor(&dir) else {
                continue;
            };
            if std::fs::symlink_metadata(p.path()).is_ok() {
                continue;
            }
            self.append(&Record::Dir { path: dir.clone() })?;
            match self.fs.create_dir(&p) {
                Ok(true) => {
                    if !self.dirs_created.contains(&dir) {
                        self.dirs_created.push(dir.clone());
                    }
                    self.new_dirs.push(dir);
                }
                Ok(false) => self.append(&Record::DirSkipped { path: dir })?,
                Err(e) => return Ok(Err(e)),
            }
        }
        Ok(Ok(()))
    }

    fn child(&self, name: &str) -> std::result::Result<DestPath, Error> {
        self.ctx
            .dest
            .join_child(name)
            .map_err(|reason| Error::JournalInvalid {
                path: self.ctx.state.journal().path().to_path_buf(),
                reason: format!("step name {name:?}: {reason}"),
            })
    }

    fn trash(&self) -> Result<StatePath> {
        self.ctx.state.trash_dir(&self.op_id)
    }

    /// Place a skill at an absent destination.
    fn place(
        &mut self,
        name: &str,
        strategy: crate::config::Strategy,
        source: &Path,
        hash: &Hash,
    ) -> Result<StepOutcome> {
        if let Err(e) = self.ensure_dirs()? {
            return Ok(failed_outcome(&e));
        }
        let child = match self.child(name) {
            Ok(c) => c,
            Err(e) => return Ok(failed_outcome(&e)),
        };
        match observe_one(&child, false) {
            Ok(ObservedKind::Missing) => {}
            Ok(_) => return Ok(StepOutcome::Skipped(SkipReason::DestOccupied)),
            Err(e) => return Ok(failed_outcome(&e)),
        }
        let src = SourcePath::new(source.to_path_buf());
        let r = match strategy {
            crate::config::Strategy::Symlink => self.fs.symlink_dir(&src, &child),
            crate::config::Strategy::Copy => self
                .fs
                .copy_tree(&src, &child, &self.op_id, hash)
                .map(|_| ()),
        };
        Ok(match r {
            Ok(()) => StepOutcome::Done,
            Err(e) => failed_outcome(&e),
        })
    }

    fn remove(&mut self, name: &str, expect: &OwnedExpectation) -> Result<StepOutcome> {
        let child = match self.child(name) {
            Ok(c) => c,
            Err(e) => return Ok(failed_outcome(&e)),
        };
        let trash = self.trash()?;
        Ok(match self.fs.remove_owned(&child, expect, &trash) {
            Ok(RemoveOutcome::Removed) => StepOutcome::Done,
            Ok(RemoveOutcome::RemovedWithResidue { trash }) => {
                self.warnings.push(
                    Diagnostic::warning(
                        "trash-residue",
                        format!(
                            "{} was removed; content not covered by its hash was left in {}",
                            child.rel(),
                            trash.display()
                        ),
                    )
                    .with_path(trash.to_string_lossy().into_owned()),
                );
                StepOutcome::Done
            }
            Ok(RemoveOutcome::Skipped(r)) => StepOutcome::Skipped(r),
            Err(e) => failed_outcome(&e),
        })
    }

    fn remove_dir(&mut self, dir: &str) -> Result<StepOutcome> {
        let Some(p) = self.ancestor(dir) else {
            return Ok(StepOutcome::Failed {
                code: "dir-not-recognized".into(),
                message: format!("{dir} is not a destination directory"),
            });
        };
        match self.fs.rmdir_if_empty(&p) {
            Ok(true) => {
                self.append(&Record::DirRemoved {
                    path: dir.to_string(),
                })?;
                self.note_removed(dir);
                Ok(StepOutcome::Done)
            }
            Ok(false) => {
                self.warnings.push(Diagnostic::info(
                    "created-dir-kept",
                    format!("{dir} was created by unam but is not empty; it is kept"),
                ));
                Ok(StepOutcome::Done)
            }
            Err(e) => Ok(failed_outcome(&e)),
        }
    }

    fn note_removed(&mut self, dir: &str) {
        if !self.dirs_removed.iter().any(|d| d == dir) {
            self.dirs_removed.push(dir.to_string());
        }
        self.removed_dirs.push(dir.to_string());
    }

    /// Run one step of a fresh apply.
    fn exec_step(&mut self, step: &Step) -> Result<StepOutcome> {
        match step {
            Step::Add {
                name,
                strategy,
                source,
                tree_hash,
                ..
            } => self.place(name, *strategy, source, tree_hash),
            Step::Remove { name, expect, .. } => self.remove(name, expect),
            Step::DropOrphan { .. } | Step::UpdateProvenance { .. } => Ok(StepOutcome::Done),
            Step::RemoveDir { dir, .. } => self.remove_dir(dir),
        }
    }
}

/// History, receipt, marker, journal removal, prune (or the partial form).
fn finish(
    ctx: &ApplyCtx<'_>,
    fs: &dyn FsOps,
    begin: &Begin,
    before: Option<&Receipt>,
    steps: &[Step],
    outcomes: &BTreeMap<u32, StepOutcome>,
    runner: &mut Runner<'_, '_>,
) -> Result<(Receipt, ApplyStatus, bool)> {
    let empty = Receipt::empty(&ctx.root, ctx.adapter.id());
    let before_r = before.unwrap_or(&empty);
    let now = ctx.clock.now_rfc3339();
    let mut receipt = build_receipt(&ReceiptUpdate {
        before: before_r,
        dest_rel: ctx.adapter.destination(),
        steps,
        outcomes,
        now: &now,
        op_id: &begin.op_id,
        selection_hash: &hash_from_text(&begin.selection_hash),
        dirs_created: &runner.dirs_created,
        dirs_removed: &runner.dirs_removed,
    });
    receipt.project_root = ctx.root_text();
    let status = status_of(outcomes);
    let mut history_written = false;
    match status {
        ApplyStatus::Applied | ApplyStatus::AppliedWithSkips => {
            let entry = HistoryEntry {
                version: 1,
                op_id: begin.op_id.clone(),
                command: begin.command.clone(),
                target: begin.target.clone(),
                started_at: begin.started_at.clone(),
                finished_at: now.clone(),
                outcome: if status == ApplyStatus::Applied {
                    "applied".into()
                } else {
                    "applied-with-skips".into()
                },
                selection_before: begin.selection_before.clone(),
                selection_after: SelectionAfter {
                    text: begin.selection_after_text.clone(),
                },
                receipt_before: begin.receipt_before.clone(),
                receipt_after: receipt.to_value(),
            };
            write_history(fs, &ctx.state, &entry)?;
            history_written = true;
            fs.write_state(&ctx.state.applied(), receipt.to_text().as_bytes())?;
            runner.append(&Record::Receipt {
                outcome: if status == ApplyStatus::Applied {
                    MarkerOutcome::Clean
                } else {
                    MarkerOutcome::Skips
                },
            })?;
            fs.remove_state(&ctx.state.journal())?;
            if let Err(e) = prune_history(fs, &ctx.state, HISTORY_KEEP) {
                runner.warnings.push(Diagnostic::warning(
                    "history-prune-failed",
                    format!("cannot prune history: {e}"),
                ));
            }
        }
        ApplyStatus::Partial => {
            fs.write_state(&ctx.state.applied(), receipt.to_text().as_bytes())?;
            runner.append(&Record::Receipt {
                outcome: MarkerOutcome::Partial,
            })?;
        }
    }
    Ok((receipt, status, history_written))
}

// ---- execute ----

/// Apply `plan` (P9): lock and selection are the caller's; this writes the
/// `begin` record, runs every step (never stopping at a failure), then the
/// final sequence.
pub fn execute(
    ctx: &ApplyCtx<'_>,
    plan: &Plan,
    prelude: &SelectionPrelude,
    fs: &dyn FsOps,
) -> Result<ApplyReport> {
    let before = load_receipt_opt(ctx)?;
    ensure_dest_root(ctx)?;
    let begin = Begin {
        version: 1,
        op_id: plan.op_id.clone(),
        command: prelude.command.clone(),
        target: ctx.adapter.id().as_str().to_string(),
        project_root: ctx.root_text(),
        started_at: ctx.clock.now_rfc3339(),
        dest_rel: ctx.adapter.destination().to_string(),
        dest_dirs_planned: plan.dest_dirs_to_create.clone(),
        selection_before: prelude.selection_before.clone(),
        selection_after_text: prelude.selection_after_text.clone(),
        selection_after_hash: prelude.selection_after_hash.clone(),
        selection_hash: plan.selection_hash.to_string(),
        receipt_before: before.as_ref().map_or(Value::Null, Receipt::to_value),
        plan: plan.steps.iter().map(journal_step).collect(),
    };
    fs.write_state(
        &ctx.state.journal(),
        Record::Begin(Box::new(begin.clone())).to_line().as_bytes(),
    )?;
    let mut runner = Runner {
        ctx,
        fs,
        op_id: plan.op_id.clone(),
        planned_dirs: plan.dest_dirs_to_create.clone(),
        dirs_created: Vec::new(),
        dirs_removed: Vec::new(),
        new_dirs: Vec::new(),
        removed_dirs: Vec::new(),
        warnings: Vec::new(),
    };
    let dest_rel = ctx.adapter.destination();
    let mut outcomes = BTreeMap::new();
    let mut results = Vec::new();
    let mut removal_applied = false;
    for step in &plan.steps {
        runner.append(&Record::Started { n: step.n() })?;
        let outcome = runner.exec_step(step)?;
        runner.append(&terminal_record(step.n(), &outcome, "ran"))?;
        if outcome == StepOutcome::Done && is_removal(step) {
            removal_applied = true;
        }
        outcomes.insert(step.n(), outcome.clone());
        results.push(result_of(step, dest_rel, outcome));
    }
    let (receipt, status, history_written) = finish(
        ctx,
        fs,
        &begin,
        before.as_ref(),
        &plan.steps,
        &outcomes,
        &mut runner,
    )?;
    Ok(ApplyReport {
        op_id: plan.op_id.clone(),
        command: prelude.command.clone(),
        results,
        status,
        receipt,
        history_written,
        warnings: runner.warnings,
        new_dirs: runner.new_dirs,
        removed_dirs: runner.removed_dirs,
        removal_applied,
        refresh: ctx.adapter.refresh_note(ApplyContext {
            dest_root_existed: plan.dest_dirs_to_create.is_empty(),
        }),
    })
}

// ---- inspect ----

fn summarize(j: &Journal, receipt_written: bool) -> JournalSummary {
    JournalSummary {
        op_id: j.begin.op_id.clone(),
        command: j.begin.command.clone(),
        target: j.begin.target.clone(),
        started_at: j.begin.started_at.clone(),
        receipt_written,
        steps: j
            .begin
            .plan
            .iter()
            .map(|s| StepSummary {
                n: s.n(),
                op: step_from_journal(s).op(),
                name: s.name().map(str::to_string),
                state: j.states.get(&s.n()).map_or("pending", StepState::label),
            })
            .collect(),
    }
}

fn read_present(ctx: &ApplyCtx<'_>) -> Result<Option<Journal>> {
    match read_journal(&ctx.state, ctx.adapter.destination())? {
        JournalRead::Present(j) => Ok(Some(*j)),
        JournalRead::Absent | JournalRead::TornBegin => Ok(None),
    }
}

/// Classify a leftover journal (P9, decision a): absent is clear; a torn
/// begin is discarded; a completed apply whose journal was left behind is
/// finalized (never replayed); anything else is interrupted.
pub fn inspect(ctx: &ApplyCtx<'_>, fs: &dyn FsOps) -> Result<Inspection> {
    let j = match read_journal(&ctx.state, ctx.adapter.destination())? {
        JournalRead::Absent => return Ok(Inspection::Clear { notes: vec![] }),
        JournalRead::TornBegin => {
            fs.remove_state(&ctx.state.journal())?;
            return Ok(Inspection::Clear {
                notes: vec!["discarded an unfinished journal (it never recorded a plan)".into()],
            });
        }
        JournalRead::Present(j) => *j,
    };
    let current = load_receipt_opt(ctx)?;
    let receipt_written = j.marker.is_some()
        || current
            .as_ref()
            .is_some_and(|r| r.last_op_id.as_deref() == Some(j.begin.op_id.as_str()));
    let any_failed = j
        .states
        .values()
        .any(|s| matches!(s, StepState::Failed { .. }));
    let all_terminal = j.states.values().all(StepState::terminal);
    if receipt_written
        && !any_failed
        && all_terminal
        && let Some(cur) = &current
    {
        let hist = ctx.state.history_file(&j.begin.op_id)?;
        if std::fs::symlink_metadata(hist.path()).is_err() {
            let skips = j
                .states
                .values()
                .any(|s| matches!(s, StepState::Skipped { .. }));
            write_history(
                fs,
                &ctx.state,
                &HistoryEntry {
                    version: 1,
                    op_id: j.begin.op_id.clone(),
                    command: j.begin.command.clone(),
                    target: j.begin.target.clone(),
                    started_at: j.begin.started_at.clone(),
                    finished_at: ctx.clock.now_rfc3339(),
                    outcome: if skips {
                        "applied-with-skips".into()
                    } else {
                        "applied".into()
                    },
                    selection_before: j.begin.selection_before.clone(),
                    selection_after: SelectionAfter {
                        text: j.begin.selection_after_text.clone(),
                    },
                    receipt_before: j.begin.receipt_before.clone(),
                    receipt_after: cur.to_value(),
                },
            )?;
        }
        fs.remove_state(&ctx.state.journal())?;
        let mut notes = vec!["finalized a completed apply (the journal was left behind)".into()];
        if let Err(e) = prune_history(fs, &ctx.state, HISTORY_KEEP) {
            notes.push(format!("cannot prune history: {e}"));
        }
        return Ok(Inspection::Clear { notes });
    }
    Ok(Inspection::Interrupted(summarize(&j, receipt_written)))
}

/// The summary of the current journal (for `resume` and `rollback`
/// confirmations); `NoJournal` when there is none.
pub fn journal_summary(ctx: &ApplyCtx<'_>) -> Result<JournalSummary> {
    match read_present(ctx)? {
        Some(j) => {
            let current = load_receipt_opt(ctx)?;
            let written = j.marker.is_some()
                || current
                    .as_ref()
                    .is_some_and(|r| r.last_op_id.as_deref() == Some(j.begin.op_id.as_str()));
            Ok(summarize(&j, written))
        }
        None => Err(Error::NoJournal {
            target: ctx.adapter.id().as_str().to_string(),
        }),
    }
}

// ---- resume ----

fn present_journal(ctx: &ApplyCtx<'_>) -> Result<Journal> {
    read_present(ctx)?.ok_or_else(|| Error::NoJournal {
        target: ctx.adapter.id().as_str().to_string(),
    })
}

fn prepare_recovery(ctx: &ApplyCtx<'_>, fs: &dyn FsOps, j: &Journal) -> Result<()> {
    ensure_dest_root(ctx)?;
    if j.torn_tail || !j.ends_with_newline {
        fs.write_state(&ctx.state.journal(), j.valid_text.as_bytes())?;
    }
    remove_stale_temp_dirs(&ctx.dest, &j.begin.op_id)?;
    Ok(())
}

/// Finish an interrupted apply (P10): steps without a terminal record are
/// resolved by observing the disk; sources come from the live catalog.
pub fn resume(ctx: &ApplyCtx<'_>, fs: &dyn FsOps) -> Result<RecoverReport> {
    let catalog = ctx.catalog.ok_or_else(|| {
        Error::io(
            "resume needs the catalog",
            std::io::Error::from(std::io::ErrorKind::InvalidInput),
        )
    })?;
    let j = present_journal(ctx)?;
    prepare_recovery(ctx, fs, &j)?;
    let begin = j.begin.clone();
    let dest_rel = ctx.adapter.destination();
    let mut steps: Vec<Step> = begin.plan.iter().map(step_from_journal).collect();
    let mut runner = Runner {
        ctx,
        fs,
        op_id: begin.op_id.clone(),
        planned_dirs: begin.dest_dirs_planned.clone(),
        dirs_created: j.dirs_created.clone(),
        dirs_removed: j.dirs_removed.clone(),
        new_dirs: Vec::new(),
        removed_dirs: Vec::new(),
        warnings: Vec::new(),
    };
    let mut outcomes: BTreeMap<u32, StepOutcome> = BTreeMap::new();
    let mut results = Vec::new();
    let mut removal_applied = false;
    for step in steps.iter_mut() {
        let n = step.n();
        let state = j.states.get(&n).cloned().unwrap_or(StepState::Pending);
        // A created directory kept because an earlier step had not finished
        // (its removal failed and was retried above) is re-evaluated by
        // observing the disk: it is removed now when empty.
        let retry_dir = match &*step {
            Step::RemoveDir { dir, .. } => {
                !runner.dirs_removed.contains(dir)
                    && runner
                        .ancestor(dir)
                        .is_some_and(|p| std::fs::symlink_metadata(p.path()).is_ok())
            }
            _ => false,
        };
        if state.terminal() && !retry_dir {
            if let Some(o) = outcome_of_state(&state) {
                outcomes.insert(n, o);
            }
            continue;
        }
        runner.append(&Record::Started { n })?;
        let (outcome, how) = resume_step(&mut runner, catalog, step)?;
        runner.append(&terminal_record(n, &outcome, how))?;
        if outcome == StepOutcome::Done && is_removal(step) {
            removal_applied = true;
        }
        outcomes.insert(n, outcome.clone());
        results.push(result_of(step, dest_rel, outcome));
    }
    let before = j.receipt_before.clone();
    let (receipt, status, _) = finish(
        ctx,
        fs,
        &begin,
        before.as_ref(),
        &steps,
        &outcomes,
        &mut runner,
    )?;
    Ok(RecoverReport {
        action: "resume",
        op_id: begin.op_id.clone(),
        command: begin.command.clone(),
        results,
        status,
        receipt: Some(receipt),
        warnings: runner.warnings,
        removal_applied,
        refresh: Some(ctx.adapter.refresh_note(ApplyContext {
            dest_root_existed: begin.dest_dirs_planned.is_empty(),
        })),
    })
}

fn matches_expectation(
    obs: &ObservedKind,
    strategy: crate::config::Strategy,
    source: &str,
    hash: &Hash,
) -> bool {
    match strategy {
        crate::config::Strategy::Symlink => {
            matches!(obs, ObservedKind::Symlink { target, .. } if target == source)
        }
        crate::config::Strategy::Copy => {
            matches!(obs, ObservedKind::Dir { tree_hash: Some(h) } if h == hash)
        }
    }
}

fn resume_step(
    runner: &mut Runner<'_, '_>,
    catalog: &Catalog,
    step: &mut Step,
) -> Result<(StepOutcome, &'static str)> {
    match step {
        Step::Add {
            id,
            name,
            strategy,
            source,
            tree_hash,
            ..
        } => {
            let live = SkillId::parse(id)
                .ok()
                .and_then(|sid| {
                    let skill = catalog.skill(&sid)?;
                    let path = catalog.abs_path(&sid)?;
                    Some((skill.name.clone(), path, skill.tree_hash.clone()))
                })
                .filter(|(live_name, _, _)| live_name == name);
            let Some((_, live_path, live_hash)) = live else {
                return Ok((
                    StepOutcome::Failed {
                        code: "source-changed".into(),
                        message: format!(
                            "{id} is no longer in the catalog under the name {name}; re-run the command"
                        ),
                    },
                    "ran",
                ));
            };
            *source = live_path.clone();
            *tree_hash = live_hash.clone();
            if let Err(e) = runner.ensure_dirs()? {
                return Ok((failed_outcome(&e), "ran"));
            }
            let child = match runner.child(name) {
                Ok(c) => c,
                Err(e) => return Ok((failed_outcome(&e), "ran")),
            };
            let want_hash = *strategy == crate::config::Strategy::Copy;
            let obs = match observe_one(&child, want_hash) {
                Ok(o) => o,
                Err(e) => return Ok((failed_outcome(&e), "ran")),
            };
            if obs == ObservedKind::Missing {
                let (s, p, h) = (*strategy, live_path, live_hash);
                return Ok((runner.place(name, s, &p, &h)?, "ran"));
            }
            if matches_expectation(&obs, *strategy, &live_path.to_string_lossy(), &live_hash) {
                return Ok((StepOutcome::Done, "observed"));
            }
            Ok((StepOutcome::Skipped(SkipReason::DestOccupied), "ran"))
        }
        Step::Remove { name, expect, .. } => {
            let child = match runner.child(name) {
                Ok(c) => c,
                Err(e) => return Ok((failed_outcome(&e), "ran")),
            };
            match observe_one(&child, false) {
                Ok(ObservedKind::Missing) => {
                    let in_trash = runner.trash()?.path().join(name.as_str());
                    if std::fs::symlink_metadata(&in_trash).is_ok() {
                        runner.warnings.push(
                            Diagnostic::warning(
                                "trash-left",
                                format!(
                                    "{} was moved aside before the interruption; it is left in {}",
                                    child.rel(),
                                    in_trash.display()
                                ),
                            )
                            .with_path(in_trash.to_string_lossy().into_owned()),
                        );
                        Ok((StepOutcome::Done, "observed"))
                    } else {
                        Ok((StepOutcome::Skipped(SkipReason::Missing), "observed"))
                    }
                }
                Ok(_) => {
                    let e = expect.clone();
                    Ok((runner.remove(name, &e)?, "ran"))
                }
                Err(e) => Ok((failed_outcome(&e), "ran")),
            }
        }
        Step::DropOrphan { .. } | Step::UpdateProvenance { .. } => Ok((StepOutcome::Done, "ran")),
        Step::RemoveDir { dir, .. } => {
            let dir = dir.clone();
            let Some(p) = runner.ancestor(&dir) else {
                return Ok((
                    StepOutcome::Failed {
                        code: "dir-not-recognized".into(),
                        message: format!("{dir} is not a destination directory"),
                    },
                    "ran",
                ));
            };
            if std::fs::symlink_metadata(p.path()).is_err() {
                if !runner.dirs_removed.contains(&dir) {
                    runner.append(&Record::DirRemoved { path: dir.clone() })?;
                }
                runner.note_removed(&dir);
                return Ok((StepOutcome::Done, "observed"));
            }
            Ok((runner.remove_dir(&dir)?, "ran"))
        }
    }
}

// ---- rollback ----

/// Undo an interrupted apply (P10, G5, G6b): every non-skipped step in
/// reverse, decided by observing the disk; then created directories, the
/// receipt, the selection, the history entry, the journal, and the empty
/// state directories. Repeatable after a crash.
pub fn rollback(ctx: &ApplyCtx<'_>, fs: &dyn FsOps) -> Result<RecoverReport> {
    let j = present_journal(ctx)?;
    prepare_recovery(ctx, fs, &j)?;
    let begin = &j.begin;
    let dest_rel = ctx.adapter.destination();
    let op_id = begin.op_id.clone();
    let trash = ctx.state.trash_dir(&op_id)?;
    let mut warnings: Vec<Diagnostic> = Vec::new();
    let mut results = Vec::new();
    let mut left = false;
    let steps: Vec<Step> = begin.plan.iter().map(step_from_journal).collect();
    let ancestor = |dir: &str| -> Option<DestPath> {
        ctx.dest
            .ancestors()
            .into_iter()
            .find(|(rel, _)| rel == dir)
            .map(|(_, p)| p)
    };
    let child_of = |name: &str| -> Result<DestPath> {
        ctx.dest
            .join_child(name)
            .map_err(|reason| Error::JournalInvalid {
                path: ctx.state.journal().path().to_path_buf(),
                reason: format!("step name {name:?}: {reason}"),
            })
    };
    for step in steps.iter().rev() {
        let n = step.n();
        if matches!(j.states.get(&n), Some(StepState::Skipped { .. })) {
            continue;
        }
        match step {
            Step::Add {
                name,
                strategy,
                source,
                tree_hash,
                ..
            } => {
                let child = child_of(name)?;
                let want_hash = *strategy == crate::config::Strategy::Copy;
                let obs = observe_one(&child, want_hash)?;
                if obs == ObservedKind::Missing {
                    continue;
                }
                let src_text = source.to_string_lossy().into_owned();
                if matches_expectation(&obs, *strategy, &src_text, tree_hash) {
                    let expect = OwnedExpectation {
                        strategy: *strategy,
                        source: src_text,
                        tree_hash: Some(tree_hash.clone()),
                    };
                    let outcome = match fs.remove_owned(&child, &expect, &trash)? {
                        RemoveOutcome::Removed => StepOutcome::Done,
                        RemoveOutcome::RemovedWithResidue { trash } => {
                            warnings.push(Diagnostic::warning(
                                "trash-residue",
                                format!(
                                    "{} was removed; content not covered by its hash was left in {}",
                                    child.rel(),
                                    trash.display()
                                ),
                            ));
                            StepOutcome::Done
                        }
                        RemoveOutcome::Skipped(r) => {
                            left = true;
                            warnings.push(Diagnostic::warning(
                                "rollback-left",
                                format!("{} was left in place ({})", child.rel(), r.code()),
                            ));
                            StepOutcome::Skipped(r)
                        }
                    };
                    results.push(result_of(step, dest_rel, outcome));
                } else {
                    left = true;
                    warnings.push(Diagnostic::warning(
                        "rollback-left",
                        format!(
                            "{} holds content unam did not place in this operation; it is left untouched",
                            child.rel()
                        ),
                    ));
                    results.push(result_of(
                        step,
                        dest_rel,
                        StepOutcome::Skipped(SkipReason::Modified),
                    ));
                }
            }
            Step::Remove {
                id, name, expect, ..
            } => {
                let child = child_of(name)?;
                if observe_one(&child, false)? != ObservedKind::Missing {
                    continue;
                }
                let live = live_source(ctx.catalog, id, expect);
                match fs.restore_owned(&child, expect, &trash, live.as_ref(), &op_id)? {
                    RestoreOutcome::Restored => {
                        results.push(result_of(step, dest_rel, StepOutcome::Done));
                    }
                    RestoreOutcome::NotRestored(reason) => {
                        left = true;
                        warnings.push(Diagnostic::warning(
                            "rollback-not-restored",
                            format!("{} could not be restored: {reason}", child.rel()),
                        ));
                        results.push(result_of(
                            step,
                            dest_rel,
                            StepOutcome::Failed {
                                code: "rollback-not-restored".into(),
                                message: reason,
                            },
                        ));
                    }
                }
            }
            Step::DropOrphan { .. } | Step::UpdateProvenance { .. } => {}
            Step::RemoveDir { dir, .. } => {
                if let Some(p) = ancestor(dir)
                    && std::fs::symlink_metadata(p.path()).is_err()
                {
                    // Recreate the parents first (shallowest first).
                    for (rel, a) in ctx.dest.ancestors() {
                        if std::fs::symlink_metadata(a.path()).is_err() {
                            fs.create_dir(&a)?;
                        }
                        if rel == *dir {
                            break;
                        }
                    }
                    results.push(result_of(step, dest_rel, StepOutcome::Done));
                }
            }
        }
    }
    // Directories this operation created, deepest first.
    for dir in j.dirs_created.iter().rev() {
        if let Some(p) = ancestor(dir) {
            fs.rmdir_if_empty(&p)?;
        }
    }
    // The receipt.
    let current = load_receipt_opt(ctx)?;
    let before_last = j.receipt_before.as_ref().and_then(|r| r.last_op_id.clone());
    let restorable = match &current {
        None => true,
        Some(c) => c.last_op_id.as_deref() == Some(op_id.as_str()) || c.last_op_id == before_last,
    };
    if restorable {
        match &j.receipt_before {
            None => {
                if current.is_some() {
                    fs.remove_state(&ctx.state.applied())?;
                }
            }
            Some(r) => fs.write_state(&ctx.state.applied(), r.to_text().as_bytes())?,
        }
    } else {
        warnings.push(Diagnostic::warning(
            "receipt-not-restored",
            "the receipt was changed after the interrupted apply; it is left as it is",
        ));
    }
    // The selection (G11).
    restore_selection(ctx, fs, begin, &mut warnings)?;
    // The history entry of the undone apply (G6b).
    let hist = ctx.state.history_file(&op_id)?;
    if std::fs::symlink_metadata(hist.path()).is_ok() {
        fs.remove_state(&hist)?;
    }
    fs.remove_state(&ctx.state.journal())?;
    // G5: empty state directories, bottom-up; `.unam` stays (the lock).
    for d in [
        trash.clone(),
        ctx.state.trash_root(),
        ctx.state.history_dir(),
        ctx.state.dir(),
    ] {
        fs.remove_state_dir_if_empty(&d)?;
    }
    let receipt = load_receipt_opt(ctx)?;
    Ok(RecoverReport {
        action: "rollback",
        op_id,
        command: begin.command.clone(),
        results,
        status: if left {
            ApplyStatus::Partial
        } else {
            ApplyStatus::Applied
        },
        receipt,
        warnings,
        removal_applied: false,
        refresh: None,
    })
}

/// The live source of a removed entry, only when it still matches what the
/// journal recorded (path text for a symlink, tree hash for a copy).
fn live_source(
    catalog: Option<&Catalog>,
    id: &str,
    expect: &OwnedExpectation,
) -> Option<SourcePath> {
    let catalog = catalog?;
    let sid = SkillId::parse(id).ok()?;
    let skill = catalog.skill(&sid)?;
    let path = catalog.abs_path(&sid)?;
    let ok = match expect.strategy {
        crate::config::Strategy::Symlink => path.to_string_lossy() == expect.source,
        crate::config::Strategy::Copy => expect.tree_hash.as_ref() == Some(&skill.tree_hash),
    };
    if ok { catalog.source_path(&sid) } else { None }
}

fn restore_selection(
    ctx: &ApplyCtx<'_>,
    fs: &dyn FsOps,
    begin: &Begin,
    warnings: &mut Vec<Diagnostic>,
) -> Result<()> {
    let path = StatePath::new(selection_path(&ctx.root));
    let before_hash = Hash::of_bytes(begin.selection_before.text.as_bytes()).to_string();
    let current = match std::fs::read(path.path()) {
        Ok(b) => Some(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => {
            return Err(Error::io(
                format!("cannot read {}", path.path().display()),
                e,
            ));
        }
    };
    let current_hash = current.as_ref().map(|b| Hash::of_bytes(b).to_string());
    // Already at the start state (never rewritten, or restored before).
    let at_start = match (&current_hash, begin.selection_before.existed) {
        (None, false) => true,
        (Some(h), true) => *h == before_hash,
        _ => false,
    };
    if at_start {
        return Ok(());
    }
    if current_hash.as_deref() == Some(begin.selection_after_hash.as_str()) {
        if begin.selection_before.existed {
            fs.write_state(&path, begin.selection_before.text.as_bytes())?;
        } else {
            fs.remove_state(&path)?;
        }
        return Ok(());
    }
    warnings.push(Diagnostic::warning(
        "selection-not-restored",
        "the selection was edited after the interrupted apply; it is left as it is",
    ));
    Ok(())
}

#[cfg(test)]
mod tests;
