//! `unam add|rm|use|deactivate|resume|rollback` (spec 5; M3 plan CLI
//! Contract, P13). Thin: build the request, call `unam_core::change`, map
//! the outcome to `Done` or `Failed` with the `--json` data. Under `--json`
//! stdout carries exactly one JSON object (the plan, the prompt, and the
//! result lines print only in human mode).

use super::{CmdResult, Ctx, Done, Failed, catalog_warnings};
use crate::output::{
    ChangeBody, ChangeHeader, ResultView, change_json, neutralize, render_plan, render_result,
    warnings_only,
};
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use unam_core::adapter::{Adapter, ApplyContext, Detection, RefreshNote, adapter_for};
use unam_core::apply::{ApplyStatus, JournalSummary, StepResult};
use unam_core::catalog::{Catalog, FlatDiagnostic};
use unam_core::change::{
    self, ChangeFailure, ChangeRequest, Decision, Edit, Outcome, Prepared, RecoverOutcome,
    RecoverRequest,
};
use unam_core::error::{Diagnostic, Error};
use unam_core::fsops::RealFs;
use unam_core::plan::{Plan, StepOutcome};
use unam_core::project::{self, ProjectRoot, TargetId};
use unam_core::state::{Receipt, StateDir, SystemClock, strategy_text};

/// The confirmation rule (P13): `--yes` proceeds; not interactive or
/// `--json` refuses; otherwise one line `y` or `yes` (any case) proceeds
/// and anything else, an empty line, or end of input declines.
pub fn confirm_with(
    reader: &mut dyn BufRead,
    interactive: bool,
    yes: bool,
    json: bool,
) -> Decision {
    if yes {
        return Decision::Proceed;
    }
    if json || !interactive {
        return Decision::Refuse;
    }
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(0) | Err(_) => Decision::Decline,
        Ok(_) => {
            let answer = line.trim().to_ascii_lowercase();
            if answer == "y" || answer == "yes" {
                Decision::Proceed
            } else {
                Decision::Decline
            }
        }
    }
}

pub(super) fn ask(ctx: &Ctx) -> Decision {
    if ctx.interactive && !ctx.yes {
        eprint!("Apply these changes? [y/N] ");
        let _ = std::io::stderr().flush();
    }
    let stdin = std::io::stdin();
    let mut lock = stdin.lock();
    confirm_with(&mut lock, ctx.interactive, ctx.yes, ctx.json)
}

pub(super) fn global(diags: &[Diagnostic]) -> Vec<FlatDiagnostic> {
    diags.iter().cloned().map(FlatDiagnostic::global).collect()
}

fn path_str(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

fn detection_text(present: bool) -> &'static str {
    if present { "present" } else { "absent" }
}

fn failure(f: ChangeFailure, mut warnings: Vec<FlatDiagnostic>) -> Failed {
    warnings.extend(global(&f.warnings));
    let warnings = warnings_only(&warnings);
    match &f.error {
        Error::Locked { pid } => {
            let data = json!({"locked": {"pid": pid}});
            Failed::with_data(f.error, data, warnings)
        }
        _ => Failed {
            error: f.error.into(),
            warnings,
        },
    }
}

fn interrupted(summary: &JournalSummary, json_mode: bool, warnings: Vec<FlatDiagnostic>) -> Failed {
    let data = json!({
        "interrupted": summary_json(summary),
        "next": ["unam resume", "unam rollback"],
    });
    if !json_mode {
        print!("{}", render_summary(summary));
    }
    Failed::with_data(
        Error::Interrupted {
            target: summary.target.clone(),
            op_id: summary.op_id.clone(),
        },
        data,
        warnings_only(&warnings),
    )
}

fn summary_json(s: &JournalSummary) -> Value {
    json!({
        "op_id": s.op_id,
        "command": s.command,
        "target": s.target,
        "started_at": s.started_at,
        "receipt_written": s.receipt_written,
        "steps": s.steps.iter().map(|x| json!({
            "n": x.n, "op": x.op, "name": x.name, "state": x.state,
        })).collect::<Vec<_>>(),
    })
}

