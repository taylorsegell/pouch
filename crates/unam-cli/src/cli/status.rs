//! `unam status` and `unam reconcile` (spec 4.11, 4.12, 5; M4 plan E2).
//! Both are read-only: no lock and no write. Exit 8 carries the report
//! like exit 10 carries the interruption: the human text prints first, the
//! `--json` envelope has `ok: false` with the full report in `data`.

use super::change::global;
use super::{CmdResult, Ctx, Done, Failed, Flags, catalog_warnings};
use crate::output::{render_plan, render_records, render_target_header, warnings_only};
use serde_json::{Value, json};
use unam_core::bundle::BundleSet;
use unam_core::catalog::{Catalog, FlatDiagnostic};
use unam_core::change as core_change;
use unam_core::error::Error;
use unam_core::project::{self, ProjectRoot, RootDetection, TargetId};
use unam_core::reconcile::{self, ReconcileReport};
use unam_core::state::SystemClock;
use unam_core::verify::{self, EntryState, FOOTER, VerifyReport, VerifyRequest};

/// What a read-only report command needs.
struct View {
    root: ProjectRoot,
    target: TargetId,
    catalog: Catalog,
    bundles: BundleSet,
    warnings: Vec<FlatDiagnostic>,
}

fn view(ctx: &Ctx) -> Result<View, Failed> {
    let root = ctx.root().map_err(Failed::from)?;
    let flag = ctx.target_flag().map_err(Failed::from)?;
    let selection = project::load_selection(&root.path).map_err(Failed::from)?;
    let (target, _) = core_change::resolve_target(flag, selection.as_ref(), &root.path, &ctx.dirs)
        .map_err(Failed::from)?;
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let warnings = catalog_warnings(ctx, &catalog);
    Ok(View {
        root,
        target,
        catalog,
        bundles: ctx.bundles(),
        warnings,
    })
}

impl View {
    fn request<'a>(&'a self, ctx: &'a Ctx) -> VerifyRequest<'a> {
        VerifyRequest {
            dirs: &ctx.dirs,
            config: &ctx.config,
            catalog: &self.catalog,
            bundles: &self.bundles,
            root: &self.root,
            target: self.target,
        }
    }
}

fn how_text(how: RootDetection) -> &'static str {
    match how {
        RootDetection::Git => "git",
        RootDetection::Explicit => "explicit",
    }
}

/// The JSON keys every report shares.
fn base_json(command: &str, v: &View, report: &VerifyReport) -> Value {
    json!({
        "command": command,
        "project_root": v.root.path.to_string_lossy(),
        "root_detection": how_text(v.root.how),
        "target": v.target.as_str(),
        "destination": report.dest_rel,
        "records": report.records.iter().map(|r| r.to_json()).collect::<Vec<_>>(),
        "summary": report.summary_json(),
        "footer": FOOTER,
    })
}

fn status_human(v: &View, report: &VerifyReport) -> String {
    let mut out = render_target_header(
        &v.root.path,
        v.root.how,
        v.target.as_str(),
        &report.dest_rel,
    );
    out.push_str(&render_records(&report.records));
    out
}

/// `unam status`.
pub fn status(ctx: &Ctx) -> CmdResult {
    let v = view(ctx)?;
    let report = verify::verify(&v.request(ctx)).map_err(Failed::from)?;
    let human = status_human(&v, &report);
    let data = base_json("status", &v, &report);
    let mut warnings = v.warnings.clone();
    warnings.extend(global(&report.diagnostics));
    let orphaned = report.count(EntryState::Orphaned);
    if orphaned > 0 {
        if !ctx.json {
            out!("{human}");
        }
        return Err(Failed::with_data(
            Error::Diverged {
                count: orphaned,
                kind: "orphaned entry(ies)".to_string(),
            },
            data,
            warnings_only(&warnings),
        ));
    }
    Ok(Done {
        data,
        warnings: warnings_only(&warnings),
        human_diags: warnings,
        human,
    })
}

