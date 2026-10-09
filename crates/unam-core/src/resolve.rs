//! Resolution (spec 4.6, 2.3, 3): a thin layer over the catalog `lookup`
//! (M2 plan D8) plus resolution of a target selection to a flat desired set
//! with provenance (plan D5, P4 to P6).
//!
//! Resolution is never fail-fast (EDG-3): every problem is collected. Order:
//! bundles in listed order, then `add`; the first occurrence of an ID fixes
//! its position; `remove` subtracts by resolved `SkillId`.

pub use crate::catalog::{Lookup, canonical_ref_text, edit_distance};

use crate::bundle::{Bundle, BundleSet, slugify};
use crate::catalog::{Catalog, SkillId, SkillRef};
use crate::config::Strategy;
use crate::error::{Diagnostic, Error, Result};
use crate::project::TargetSelection;
use serde::{Serialize, Serializer};
use std::collections::HashMap;

/// A failed lookup of a parsed reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveError {
    NotFound {
        candidates: Vec<String>,
        unreachable: Vec<String>,
    },
    Ambiguous(Vec<String>),
}

/// Resolve one reference to its `SkillId` (spec 4.6).
pub fn resolve_ref(r: &SkillRef, catalog: &Catalog) -> std::result::Result<SkillId, ResolveError> {
    match catalog.lookup(r) {
        Lookup::Found(s) => Ok(s.id.clone()),
        Lookup::NotFound {
            candidates,
            unreachable,
        } => Err(ResolveError::NotFound {
            candidates,
            unreachable,
        }),
        Lookup::Ambiguous(ids) => Err(ResolveError::Ambiguous(ids)),
    }
}

/// Parse and resolve `text`; errors as `Catalog::resolve_text` builds them.
pub fn resolve_text(catalog: &Catalog, text: &str) -> Result<SkillId> {
    catalog.resolve_text(text).map(|s| s.id.clone())
}

/// Version pin of a desired entry. Milestone M2 has `Follow` only.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Pin {
    Follow,
}

/// Why an entry is desired. Serializes as `{"bundle": slug}` or
/// `{"adhoc": true}`; serialized into receipts since M3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provenance {
    Bundle(String),
    Adhoc,
}

impl Serialize for Provenance {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut m = s.serialize_map(Some(1))?;
        match self {
            Provenance::Bundle(slug) => m.serialize_entry("bundle", slug)?,
            Provenance::Adhoc => m.serialize_entry("adhoc", &true)?,
        }
        m.end()
    }
}

impl<'de> serde::Deserialize<'de> for Provenance {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        use serde::de::Error as _;
        let map = serde_json::Map::<String, serde_json::Value>::deserialize(d)?;
        if map.len() != 1 {
            return Err(D::Error::custom("a provenance record has exactly one key"));
        }
        match (map.get("bundle"), map.get("adhoc")) {
            (Some(serde_json::Value::String(slug)), None) => Ok(Provenance::Bundle(slug.clone())),
            (None, Some(serde_json::Value::Bool(true))) => Ok(Provenance::Adhoc),
            _ => Err(D::Error::custom(
                "expected {\"bundle\": <slug>} or {\"adhoc\": true}",
            )),
        }
    }
}

/// Canonical provenance (plan P6): bundle records sorted bytewise by slug
/// and deduplicated, then `Adhoc` last when present.
pub fn canonical_provenance(records: &[Provenance]) -> Vec<Provenance> {
    let mut slugs: Vec<&str> = records
        .iter()
        .filter_map(|p| match p {
            Provenance::Bundle(s) => Some(s.as_str()),
            Provenance::Adhoc => None,
        })
        .collect();
    slugs.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    slugs.dedup();
    let mut out: Vec<Provenance> = slugs
        .into_iter()
        .map(|s| Provenance::Bundle(s.to_string()))
        .collect();
    if records.contains(&Provenance::Adhoc) {
        out.push(Provenance::Adhoc);
    }
    out
}

