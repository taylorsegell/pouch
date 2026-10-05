//! `unam bundle save|ls|show|edit|duplicate|rm|add` (spec 5; M2 plan D6,
//! P10, P11, P13, CLI Contract). Thin: load what the subcommand needs, call
//! `unam-core`, render.

use super::{CmdResult, Ctx, Done, Failed};
use crate::output::{neutralize, table, warnings_only};
use serde_json::{Value, json};
use unam_core::bundle::{self, Bundle};
use unam_core::catalog::{Catalog, FlatDiagnostic, SOURCE_AND_CONFIG};
use unam_core::error::Error;
use unam_core::project::{self, index};
use unam_core::resolve::{self, ProblemKind};

/// Config-level plus source-level warnings of a catalog load.
fn catalog_warnings(ctx: &Ctx, catalog: &Catalog) -> Vec<FlatDiagnostic> {
    let mut all = ctx.config_diags.clone();
    all.extend(catalog.flat_diagnostics(None, SOURCE_AND_CONFIG));
    warnings_only(&all)
}

fn failed(error: Error, warnings: &[FlatDiagnostic]) -> Failed {
    Failed {
        error: error.into(),
        warnings: warnings.to_vec(),
    }
}

fn done(data: Value, warnings: Vec<FlatDiagnostic>, human: String) -> CmdResult {
    Ok(Done {
        data,
        human_diags: warnings.clone(),
        warnings,
        human,
    })
}

fn path_str(p: &std::path::Path) -> String {
    p.to_string_lossy().into_owned()
}

fn not_found(name: &str) -> Error {
    let slug = bundle::slugify(name);
    Error::BundleNotFound {
        slug: if slug.is_empty() {
            name.trim().to_string()
        } else {
            slug
        },
    }
}

/// `bundle save <name> [--force]` (plan P10).
pub fn save(ctx: &Ctx, name: &str, force: bool) -> CmdResult {
    let cwd = std::env::current_dir()
        .map_err(|e| Failed::from(Error::io("cannot read the working directory", e)))?;
    let root = project::detect_root(&cwd, None).map_err(Failed::from)?;
    let no_selection = |reason: &str| Error::NoSelection {
        root: root.path.clone(),
        reason: reason.to_string(),
    };
    let selection = project::load_selection(&root.path)
        .map_err(Failed::from)?
        .ok_or_else(|| Failed::from(no_selection("`.unam/selection.yaml` does not exist")))?;
    let (target, tsel) = match selection.targets.len() {
        0 => return Err(no_selection("the selection has no targets").into()),
        1 => selection
            .targets
            .iter()
            .next()
            .map(|(t, s)| (*t, s.clone()))
            .ok_or_else(|| Failed::from(no_selection("the selection has no targets")))?,
        _ => {
            return Err(Error::TargetAmbiguous {
                targets: selection.targets.keys().map(|t| t.to_string()).collect(),
            }
            .into());
        }
    };
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let mut warnings = catalog_warnings(ctx, &catalog);
    let bundles = ctx.bundles();
    let resolution =
        resolve::resolve_selection(&tsel, &bundles, &catalog, ctx.config.defaults.strategy);
    if let Some(error) = resolution.first_error() {
        warnings.extend(
            resolution
                .problems
                .iter()
                .map(|p| FlatDiagnostic::global(p.to_diagnostic())),
        );
        return Err(failed(error, &warnings));
    }
    if resolution.resolved.is_empty() {
        return Err(failed(
            no_selection("the selection resolves to no skills"),
            &warnings,
        ));
    }
    let skills: Vec<String> = resolution
        .resolved
        .iter()
        .map(|e| e.id.to_string())
        .collect();
    let out = bundle::save(&ctx.dirs, &bundles, name, &skills, force)
        .map_err(|e| failed(e, &warnings))?;
    let mut human = format!(
        "Saved bundle {} ({} skills) -> {}",
        neutralize(&out.slug),
        skills.len(),
        neutralize(&path_str(&out.path))
    );
    if out.overwritten {
        human.push_str(" (overwrote existing)");
    }
    human.push('\n');
    done(
        json!({
            "slug": out.slug,
            "name": out.name,
            "path": path_str(&out.path),
            "project_root": path_str(&root.path),
            "target": target.as_str(),
            "skill_count": skills.len(),
            "skills": skills,
            "overwritten": out.overwritten,
        }),
        warnings,
        human,
    )
}

