//! `unam show <ref>`. "Bundles referencing it" arrives in milestone M2 (M2
//! plan P16); "projects where active" stays omitted until milestone M4.

use super::{CmdResult, Ctx, Done, Failed};
use crate::output::{diag_line, neutralize, warnings_only};
use serde_json::json;
use unam_core::catalog::{ALL_CLASSES, SOURCE_AND_CONFIG};

pub fn run(ctx: &Ctx, reference: &str) -> CmdResult {
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let mut all = ctx.config_diags.clone();
    all.extend(catalog.flat_diagnostics(None, SOURCE_AND_CONFIG));
    let warnings = warnings_only(&all);
    let skill = catalog.resolve_text(reference).map_err(|e| Failed {
        error: e.into(),
        warnings: warnings.clone(),
    })?;
    let path = catalog
        .abs_path(&skill.id)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let id = skill.id.to_string();
    let skill_diags: Vec<_> = catalog
        .flat_diagnostics(Some(&skill.id.source), ALL_CLASSES)
        .into_iter()
        .filter(|d| d.skill.as_deref() == Some(id.as_str()))
        .collect();

    let mut human = format!("{}  ({})\n\n", neutralize(&skill.name), neutralize(&id));
    human.push_str(&format!(
        "Description: {}\n",
        neutralize(&skill.description)
    ));
    human.push_str(&format!("Source: {}\n", skill.id.source));
    let category = if skill.category.is_empty() {
        "(none)".to_string()
    } else {
        neutralize(&skill.category).into_owned()
    };
    human.push_str(&format!("Category: {category}\n"));
    human.push_str(&format!("Path: {}\n", neutralize(&path)));
    human.push_str(&format!("Tree hash: {}\n", skill.tree_hash));
    let bundles = unam_core::resolve::bundles_referencing(&ctx.bundles(), &catalog, &skill.id);
    let bundles_human = if bundles.is_empty() {
        "(none)".to_string()
    } else {
        neutralize(&bundles.join(", ")).into_owned()
    };
    human.push_str(&format!("Bundles: {bundles_human}\n"));
    human.push_str(&format!("Supporting files ({}):\n", skill.supporting.len()));
    for f in &skill.supporting {
        human.push_str(&format!("  {}\n", neutralize(f)));
    }
    human.push_str("Frontmatter:\n");
    let yaml = serde_norway::to_string(&skill.frontmatter).unwrap_or_default();
    for line in yaml.lines() {
        human.push_str(&format!("  {}\n", neutralize(line)));
    }
    if !skill_diags.is_empty() {
        human.push_str("Diagnostics:\n");
        for d in &skill_diags {
            human.push_str(&format!("  {}\n", diag_line(d)));
        }
    }

    Ok(Done {
        data: json!({
            "id": skill.id,
            "name": skill.name,
            "description": skill.description,
            "category": skill.category,
            "source": skill.id.source,
            "rel_path": skill.id.rel_path.as_str(),
            "path": path,
            "frontmatter": skill.frontmatter,
            "supporting": skill.supporting,
            "tree_hash": skill.tree_hash,
            "bundles": bundles,
            "diagnostics": skill_diags,
        }),
        human_diags: warnings.clone(),
        warnings,
        human,
    })
}
