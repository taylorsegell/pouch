//! `pouch search <query> [--category p] [--source a]`.

use super::{CmdResult, Ctx, Done, Failed};
use crate::output::{HEAD_CHARS, head, neutralize, table, warnings_only};
use pouch_core::catalog::{ALL_CLASSES, SOURCE_AND_CONFIG, search};
use pouch_core::error::Error;
use pouch_core::source::SourceAlias;
use serde_json::json;

pub fn run(ctx: &Ctx, query: &str, category: Option<&str>, source: Option<&str>) -> CmdResult {
    let filter = match source {
        None => None,
        Some(a) => {
            let known = ctx.config.sources.iter().any(|s| s.alias == a);
            match SourceAlias::parse(a) {
                Ok(alias) if known => Some(alias),
                _ => {
                    return Err(Failed::from(Error::UnknownSourceFilter {
                        alias: a.to_string(),
                    }));
                }
            }
        }
    };
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let hits = search::search(&catalog, query, category, filter.as_ref());
    let mut all = ctx.config_diags.clone();
    all.extend(catalog.flat_diagnostics(None, SOURCE_AND_CONFIG));
    let mut rows = Vec::new();
    let results: Vec<_> = hits
        .iter()
        .filter_map(|h| catalog.by_id.get(&h.id).map(|&i| (h, &catalog.skills[i])))
        .map(|(h, s)| {
            rows.push(vec![
                neutralize(&s.id.to_string()).into_owned(),
                neutralize(&s.name).into_owned(),
                neutralize(&s.category).into_owned(),
                s.id.source.to_string(),
                head(&s.description, HEAD_CHARS),
            ]);
            let skill_diags: Vec<_> = catalog
                .flat_diagnostics(Some(&s.id.source), ALL_CLASSES)
                .into_iter()
                .filter(|d| d.skill.as_deref() == Some(s.id.to_string().as_str()))
                .collect();
            json!({
                "id": s.id,
                "name": s.name,
                "category": s.category,
                "source": s.id.source,
                "description": s.description,
                "score": h.score,
                "diagnostics": skill_diags,
            })
        })
        .collect();
    let human = if rows.is_empty() {
        format!("No skills match `{}`.\n", neutralize(query))
    } else {
        table(&["ID", "NAME", "CATEGORY", "SOURCE", "DESCRIPTION"], &rows)
    };
    let warnings = warnings_only(&all);
    Ok(Done {
        data: json!({
            "query": query,
            "category": category,
            "source": source,
            "total": results.len(),
            "results": results,
        }),
        human_diags: warnings.clone(),
        warnings,
        human,
    })
}