/// One resolved, desired skill.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredEntry {
    pub id: SkillId,
    pub name: String,
    pub strategy: Strategy,
    pub pin: Pin,
    /// Every provenance record, deduplicated, in first-seen order.
    pub provenance: Vec<Provenance>,
}

impl DesiredEntry {
    /// The effective provenance: the last record (PRD 5.3).
    pub fn winner(&self) -> Option<&Provenance> {
        self.provenance.last()
    }
}

/// Kind of a resolution problem (plan P4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    /// The reference text does not parse (exit 2).
    Invalid,
    /// No skill matches (exit 4).
    Unresolved,
    /// Several skills match (exit 5).
    Ambiguous,
    /// A bundle named in the selection is not loaded (exit 4).
    BundleMissing,
}

/// Where a problem came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    Bundle(String),
    Add,
    Remove,
}

impl Origin {
    fn describe(&self) -> String {
        match self {
            Origin::Bundle(slug) => format!("from bundle `{slug}`"),
            Origin::Add => "in add".to_string(),
            Origin::Remove => "in remove".to_string(),
        }
    }
}

/// A reference that did not resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub kind: ProblemKind,
    /// Canonical reference text (the bundle slug for `BundleMissing`).
    pub reference: String,
    pub message: String,
    pub candidates: Vec<String>,
    pub unreachable: Vec<String>,
    pub origins: Vec<Origin>,
}

impl Problem {
    /// The error a command that needs a complete set fails with.
    pub fn to_error(&self) -> Error {
        match self.kind {
            ProblemKind::Invalid => Error::InvalidRef {
                reference: self.reference.clone(),
                reason: self.message.clone(),
            },
            ProblemKind::Unresolved => Error::NotFound {
                reference: self.reference.clone(),
                message: self.message.clone(),
                candidates: self.candidates.clone(),
            },
            ProblemKind::Ambiguous => Error::Ambiguous {
                reference: self.reference.clone(),
                candidates: self.candidates.clone(),
            },
            ProblemKind::BundleMissing => Error::BundleNotFound {
                slug: self.reference.clone(),
            },
        }
    }

    /// Stable diagnostic code (plan P2).
    pub fn code(&self) -> &'static str {
        match self.kind {
            ProblemKind::Invalid => "ref-invalid",
            ProblemKind::Unresolved => "ref-not-found",
            ProblemKind::Ambiguous => "ref-ambiguous",
            ProblemKind::BundleMissing => "bundle-missing",
        }
    }

    /// The problem as an error diagnostic naming its origins.
    pub fn to_diagnostic(&self) -> Diagnostic {
        let origins: Vec<String> = self.origins.iter().map(Origin::describe).collect();
        let mut message = self.message.clone();
        if !self.candidates.is_empty() {
            message.push_str(&format!(" (candidates: {})", self.candidates.join(", ")));
        }
        if !origins.is_empty() {
            message.push_str(&format!(" [{}]", origins.join(", ")));
        }
        let mut d = Diagnostic::error(self.code(), message);
        if let Some(r) = self.to_error().remedy() {
            d = d.with_remedy(r);
        }
        d
    }
}

/// The resolved desired set of one target.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Resolution {
    pub resolved: Vec<DesiredEntry>,
    pub problems: Vec<Problem>,
}

impl Resolution {
    pub fn is_clean(&self) -> bool {
        self.problems.is_empty()
    }

    /// The error of the first problem (the command's exit code).
    pub fn first_error(&self) -> Option<Error> {
        self.problems.first().map(Problem::to_error)
    }
}

