//! Unit and property tests of `plan` (M3 plan, plan.rs specification).

use super::*;
use crate::adapter::Scope;
use crate::catalog::tests_support::catalog_of;
use crate::config::Strategy;
use crate::observe::Shadow;
use crate::resolve::Pin;
use proptest::prelude::*;
use proptest::strategy::Strategy as _;
use std::path::Path;

const REL: &str = ".claude/skills";
const OP: &str = "0000000000001-0000abcd";

fn cat() -> Catalog {
    catalog_of(
        &[
            ("fx", "sdlc/code-review", "code-review"),
            ("fx", "testing/pytest-fixtures", "pytest-fixtures"),
            ("fx", "data/schema-migration", "schema-migration"),
            ("fx", "a/error-handling", "error-handling"),
            ("fx", "b/error-handling", "error-handling"),
            ("hs", "ops/deploy-checklist", "deploy-checklist"),
            ("hs", "api/rest-design", "rest-design"),
            ("hs", "x/alpha", "alpha"),
        ],
        &[],
    )
}

fn header() -> PlanHeader {
    PlanHeader {
        root: PathBuf::from("/p"),
        root_how: RootDetection::Git,
        target: TargetId::ClaudeCode,
        dest_rel: REL.into(),
        strategy: Strategy::Symlink,
        detection_text: None,
        consumers: vec![],
    }
}

fn want(c: &Catalog, id: &str, strategy: Strategy, prov: Vec<Provenance>) -> DesiredEntry {
    let id = SkillId::parse(id).unwrap();
    DesiredEntry {
        name: c.skill(&id).unwrap().name.clone(),
        id,
        strategy,
        pin: Pin::Follow,
        provenance: prov,
    }
}

fn adhoc() -> Vec<Provenance> {
    vec![Provenance::Adhoc]
}

fn observed(entries: &[(&str, ObservedKind)]) -> Observed {
    Observed {
        dest_root: DestRootState::Dir,
        entries: entries
            .iter()
            .map(|(n, k)| (n.to_string(), k.clone()))
            .collect(),
        shadows: vec![],
    }
}

fn empty_receipt() -> Receipt {
    Receipt::empty(Path::new("/p"), TargetId::ClaudeCode)
}

fn run(
    c: &Catalog,
    desired: &[DesiredEntry],
    receipt: &Receipt,
    obs: &Observed,
    carry: &BTreeSet<String>,
    deactivate: bool,
) -> Plan {
    diff(&DiffInput {
        header: header(),
        op_id: OP.into(),
        desired,
        carry_ids: carry,
        receipt,
        observed: obs,
        catalog: c,
        selection_hash: Hash::of_bytes(b"sel"),
        budget_tokens: 100_000,
        release_created_dirs: deactivate,
    })
}

/// Apply every step as Done and return the new receipt.
fn apply_all(before: &Receipt, plan: &Plan) -> Receipt {
    let outcomes: BTreeMap<u32, StepOutcome> = plan
        .steps
        .iter()
        .map(|s| (s.n(), StepOutcome::Done))
        .collect();
    build_receipt(&ReceiptUpdate {
        before,
        dest_rel: REL,
        steps: &plan.steps,
        outcomes: &outcomes,
        now: "T1",
        op_id: OP,
        selection_hash: &plan.selection_hash,
        dirs_created: &[],
        dirs_removed: &[],
    })
}

/// The observation a receipt implies (every entry present and owned).
fn observed_from(r: &Receipt) -> Observed {
    let mut entries = BTreeMap::new();
    for e in r.skill_entries() {
        let k = match e.strategy {
            Strategy::Symlink => ObservedKind::Symlink {
                target: e.source_path.clone(),
                resolves: true,
            },
            Strategy::Copy => ObservedKind::Dir {
                tree_hash: Some(hash_from_text(&e.tree_hash)),
            },
        };
        entries.insert(e.name.clone(), k);
    }
    Observed {
        dest_root: DestRootState::Dir,
        entries,
        shadows: vec![],
    }
}

fn ops(p: &Plan) -> Vec<&'static str> {
    p.steps.iter().map(Step::op).collect()
}

