//! `unam doctor [--evidence]` (spec 5; M4 plan E3).

use super::{CmdResult, Ctx, Done, Failed, catalog_warnings};
use crate::output::{neutralize, render_target_header, table, warnings_only};
use serde_json::json;
use unam_core::change as core_change;
use unam_core::doctor::{self, DoctorRequest};
use unam_core::error::Error;
use unam_core::project::{self, RootDetection};

pub fn run(ctx: &Ctx, evidence: bool) -> CmdResult {
    let root = ctx.root().map_err(Failed::from)?;
    let flag = ctx.target_flag().map_err(Failed::from)?;
    let selection = project::load_selection(&root.path).map_err(Failed::from)?;
    let (target, _) = core_change::resolve_target(flag, selection.as_ref(), &root.path, &ctx.dirs)
        .map_err(Failed::from)?;
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let warnings = catalog_warnings(ctx, &catalog);
    let report = doctor::run(&DoctorRequest {
        dirs: &ctx.dirs,
        catalog: &catalog,
        root: &root,
        target,
        evidence,
    })
    .map_err(Failed::from)?;

    let mut human = render_target_header(&root.path, root.how, target.as_str(), &report.dest_rel);
    let rows: Vec<Vec<String>> = report
        .checks
        .iter()
        .map(|c| {
            vec![
                c.status.as_str().to_string(),
                c.id.to_string(),
                neutralize(&c.message).into_owned(),
            ]
        })
        .collect();
    human.push_str(&table(&["RESULT", "CHECK", "DETAIL"], &rows));
    for c in report.checks.iter().filter(|c| c.remedy.is_some()) {
        if let Some(r) = &c.remedy {
            human.push_str(&format!("  hint ({}): {}\n", c.id, neutralize(r)));
        }
    }
    if let Some(rows) = &report.evidence {
        human.push_str("\nAdapter evidence:\n");
        let table_rows: Vec<Vec<String>> = rows
            .iter()
            .map(|e| {
                vec![
                    e.adapter.clone(),
                    e.claim_key.clone(),
                    e.status.to_string(),
                    e.source.clone(),
                    e.claim.clone(),
                ]
            })
            .collect();
        human.push_str(&table(
            &["ADAPTER", "CLAIM", "STATUS", "SOURCE", "DETAIL"],
            &table_rows,
        ));
    }
    let data = json!({
        "command": "doctor",
        "project_root": root.path.to_string_lossy(),
        "root_detection": match root.how {
            RootDetection::Git => "git",
            RootDetection::Explicit => "explicit",
        },
        "target": target.as_str(),
        "destination": report.dest_rel,
        "checks": report.checks.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
        "evidence": report
            .evidence
            .as_ref()
            .map(|e| e.iter().map(|r| r.to_json()).collect::<Vec<_>>()),
    });
    let failed = report.failed();
    let all = warnings;
    if failed > 0 {
        if !ctx.json {
            print!("{human}");
        }
        return Err(Failed::with_data(
            Error::DoctorFailed { failed },
            data,
            warnings_only(&all),
        ));
    }
    Ok(Done {
        data,
        warnings: warnings_only(&all),
        human_diags: all,
        human,
    })
}
