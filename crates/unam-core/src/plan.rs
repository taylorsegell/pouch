//! The pure plan (spec 4.8; M3 plan P6, P7, P8, G3): classify the desired
//! set against the receipt and the observation into steps, conflicts, and
//! warnings. `diff` never touches the filesystem. `build_receipt` (also
//! pure) turns a receipt plus step outcomes into the next receipt.

use crate::catalog::{Catalog, SkillId};
use crate::config::Strategy;
use crate::error::Diagnostic;
use crate::fsops::{ObservedKind, OwnedExpectation, SkipReason, dest_prefixes, validate_dest_name};
use crate::hash::Hash;
use crate::observe::{DestRootState, Observed};
use crate::project::{RootDetection, TargetId};
use crate::resolve::{DesiredEntry, Provenance, canonical_provenance};
use crate::state::{Entry, Receipt, SkillEntry, strategy_text};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// A `Hash` from its stored text (receipts and journals hold text).
pub fn hash_from_text(text: &str) -> Hash {
    serde_json::from_value(Value::String(text.to_string()))
        .unwrap_or_else(|_| Hash::of_bytes(text.as_bytes()))
}

/// Why an orphaned receipt entry is dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrphanReason {
    Missing,
    Modified,
}

impl OrphanReason {
    pub fn as_str(self) -> &'static str {
        match self {
            OrphanReason::Missing => "missing",
            OrphanReason::Modified => "modified",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "missing" => Some(OrphanReason::Missing),
            "modified" => Some(OrphanReason::Modified),
            _ => None,
        }
    }
}

/// One plan step.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Add {
        n: u32,
        id: String,
        name: String,
        strategy: Strategy,
        source: PathBuf,
        tree_hash: Hash,
        provenance: Vec<Provenance>,
    },
    Remove {
        n: u32,
        id: String,
        name: String,
        expect: OwnedExpectation,
    },
    DropOrphan {
        n: u32,
        id: String,
        name: String,
        reason: OrphanReason,
    },
    UpdateProvenance {
        n: u32,
        id: String,
        name: String,
        provenance: Vec<Provenance>,
    },
    /// Remove a directory this tool created, only when empty (G3).
    RemoveDir { n: u32, dir: String },
}

impl Step {
    pub fn n(&self) -> u32 {
        match self {
            Step::Add { n, .. }
            | Step::Remove { n, .. }
            | Step::DropOrphan { n, .. }
            | Step::UpdateProvenance { n, .. }
            | Step::RemoveDir { n, .. } => *n,
        }
    }

    fn set_n(&mut self, value: u32) {
        match self {
            Step::Add { n, .. }
            | Step::Remove { n, .. }
            | Step::DropOrphan { n, .. }
            | Step::UpdateProvenance { n, .. }
            | Step::RemoveDir { n, .. } => *n = value,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Step::Add { name, .. }
            | Step::Remove { name, .. }
            | Step::DropOrphan { name, .. }
            | Step::UpdateProvenance { name, .. } => Some(name),
            Step::RemoveDir { .. } => None,
        }
    }

    pub fn id(&self) -> Option<&str> {
        match self {
            Step::Add { id, .. }
            | Step::Remove { id, .. }
            | Step::DropOrphan { id, .. }
            | Step::UpdateProvenance { id, .. } => Some(id),
            Step::RemoveDir { .. } => None,
        }
    }