fn none() -> BTreeSet<String> {
    BTreeSet::new()
}

/// A receipt holding `ids` applied with `strategy`.
fn receipt_with(c: &Catalog, ids: &[&str], strategy: Strategy) -> Receipt {
    let desired: Vec<DesiredEntry> = ids.iter().map(|i| want(c, i, strategy, adhoc())).collect();
    let p = run(
        c,
        &desired,
        &empty_receipt(),
        &observed(&[]),
        &none(),
        false,
    );
    apply_all(&empty_receipt(), &p)
}

// ---- classification table ----

#[test]
fn row_no_receipt_missing_is_add() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let p = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
    assert_eq!(ops(&p), vec!["add"]);
    let Step::Add {
        source, tree_hash, ..
    } = &p.steps[0]
    else {
        panic!()
    };
    assert_eq!(source, &PathBuf::from("/fx/sdlc/code-review"));
    assert_eq!(tree_hash, &Hash::of_bytes(b"sdlc/code-review"));
}

#[test]
fn row_no_receipt_present_is_foreign_conflict_naming_both_paths() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    for k in [
        ObservedKind::Dir { tree_hash: None },
        ObservedKind::File,
        ObservedKind::Symlink {
            target: "/x".into(),
            resolves: false,
        },
        ObservedKind::Other,
    ] {
        let p = run(
            &c,
            &d,
            &empty_receipt(),
            &observed(&[("code-review", k)]),
            &none(),
            false,
        );
        assert!(p.steps.is_empty());
        assert_eq!(p.conflicts.len(), 1);
        assert_eq!(p.conflicts[0].code(), "foreign-at-dest");
        let m = p.conflicts[0].message();
        assert!(
            m.contains(".claude/skills/code-review") && m.contains("/fx/sdlc/code-review"),
            "{m}"
        );
        assert!(p.is_blocked());
    }
}

#[test]
fn row_owned_same_is_unchanged_and_provenance_change_is_update() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let obs = observed_from(&r);
    let same = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let p = run(&c, &same, &r, &obs, &none(), false);
    assert!(p.is_empty(), "{:?}", p.steps);
    assert_eq!(p.unchanged.len(), 1);
    let other = vec![want(
        &c,
        "fx:sdlc/code-review",
        Strategy::Symlink,
        vec![Provenance::Bundle("b".into())],
    )];
    let p = run(&c, &other, &r, &obs, &none(), false);
    assert_eq!(ops(&p), vec!["update_provenance"]);
}

#[test]
fn row_owned_strategy_change_is_remove_then_add() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Copy, adhoc())];
    let p = run(&c, &d, &r, &observed_from(&r), &none(), false);
    assert_eq!(ops(&p), vec!["remove", "add"]);
}

#[test]
fn row_receipted_missing_is_repair_add() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let p = run(&c, &d, &r, &observed(&[]), &none(), false);
    assert_eq!(ops(&p), vec!["add"]);
}

#[test]
fn row_receipted_present_not_owned_is_modified_conflict() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let obs = observed(&[(
        "code-review",
        ObservedKind::Symlink {
            target: "/elsewhere".into(),
            resolves: true,
        },
    )]);
    let p = run(&c, &d, &r, &obs, &none(), false);
    assert_eq!(p.conflicts[0].code(), "modified-at-dest");
    assert!(p.steps.is_empty());
}

#[test]
fn undesired_receipt_entries_remove_or_drop() {
    let c = cat();
    let r = receipt_with(
        &c,
        &[
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "hs:x/alpha",
        ],
        Strategy::Copy,
    );
    let mut obs = observed_from(&r);
    obs.entries.remove("pytest-fixtures");
    obs.entries.insert(
        "alpha".into(),
        ObservedKind::Dir {
            tree_hash: Some(Hash::of_bytes(b"edited")),
        },
    );
    let p = run(&c, &[], &r, &obs, &none(), false);
    let got: Vec<(String, String)> = p
        .steps
        .iter()
        .map(|s| (s.op().to_string(), s.name().unwrap().to_string()))
        .collect();
    assert_eq!(
        got,
        vec![
            ("remove".into(), "code-review".into()),
            ("drop_orphan".into(), "alpha".into()),
            ("drop_orphan".into(), "pytest-fixtures".into()),
        ]
    );
    let Step::DropOrphan { reason, .. } = &p.steps[1] else {
        panic!()
    };
    assert_eq!(*reason, OrphanReason::Modified);
    assert!(p.removes_something());
}

