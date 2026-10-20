//! `pouch source add|list|rm`.

use super::change::{ask, global};
use super::{CmdResult, Ctx, Done, Failed};
use crate::output::{neutralize, render_plan, result_json, table, warnings_only};
use pouch_core::apply::ApplyStatus;
use pouch_core::catalog::{ALL_CLASSES, FlatDiagnostic};
use pouch_core::change::{self, ChangeRequest, Decision, Edit, Outcome};
use pouch_core::error::Error;
use pouch_core::fsops::RealFs;
use pouch_core::source::{self, Planned, SourceKind};
use pouch_core::state::SystemClock;
use serde_json::{Value, json};

fn kind_str(kind: Option<SourceKind>) -> &'static str {
    kind.map_or("unknown", SourceKind::as_str)
}

pub fn add(ctx: &Ctx, alias: &str, path: &str) -> CmdResult {
    let mut config = ctx.config.clone();
    let out =
        source::add(&mut config, &ctx.dirs, alias, path, ctx.options).map_err(|e| Failed {
            error: e.into(),
            warnings: warnings_only(&ctx.config_diags),
        })?;
    let source_diags = out.catalog.flat_diagnostics(None, ALL_CLASSES);
    let mut all = ctx.config_diags.clone();
    all.extend(source_diags.iter().cloned());
    let data = json!({
        "alias": out.entry.alias,
        "path": out.entry.path,
        "expanded_path": out.expanded_path.to_string_lossy(),
        "kind": out.kind.as_str(),
        "precedence": out.entry.precedence,
        "skill_count": out.skill_count,
        "diagnostics": source_diags,
    });
    let kind_human = match out.kind {
        SourceKind::Directory => "directory",
        SourceKind::GitRepo => "git repo",
    };
    let human = format!(
        "Registered source {} -> {} ({kind_human})\nSkills found: {}\n",
        out.entry.alias,
        neutralize(&out.entry.path),
        out.skill_count
    );
    Ok(Done {
        data,
        warnings: warnings_only(&all),
        human_diags: all,
        human,
    })
}

pub fn list(ctx: &Ctx) -> CmdResult {
    let rows = source::list(&ctx.config, &ctx.dirs, ctx.options).map_err(Failed::from)?;
    let mut all = ctx.config_diags.clone();
    let sources: Vec<_> = rows
        .iter()
        .map(|r| {
            all.extend(r.diagnostics.iter().cloned());
            json!({
                "alias": r.alias,
                "path": r.raw_path,
                "expanded_path": r.expanded_path.to_string_lossy(),
                "kind": kind_str(r.kind),
                "precedence": r.precedence,
                "reachable": r.reachable,
                "skill_count": r.skill_count,
                "last_scan": r.last_scan,
                "diagnostics": r.diagnostics,
            })
        })
        .collect();
    let table_rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| {
            vec![
                r.alias.clone(),
                neutralize(&r.raw_path).into_owned(),
                kind_str(r.kind).to_string(),
                r.skill_count.map_or("-".to_string(), |n| n.to_string()),
                r.last_scan.clone().unwrap_or_else(|| "-".to_string()),
                if r.reachable { "ok" } else { "unreachable" }.to_string(),
            ]
        })
        .collect();
    let human = if rows.is_empty() {
        "No sources registered. Run `pouch source add <alias> <path>`.\n".to_string()
    } else {
        table(
            &["ALIAS", "PATH", "KIND", "SKILLS", "LAST SCAN", "STATUS"],
            &table_rows,
        )
    };
    let warnings = warnings_only(&all);
    Ok(Done {
        data: json!({ "sources": sources }),
        human_diags: warnings.clone(),
        warnings,
        human,
    })
}