fn render_summary(s: &JournalSummary) -> String {
    let mut out = format!(
        "Interrupted apply {} ({} of {}, started {})\n",
        s.op_id, s.command, s.target, s.started_at
    );
    for x in &s.steps {
        out.push_str(&format!(
            "  {:>3} {:<18} {:<28} {}\n",
            x.n,
            x.op,
            neutralize(x.name.as_deref().unwrap_or("")),
            x.state
        ));
    }
    out
}

fn receipt_json(root: &std::path::Path, target: TargetId, r: &Receipt) -> Value {
    json!({
        "path": path_str(StateDir::new(root, target).applied().path()),
        "entries": r.entries.len(),
        "last_op_id": r.last_op_id,
    })
}

fn selection_json(root: &ProjectRoot, p: &Prepared) -> Value {
    let path = path_str(&project::selection_path(&root.path));
    match &p.tsel_after {
        None => json!({"written": p.selection_written, "path": path, "cleared": true}),
        Some(t) => json!({
            "written": p.selection_written,
            "path": path,
            "bundles": t.bundles,
            "add": t.add,
            "remove": t.remove,
        }),
    }
}

/// Status, failed count, skipped count of a result list.
fn tally(results: &[StepResult]) -> (usize, usize) {
    let failed = results
        .iter()
        .filter(|r| matches!(r.outcome, StepOutcome::Failed { .. }))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r.outcome, StepOutcome::Skipped(_)))
        .count();
    (failed, skipped)
}

struct Run {
    command: &'static str,
    root: ProjectRoot,
    target: TargetId,
    /// `Then ...` lines of the adapter (printed and in `--json` once the
    /// change is applied; unam never runs them).
    followups: Vec<String>,
}

/// The adapter's followups as one line each: `command  (reason)`.
fn followup_lines(
    adapter: Option<&dyn Adapter>,
    root: &std::path::Path,
    dirs: &unam_core::paths::Dirs,
) -> Vec<String> {
    adapter
        .map(|a| {
            a.followups(root, dirs)
                .into_iter()
                .map(|f| format!("{}  ({})", f.command, f.reason))
                .collect()
        })
        .unwrap_or_default()
}

/// The consumers of a shared destination, for the "Visible to" line of a
/// result: only `agents-neutral` (the `claude-code` output is unchanged).
fn shared_consumers(target: TargetId) -> Vec<String> {
    if target == TargetId::AgentsNeutral {
        adapter_for(target)
            .map(|a| a.consumers().iter().map(|c| c.to_string()).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

impl Run {
    fn header<'b>(&'b self, plan: &'b Plan, root_text: &'b str) -> ChangeHeader<'b> {
        ChangeHeader {
            command: self.command,
            project_root: root_text,
            root_detection: self.root.how,
            target: self.target.as_str(),
            destination: &plan.header.dest_rel,
            strategy: Some(strategy_text(plan.header.strategy)),
            detection: detection_text(plan.header.detection_text.is_none()),
        }
    }

    fn data(&self, p: &Prepared, status: &str, applied: Option<&ApplyData<'_>>) -> Value {
        let root_text = path_str(&self.root.path);
        let consumers: Vec<String> = p.plan.header.consumers.clone();
        let empty: Vec<StepResult> = Vec::new();
        let results = applied.map_or(&empty[..], |a| a.results);
        change_json(
            &self.header(&p.plan, &root_text),
            &ChangeBody {
                status,
                op_id: applied.map(|_| p.plan.op_id.as_str()),
                selection: selection_json(&self.root, p),
                plan: Some(&p.plan),
                results,
                receipt: applied.map_or(Value::Null, |a| {
                    receipt_json(&self.root.path, self.target, a.receipt)
                }),
                refresh: applied.map(|a| a.refresh),
                followups: if applied.is_some() {
                    self.followups.as_slice()
                } else {
                    &[]
                },
                removal_applied: applied.is_some_and(|a| a.removal_applied),
                gitignore_lines: &p.gitignore_lines,
                visible_to: &consumers,
            },
        )
    }
}

struct ApplyData<'a> {
    results: &'a [StepResult],
    receipt: &'a Receipt,
    refresh: &'a RefreshNote,
    removal_applied: bool,
}