#[test]
fn carried_entries_are_never_removed() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let carry: BTreeSet<String> = ["fx:sdlc/code-review".to_string()].into();
    let p = run(&c, &[], &r, &observed_from(&r), &carry, false);
    assert!(p.steps.is_empty());
    assert_eq!(p.carried.len(), 1);
    assert!(
        p.warnings
            .iter()
            .any(|w| w.code == "owned-source-unreachable")
    );
}

#[test]
fn name_reuse_removes_before_adds() {
    let c = cat();
    let r = receipt_with(&c, &["fx:a/error-handling"], Strategy::Symlink);
    let d = vec![want(&c, "fx:b/error-handling", Strategy::Symlink, adhoc())];
    let p = run(&c, &d, &r, &observed_from(&r), &none(), false);
    assert_eq!(ops(&p), vec!["remove", "add"]);
    assert_eq!(p.steps[0].id(), Some("fx:a/error-handling"));
    assert_eq!(p.steps[1].id(), Some("fx:b/error-handling"));
    assert_eq!(p.steps[0].n(), 1);
    assert_eq!(p.steps[1].n(), 2);
}

#[test]
fn invalid_and_duplicate_name_conflicts() {
    let mut c = cat();
    let i = c.by_id[&SkillId::parse("hs:x/alpha").unwrap()];
    c.skills[i].name = "..".into();
    c.reindex();
    let d = vec![
        want(&c, "hs:x/alpha", Strategy::Symlink, adhoc()),
        want(&c, "fx:a/error-handling", Strategy::Symlink, adhoc()),
        want(&c, "fx:b/error-handling", Strategy::Symlink, adhoc()),
        want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc()),
    ];
    let p = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
    let codes: Vec<&str> = p.conflicts.iter().map(Conflict::code).collect();
    assert_eq!(codes, vec!["invalid-name", "duplicate-name"]);
    let Conflict::DuplicateName { ids, .. } = &p.conflicts[1] else {
        panic!()
    };
    assert_eq!(
        ids,
        &vec![
            "fx:a/error-handling".to_string(),
            "fx:b/error-handling".to_string()
        ]
    );
    assert_eq!(ops(&p), vec!["add"], "valid entries still plan");
}

#[test]
fn outside_and_not_directory_roots_give_conflicts_and_no_steps() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let mut obs = observed(&[]);
    obs.dest_root = DestRootState::Outside {
        resolved: "/elsewhere".into(),
    };
    let p = run(&c, &d, &empty_receipt(), &obs, &none(), false);
    assert_eq!(p.conflicts[0].code(), "dest-outside-project");
    assert!(p.steps.is_empty());
    obs.dest_root = DestRootState::NotDir;
    let p = run(&c, &d, &empty_receipt(), &obs, &none(), false);
    assert_eq!(p.conflicts[0].code(), "dest-not-directory");
    assert!(p.steps.is_empty());
}

#[test]
fn estimate_formula_and_over_budget() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    // name "code-review" (11) + description "About code-review." (18) = 29
    // -> ceil(29 / 4) = 8, + 20 = 28.
    let p = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
    assert_eq!(
        p.estimate,
        Estimate {
            skills: 1,
            tokens: 28
        }
    );
    let p2 = diff(&DiffInput {
        header: header(),
        op_id: OP.into(),
        desired: &d,
        carry_ids: &none(),
        receipt: &empty_receipt(),
        observed: &observed(&[]),
        catalog: &c,
        selection_hash: Hash::of_bytes(b"s"),
        budget_tokens: 10,
        release_created_dirs: false,
    });
    assert!(p2.warnings.iter().any(|w| w.code == "catalog-over-budget"));
    assert!(!p.warnings.iter().any(|w| w.code == "catalog-over-budget"));
}