    pub fn op(&self) -> &'static str {
        match self {
            Step::Add { .. } => "add",
            Step::Remove { .. } => "remove",
            Step::DropOrphan { .. } => "drop_orphan",
            Step::UpdateProvenance { .. } => "update_provenance",
            Step::RemoveDir { .. } => "remove_dir",
        }
    }

    /// The `--json` step object.
    pub fn to_json(&self, dest_rel: &str) -> Value {
        let dest = |name: &str| format!("{dest_rel}/{name}");
        match self {
            Step::Add {
                n,
                id,
                name,
                strategy,
                source,
                provenance,
                ..
            } => json!({
                "n": n, "op": "add", "id": id, "name": name, "dest": dest(name),
                "strategy": strategy_text(*strategy),
                "source": source.to_string_lossy(),
                "provenance": provenance,
            }),
            Step::Remove {
                n,
                id,
                name,
                expect,
            } => json!({
                "n": n, "op": "remove", "id": id, "name": name, "dest": dest(name),
                "strategy": strategy_text(expect.strategy),
            }),
            Step::DropOrphan {
                n,
                id,
                name,
                reason,
            } => json!({
                "n": n, "op": "drop_orphan", "id": id, "name": name, "dest": dest(name),
                "reason": reason.as_str(),
            }),
            Step::UpdateProvenance {
                n,
                id,
                name,
                provenance,
            } => json!({
                "n": n, "op": "update_provenance", "id": id, "name": name, "dest": dest(name),
                "provenance": provenance,
            }),
            Step::RemoveDir { n, dir } => json!({"n": n, "op": "remove_dir", "dir": dir}),
        }
    }
}

/// How a step ended (G8b: defined here, used by `apply`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepOutcome {
    Done,
    Failed { code: String, message: String },
    Skipped(SkipReason),
}

/// One step result for reports and `--json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepResult {
    pub n: u32,
    pub op: &'static str,
    pub id: Option<String>,
    pub name: Option<String>,
    pub dest: Option<String>,
    pub outcome: StepOutcome,
}

/// A blocking conflict (every conflict blocks; exit 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Conflict {
    ForeignAtDest {
        dest: String,
        found: String,
        source: String,
        id: String,
    },
    ModifiedAtDest {
        dest: String,
        id: String,
        found: String,
    },
    DuplicateName {
        name: String,
        ids: Vec<String>,
    },
    InvalidName {
        id: String,
        name: String,
        reason: String,
    },
    DestOutsideProject {
        dest_root: String,
        resolved: String,
    },
    DestNotDirectory {
        dest_root: String,
    },
}

impl Conflict {
    pub fn code(&self) -> &'static str {
        match self {
            Conflict::ForeignAtDest { .. } => "foreign-at-dest",
            Conflict::ModifiedAtDest { .. } => "modified-at-dest",
            Conflict::DuplicateName { .. } => "duplicate-name",
            Conflict::InvalidName { .. } => "invalid-name",
            Conflict::DestOutsideProject { .. } => "dest-outside-project",
            Conflict::DestNotDirectory { .. } => "dest-not-directory",
        }
    }

    /// One line naming the data.
    pub fn message(&self) -> String {
        match self {
            Conflict::ForeignAtDest {
                dest,
                found,
                source,
                id,
            } => format!(
                "{dest} already exists ({found}, not managed by unam); {id} from {source} cannot be placed there"
            ),
            Conflict::ModifiedAtDest { dest, id, found } => format!(
                "{dest} ({id}) was changed after unam placed it (found {found}); it is no longer proven tool-owned"
            ),
            Conflict::DuplicateName { name, ids } => {
                format!(
                    "{} skills share the name {name}: {}",
                    ids.len(),
                    ids.join(", ")
                )
            }
            Conflict::InvalidName { id, name, reason } => {
                format!("{id} has the name {name:?}, which cannot be a destination: {reason}")
            }
            Conflict::DestOutsideProject {
                dest_root,
                resolved,
            } => format!("{dest_root} resolves to {resolved}, outside the project"),
            Conflict::DestNotDirectory { dest_root } => {
                format!("{dest_root} exists and is not a directory")
            }
        }
    }

    pub fn to_json(&self) -> Value {
        let mut v = match self {
            Conflict::ForeignAtDest {
                dest,
                found,
                source,
                id,
            } => json!({"dest": dest, "found": found, "source": source, "id": id}),
            Conflict::ModifiedAtDest { dest, id, found } => {
                json!({"dest": dest, "id": id, "found": found})
            }
            Conflict::DuplicateName { name, ids } => json!({"name": name, "ids": ids}),
            Conflict::InvalidName { id, name, reason } => {
                json!({"id": id, "name": name, "reason": reason})
            }
            Conflict::DestOutsideProject {
                dest_root,
                resolved,
            } => json!({"dest_root": dest_root, "resolved": resolved}),
            Conflict::DestNotDirectory { dest_root } => json!({"dest_root": dest_root}),
        };
        let mut out = serde_json::Map::new();
        out.insert("code".into(), json!(self.code()));
        out.insert("message".into(), json!(self.message()));
        if let Some(o) = v.as_object_mut() {
            for (k, x) in std::mem::take(o) {
                out.insert(k, x);
            }
        }
        Value::Object(out)
    }
}