fn edit_run(ctx: &Ctx, command: &'static str, edit: Edit) -> CmdResult {
    let root = ctx.root().map_err(Failed::from)?;
    let flag = ctx.target_flag().map_err(Failed::from)?;
    let selection = project::load_selection(&root.path).map_err(Failed::from)?;
    let (target, _) = change::resolve_target(flag, selection.as_ref(), &root.path, &ctx.dirs)
        .map_err(Failed::from)?;
    let deactivate = edit == Edit::Deactivate;
    let catalog = if deactivate {
        Catalog::default()
    } else {
        ctx.catalog().map_err(Failed::from)?
    };
    let mut warnings = if deactivate {
        warnings_only(&ctx.config_diags)
    } else {
        catalog_warnings(ctx, &catalog)
    };
    let bundles = ctx.bundles();
    let clock = SystemClock;
    let req = ChangeRequest {
        dirs: &ctx.dirs,
        config: &ctx.config,
        catalog: &catalog,
        bundles: &bundles,
        root: &root,
        target,
        edit,
        command,
        clock: &clock,
    };
    let adapter = adapter_for(target);
    let mut decide = |plan: &Plan| -> Decision {
        if !ctx.json {
            let note = adapter.map(|a| {
                a.refresh_note(ApplyContext {
                    dest_root_existed: plan.dest_dirs_to_create.is_empty(),
                })
            });
            print!("{}", render_plan(plan, note.as_ref()));
        }
        ask(ctx)
    };
    let outcome = match change::run(&req, &RealFs, &mut decide) {
        Ok(o) => o,
        Err(f) => return Err(failure(f, warnings)),
    };
    let run = Run {
        command,
        root: root.clone(),
        target,
        followups: followup_lines(adapter, &root.path, &ctx.dirs),
    };
    let shared = shared_consumers(target);
    match outcome {
        Outcome::Interrupted(summary) => Err(interrupted(&summary, ctx.json, warnings)),
        Outcome::Blocked(p) => {
            warnings.extend(global(&p.warnings));
            if !ctx.json {
                print!("{}", render_plan(&p.plan, None));
            }
            let data = run.data(&p, "blocked", None);
            Err(Failed::with_data(
                Error::Conflicts {
                    count: p.plan.conflicts.len(),
                },
                data,
                warnings_only(&warnings),
            ))
        }
        Outcome::Refused(p) => {
            warnings.extend(global(&p.warnings));
            let data = run.data(&p, "refused", None);
            Err(Failed::with_data(
                Error::ConfirmationRequired,
                data,
                warnings_only(&warnings),
            ))
        }
        Outcome::Declined(p) => {
            warnings.extend(global(&p.warnings));
            Ok(Done {
                data: run.data(&p, "declined", None),
                warnings: warnings_only(&warnings),
                human_diags: warnings,
                human: "Cancelled: nothing was changed.\n".to_string(),
            })
        }
        Outcome::NoChanges(p) => {
            warnings.extend(global(&p.warnings));
            let mut human = render_plan(&p.plan, None);
            human.push_str(&render_result(&ResultView {
                results: &[],
                refresh: None,
                followups: &[],
                removal_applied: false,
                gitignore_lines: &p.gitignore_lines,
                visible_to: &shared,
            }));
            Ok(Done {
                data: run.data(&p, "no-changes", None),
                warnings: warnings_only(&warnings),
                human_diags: warnings,
                human,
            })
        }
        Outcome::Applied(p, report) => {
            warnings.extend(global(&p.warnings));
            warnings.extend(global(&report.warnings));
            let applied = ApplyData {
                results: &report.results,
                receipt: &report.receipt,
                refresh: &report.refresh,
                removal_applied: report.removal_applied,
            };
            let human = render_result(&ResultView {
                results: &report.results,
                refresh: Some(&report.refresh),
                followups: &run.followups,
                removal_applied: report.removal_applied,
                gitignore_lines: &p.gitignore_lines,
                visible_to: &shared,
            });
            if report.status == ApplyStatus::Applied {
                Ok(Done {
                    data: run.data(&p, "applied", Some(&applied)),
                    warnings: warnings_only(&warnings),
                    human_diags: warnings,
                    human,
                })
            } else {
                if !ctx.json {
                    print!("{human}");
                }
                let (failed, skipped) = tally(&report.results);
                Err(Failed::with_data(
                    Error::ApplyFailed { failed, skipped },
                    run.data(&p, "partial", Some(&applied)),
                    warnings_only(&warnings),
                ))
            }
        }
    }
}