#[test]
fn shadow_warning_only_for_shown_names() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let mut obs = observed(&[]);
    obs.shadows = vec![
        Shadow {
            name: "code-review".into(),
            by: PathBuf::from("/home/.claude/skills/code-review"),
            scope: Scope::User,
        },
        Shadow {
            name: "other".into(),
            by: PathBuf::from("/e/other"),
            scope: Scope::Enterprise,
        },
    ];
    let p = run(&c, &d, &empty_receipt(), &obs, &none(), false);
    let sh: Vec<&Diagnostic> = p.warnings.iter().filter(|w| w.code == "shadowed").collect();
    assert_eq!(sh.len(), 1);
    assert!(sh[0].message.contains("user scope wins"));
}

#[test]
fn dest_dirs_to_create_only_with_an_add() {
    let c = cat();
    let d = vec![want(&c, "fx:sdlc/code-review", Strategy::Symlink, adhoc())];
    let mut obs = observed(&[]);
    obs.dest_root = DestRootState::Missing {
        to_create: vec![".claude".into(), ".claude/skills".into()],
    };
    let p = run(&c, &d, &empty_receipt(), &obs, &none(), false);
    assert_eq!(
        p.dest_dirs_to_create,
        vec![".claude".to_string(), ".claude/skills".to_string()]
    );
    let p = run(&c, &[], &empty_receipt(), &obs, &none(), false);
    assert!(p.dest_dirs_to_create.is_empty());
}

// ---- RemoveDir (G3) ----

#[test]
fn remove_dir_steps_only_for_deactivate_when_nothing_remains() {
    let c = cat();
    let mut r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    r.created_dirs = vec![
        ".claude".into(),
        ".claude/skills".into(),
        "..".into(),
        ".claude/other".into(),
    ];
    let obs = observed_from(&r);
    let p = run(&c, &[], &r, &obs, &none(), true);
    let got: Vec<Value> = p.steps.iter().map(|s| s.to_json(REL)).collect();
    assert_eq!(ops(&p), vec!["remove", "remove_dir", "remove_dir"]);
    assert_eq!(got[1]["dir"], ".claude/skills");
    assert_eq!(got[2]["dir"], ".claude");
    assert_eq!(
        p.warnings
            .iter()
            .filter(|w| w.code == "receipt-entry-ignored")
            .count(),
        2
    );
    assert!(p.removes_something());
    // Not for any other edit.
    let p = run(&c, &[], &r, &obs, &none(), false);
    assert_eq!(ops(&p), vec!["remove"]);
    // Not while a carried entry remains.
    let carry: BTreeSet<String> = ["fx:sdlc/code-review".to_string()].into();
    let p = run(&c, &[], &r, &obs, &carry, true);
    assert!(p.steps.is_empty());
    // A RemoveDir alone is no SES-2 removal.
    let mut r2 = empty_receipt();
    r2.created_dirs = vec![".claude".into()];
    let p = run(&c, &[], &r2, &observed(&[]), &none(), true);
    assert_eq!(ops(&p), vec!["remove_dir"]);
    assert!(!p.removes_something());
}

// ---- receipt view ----

#[test]
fn inconsistent_receipt_entries_are_ignored_and_carried() {
    let c = cat();
    let mut r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    if let Entry::Skill(s) = &mut r.entries[0] {
        s.dest = "../../etc".into();
    }
    let p = run(&c, &[], &r, &observed(&[]), &none(), false);
    assert!(p.steps.is_empty());
    assert!(p.warnings.iter().any(|w| w.code == "receipt-entry-ignored"));
    // And `build_receipt` keeps it untouched.
    let after = apply_all(&r, &p);
    assert_eq!(after.entries, r.entries);
}

// ---- build_receipt ----