pub fn rm(ctx: &Ctx, alias: &str, force: bool) -> CmdResult {
    let mut config = ctx.config.clone();
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let bundles = ctx.bundles();
    let out = source::remove(&mut config, &ctx.dirs, &catalog, &bundles, alias, force)
        .map_err(Failed::from)?;
    let mut all = ctx.config_diags.clone();
    all.extend(out.diagnostics.iter().cloned().map(FlatDiagnostic::global));
    let mut human = format!("Removed source {}\n", out.alias);
    if !out.dangling_projects.is_empty() {
        human.push_str("These project selections now hold unresolvable refs:\n");
        for p in &out.dangling_projects {
            human.push_str(&format!("  {}\n", neutralize(p)));
        }
    }
    let warnings = warnings_only(&all);
    Ok(Done {
        data: json!({
            "alias": out.alias,
            "removed": true,
            "forced": out.forced,
            "dangling_projects": out.dangling_projects,
        }),
        human_diags: warnings.clone(),
        warnings,
        human,
    })
}

/// How one dependent project ended.
struct ProjectEnd {
    status: &'static str,
    results: Vec<Value>,
    error: Option<String>,
}

impl ProjectEnd {
    fn new(status: &'static str) -> Self {
        ProjectEnd {
            status,
            results: Vec::new(),
            error: None,
        }
    }

    fn failed(&self) -> bool {
        matches!(self.status, "failed" | "interrupted")
    }
}

/// The status of a project that is not repaired.
fn idle_status(p: &source::ProjectRelocation) -> &'static str {
    match &p.planned {
        Planned::Plan(prep) if prep.plan.is_blocked() => "blocked",
        Planned::Plan(_) => "no-changes",
        Planned::Unplanned(_) => "unplanned",
    }
}

/// Apply the repair plan of one project through the journaled executor.
fn repair(
    ctx: &Ctx,
    rel: &source::Relocation,
    p: &source::ProjectRelocation,
    bundles: &pouch_core::bundle::BundleSet,
) -> ProjectEnd {
    let clock = SystemClock;
    let req = ChangeRequest {
        dirs: &ctx.dirs,
        config: &rel.config,
        catalog: &rel.catalog,
        bundles,
        root: &p.project,
        target: p.target,
        edit: Edit::Reconcile,
        command: "source relocate",
        clock: &clock,
    };
    match change::run(&req, &RealFs, &mut |_| Decision::Proceed) {
        Ok(Outcome::Applied(_, report)) => ProjectEnd {
            status: if report.status == ApplyStatus::Applied {
                "repaired"
            } else {
                "failed"
            },
            results: report.results.iter().map(result_json).collect(),
            error: None,
        },
        Ok(Outcome::NoChanges(_)) => ProjectEnd::new("no-changes"),
        Ok(Outcome::Blocked(_)) => ProjectEnd::new("blocked"),
        Ok(Outcome::Interrupted(_)) => ProjectEnd::new("interrupted"),
        Ok(Outcome::Refused(_) | Outcome::Declined(_)) => ProjectEnd::new("declined"),
        Err(f) => ProjectEnd {
            status: "failed",
            results: Vec::new(),
            error: Some(f.error.to_string()),
        },
    }
}