/// The plan header (spec 5 rendering).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanHeader {
    pub root: PathBuf,
    pub root_how: RootDetection,
    pub target: TargetId,
    pub dest_rel: String,
    pub strategy: Strategy,
    /// `Some("not detected; .claude/skills will be created")` when absent.
    pub detection_text: Option<String>,
    pub consumers: Vec<String>,
}

/// The catalog estimate (always labeled an estimate).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Estimate {
    pub skills: usize,
    pub tokens: u64,
}

/// An entry listed as unchanged or carried.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEntry {
    pub id: String,
    pub name: String,
    pub dest: String,
}

impl PlanEntry {
    fn to_json(&self) -> Value {
        json!({"id": self.id, "name": self.name, "dest": self.dest})
    }
}

/// A plan.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub header: PlanHeader,
    pub op_id: String,
    pub steps: Vec<Step>,
    pub unchanged: Vec<PlanEntry>,
    pub carried: Vec<PlanEntry>,
    pub conflicts: Vec<Conflict>,
    pub warnings: Vec<Diagnostic>,
    pub estimate: Estimate,
    pub selection_hash: Hash,
    pub dest_dirs_to_create: Vec<String>,
}

impl Plan {
    /// No steps and no conflicts.
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.conflicts.is_empty()
    }

    pub fn is_blocked(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Any Remove or DropOrphan (a RemoveDir does not count).
    pub fn removes_something(&self) -> bool {
        self.steps
            .iter()
            .any(|s| matches!(s, Step::Remove { .. } | Step::DropOrphan { .. }))
    }

    /// The `plan` object of the CLI Contract.
    pub fn to_json(&self) -> Value {
        json!({
            "steps": self.steps.iter().map(|s| s.to_json(&self.header.dest_rel)).collect::<Vec<_>>(),
            "unchanged": self.unchanged.iter().map(PlanEntry::to_json).collect::<Vec<_>>(),
            "carried": self.carried.iter().map(PlanEntry::to_json).collect::<Vec<_>>(),
            "conflicts": self.conflicts.iter().map(Conflict::to_json).collect::<Vec<_>>(),
            "warnings": self.warnings,
            "estimate": {"skills": self.estimate.skills, "tokens": self.estimate.tokens, "estimate": true},
        })
    }
}

/// Input of `diff`.
#[derive(Debug, Clone)]
pub struct DiffInput<'a> {
    pub header: PlanHeader,
    pub op_id: String,
    pub desired: &'a [DesiredEntry],
    /// Receipt skill IDs kept without classification (owned, unreachable).
    pub carry_ids: &'a BTreeSet<String>,
    pub receipt: &'a Receipt,
    pub observed: &'a Observed,
    pub catalog: &'a Catalog,
    pub selection_hash: Hash,
    pub budget_tokens: u32,
    /// True only for `deactivate` (G3).
    pub release_created_dirs: bool,
}

fn found_text(k: &ObservedKind) -> &'static str {
    match k {
        ObservedKind::Missing => "missing",
        ObservedKind::Symlink { .. } => "symlink",
        ObservedKind::Dir { .. } => "directory",
        ObservedKind::File => "file",
        ObservedKind::Other => "other",
    }
}

