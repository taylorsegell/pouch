//! Generated bundle briefs (spec 4.8 step 7, 2.4; M5 plan M5-8 to M5-10).
//!
//! A brief is the directory `<dest>/<slug>-brief/` holding only `SKILL.md`:
//! frontmatter (`name`, a one-line `description`) built with the YAML
//! serializer, then the trimmed bundle `instruction`. It is wanted for each
//! bundle named in the target selection unless the bundle opts out with
//! `metadata.pouch.materialize_instruction: false`. Everything here is
//! pure except `observe_briefs`.

use crate::bundle::{Bundle, BundleSet};
use crate::error::{Diagnostic, Result};
use crate::fsops::{BriefObserved, DestPath, observe_brief, validate_dest_name};
use crate::hash::Hash;
use crate::observe::DestRootState;
use crate::plan::{Conflict, Step};
use crate::project::TargetSelection;
use crate::state::{GeneratedEntry, Receipt};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Characters of the first instruction line kept in the description.
const DESCRIPTION_LINE_CHARS: usize = 160;

/// Suffix of a brief's directory name.
pub const BRIEF_SUFFIX: &str = "-brief";

/// One wanted brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedSpec {
    pub bundle: String,
    /// The destination name, `<slug>-brief`.
    pub name: String,
    /// The exact `SKILL.md` bytes.
    pub content: String,
    pub content_hash: Hash,
}

/// The wanted briefs and what the destination holds for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GeneratedInput {
    pub wanted: Vec<GeneratedSpec>,
    /// Observations by destination name, for every wanted brief and every
    /// receipted one.
    pub observed: BTreeMap<String, BriefObserved>,
}

/// What `diff_generated` decided.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GeneratedDiff {
    /// Unnumbered steps: removes first, then writes, each by name.
    pub steps: Vec<Step>,
    pub conflicts: Vec<Conflict>,
    /// Receipted briefs that stay in the destination (wanted again).
    pub staying: usize,
    /// Wanted briefs the user edited: left untouched, receipt entry kept.
    pub warnings: Vec<Diagnostic>,
}

/// The instruction text of a brief: everything after the frontmatter.
pub fn brief_body(content: &str) -> &str {
    content
        .strip_prefix("---\n")
        .and_then(|rest| rest.split_once("\n---\n"))
        .map_or(content, |(_, body)| body.trim())
}

/// The destination name of a bundle's brief.
pub fn brief_name(slug: &str) -> String {
    format!("{slug}{BRIEF_SUFFIX}")
}

/// The bundles of `tsel` that want a brief, with their content (M5-9): a
/// bundle opts out with `materialize_instruction: false`; an absent value
/// means on; a bundle without an instruction has no brief. Sorted by name.
pub fn wanted(tsel: &TargetSelection, bundles: &BundleSet) -> Vec<GeneratedSpec> {
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut out: BTreeMap<String, GeneratedSpec> = BTreeMap::new();
    for text in &tsel.bundles {
        let Some(bundle) = bundles.get(text) else {
            continue;
        };
        if !seen.insert(bundle.slug.clone()) {
            continue;
        }
        if let Some(spec) = spec_of(bundle) {
            out.insert(spec.name.clone(), spec);
        }
    }
    out.into_values().collect()
}

#[derive(Serialize)]
struct Frontmatter<'a> {
    name: &'a str,
    description: &'a str,
}

/// The brief of one bundle, or `None` when it opts out, has no instruction,
/// or the frontmatter cannot be serialized.
fn spec_of(bundle: &Bundle) -> Option<GeneratedSpec> {
    if bundle.metadata.materialize_instruction == Some(false) {
        return None;
    }
    let instruction = bundle.instruction.as_deref()?.trim();
    if instruction.is_empty() {
        return None;
    }
    let name = brief_name(&bundle.slug);
    let content = brief_content(&name, &bundle.name, instruction)?;
    Some(GeneratedSpec {
        bundle: bundle.slug.clone(),
        content_hash: Hash::of_bytes(content.as_bytes()),
        name,
        content,
    })
}