/// `unam add <ref>...`.
pub fn add(ctx: &Ctx, references: Vec<String>) -> CmdResult {
    edit_run(ctx, "add", Edit::Add(references))
}

/// `unam rm <ref>...`.
pub fn rm(ctx: &Ctx, references: Vec<String>) -> CmdResult {
    edit_run(ctx, "rm", Edit::Remove(references))
}

/// `unam use <bundle> --replace|--add`.
pub fn use_bundle(ctx: &Ctx, bundle: String, replace: bool) -> CmdResult {
    let edit = if replace {
        Edit::UseReplace(bundle)
    } else {
        Edit::UseAdd(bundle)
    };
    edit_run(ctx, "use", edit)
}

/// `unam deactivate [--all]` (`--all` equals the default with one adapter).
pub fn deactivate(ctx: &Ctx, _all: bool) -> CmdResult {
    edit_run(ctx, "deactivate", Edit::Deactivate)
}

fn recover_run(ctx: &Ctx, action: &'static str) -> CmdResult {
    let root = ctx.root().map_err(Failed::from)?;
    let flag = ctx.target_flag().map_err(Failed::from)?;
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let mut warnings = catalog_warnings(ctx, &catalog);
    let clock = SystemClock;
    let req = RecoverRequest {
        dirs: &ctx.dirs,
        config: &ctx.config,
        catalog: &catalog,
        root: &root,
        target: flag,
        clock: &clock,
    };
    let mut decide = |s: &JournalSummary| -> Decision {
        if !ctx.json {
            print!("{}", render_summary(s));
            println!("Next: {action} the steps above.");
        }
        ask(ctx)
    };
    let outcome = if action == "resume" {
        change::resume(&req, &RealFs, &mut decide)
    } else {
        change::rollback(&req, &RealFs, &mut decide)
    };
    let outcome = match outcome {
        Ok(o) => o,
        Err(f) => return Err(failure(f, warnings)),
    };
    let header_for = |target: &str| -> Value {
        let t = TargetId::parse(target).unwrap_or(TargetId::ClaudeCode);
        let adapter = adapter_for(t);
        let present = adapter
            .is_some_and(|a| matches!(a.detect(&root.path, &ctx.dirs), Detection::Present { .. }));
        json!({
            "command": action,
            "project_root": path_str(&root.path),
            "root_detection": match root.how {
                project::RootDetection::Git => "git",
                project::RootDetection::Explicit => "explicit",
            },
            "target": target,
            "destination": adapter.map(|a| a.destination()),
            "strategy": Value::Null,
            "detection": detection_text(present),
        })
    };
    let body = |status: &str,
                summary: &JournalSummary,
                report: Option<&unam_core::apply::RecoverReport>|
     -> Value {
        let mut v = header_for(&summary.target);
        let o = v.as_object_mut();
        if let Some(o) = o {
            let t = TargetId::parse(&summary.target).unwrap_or(TargetId::ClaudeCode);
            o.insert("status".into(), json!(status));
            o.insert("op_id".into(), json!(summary.op_id));
            o.insert("selection".into(), Value::Null);
            o.insert("plan".into(), Value::Null);
            o.insert(
                "results".into(),
                json!(
                    report
                        .map(|r| r
                            .results
                            .iter()
                            .map(crate::output::result_json)
                            .collect::<Vec<Value>>())
                        .unwrap_or_default()
                ),
            );
            o.insert(
                "receipt".into(),
                report
                    .and_then(|r| r.receipt.as_ref())
                    .map_or(Value::Null, |r| receipt_json(&root.path, t, r)),
            );
            o.insert(
                "refresh".into(),
                json!(report.and_then(|r| r.refresh.as_ref())),
            );
            o.insert(
                "followups".into(),
                json!(if report.is_some() {
                    followup_lines(adapter_for(t), &root.path, &ctx.dirs)
                } else {
                    Vec::new()
                }),
            );
            o.insert(
                "removal_caveat".into(),
                if report.is_some_and(|r| r.removal_applied) {
                    json!(crate::output::REMOVAL_CAVEAT)
                } else {
                    Value::Null
                },
            );
            o.insert("gitignore_lines".into(), json!([]));
            o.insert(
                "visible_to".into(),
                json!(
                    adapter_for(t)
                        .map(|a| a.consumers().to_vec())
                        .unwrap_or_default()
                ),
            );
            o.insert(
                "recovered".into(),
                json!({"op_id": summary.op_id, "action": action}),
            );
        }
        v
    };
    match outcome {
        RecoverOutcome::Refused(s) => Err(Failed::with_data(
            Error::ConfirmationRequired,
            body("refused", &s, None),
            warnings_only(&warnings),
        )),
        RecoverOutcome::Declined(s) => Ok(Done {
            data: body("declined", &s, None),
            warnings: warnings_only(&warnings),
            human_diags: warnings,
            human: "Cancelled: nothing was changed.\n".to_string(),
        }),
        RecoverOutcome::Recovered(report, s) => {
            warnings.extend(global(&report.warnings));
            let recovered_target = TargetId::parse(&s.target).unwrap_or(TargetId::ClaudeCode);
            let followups = followup_lines(adapter_for(recovered_target), &root.path, &ctx.dirs);
            let shared = shared_consumers(recovered_target);
            let mut human = render_result(&ResultView {
                results: &report.results,
                refresh: report.refresh.as_ref(),
                followups: &followups,
                removal_applied: report.removal_applied,
                gitignore_lines: &[],
                visible_to: &shared,
            });
            human.insert_str(
                0,
                &format!(
                    "{} of {} finished.\n",
                    if action == "resume" {
                        "Resume"
                    } else {
                        "Rollback"
                    },
                    s.op_id
                ),
            );
            if report.status == ApplyStatus::Applied {
                Ok(Done {
                    data: body("applied", &s, Some(&report)),
                    warnings: warnings_only(&warnings),
                    human_diags: warnings,
                    human,
                })
            } else {
                if !ctx.json {
                    print!("{human}");
                }
                let (failed, skipped) = tally(&report.results);
                let left = report
                    .warnings
                    .iter()
                    .filter(|w| w.code.starts_with("rollback-"))
                    .count();
                Err(Failed::with_data(
                    Error::ApplyFailed {
                        failed,
                        skipped: skipped + left,
                    },
                    body("partial", &s, Some(&report)),
                    warnings_only(&warnings),
                ))
            }
        }
    }
}