/// True when the observation matches the receipt entry (ownership proof).
pub fn is_owned(r: &SkillEntry, obs: &ObservedKind) -> bool {
    match r.strategy {
        Strategy::Symlink => {
            matches!(obs, ObservedKind::Symlink { target, .. } if *target == r.source_path)
        }
        Strategy::Copy => {
            matches!(obs, ObservedKind::Dir { tree_hash: Some(h) } if h.as_str() == r.tree_hash)
        }
    }
}

/// The expectation that proves ownership of a receipt entry.
pub fn expectation_of(r: &SkillEntry) -> OwnedExpectation {
    OwnedExpectation {
        strategy: r.strategy,
        source: r.source_path.clone(),
        tree_hash: Some(hash_from_text(&r.tree_hash)),
    }
}

fn ceil_div4(n: usize) -> u64 {
    (n as u64).div_ceil(4)
}

/// The pure diff (spec 4.8; the classification table of P7).
pub fn diff(input: &DiffInput<'_>) -> Plan {
    let dest_rel = input.header.dest_rel.as_str();
    let dest = |name: &str| format!("{dest_rel}/{name}");
    let mut conflicts: Vec<Conflict> = Vec::new();
    let mut warnings: Vec<Diagnostic> = Vec::new();

    // 1. Names.
    let mut valid: Vec<&DesiredEntry> = Vec::new();
    for d in input.desired {
        match validate_dest_name(&d.name) {
            Ok(()) => valid.push(d),
            Err(reason) => conflicts.push(Conflict::InvalidName {
                id: d.id.to_string(),
                name: d.name.clone(),
                reason,
            }),
        }
    }
    let mut by_name: BTreeMap<&str, Vec<&DesiredEntry>> = BTreeMap::new();
    for d in &valid {
        by_name.entry(d.name.as_str()).or_default().push(d);
    }
    let mut dup_reported: BTreeSet<&str> = BTreeSet::new();
    let mut remaining: Vec<&DesiredEntry> = Vec::new();
    for d in &valid {
        let group = &by_name[d.name.as_str()];
        if group.len() > 1 {
            if dup_reported.insert(d.name.as_str()) {
                conflicts.push(Conflict::DuplicateName {
                    name: d.name.clone(),
                    ids: group.iter().map(|x| x.id.to_string()).collect(),
                });
            }
        } else {
            remaining.push(d);
        }
    }

    // 2. Root state.
    let root_blocked = match &input.observed.dest_root {
        DestRootState::Outside { resolved } => {
            conflicts.push(Conflict::DestOutsideProject {
                dest_root: dest_rel.to_string(),
                resolved: resolved.clone(),
            });
            true
        }
        DestRootState::NotDir => {
            conflicts.push(Conflict::DestNotDirectory {
                dest_root: dest_rel.to_string(),
            });
            true
        }
        _ => false,
    };

    // 3. Receipt view.
    let mut view: BTreeMap<&str, &SkillEntry> = BTreeMap::new();
    let mut inconsistent = 0usize;
    for e in input.receipt.skill_entries() {
        if !e.consistent(dest_rel) {
            inconsistent += 1;
            warnings.push(Diagnostic::warning(
                "receipt-entry-ignored",
                format!(
                    "receipt entry {} ({}) does not match the destination; it is carried and never touched",
                    e.skill_id, e.dest
                ),
            ));
            continue;
        }
        if view.contains_key(e.name.as_str()) {
            inconsistent += 1;
            warnings.push(Diagnostic::warning(
                "receipt-entry-ignored",
                format!(
                    "receipt entry {} repeats the destination {}; the second entry is ignored",
                    e.skill_id, e.dest
                ),
            ));
            continue;
        }
        view.insert(e.name.as_str(), e);
    }

    // 4. Classification.
    let mut removes: Vec<Step> = Vec::new();
    let mut drops: Vec<Step> = Vec::new();
    let mut adds: Vec<Step> = Vec::new();
    let mut updates: Vec<Step> = Vec::new();
    let mut unchanged: Vec<PlanEntry> = Vec::new();
    let mut matched: BTreeSet<&str> = BTreeSet::new();
    let mut shown: BTreeSet<String> = BTreeSet::new();
    let make_add = |d: &DesiredEntry| -> Option<Step> {
        let skill = input.catalog.skill(&d.id)?;
        let source = input.catalog.abs_path(&d.id)?;
        Some(Step::Add {
            n: 0,
            id: d.id.to_string(),
            name: d.name.clone(),
            strategy: d.strategy,
            source,
            tree_hash: skill.tree_hash.clone(),
            provenance: canonical_provenance(&d.provenance),
        })
    };
    for d in &remaining {
        let name = d.name.as_str();
        let obs = input.observed.kind(name);
        let id = d.id.to_string();
        let r = view.get(name).copied();
        if r.is_some() {
            matched.insert(name);
        }
        let want_prov = canonical_provenance(&d.provenance);
        match r {
            None => {
                if obs == ObservedKind::Missing {
                    if let Some(s) = make_add(d) {
                        adds.push(s);
                        shown.insert(d.name.clone());
                    }
                } else {
                    conflicts.push(Conflict::ForeignAtDest {
                        dest: dest(name),
                        found: found_text(&obs).to_string(),
                        source: input
                            .catalog
                            .abs_path(&d.id)
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        id,
                    });
                }
            }
            Some(r) => {
                if obs == ObservedKind::Missing {
                    if let Some(s) = make_add(d) {
                        adds.push(s);
                        shown.insert(d.name.clone());
                    }
                } else if is_owned(r, &obs) {
                    if r.skill_id == id && r.strategy == d.strategy {
                        if canonical_provenance(&r.provenance) == want_prov {
                            unchanged.push(PlanEntry {
                                id: id.clone(),
                                name: d.name.clone(),
                                dest: dest(name),
                            });
                            if r.strategy == Strategy::Symlink
                                && let ObservedKind::Symlink {
                                    resolves: false, ..
                                } = &obs
                                && let Some(live) = input.catalog.abs_path(&d.id)
                                && live.to_string_lossy() != r.source_path
                            {
                                warnings.push(Diagnostic::info(
                                    "source-moved",
                                    format!(
                                        "{} links to {}, which no longer resolves; the source now lives at {}",
                                        dest(name),
                                        r.source_path,
                                        live.display()
                                    ),
                                ));
                            }
                        } else {
                            updates.push(Step::UpdateProvenance {
                                n: 0,
                                id: id.clone(),
                                name: d.name.clone(),
                                provenance: want_prov,
                            });
                        }
                        shown.insert(d.name.clone());
                    } else {
                        removes.push(Step::Remove {
                            n: 0,
                            id: r.skill_id.clone(),
                            name: r.name.clone(),
                            expect: expectation_of(r),
                        });
                        if let Some(s) = make_add(d) {
                            adds.push(s);
                            shown.insert(d.name.clone());
                        }
                    }
                } else {
                    conflicts.push(Conflict::ModifiedAtDest {
                        dest: dest(name),
                        id,
                        found: found_text(&obs).to_string(),
                    });
                }
            }
        }
    }

    // 5. Receipt entries no longer desired.
    let mut carried: Vec<PlanEntry> = Vec::new();
    let mut remaining_valid = 0usize;
    for (name, r) in &view {
        if matched.contains(name) {
            remaining_valid += 1;
            continue;
        }
        if input.carry_ids.contains(&r.skill_id) {
            carried.push(PlanEntry {
                id: r.skill_id.clone(),
                name: r.name.clone(),
                dest: r.dest.clone(),
            });
            warnings.push(Diagnostic::warning(
                "owned-source-unreachable",
                format!(
                    "{} ({}) is still selected but its source is unreachable; it is kept as it is",
                    r.dest, r.skill_id
                ),
            ));
            remaining_valid += 1;
            continue;
        }
        let obs = input.observed.kind(name);
        if obs == ObservedKind::Missing {
            drops.push(Step::DropOrphan {
                n: 0,
                id: r.skill_id.clone(),
                name: r.name.clone(),
                reason: OrphanReason::Missing,
            });
        } else if is_owned(r, &obs) {
            removes.push(Step::Remove {
                n: 0,
                id: r.skill_id.clone(),
                name: r.name.clone(),
                expect: expectation_of(r),
            });
        } else {
            drops.push(Step::DropOrphan {
                n: 0,
                id: r.skill_id.clone(),
                name: r.name.clone(),
                reason: OrphanReason::Modified,
            });
        }
    }

    // 6. Order and number.
    removes.sort_by(|a, b| {
        a.name()
            .unwrap_or("")
            .as_bytes()
            .cmp(b.name().unwrap_or("").as_bytes())
    });
    drops.sort_by(|a, b| {
        a.name()
            .unwrap_or("")
            .as_bytes()
            .cmp(b.name().unwrap_or("").as_bytes())
    });
    let mut steps: Vec<Step> = Vec::new();
    steps.extend(removes);
    steps.extend(drops);
    steps.extend(adds);
    steps.extend(updates);

    // 10. Created directories (deactivate only).
    if input.release_created_dirs {
        let prefixes = dest_prefixes(dest_rel);
        let mut valid_dirs: Vec<&String> = Vec::new();
        for d in &input.receipt.created_dirs {
            if prefixes.contains(d) {
                valid_dirs.push(d);
            } else {
                warnings.push(Diagnostic::warning(
                    "receipt-entry-ignored",
                    format!("created directory claim {d:?} is not a destination prefix; ignored"),
                ));
            }
        }
        if remaining_valid + inconsistent == 0 {
            for d in valid_dirs.into_iter().rev() {
                steps.push(Step::RemoveDir {
                    n: 0,
                    dir: d.clone(),
                });
            }
        }
    }
    if root_blocked {
        steps.clear();
    }
    for (i, s) in steps.iter_mut().enumerate() {
        s.set_n(u32::try_from(i + 1).unwrap_or(u32::MAX));
    }

    // 7. Warnings: shadows, budget.
    for sh in &input.observed.shadows {
        if shown.contains(&sh.name) {
            warnings.push(
                Diagnostic::warning(
                    "shadowed",
                    format!(
                        "{} is shadowed by {} ({} scope wins)",
                        sh.name,
                        sh.by.display(),
                        sh.scope.as_str()
                    ),
                )
                .with_path(sh.by.to_string_lossy().into_owned()),
            );
        }
    }

    // 8. Estimate.
    let mut tokens: u64 = 0;
    for d in input.desired {
        let desc = input
            .catalog
            .skill(&d.id)
            .map(|s| s.description.len())
            .unwrap_or(0);
        tokens += ceil_div4(d.name.len() + desc) + 20;
    }
    for c in &carried {
        tokens += ceil_div4(c.name.len()) + 20;
    }
    let estimate = Estimate {
        skills: input.desired.len() + carried.len(),
        tokens,
    };
    if tokens > u64::from(input.budget_tokens) {
        warnings.push(Diagnostic::warning(
            "catalog-over-budget",
            format!(
                "the estimated catalog is ~{tokens} tokens, over the budget of {} (estimate)",
                input.budget_tokens
            ),
        ));
    }

    // 9. Directories to create.
    let has_add = steps.iter().any(|s| matches!(s, Step::Add { .. }));
    let dest_dirs_to_create = match &input.observed.dest_root {
        DestRootState::Missing { to_create } if has_add => to_create.clone(),
        _ => Vec::new(),
    };

    Plan {
        header: input.header.clone(),
        op_id: input.op_id.clone(),
        steps,
        unchanged,
        carried,
        conflicts,
        warnings,
        estimate,
        selection_hash: input.selection_hash.clone(),
        dest_dirs_to_create,
    }
}