/// Look up `text` and turn a miss into a `Problem` with `origin`.
fn lookup_text(
    catalog: &Catalog,
    text: &str,
    origin: Origin,
) -> std::result::Result<SkillId, Box<Problem>> {
    let reference = canonical_ref_text(text);
    let problem = |kind, message: String, candidates, unreachable| {
        Box::new(Problem {
            kind,
            reference: reference.clone(),
            message,
            candidates,
            unreachable,
            origins: vec![origin.clone()],
        })
    };
    let r = match SkillRef::parse(text) {
        Ok(r) => r,
        Err(e) => {
            let reason = match &e {
                Error::InvalidRef { reason, .. } => reason.clone(),
                other => other.to_string(),
            };
            return Err(problem(
                ProblemKind::Invalid,
                format!("invalid reference `{reference}`: {reason}"),
                Vec::new(),
                Vec::new(),
            ));
        }
    };
    match resolve_ref(&r, catalog) {
        Ok(id) => Ok(id),
        Err(ResolveError::NotFound {
            candidates,
            unreachable,
        }) => Err(problem(
            ProblemKind::Unresolved,
            Catalog::not_found_message(text, &unreachable),
            candidates,
            unreachable,
        )),
        Err(ResolveError::Ambiguous(ids)) => Err(problem(
            ProblemKind::Ambiguous,
            format!(
                "reference `{reference}` is ambiguous: {} skills match",
                ids.len()
            ),
            ids,
            Vec::new(),
        )),
    }
}

struct Builder<'a> {
    catalog: &'a Catalog,
    strategy: Strategy,
    resolved: Vec<DesiredEntry>,
    position: HashMap<SkillId, usize>,
    problems: Vec<Problem>,
}

impl Builder<'_> {
    fn push_problem(&mut self, p: Problem) {
        if let Some(existing) = self
            .problems
            .iter_mut()
            .find(|q| q.kind == p.kind && q.reference == p.reference)
        {
            for o in p.origins {
                if !existing.origins.contains(&o) {
                    existing.origins.push(o);
                }
            }
        } else {
            self.problems.push(p);
        }
    }

    fn add(&mut self, text: &str, provenance: Provenance, origin: Origin) {
        match lookup_text(self.catalog, text, origin) {
            Err(p) => self.push_problem(*p),
            Ok(id) => match self.position.get(&id) {
                Some(&i) => {
                    let e = &mut self.resolved[i];
                    if !e.provenance.contains(&provenance) {
                        e.provenance.push(provenance);
                    }
                }
                None => {
                    let name = self
                        .catalog
                        .by_id
                        .get(&id)
                        .map(|&i| self.catalog.skills[i].name.clone())
                        .unwrap_or_default();
                    self.position.insert(id.clone(), self.resolved.len());
                    self.resolved.push(DesiredEntry {
                        id,
                        name,
                        strategy: self.strategy,
                        pin: Pin::Follow,
                        provenance: vec![provenance],
                    });
                }
            },
        }
    }

    fn remove(&mut self, text: &str) {
        match lookup_text(self.catalog, text, Origin::Remove) {
            Ok(id) => {
                if self.position.remove(&id).is_some() {
                    self.resolved.retain(|e| e.id != id);
                    self.position = self
                        .resolved
                        .iter()
                        .enumerate()
                        .map(|(i, e)| (e.id.clone(), i))
                        .collect();
                }
            }
            Err(p) => {
                let before = self.problems.len();
                self.problems
                    .retain(|q| q.kind == ProblemKind::BundleMissing || q.reference != p.reference);
                if self.problems.len() == before {
                    self.push_problem(*p);
                }
            }
        }
    }
}

/// Resolve one target's selection to a flat desired set (plan D5, P4 to P6).
pub fn resolve_selection(
    target: &TargetSelection,
    bundles: &BundleSet,
    catalog: &Catalog,
    default_strategy: Strategy,
) -> Resolution {
    let mut b = Builder {
        catalog,
        strategy: target.strategy.unwrap_or(default_strategy),
        resolved: Vec::new(),
        position: HashMap::new(),
        problems: Vec::new(),
    };
    for text in &target.bundles {
        let slug = slugify(text);
        let found = if slug.is_empty() {
            None
        } else {
            bundles.get(&slug)
        };
        match found {
            None => {
                let reference = if slug.is_empty() {
                    text.trim().to_string()
                } else {
                    slug.clone()
                };
                let message = if slug.is_empty() {
                    format!("bundle name `{}` has an empty slug", text.trim())
                } else {
                    format!("bundle `{slug}` is not loaded")
                };
                b.push_problem(Problem {
                    kind: ProblemKind::BundleMissing,
                    reference: reference.clone(),
                    message,
                    candidates: Vec::new(),
                    unreachable: Vec::new(),
                    origins: vec![Origin::Bundle(reference)],
                });
            }
            Some(bundle) => {
                for entry in &bundle.skills {
                    b.add(
                        entry,
                        Provenance::Bundle(bundle.slug.clone()),
                        Origin::Bundle(bundle.slug.clone()),
                    );
                }
            }
        }
    }
    for text in &target.add {
        b.add(text, Provenance::Adhoc, Origin::Add);
    }
    for text in &target.remove {
        b.remove(text);
    }
    Resolution {
        resolved: b.resolved,
        problems: b.problems,
    }
}