/// `unam resume`.
pub fn resume(ctx: &Ctx) -> CmdResult {
    recover_run(ctx, "resume")
}

/// `unam rollback`: undo an interrupted apply from the journal, or, with no
/// journal, restore the previous applied state from history (spec 4.10).
pub fn rollback(ctx: &Ctx) -> CmdResult {
    let root = ctx.root().map_err(Failed::from)?;
    let flag = ctx.target_flag().map_err(Failed::from)?;
    if change::has_journal(&root.path, flag) {
        recover_run(ctx, "rollback")
    } else {
        edit_run(ctx, "rollback", Edit::FromHistory)
    }
}

/// `unam reconcile --repair`: the repair plan runs through the journaled
/// executor like any other change (spec 4.12).
pub fn reconcile_repair(ctx: &Ctx) -> CmdResult {
    edit_run(ctx, "reconcile", Edit::Reconcile)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn decide(input: &str, interactive: bool, yes: bool, json: bool) -> Decision {
        confirm_with(&mut Cursor::new(input.as_bytes()), interactive, yes, json)
    }

    #[test]
    fn confirm_with_answers() {
        assert_eq!(decide("y\n", true, false, false), Decision::Proceed);
        assert_eq!(decide("Y\n", true, false, false), Decision::Proceed);
        assert_eq!(decide("yes\n", true, false, false), Decision::Proceed);
        assert_eq!(decide(" YES \n", true, false, false), Decision::Proceed);
        assert_eq!(decide("n\n", true, false, false), Decision::Decline);
        assert_eq!(decide("\n", true, false, false), Decision::Decline);
        assert_eq!(decide("", true, false, false), Decision::Decline);
        assert_eq!(decide("garbage\n", true, false, false), Decision::Decline);
    }

    #[test]
    fn confirm_with_flags() {
        assert_eq!(decide("", false, true, false), Decision::Proceed);
        assert_eq!(decide("", true, true, false), Decision::Proceed);
        assert_eq!(decide("y\n", false, false, false), Decision::Refuse);
        assert_eq!(decide("y\n", true, false, true), Decision::Refuse);
        assert_eq!(decide("", true, true, true), Decision::Proceed);
    }
}