/// Input of `build_receipt`.
#[derive(Debug, Clone, Copy)]
pub struct ReceiptUpdate<'a> {
    pub before: &'a Receipt,
    pub dest_rel: &'a str,
    pub steps: &'a [Step],
    pub outcomes: &'a BTreeMap<u32, StepOutcome>,
    pub now: &'a str,
    pub op_id: &'a str,
    pub selection_hash: &'a Hash,
    pub dirs_created: &'a [String],
    pub dirs_removed: &'a [String],
}

/// Build the next receipt from `before` and the step outcomes (pure).
pub fn build_receipt(u: &ReceiptUpdate<'_>) -> Receipt {
    let ReceiptUpdate {
        before,
        dest_rel,
        steps,
        outcomes,
        now,
        op_id,
        selection_hash,
        dirs_created,
        dirs_removed,
    } = *u;
    let mut entries: Vec<Entry> = before.entries.clone();
    let find = |entries: &Vec<Entry>, name: &str| -> Option<usize> {
        entries.iter().position(|e| match e {
            Entry::Skill(s) => s.name == name && s.consistent(dest_rel),
            Entry::Other(_) => false,
        })
    };
    for step in steps {
        let Some(outcome) = outcomes.get(&step.n()) else {
            continue;
        };
        match step {
            Step::Remove { name, .. } => {
                let release = match outcome {
                    StepOutcome::Done => true,
                    StepOutcome::Skipped(r) => matches!(
                        r,
                        SkipReason::Missing | SkipReason::Modified | SkipReason::Foreign
                    ),
                    StepOutcome::Failed { .. } => false,
                };
                if release && let Some(i) = find(&entries, name) {
                    entries.remove(i);
                }
            }
            Step::DropOrphan { name, .. } => {
                if *outcome == StepOutcome::Done
                    && let Some(i) = find(&entries, name)
                {
                    entries.remove(i);
                }
            }
            Step::Add {
                id,
                name,
                strategy,
                source,
                tree_hash,
                provenance,
                ..
            } => {
                if *outcome == StepOutcome::Done {
                    if let Some(i) = find(&entries, name) {
                        entries.remove(i);
                    }
                    entries.push(Entry::Skill(Box::new(SkillEntry {
                        skill_id: id.clone(),
                        name: name.clone(),
                        dest: format!("{dest_rel}/{name}"),
                        strategy: *strategy,
                        source_path: source.to_string_lossy().into_owned(),
                        tree_hash: tree_hash.to_string(),
                        git_commit: Value::Null,
                        pin: "follow".into(),
                        provenance: canonical_provenance(provenance),
                        applied_at: now.to_string(),
                        extra: serde_json::Map::new(),
                    })));
                }
            }
            Step::UpdateProvenance {
                name, provenance, ..
            } => {
                if *outcome == StepOutcome::Done
                    && let Some(i) = find(&entries, name)
                    && let Entry::Skill(s) = &mut entries[i]
                {
                    s.provenance = canonical_provenance(provenance);
                }
            }
            Step::RemoveDir { .. } => {}
        }
    }
    let (mut skills, others): (Vec<Entry>, Vec<Entry>) = entries
        .into_iter()
        .partition(|e| matches!(e, Entry::Skill(_)));
    skills.sort_by(|a, b| match (a, b) {
        (Entry::Skill(x), Entry::Skill(y)) => x.dest.as_bytes().cmp(y.dest.as_bytes()),
        _ => std::cmp::Ordering::Equal,
    });
    skills.extend(others);
    let mut created_dirs: Vec<String> = before.created_dirs.clone();
    for d in dirs_created {
        if !created_dirs.contains(d) {
            created_dirs.push(d.clone());
        }
    }
    created_dirs.retain(|d| !dirs_removed.contains(d));
    Receipt {
        version: 1,
        project_root: before.project_root.clone(),
        target: before.target.clone(),
        applied_at: now.to_string(),
        selection_hash: selection_hash.to_string(),
        last_op_id: Some(op_id.to_string()),
        created_dirs,
        entries: skills,
        extra: before.extra.clone(),
    }
}

/// The skill ID of a desired entry as text (helper for callers).
pub fn id_text(id: &SkillId) -> String {
    id.to_string()
}

#[cfg(test)]
mod tests;