/// Bare `unam` inside a project with `.unam/`: the status table, or nothing
/// when anything goes wrong (the usage text has already been printed).
pub fn bare(flags: &Flags) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let root = core_change::resolve_root(flags.project.as_deref(), &cwd).ok()?;
    if !root.path.join(".unam").is_dir() {
        return None;
    }
    let ctx = Ctx::load(flags).ok()?;
    let v = match view(&ctx) {
        Ok(v) => v,
        // Two targets and no `--target`: the usage text is already printed;
        // add the one-line hint, still exit 0 (M5-3).
        Err(f) => {
            return match f.error.downcast_ref::<Error>() {
                Some(Error::TargetAmbiguous { targets }) => Some(format!(
                    "This project uses more than one target ({}); pass --target <id> to choose one, for example `unam --target {} status`.\n",
                    targets.join(", "),
                    targets.first().map_or("claude-code", String::as_str)
                )),
                _ => None,
            };
        }
    };
    let report = verify::verify(&v.request(&ctx)).ok()?;
    Some(status_human(&v, &report))
}

fn reconcile_human(v: &View, r: &ReconcileReport) -> String {
    let mut out = render_target_header(
        &v.root.path,
        v.root.how,
        v.target.as_str(),
        &r.verify.dest_rel,
    );
    if r.divergences.is_empty() {
        out.push_str("No divergence: the destination matches the selection and the receipt.\n");
    } else {
        out.push_str(&format!(
            "{} divergence(s), by cause:\n",
            r.divergences.len()
        ));
        let mut seen: Vec<&'static str> = Vec::new();
        for c in r.divergences.iter().map(|d| d.cause.as_str()) {
            if seen.contains(&c) {
                continue;
            }
            seen.push(c);
            out.push_str(&format!("\n{c}\n"));
            for d in r.divergences.iter().filter(|d| d.cause.as_str() == c) {
                out.push_str(&format!("  ! {}\n", crate::output::neutralize(&d.detail)));
            }
        }
    }
    if !r.findings.is_empty() {
        out.push_str("\nAlso noted (not divergences):\n");
        for f in &r.findings {
            out.push_str(&format!(
                "  - {}: {} ({})\n",
                f.kind,
                crate::output::neutralize(&f.dest),
                crate::output::neutralize(&f.detail)
            ));
        }
    }
    if !r.divergences.is_empty() {
        out.push('\n');
        match (&r.plan, &r.repair_blocked) {
            (_, Some(why)) => {
                out.push_str(&format!(
                    "Repair is blocked: {}\n",
                    crate::output::neutralize(why)
                ));
                if let Some(plan) = &r.plan {
                    out.push('\n');
                    out.push_str(&render_plan(plan, None));
                }
            }
            (Some(plan), None) => {
                out.push_str("Repair plan (`unam reconcile --repair` applies it):\n\n");
                out.push_str(&render_plan(plan, None));
            }
            (None, None) => {}
        }
    }
    out
}

/// `unam reconcile [--repair]`.
pub fn reconcile(ctx: &Ctx, repair: bool) -> CmdResult {
    if repair {
        return super::change::reconcile_repair(ctx);
    }
    let v = view(ctx)?;
    let clock = SystemClock;
    let r = reconcile::report(&v.request(ctx), &clock).map_err(Failed::from)?;
    let human = reconcile_human(&v, &r);
    let mut data = base_json("reconcile", &v, &r.verify);
    if let Some(o) = data.as_object_mut() {
        o.insert(
            "divergences".into(),
            json!(
                r.divergences
                    .iter()
                    .map(|d| d.to_json())
                    .collect::<Vec<_>>()
            ),
        );
        o.insert(
            "findings".into(),
            json!(r.findings.iter().map(|f| f.to_json()).collect::<Vec<_>>()),
        );
        o.insert(
            "plan".into(),
            r.plan.as_ref().map_or(Value::Null, |p| p.to_json()),
        );
        o.insert("repair_blocked".into(), json!(r.repair_blocked));
    }
    let mut warnings = v.warnings.clone();
    warnings.extend(global(&r.verify.diagnostics));
    if !r.divergences.is_empty() {
        if !ctx.json {
            out!("{human}");
        }
        return Err(Failed::with_data(
            Error::Diverged {
                count: r.divergences.len(),
                kind: "divergence(s)".to_string(),
            },
            data,
            warnings_only(&warnings),
        ));
    }
    Ok(Done {
        data,
        warnings: warnings_only(&warnings),
        human_diags: warnings,
        human,
    })
}