/// `bundle ls`: never an error; broken manifests are diagnostics.
pub fn ls(ctx: &Ctx) -> CmdResult {
    let set = ctx.bundles();
    let mut bundles: Vec<&Bundle> = set.bundles.iter().collect();
    bundles.sort_by(|a, b| a.slug.cmp(&b.slug));
    let diags: Vec<FlatDiagnostic> = set
        .diagnostics
        .iter()
        .cloned()
        .map(FlatDiagnostic::global)
        .collect();
    let rows: Vec<Value> = bundles
        .iter()
        .map(|b| {
            json!({
                "slug": b.slug,
                "name": b.name,
                "description": b.description,
                "skill_count": b.skills.len(),
                "has_instruction": b.instruction.is_some(),
                "path": path_str(&b.path),
            })
        })
        .collect();
    let human = if bundles.is_empty() {
        format!(
            "No bundles in {}\n",
            neutralize(&path_str(&ctx.dirs.bundles))
        )
    } else {
        table(
            &["SLUG", "NAME", "SKILLS", "DESCRIPTION"],
            &bundles
                .iter()
                .map(|b| {
                    vec![
                        neutralize(&b.slug).into_owned(),
                        neutralize(&b.name).into_owned(),
                        b.skills.len().to_string(),
                        crate::output::head(&b.description, crate::output::HEAD_CHARS),
                    ]
                })
                .collect::<Vec<_>>(),
        )
    };
    let mut all = ctx.config_diags.clone();
    all.extend(diags.iter().cloned());
    done(
        json!({ "bundles": rows, "diagnostics": diags }),
        warnings_only(&all),
        human,
    )
}

fn status_of(kind: ProblemKind) -> &'static str {
    match kind {
        ProblemKind::Invalid => "invalid",
        ProblemKind::Unresolved | ProblemKind::BundleMissing => "unresolved",
        ProblemKind::Ambiguous => "ambiguous",
    }
}

/// `bundle show <name>`: manifest fields plus each entry's resolution.
pub fn show(ctx: &Ctx, name: &str) -> CmdResult {
    let set = ctx.bundles();
    let b = set.get(name).ok_or_else(|| Failed::from(not_found(name)))?;
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let warnings = catalog_warnings(ctx, &catalog);
    let entries = resolve::resolve_entries(b, &catalog);
    let mut problems = 0usize;
    let mut human = format!("{}  ({})\n\n", neutralize(&b.name), neutralize(&b.slug));
    human.push_str(&format!("Description: {}\n", neutralize(&b.description)));
    human.push_str(&format!("Path: {}\n", neutralize(&path_str(&b.path))));
    if let Some(i) = &b.instruction {
        human.push_str(&format!("Instruction: {}\n", neutralize(i)));
    }
    human.push_str(&format!("Skills ({}):\n", entries.len()));
    let skills: Vec<Value> = entries
        .iter()
        .map(|e| match &e.outcome {
            Ok(id) => {
                human.push_str(&format!("  {}  ->  {}\n", neutralize(&e.entry), id));
                json!({
                    "entry": e.entry,
                    "status": "resolved",
                    "id": id.to_string(),
                    "candidates": [],
                })
            }
            Err(p) => {
                problems += 1;
                let detail = if p.candidates.is_empty() {
                    p.message.clone()
                } else {
                    p.candidates.join(", ")
                };
                human.push_str(&format!(
                    "  {}  !! {}: {}\n",
                    neutralize(&e.entry),
                    status_of(p.kind),
                    neutralize(&detail)
                ));
                json!({
                    "entry": e.entry,
                    "status": status_of(p.kind),
                    "id": null,
                    "candidates": p.candidates,
                })
            }
        })
        .collect();
    done(
        json!({
            "slug": b.slug,
            "name": b.name,
            "description": b.description,
            "instruction": b.instruction,
            "path": path_str(&b.path),
            "skills": skills,
            "problems": problems,
        }),
        warnings,
        human,
    )
}

/// `bundle edit <name>` (plan P13): `EDITOR` is read through `Dirs.env`.
pub fn edit(ctx: &Ctx, name: &str) -> CmdResult {
    let set = ctx.bundles();
    let editor = ctx
        .dirs
        .env
        .get("EDITOR")
        .map(|v| v.to_string_lossy().into_owned());
    let out =
        bundle::edit_in_editor(&ctx.dirs, &set, name, editor.as_deref()).map_err(Failed::from)?;
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let mut warnings = catalog_warnings(ctx, &catalog);
    let mut problems = 0usize;
    let reloaded = ctx.bundles();
    if let Some(b) = reloaded.get(&out.slug) {
        for e in resolve::resolve_entries(b, &catalog) {
            if let Err(p) = e.outcome {
                problems += 1;
                let mut d = p.to_diagnostic();
                d.severity = unam_core::error::Severity::Warning;
                warnings.push(FlatDiagnostic::global(d.with_path(&b.file_name)));
            }
        }
    }
    let human = if out.changed {
        format!("Bundle {} updated\n", neutralize(&out.slug))
    } else {
        "No changes\n".to_string()
    };
    done(
        json!({
            "slug": out.slug,
            "path": path_str(&out.path),
            "changed": out.changed,
            "problems": problems,
        }),
        warnings,
        human,
    )
}