/// A string on one line: control characters become spaces.
fn one_line(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// The exact `SKILL.md` text: serializer-built frontmatter, a blank line,
/// the trimmed instruction, a final newline.
fn brief_content(name: &str, bundle_name: &str, instruction: &str) -> Option<String> {
    let first: String = one_line(instruction.lines().next().unwrap_or(""))
        .chars()
        .take(DESCRIPTION_LINE_CHARS)
        .collect();
    let description = format!(
        "Instructions for working with the {} bundle: {first}",
        one_line(bundle_name)
    );
    let yaml = serde_norway::to_string(&Frontmatter {
        name,
        description: description.trim(),
    })
    .ok()?;
    Some(format!("---\n{yaml}---\n\n{instruction}\n"))
}

/// Observe the brief destinations for `names` (empty unless the destination
/// root is a plain directory).
pub fn observe_briefs(
    root: &Path,
    dest_rel: &str,
    dest_root: &DestRootState,
    names: &BTreeSet<String>,
) -> Result<BTreeMap<String, BriefObserved>> {
    let mut out = BTreeMap::new();
    if *dest_root != DestRootState::Dir {
        return Ok(out);
    }
    let Ok(droot) = DestPath::dest_root(root, dest_rel) else {
        return Ok(out);
    };
    for name in names {
        if validate_dest_name(name).is_err() {
            continue;
        }
        let Ok(child) = droot.join_child(name) else {
            continue;
        };
        out.insert(name.clone(), observe_brief(&child)?);
    }
    Ok(out)
}

/// The generated side of the diff (M5-10). `skill_ids` maps each desired
/// skill's destination name to its ID (a brief name equal to a skill name is
/// a duplicate).
pub fn diff_generated(
    dest_rel: &str,
    input: &GeneratedInput,
    receipt: &Receipt,
    skill_ids: &BTreeMap<String, String>,
) -> GeneratedDiff {
    let mut out = GeneratedDiff::default();
    let dest = |name: &str| format!("{dest_rel}/{name}");
    let mut receipted: BTreeMap<String, GeneratedEntry> = BTreeMap::new();
    for g in receipt.generated_entries() {
        if g.consistent(dest_rel) {
            receipted.entry(g.name.clone()).or_insert(g);
        }
    }
    let mut writes: BTreeMap<String, Step> = BTreeMap::new();
    let mut removes: BTreeMap<String, Step> = BTreeMap::new();
    let mut wanted_names: BTreeSet<&str> = BTreeSet::new();
    for spec in &input.wanted {
        let name = spec.name.as_str();
        if let Err(reason) = validate_dest_name(name) {
            out.conflicts.push(Conflict::InvalidName {
                id: format!("bundle {}", spec.bundle),
                name: spec.name.clone(),
                reason,
            });
            continue;
        }
        wanted_names.insert(name);
        if let Some(skill_id) = skill_ids.get(name) {
            out.conflicts.push(Conflict::DuplicateName {
                name: spec.name.clone(),
                ids: vec![skill_id.clone(), format!("brief of bundle {}", spec.bundle)],
            });
            continue;
        }
        let observed = input
            .observed
            .get(name)
            .cloned()
            .unwrap_or(BriefObserved::Missing);
        let write_as = |adopt: bool| Step::WriteGenerated {
            n: 0,
            bundle: spec.bundle.clone(),
            name: spec.name.clone(),
            content: spec.content.clone(),
            content_hash: spec.content_hash.clone(),
            adopt,
        };
        let write = || write_as(false);
        match (receipted.get(name), observed) {
            (None, BriefObserved::Missing) => {
                writes.insert(spec.name.clone(), write());
            }
            (None, other) => out.conflicts.push(Conflict::ForeignAtDest {
                dest: dest(name),
                found: found_of(&other).to_string(),
                source: format!("bundle {}", spec.bundle),
                id: spec.name.clone(),
            }),
            (Some(_), BriefObserved::Missing) => {
                writes.insert(spec.name.clone(), write());
            }
            (Some(r), BriefObserved::Brief { hash }) if hash.as_str() == r.content_hash => {
                if hash != spec.content_hash {
                    removes.insert(
                        spec.name.clone(),
                        Step::RemoveGenerated {
                            n: 0,
                            bundle: r.bundle.clone(),
                            name: spec.name.clone(),
                            expect_hash: hash,
                        },
                    );
                    writes.insert(spec.name.clone(), write());
                }
            }
            (Some(_), BriefObserved::Brief { hash }) if hash == spec.content_hash => {
                // Hand-edited to exactly the desired text (or the receipt
                // hash is stale): adopt it. The write step observes the
                // matching content, writes nothing, and refreshes the receipt.
                writes.insert(spec.name.clone(), write_as(true));
            }
            (Some(_), BriefObserved::Brief { .. }) => {
                // Edited by the user and still wanted: leave it, keep the
                // receipt entry, and say so; never block the plan.
                out.warnings.push(
                    Diagnostic::warning(
                        "brief-modified",
                        format!(
                            "{} (the brief of bundle {}) was changed after pouch wrote it; it is left as it is",
                            dest(name),
                            spec.bundle
                        ),
                    )
                    .with_path(dest(name)),
                );
            }
            (Some(_), other) => out.conflicts.push(Conflict::ModifiedAtDest {
                dest: dest(name),
                id: spec.name.clone(),
                found: found_of(&other).to_string(),
            }),
        }
    }
    out.staying = receipted
        .keys()
        .filter(|n| wanted_names.contains(n.as_str()))
        .count();
    // Receipted briefs no longer wanted: remove by the receipt hash. The
    // executor skips (and the receipt releases) one that is missing, edited,
    // or not a brief.
    for (name, r) in &receipted {
        // Already gone from disk: nothing to remove, and no step to show.
        // The receipt entry is released when the next apply finishes.
        if !wanted_names.contains(name.as_str()) && !missing_on_disk(&input.observed, name) {
            removes.insert(
                name.clone(),
                Step::RemoveGenerated {
                    n: 0,
                    bundle: r.bundle.clone(),
                    name: name.clone(),
                    expect_hash: crate::plan::hash_from_text(&r.content_hash),
                },
            );
        }
    }
    out.steps.extend(removes.into_values());
    out.steps.extend(writes.into_values());
    out
}

/// True when the brief was observed and is absent.
fn missing_on_disk(observed: &BTreeMap<String, BriefObserved>, name: &str) -> bool {
    matches!(observed.get(name), Some(BriefObserved::Missing))
}

/// The word `found` carries in a conflict.
fn found_of(o: &BriefObserved) -> &'static str {
    match o {
        BriefObserved::Missing => "missing",
        BriefObserved::Brief { .. } => "directory",
        BriefObserved::Other { found } => found,
    }
}

#[cfg(test)]
mod tests;