#[test]
fn build_receipt_table() {
    let c = cat();
    let mut before = receipt_with(
        &c,
        &[
            "fx:sdlc/code-review",
            "fx:testing/pytest-fixtures",
            "hs:x/alpha",
        ],
        Strategy::Copy,
    );
    before.entries.insert(
        0,
        Entry::Other(serde_json::json!({"kind": "generated", "z": 1})),
    );
    before
        .extra
        .insert("future".into(), serde_json::json!(true));
    before.created_dirs = vec![".claude".into()];
    let steps = vec![
        Step::Remove {
            n: 1,
            id: "fx:sdlc/code-review".into(),
            name: "code-review".into(),
            expect: OwnedExpectation {
                strategy: Strategy::Copy,
                source: "/fx/sdlc/code-review".into(),
                tree_hash: None,
            },
        },
        Step::Remove {
            n: 2,
            id: "fx:testing/pytest-fixtures".into(),
            name: "pytest-fixtures".into(),
            expect: OwnedExpectation {
                strategy: Strategy::Copy,
                source: "/fx".into(),
                tree_hash: None,
            },
        },
        Step::Add {
            n: 3,
            id: "hs:api/rest-design".into(),
            name: "rest-design".into(),
            strategy: Strategy::Copy,
            source: PathBuf::from("/hs/api/rest-design"),
            tree_hash: Hash::of_bytes(b"api/rest-design"),
            provenance: vec![Provenance::Adhoc, Provenance::Bundle("b".into())],
        },
        Step::Add {
            n: 4,
            id: "hs:ops/deploy-checklist".into(),
            name: "deploy-checklist".into(),
            strategy: Strategy::Copy,
            source: PathBuf::from("/hs/ops/deploy-checklist"),
            tree_hash: Hash::of_bytes(b"x"),
            provenance: vec![],
        },
        Step::UpdateProvenance {
            n: 5,
            id: "hs:x/alpha".into(),
            name: "alpha".into(),
            provenance: vec![
                Provenance::Bundle("z".into()),
                Provenance::Bundle("a".into()),
            ],
        },
    ];
    let outcomes: BTreeMap<u32, StepOutcome> = [
        (1, StepOutcome::Done),
        (
            2,
            StepOutcome::Failed {
                code: "io-error".into(),
                message: "m".into(),
            },
        ),
        (3, StepOutcome::Done),
        (4, StepOutcome::Skipped(SkipReason::DestOccupied)),
        (5, StepOutcome::Done),
    ]
    .into();
    let after = build_receipt(&ReceiptUpdate {
        before: &before,
        dest_rel: REL,
        steps: &steps,
        outcomes: &outcomes,
        now: "T2",
        op_id: OP,
        selection_hash: &Hash::of_bytes(b"s2"),
        dirs_created: &[".claude".into(), ".claude/skills".into()],
        dirs_removed: &[".claude".into()],
    });
    let names: Vec<String> = after.skill_entries().map(|s| s.name.clone()).collect();
    assert_eq!(names, vec!["alpha", "pytest-fixtures", "rest-design"]);
    assert!(matches!(after.entries.last(), Some(Entry::Other(_))));
    assert_eq!(after.extra.get("future"), Some(&serde_json::json!(true)));
    assert_eq!(after.created_dirs, vec![".claude/skills".to_string()]);
    assert_eq!(after.last_op_id.as_deref(), Some(OP));
    let alpha = after.skill_entries().find(|s| s.name == "alpha").unwrap();
    assert_eq!(
        alpha.provenance,
        vec![
            Provenance::Bundle("a".into()),
            Provenance::Bundle("z".into())
        ]
    );
    assert_eq!(alpha.applied_at, "T1", "update keeps applied_at");
    let rest = after
        .skill_entries()
        .find(|s| s.name == "rest-design")
        .unwrap();
    assert_eq!(rest.applied_at, "T2");
    assert_eq!(
        rest.provenance,
        vec![Provenance::Bundle("b".into()), Provenance::Adhoc]
    );
    // Skips that release ownership drop the entry; others keep it.
    for (reason, kept) in [
        (SkipReason::Missing, false),
        (SkipReason::Modified, false),
        (SkipReason::Foreign, false),
        (SkipReason::CrossDevice, true),
    ] {
        let o: BTreeMap<u32, StepOutcome> = [(1, StepOutcome::Skipped(reason))].into();
        let a = build_receipt(&ReceiptUpdate {
            before: &before,
            dest_rel: REL,
            steps: &steps[..1],
            outcomes: &o,
            now: "T3",
            op_id: OP,
            selection_hash: &Hash::of_bytes(b"s"),
            dirs_created: &[],
            dirs_removed: &[],
        });
        assert_eq!(
            a.skill_entries().any(|s| s.name == "code-review"),
            kept,
            "{reason:?}"
        );
    }
}

