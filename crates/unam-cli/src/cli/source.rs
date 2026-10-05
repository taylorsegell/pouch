//! `unam source add|list|rm`.

use super::{CmdResult, Ctx, Done, Failed};
use crate::output::{neutralize, table, warnings_only};
use serde_json::json;
use unam_core::catalog::{ALL_CLASSES, FlatDiagnostic};
use unam_core::source::{self, SourceKind};

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
        "No sources registered. Run `unam source add <alias> <path>`.\n".to_string()
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
    let out = source::remove(&mut config, &ctx.dirs, alias, force).map_err(Failed::from)?;
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