/// `bundle duplicate <name> <new-name> [--force]`.
pub fn duplicate(ctx: &Ctx, name: &str, new_name: &str, force: bool) -> CmdResult {
    let set = ctx.bundles();
    let source = set
        .get(name)
        .map(|b| b.slug.clone())
        .ok_or_else(|| Failed::from(not_found(name)))?;
    let out = bundle::duplicate(&ctx.dirs, &set, name, new_name, force).map_err(Failed::from)?;
    let human = format!(
        "Copied {} -> {} ({})\n",
        neutralize(&source),
        neutralize(&out.slug),
        neutralize(&path_str(&out.path))
    );
    done(
        json!({
            "source": source,
            "slug": out.slug,
            "name": out.name,
            "path": path_str(&out.path),
            "overwritten": out.overwritten,
        }),
        warnings_only(&ctx.config_diags),
        human,
    )
}

/// `bundle rm <name>` deletes the manifest; `bundle rm <name> <ref>...`
/// removes entries (plan P11).
pub fn rm(ctx: &Ctx, name: &str, references: &[String], rewrite: bool) -> CmdResult {
    let set = ctx.bundles();
    if references.is_empty() {
        let slug = set
            .get(name)
            .map(|b| b.slug.clone())
            .ok_or_else(|| Failed::from(not_found(name)))?;
        let (projects, index_diags) = index::projects_referencing_bundle(&ctx.dirs, &slug);
        let out = bundle::delete(&ctx.dirs, &set, name).map_err(Failed::from)?;
        let mut all = ctx.config_diags.clone();
        all.extend(index_diags.into_iter().map(FlatDiagnostic::global));
        let mut human = format!(
            "Deleted bundle {} ({})\n",
            neutralize(&out.slug),
            neutralize(&path_str(&out.path))
        );
        if !projects.is_empty() {
            human.push_str("These project selections reference it:\n");
            for p in &projects {
                human.push_str(&format!("  {}\n", neutralize(p)));
            }
            human.push_str(
                "They keep the reference, which no longer resolves until the bundle is restored.\n",
            );
        }
        return done(
            json!({
                "slug": out.slug,
                "path": path_str(&out.path),
                "deleted": true,
                "referencing_projects": projects,
            }),
            warnings_only(&all),
            human,
        );
    }
    if set.get(name).is_none() {
        return Err(not_found(name).into());
    }
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let warnings = catalog_warnings(ctx, &catalog);
    let out = bundle::remove_refs(&ctx.dirs, &set, &catalog, name, references, rewrite)
        .map_err(|e| failed(e, &warnings))?;
    let n = out.removed.len();
    let human = format!(
        "Removed {n} entr{} from {}\n",
        if n == 1 { "y" } else { "ies" },
        neutralize(&out.slug)
    );
    done(
        json!({
            "slug": out.slug,
            "path": path_str(&out.path),
            "removed": out.removed,
            "rewrite": out.rewrite,
            "skill_count": out.skill_count,
        }),
        warnings,
        human,
    )
}

/// `bundle add <name> <ref>... [--rewrite]` (plan P11).
pub fn add(ctx: &Ctx, name: &str, references: &[String], rewrite: bool) -> CmdResult {
    let set = ctx.bundles();
    if set.get(name).is_none() {
        return Err(not_found(name).into());
    }
    let catalog = ctx.catalog().map_err(Failed::from)?;
    let warnings = catalog_warnings(ctx, &catalog);
    let out = bundle::add_refs(&ctx.dirs, &set, &catalog, name, references, rewrite)
        .map_err(|e| failed(e, &warnings))?;
    let mut human = format!(
        "Added {} skill(s) to {}\n",
        out.added.len(),
        neutralize(&out.slug)
    );
    if !out.skipped.is_empty() {
        human.push_str(&format!(
            "Already present: {}\n",
            neutralize(&out.skipped.join(", "))
        ));
    }
    done(
        json!({
            "slug": out.slug,
            "path": path_str(&out.path),
            "added": out.added,
            "skipped": out.skipped,
            "rewrite": out.rewrite,
            "skill_count": out.skill_count,
        }),
        warnings,
        human,
    )
}