#[test]
fn plan_json_shapes() {
    let c = cat();
    let r = receipt_with(&c, &["fx:sdlc/code-review"], Strategy::Symlink);
    let d = vec![want(
        &c,
        "fx:testing/pytest-fixtures",
        Strategy::Symlink,
        adhoc(),
    )];
    let p = run(&c, &d, &r, &observed_from(&r), &none(), false);
    let v = p.to_json();
    assert_eq!(v["steps"][0]["op"], "remove");
    assert_eq!(v["steps"][0]["dest"], ".claude/skills/code-review");
    assert_eq!(v["steps"][1]["op"], "add");
    assert_eq!(v["steps"][1]["source"], "/fx/testing/pytest-fixtures");
    assert_eq!(v["steps"][1]["provenance"][0]["adhoc"], true);
    assert_eq!(v["estimate"]["estimate"], true);
    for k in [
        "steps",
        "unchanged",
        "carried",
        "conflicts",
        "warnings",
        "estimate",
    ] {
        assert!(v.get(k).is_some(), "{k}");
    }
}

// ---- property tests ----

fn has_text(entries: &[Entry], text: &str) -> bool {
    entries.iter().any(|a| {
        let got = a.to_value().to_string();
        got == text
    })
}

const POOL: &[&str] = &[
    "fx:sdlc/code-review",
    "fx:testing/pytest-fixtures",
    "fx:data/schema-migration",
    "fx:a/error-handling",
    "fx:b/error-handling",
    "hs:ops/deploy-checklist",
    "hs:api/rest-design",
    "hs:x/alpha",
];

fn prov_from(bits: u8) -> Vec<Provenance> {
    let mut v = Vec::new();
    if bits & 1 != 0 {
        v.push(Provenance::Bundle("a".into()));
    }
    if bits & 2 != 0 {
        v.push(Provenance::Bundle("b".into()));
    }
    if bits & 4 != 0 || v.is_empty() {
        v.push(Provenance::Adhoc);
    }
    v
}

prop_compose! {
    fn desired_set()(picks in proptest::sample::subsequence((0..POOL.len()).collect::<Vec<_>>(), 0..=POOL.len()).prop_shuffle(),
                     provs in proptest::collection::vec(0u8..8, POOL.len()),
                     copy in any::<bool>()) -> Vec<DesiredEntry> {
        let c = cat();
        let strategy = if copy { Strategy::Copy } else { Strategy::Symlink };
        picks.into_iter().map(|i| want(&c, POOL[i], strategy, prov_from(provs[i]))).collect()
    }
}