/// `pouch source relocate <alias> <path>` (spec 4.2; M4-10): plan the repair
/// of every dependent project, confirm once, then save the config, drop the
/// alias cache file, and apply each plan.
pub fn relocate(ctx: &Ctx, alias: &str, path: &str) -> CmdResult {
    let bundles = ctx.bundles();
    let clock = SystemClock;
    let rel = source::plan_relocation(
        &ctx.config,
        &ctx.dirs,
        &bundles,
        alias,
        path,
        ctx.options,
        &clock,
    )
    .map_err(|e| Failed {
        error: e.into(),
        warnings: warnings_only(&ctx.config_diags),
    })?;
    let mut all = ctx.config_diags.clone();
    all.extend(global(&rel.diagnostics));
    // Which projects will be repaired, and what to tell about the others.
    let runnable = |p: &source::ProjectRelocation| match &p.planned {
        Planned::Plan(prep) => !prep.plan.is_blocked() && !prep.plan.steps.is_empty(),
        Planned::Unplanned(_) => false,
    };
    let mut human = String::new();
    let mut preview: Vec<Value> = Vec::new();
    for p in &rel.projects {
        let root_text = p.project.path.to_string_lossy().into_owned();
        let (plan_json, status, error) = match &p.planned {
            Planned::Plan(prep) => {
                if !ctx.json {
                    human.push_str(&render_plan(&prep.plan, None));
                    if prep.plan.is_blocked() {
                        human.push_str("  Not repaired: conflicts block this project.\n");
                    }
                    human.push('\n');
                }
                let status = if prep.plan.is_blocked() {
                    "blocked"
                } else if prep.plan.steps.is_empty() {
                    "no-changes"
                } else {
                    "planned"
                };
                (prep.plan.to_json(), status, None)
            }
            Planned::Unplanned(msg) => {
                if !ctx.json {
                    human.push_str(&format!(
                        "Project   {}  ({})\n  Not repaired: {}\n\n",
                        neutralize(&root_text),
                        p.target,
                        neutralize(msg)
                    ));
                }
                (Value::Null, "unplanned", Some(msg.clone()))
            }
        };
        preview.push(json!({
            "root": root_text,
            "target": p.target.as_str(),
            "status": status,
            "plan": plan_json,
            "results": [],
            "error": error,
        }));
    }
    let data = |status: &str, projects: Vec<Value>| -> Value {
        json!({
            "alias": rel.alias,
            "old_path": rel.old_path,
            "path": rel.entry.path,
            "expanded_path": rel.expanded_path.to_string_lossy(),
            "status": status,
            "projects": projects,
        })
    };
    if rel.projects.iter().any(runnable) {
        if !ctx.json {
            out!("{human}");
        }
        match ask(ctx) {
            Decision::Proceed => {}
            Decision::Decline => {
                return Ok(Done {
                    data: data("declined", preview),
                    warnings: warnings_only(&all),
                    human_diags: all,
                    human: "Cancelled: nothing was changed.\n".to_string(),
                });
            }
            Decision::Refuse => {
                return Err(Failed::with_data(
                    Error::ConfirmationRequired,
                    data("refused", preview),
                    warnings_only(&all),
                ));
            }
        }
        human.clear();
    }
    source::commit_relocation(&rel, &ctx.dirs).map_err(|e| Failed {
        error: e.into(),
        warnings: warnings_only(&all),
    })?;
    let mut ends: Vec<Value> = Vec::new();
    let mut failed = 0usize;
    let mut summary = format!(
        "Relocated source {} -> {}\n",
        rel.alias,
        neutralize(&rel.entry.path)
    );
    for (p, base) in rel.projects.iter().zip(preview) {
        let end = if runnable(p) {
            repair(ctx, &rel, p, &bundles)
        } else {
            ProjectEnd::new(idle_status(p))
        };
        if end.failed() {
            failed += 1;
        }
        let mut v = base;
        if let Some(o) = v.as_object_mut() {
            o.insert("status".into(), json!(end.status));
            o.insert("results".into(), json!(end.results));
            if end.error.is_some() {
                o.insert("error".into(), json!(end.error));
            }
        }
        summary.push_str(&format!(
            "  {}  {} ({})\n",
            end.status,
            neutralize(&p.project.path.to_string_lossy()),
            p.target
        ));
        ends.push(v);
    }
    if rel.projects.is_empty() {
        summary.push_str("No indexed project holds an entry from this source.\n");
    }
    let out = if ctx.json { String::new() } else { summary };
    if failed > 0 {
        if !ctx.json {
            out!("{out}");
        }
        return Err(Failed::with_data(
            Error::ApplyFailed { failed, skipped: 0 },
            data("relocated", ends),
            warnings_only(&all),
        ));
    }
    Ok(Done {
        data: data("relocated", ends),
        warnings: warnings_only(&all),
        human_diags: all,
        human: out,
    })
}