/// One bundle entry with its resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryResolution {
    pub entry: String,
    pub outcome: std::result::Result<SkillId, Problem>,
}

/// Resolve every entry of `bundle` (used by `bundle show` and removal).
pub fn resolve_entries(bundle: &Bundle, catalog: &Catalog) -> Vec<EntryResolution> {
    bundle
        .skills
        .iter()
        .map(|entry| EntryResolution {
            entry: entry.clone(),
            outcome: lookup_text(catalog, entry, Origin::Bundle(bundle.slug.clone()))
                .map_err(|p| *p),
        })
        .collect()
}

/// Sorted slugs of bundles with at least one entry resolving to `id`.
pub fn bundles_referencing(bundles: &BundleSet, catalog: &Catalog, id: &SkillId) -> Vec<String> {
    let mut out: Vec<String> = bundles
        .bundles
        .iter()
        .filter(|b| {
            resolve_entries(b, catalog)
                .iter()
                .any(|e| e.outcome.as_ref().ok() == Some(id))
        })
        .map(|b| b.slug.clone())
        .collect();
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::BundleMetadata;
    use crate::catalog::tests_support::catalog_of;
    use std::path::PathBuf;

    /// Duplicates across categories (`error-handling` twice in `fx`) and
    /// across sources (`schema-migration` in `fx` and `hs`); `hs` ranks
    /// after `fx`; `gone` is unreachable.
    fn cat() -> Catalog {
        let mut c = catalog_of(
            &[
                ("fx", "sdlc/code-review", "code-review"),
                ("fx", "testing/pytest-fixtures", "pytest-fixtures"),
                ("fx", "alpha-root-skill", "alpha-root-skill"),
                ("fx", "data/schema-migration", "schema-migration"),
                (
                    "fx",
                    "engineering/backend/api/error-handling",
                    "error-handling",
                ),
                (
                    "fx",
                    "engineering/frontend/error-handling",
                    "error-handling",
                ),
                ("hs", "data/schema-migration", "schema-migration"),
                ("hs", "ops/deploy-checklist", "deploy-checklist"),
                ("hs", "api/rest-design", "rest-design"),
            ],
            &["gone"],
        );
        for s in &mut c.sources {
            s.precedence = match s.alias.as_str() {
                "hs" => 20,
                "fx" => 10,
                _ => 30,
            };
        }
        c
    }

    fn bundle(slug: &str, skills: &[&str]) -> Bundle {
        Bundle {
            slug: slug.into(),
            name: slug.into(),
            description: String::new(),
            skills: skills.iter().map(|s| s.to_string()).collect(),
            instruction: None,
            metadata: BundleMetadata::default(),
            path: PathBuf::from(format!("/b/{slug}.yaml")),
            file_name: format!("{slug}.yaml"),
        }
    }

    fn set(bundles: Vec<Bundle>) -> BundleSet {
        BundleSet {
            bundles,
            diagnostics: vec![],
        }
    }

    fn sel(bundles: &[&str], add: &[&str], remove: &[&str]) -> TargetSelection {
        let v = |x: &[&str]| x.iter().map(|s| s.to_string()).collect();
        TargetSelection {
            strategy: None,
            bundles: v(bundles),
            add: v(add),
            remove: v(remove),
        }
    }

    fn run(t: &TargetSelection, s: &BundleSet) -> Resolution {
        resolve_selection(t, s, &cat(), Strategy::Symlink)
    }

    fn ids(r: &Resolution) -> Vec<String> {
        r.resolved.iter().map(|e| e.id.to_string()).collect()
    }

    fn std_set() -> BundleSet {
        set(vec![
            bundle("backend-dev", &["fx:code-review", "pytest-fixtures"]),
            bundle("ops", &["hs:ops/deploy-checklist", "fx:sdlc/code-review"]),
        ])
    }

    #[test]
    fn order_bundles_then_add_first_occurrence_fixes_position() {
        let r = run(
            &sel(
                &["ops", "backend-dev"],
                &["hs:api/rest-design", "fx:pytest-fixtures"],
                &[],
            ),
            &std_set(),
        );
        assert!(r.is_clean());
        assert_eq!(
            ids(&r),
            vec![
                "hs:ops/deploy-checklist",
                "fx:sdlc/code-review",
                "fx:testing/pytest-fixtures",
                "hs:api/rest-design"
            ]
        );
        assert_eq!(r.resolved[0].name, "deploy-checklist");
        assert_eq!(r.resolved[0].pin, Pin::Follow);
    }

    #[test]
    fn provenance_accumulates_dedupes_and_winner_is_last() {
        let r = run(
            &sel(
                &["backend-dev", "ops", "ops"],
                &["fx:sdlc/code-review", "code-review"],
                &[],
            ),
            &std_set(),
        );
        let cr = &r.resolved[0];
        assert_eq!(cr.id.to_string(), "fx:sdlc/code-review");
        assert_eq!(
            cr.provenance,
            vec![
                Provenance::Bundle("backend-dev".into()),
                Provenance::Bundle("ops".into()),
                Provenance::Adhoc
            ]
        );
        assert_eq!(cr.winner(), Some(&Provenance::Adhoc));
        assert_eq!(
            serde_json::to_value(&cr.provenance).unwrap(),
            serde_json::json!([{"bundle": "backend-dev"}, {"bundle": "ops"}, {"adhoc": true}])
        );
    }

    #[test]
    fn remove_by_equivalent_spelling_drops_all_provenance() {
        let r = run(
            &sel(
                &["backend-dev"],
                &["fx:code-review"],
                &["fx:sdlc/code-review/"],
            ),
            &std_set(),
        );
        assert_eq!(ids(&r), vec!["fx:testing/pytest-fixtures"]);
        assert!(r.is_clean());
        // Re-adding after removal is not possible within one selection: remove wins.
    }

    #[test]
    fn remove_of_unselected_valid_id_is_a_no_op() {
        let r = run(
            &sel(&["backend-dev"], &[], &["hs:api/rest-design"]),
            &std_set(),
        );
        assert_eq!(
            ids(&r),
            vec!["fx:sdlc/code-review", "fx:testing/pytest-fixtures"]
        );
        assert!(r.is_clean());
    }

    #[test]
    fn overlapping_bundles_edg7() {
        let r = run(&sel(&["ops"], &[], &[]), &std_set());
        let cr = r
            .resolved
            .iter()
            .find(|e| e.id.to_string() == "fx:sdlc/code-review")
            .unwrap();
        assert_eq!(cr.provenance, vec![Provenance::Bundle("ops".into())]);
    }

    #[test]
    fn problems_collected_not_fail_fast_edg3() {
        let s = set(vec![bundle(
            "mixed",
            &["code-reviw", "pytest-fixtures", "error-handling", "fx:"],
        )]);
        let r = run(&sel(&["mixed"], &["schema-migration"], &[]), &s);
        assert_eq!(ids(&r), vec!["fx:testing/pytest-fixtures"]);
        let kinds: Vec<ProblemKind> = r.problems.iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ProblemKind::Unresolved,
                ProblemKind::Ambiguous,
                ProblemKind::Invalid,
                ProblemKind::Ambiguous
            ]
        );
        assert_eq!(r.problems[0].candidates, vec!["fx:sdlc/code-review"]);
        assert_eq!(r.problems[0].origins, vec![Origin::Bundle("mixed".into())]);
        // Ambiguous across sources: candidates in source-precedence order.
        assert_eq!(
            r.problems[3].candidates,
            vec!["fx:data/schema-migration", "hs:data/schema-migration"]
        );
        assert_eq!(r.problems[3].origins, vec![Origin::Add]);
        assert_eq!(r.first_error().unwrap().exit_code(), 4);
    }

    #[test]
    fn problems_deduplicated_with_origins_accumulated() {
        let s = set(vec![bundle("a", &["nope"]), bundle("b", &["nope"])]);
        let r = run(&sel(&["a", "b"], &["nope"], &[]), &s);
        assert_eq!(r.problems.len(), 1);
        assert_eq!(
            r.problems[0].origins,
            vec![
                Origin::Bundle("a".into()),
                Origin::Bundle("b".into()),
                Origin::Add
            ]
        );
    }

    #[test]
    fn unresolved_remove_cancels_the_same_dangling_add() {
        let r = run(
            &sel(
                &[],
                &["fx:sdlc/gone-skill", "code-review"],
                &["fx:sdlc/gone-skill/"],
            ),
            &std_set(),
        );
        assert!(r.is_clean(), "{:?}", r.problems);
        assert_eq!(ids(&r), vec!["fx:sdlc/code-review"]);
    }

    #[test]
    fn unresolved_remove_without_dangling_entry_is_its_own_problem() {
        let r = run(&sel(&[], &["code-review"], &["nope"]), &std_set());
        assert_eq!(r.problems.len(), 1);
        assert_eq!(r.problems[0].kind, ProblemKind::Unresolved);
        assert_eq!(r.problems[0].origins, vec![Origin::Remove]);
    }

    #[test]
    fn missing_bundle_slugified_name_and_empty_slug() {
        let s = set(vec![bundle("backend-dev", &["code-review"])]);
        let r = run(&sel(&["Backend Dev", "nope", "!!!", "No Pe"], &[], &[]), &s);
        assert_eq!(ids(&r), vec!["fx:sdlc/code-review"]);
        let refs: Vec<(&str, ProblemKind)> = r
            .problems
            .iter()
            .map(|p| (p.reference.as_str(), p.kind))
            .collect();
        assert_eq!(
            refs,
            vec![
                ("nope", ProblemKind::BundleMissing),
                ("!!!", ProblemKind::BundleMissing),
                ("no-pe", ProblemKind::BundleMissing)
            ]
        );
        assert_eq!(r.problems[0].to_error().code(), "bundle-not-found");
    }

    #[test]
    fn invalid_entry_text_and_unreachable_note() {
        let r = run(&sel(&[], &["fx:", "gone:x/y"], &[]), &std_set());
        assert_eq!(r.problems[0].kind, ProblemKind::Invalid);
        assert_eq!(r.problems[0].reference, "fx:");
        assert_eq!(r.problems[1].kind, ProblemKind::Unresolved);
        assert_eq!(r.problems[1].unreachable, vec!["gone"]);
        assert!(
            r.problems[1].message.contains("`gone` is unreachable"),
            "{}",
            r.problems[1].message
        );
    }

    #[test]
    fn strategy_inheritance_and_override() {
        let mut t = sel(&[], &["code-review"], &[]);
        let r = resolve_selection(&t, &std_set(), &cat(), Strategy::Copy);
        assert_eq!(r.resolved[0].strategy, Strategy::Copy);
        t.strategy = Some(Strategy::Symlink);
        let r = resolve_selection(&t, &std_set(), &cat(), Strategy::Copy);
        assert_eq!(r.resolved[0].strategy, Strategy::Symlink);
    }

    #[test]
    fn empty_selection() {
        let r = run(&TargetSelection::default(), &std_set());
        assert!(r.resolved.is_empty() && r.is_clean());
        assert!(r.first_error().is_none());
    }

    #[test]
    fn problem_error_and_diagnostic_mapping() {
        let s = set(vec![bundle("b", &["error-handling"])]);
        let r = run(&sel(&["b", "missing"], &["fx:", "nope"], &[]), &s);
        let got: Vec<(&str, &str, i32)> = r
            .problems
            .iter()
            .map(|p| (p.code(), p.to_error().code(), p.to_error().exit_code()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("ref-ambiguous", "ambiguous", 5),
                ("bundle-missing", "bundle-not-found", 4),
                ("ref-invalid", "invalid-ref", 2),
                ("ref-not-found", "not-found", 4)
            ]
        );
        let d = r.problems[0].to_diagnostic();
        assert_eq!(d.code, "ref-ambiguous");
        assert_eq!(d.severity, crate::error::Severity::Error);
        assert!(d.message.contains("from bundle `b`"), "{}", d.message);
        assert!(r.problems[2].to_diagnostic().message.contains("in add"));
        assert_eq!(r.first_error().unwrap().code(), "ambiguous");
    }

    #[test]
    fn resolve_entries_statuses_and_bundles_referencing() {
        let c = cat();
        let b = bundle("x", &["code-review", "error-handling", "nope", "fx:"]);
        let out = resolve_entries(&b, &c);
        assert_eq!(
            out[0].outcome.as_ref().unwrap().to_string(),
            "fx:sdlc/code-review"
        );
        assert_eq!(
            out[1].outcome.as_ref().unwrap_err().kind,
            ProblemKind::Ambiguous
        );
        assert_eq!(
            out[2].outcome.as_ref().unwrap_err().kind,
            ProblemKind::Unresolved
        );
        assert_eq!(
            out[3].outcome.as_ref().unwrap_err().kind,
            ProblemKind::Invalid
        );
        let s = set(vec![
            bundle("zeta", &["fx:sdlc/code-review"]),
            bundle("alpha", &["code-review"]),
            bundle("none", &["error-handling"]),
        ]);
        let id = resolve_text(&c, "code-review").unwrap();
        assert_eq!(bundles_referencing(&s, &c, &id), vec!["alpha", "zeta"]);
        let other = resolve_text(&c, "hs:api/rest-design").unwrap();
        assert!(bundles_referencing(&s, &c, &other).is_empty());
    }

    #[test]
    fn provenance_round_trip_and_unknown_shape_rejected() {
        for p in [Provenance::Bundle("b".into()), Provenance::Adhoc] {
            let text = serde_json::to_string(&p).unwrap();
            let back: Provenance = serde_json::from_str(&text).unwrap();
            assert_eq!(back, p);
        }
        for bad in [
            r#"{"adhoc": false}"#,
            r#"{"bundle": 1}"#,
            r#"{"other": "x"}"#,
            r#"{"bundle": "a", "adhoc": true}"#,
            r#"{}"#,
            r#""bundle""#,
        ] {
            assert!(serde_json::from_str::<Provenance>(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn canonical_provenance_orders_dedupes_and_puts_adhoc_last() {
        let b = |s: &str| Provenance::Bundle(s.into());
        assert_eq!(
            canonical_provenance(&[Provenance::Adhoc, b("z"), b("a"), b("z")]),
            vec![b("a"), b("z"), Provenance::Adhoc]
        );
        assert_eq!(canonical_provenance(&[]), Vec::<Provenance>::new());
        assert_eq!(
            canonical_provenance(&[Provenance::Adhoc, Provenance::Adhoc]),
            vec![Provenance::Adhoc]
        );
    }

    #[test]
    fn resolve_ref_maps_lookup() {
        let c = cat();
        let r = SkillRef::parse("schema-migration").unwrap();
        assert_eq!(
            resolve_ref(&r, &c),
            Err(ResolveError::Ambiguous(vec![
                "fx:data/schema-migration".into(),
                "hs:data/schema-migration".into()
            ]))
        );
        let r = SkillRef::parse("hs:deploy-checklist").unwrap();
        assert_eq!(
            resolve_ref(&r, &c).unwrap().to_string(),
            "hs:ops/deploy-checklist"
        );
        assert_eq!(resolve_text(&c, "nope").unwrap_err().exit_code(), 4);
    }
}