proptest! {
    // The duplicate-name pair makes many inputs conflict; those are
    // rejected (the properties hold for conflict-free plans only).
    #![proptest_config(ProptestConfig {
        max_global_rejects: 1 << 16,
        ..ProptestConfig::default()
    })]

    #[test]
    fn diff_is_idempotent_after_a_full_apply(d in desired_set()) {
        let c = cat();
        let p1 = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
        prop_assume!(p1.conflicts.is_empty());
        let r = apply_all(&empty_receipt(), &p1);
        let p2 = run(&c, &d, &r, &observed_from(&r), &none(), false);
        prop_assert!(p2.steps.is_empty(), "{:?}", p2.steps);
        prop_assert!(p2.conflicts.is_empty());
        prop_assert_eq!(p2.unchanged.len(), d.len());
    }

    #[test]
    fn unchanged_entries_are_carried_forward_byte_identical(
        first in desired_set(), second in desired_set(), extra in any::<bool>())
    {
        let c = cat();
        let p1 = run(&c, &first, &empty_receipt(), &observed(&[]), &none(), false);
        prop_assume!(p1.conflicts.is_empty());
        let mut r = apply_all(&empty_receipt(), &p1);
        r.entries.push(Entry::Other(serde_json::json!({"kind": "generated", "b": 2, "a": 1})));
        if extra && let Some(Entry::Skill(s)) = r.entries.first_mut() {
            s.extra.insert("future_field".into(), serde_json::json!([1, 2]));
        }
        let p2 = run(&c, &second, &r, &observed_from(&r), &none(), false);
        prop_assume!(p2.conflicts.is_empty());
        let after = apply_all(&r, &p2);
        let touched: BTreeSet<&str> = p2.steps.iter().filter_map(Step::name).collect();
        for e in &r.entries {
            match e {
                Entry::Skill(s) if !touched.contains(s.name.as_str()) => {
                    let text = s.to_value().to_string();
                    prop_assert!(
                        has_text(&after.entries, &text),
                        "lost or changed: {}", text
                    );
                }
                Entry::Other(v) => {
                    let text = v.to_string();
                    prop_assert!(has_text(&after.entries, &text));
                }
                _ => {}
            }
        }
    }

    #[test]
    fn diff_never_touches_unreceipted_or_foreign_names(
        first in desired_set(), second in desired_set(), foreign in 0usize..POOL.len(), drop in 0usize..POOL.len())
    {
        let c = cat();
        let p1 = run(&c, &first, &empty_receipt(), &observed(&[]), &none(), false);
        prop_assume!(p1.conflicts.is_empty());
        let r = apply_all(&empty_receipt(), &p1);
        let mut obs = observed_from(&r);
        // A hand deletion and a planted foreign name.
        let drop_name = c.skill(&SkillId::parse(POOL[drop]).unwrap()).unwrap().name.clone();
        obs.entries.remove(&drop_name);
        let fname = c.skill(&SkillId::parse(POOL[foreign]).unwrap()).unwrap().name.clone();
        let receipted: BTreeSet<String> = r.skill_entries().map(|s| s.name.clone()).collect();
        if !receipted.contains(&fname) {
            obs.entries.insert(fname.clone(), ObservedKind::Dir { tree_hash: None });
        }
        let p2 = run(&c, &second, &r, &obs, &none(), false);
        prop_assume!(p2.conflicts.is_empty());
        for s in &p2.steps {
            match s {
                Step::Remove { name, .. } | Step::DropOrphan { name, .. } | Step::UpdateProvenance { name, .. } => {
                    prop_assert!(receipted.contains(name), "{} not receipted", name);
                }
                Step::Add { name, .. } => {
                    prop_assert!(
                        obs.kind(name) == ObservedKind::Missing
                            || p2.steps.iter().any(|x| matches!(x, Step::Remove { name: n, .. } if n == name)),
                        "add onto an occupied name {}", name
                    );
                    prop_assert!(!(name == &fname && !receipted.contains(&fname)));
                }
                Step::RemoveDir { .. } => {}
            }
        }
    }

    #[test]
    fn diff_is_deterministic(d in desired_set()) {
        let c = cat();
        let a = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
        let b = run(&c, &d, &empty_receipt(), &observed(&[]), &none(), false);
        prop_assert_eq!(a, b);
    }

    #[test]
    fn steps_are_ordered_removes_droporphans_adds_updates(first in desired_set(), second in desired_set()) {
        let c = cat();
        let p1 = run(&c, &first, &empty_receipt(), &observed(&[]), &none(), false);
        prop_assume!(p1.conflicts.is_empty());
        let r = apply_all(&empty_receipt(), &p1);
        let p2 = run(&c, &second, &r, &observed_from(&r), &none(), false);
        let rank = |s: &Step| match s {
            Step::Remove { .. } => 0,
            Step::DropOrphan { .. } => 1,
            Step::Add { .. } => 2,
            Step::UpdateProvenance { .. } => 3,
            Step::RemoveDir { .. } => 4,
        };
        let ranks: Vec<u8> = p2.steps.iter().map(rank).collect();
        let mut sorted = ranks.clone();
        sorted.sort();
        prop_assert_eq!(ranks, sorted);
        for (i, s) in p2.steps.iter().enumerate() {
            prop_assert_eq!(s.n() as usize, i + 1);
        }
    }
}
