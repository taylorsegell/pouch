//! Unit tests of `generated` (M5 plan, Test Plan: `generated.rs` unit and
//! `plan::diff` generated cases). Written red-first.

use super::*;
use crate::bundle::{Bundle, BundleMetadata};
use crate::catalog::frontmatter::parse_skill_md;
use crate::project::{TargetId, TargetSelection};
use crate::state::{Entry, GeneratedEntry};
use std::path::PathBuf;

const REL: &str = ".agents/skills";

fn bundle(slug: &str, name: &str, instruction: Option<&str>, opt: Option<bool>) -> Bundle {
    Bundle {
        slug: slug.into(),
        name: name.into(),
        description: "Load 1 skills as a bundle".to_string(),
        skills: vec!["code-review".into()],
        instruction: instruction.map(str::to_string),
        metadata: BundleMetadata {
            version: None,
            materialize_instruction: opt,
        },
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

fn tsel(bundles: &[&str]) -> TargetSelection {
    TargetSelection {
        bundles: bundles.iter().map(|s| s.to_string()).collect(),
        ..TargetSelection::default()
    }
}

fn spec_of(slug: &str, instruction: &str) -> GeneratedSpec {
    let b = set(vec![bundle(slug, slug, Some(instruction), None)]);
    wanted(&tsel(&[slug]), &b).remove(0)
}

fn entry_for(spec: &GeneratedSpec) -> GeneratedEntry {
    GeneratedEntry {
        bundle: spec.bundle.clone(),
        name: spec.name.clone(),
        dest: format!("{REL}/{}", spec.name),
        content_hash: spec.content_hash.to_string(),
        applied_at: "T0".into(),
    }
}

fn receipt_of(entries: &[GeneratedEntry]) -> Receipt {
    let mut r = Receipt::empty(Path::new("/p"), TargetId::AgentsNeutral);
    r.entries = entries.iter().map(|e| Entry::Other(e.to_value())).collect();
    r
}

fn input(wanted: Vec<GeneratedSpec>, obs: &[(&str, BriefObserved)]) -> GeneratedInput {
    GeneratedInput {
        wanted,
        observed: obs
            .iter()
            .map(|(n, o)| (n.to_string(), o.clone()))
            .collect(),
    }
}

fn brief_with(hash: &Hash) -> BriefObserved {
    BriefObserved::Brief { hash: hash.clone() }
}

fn ops(d: &GeneratedDiff) -> Vec<(&'static str, String)> {
    d.steps
        .iter()
        .map(|s| (s.op(), s.name().unwrap_or("").to_string()))
        .collect()
}

fn none_skills() -> BTreeMap<String, String> {
    BTreeMap::new()
}

// ---- content ----

#[test]
fn brief_name_comes_from_the_slug() {
    assert_eq!(brief_name("backend-dev"), "backend-dev-brief");
    assert_eq!(spec_of("backend-dev", "Do it.").name, "backend-dev-brief");
}

#[test]
fn content_parses_back_and_the_body_is_the_trimmed_instruction() {
    let s = spec_of("backend-dev", "  \n Review first.\n\nThen test.  \n\n");
    let parsed = parse_skill_md(s.content.as_bytes(), "backend-dev-brief").unwrap();
    assert_eq!(parsed.name, "backend-dev-brief");
    assert!(!parsed.description.contains('\n'));
    assert!(!parsed.description.is_empty());
    assert!(
        s.content.ends_with("Review first.\n\nThen test.\n"),
        "{}",
        s.content
    );
    assert!(
        s.content.starts_with("---\nname: backend-dev-brief\n"),
        "{}",
        s.content
    );
}

#[test]
fn frontmatter_is_built_with_the_serializer_not_string_concatenation() {
    // A bundle name full of YAML metacharacters and a first line that would
    // break a naive `description: {text}`.
    let b = set(vec![bundle(
        "odd",
        "He said: \"hi\" # not a comment: [x]",
        Some("key: value # tail\nsecond line"),
        None,
    )]);
    let s = wanted(&tsel(&["odd"]), &b).remove(0);
    let parsed = parse_skill_md(s.content.as_bytes(), "odd-brief").unwrap();
    assert_eq!(parsed.name, "odd-brief");
    assert!(
        parsed
            .description
            .contains("He said: \"hi\" # not a comment: [x]")
    );
    assert!(parsed.description.contains("key: value # tail"));
    assert!(!parsed.description.contains("second line"));
}

#[test]
fn special_characters_in_the_instruction_survive_in_the_body() {
    let text = "Use `---` carefully\n---\nand unicode: caf\u{e9} \u{65e5}\u{672c} \u{1f600}\n";
    let s = spec_of("fence", text);
    let (front, body) = s
        .content
        .strip_prefix("---\n")
        .and_then(|r| r.split_once("\n---\n"))
        .unwrap();
    assert!(front.starts_with("name: fence-brief"));
    assert!(
        body.contains("Use `---` carefully\n---\nand unicode: caf\u{e9}"),
        "{body}"
    );
    parse_skill_md(s.content.as_bytes(), "fence-brief").unwrap();
}

#[test]
fn control_characters_never_reach_the_description() {
    let b = set(vec![bundle("c", "n", Some("line\u{7}one\u{1b}[31m"), None)]);
    let s = wanted(&tsel(&["c"]), &b).remove(0);
    let parsed = parse_skill_md(s.content.as_bytes(), "c-brief").unwrap();
    assert!(
        !parsed.description.chars().any(char::is_control),
        "{:?}",
        parsed.description
    );
}

#[test]
fn the_hash_is_blake3_of_the_exact_bytes() {
    let s = spec_of("backend-dev", "Do it.");
    assert_eq!(s.content_hash, Hash::of_bytes(s.content.as_bytes()));
    assert!(s.content_hash.as_str().starts_with("blake3:"));
    // Same input, same bytes.
    assert_eq!(s, spec_of("backend-dev", "Do it."));
    assert_ne!(
        s.content_hash,
        spec_of("backend-dev", "Do it differently.").content_hash
    );
}

// ---- opt-out and selection rules (M5-9) ----

#[test]
fn opt_out_rules() {
    let b = set(vec![
        bundle("on-default", "On", Some("A."), None),
        bundle("on-true", "On", Some("B."), Some(true)),
        bundle("off", "Off", Some("C."), Some(false)),
        bundle("blank", "Blank", None, None),
    ]);
    let names: Vec<String> = wanted(&tsel(&["on-default", "on-true", "off", "blank"]), &b)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(names, vec!["on-default-brief", "on-true-brief"]);
}

#[test]
fn only_bundles_named_in_the_selection_get_a_brief_deduped_and_sorted() {
    let b = set(vec![
        bundle("zeta", "Z", Some("Z."), None),
        bundle("alpha", "A", Some("A."), None),
        bundle("unused", "U", Some("U."), None),
    ]);
    let got: Vec<String> = wanted(&tsel(&["zeta", "alpha", "Zeta", "missing"]), &b)
        .into_iter()
        .map(|s| s.name)
        .collect();
    assert_eq!(got, vec!["alpha-brief", "zeta-brief"]);
    assert!(wanted(&TargetSelection::default(), &b).is_empty());
}

// ---- observation ----

#[test]
fn observation_missing_match_and_edited() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path();
    let dest = root.join(REL);
    std::fs::create_dir_all(&dest).unwrap();
    let s = spec_of("backend-dev", "Do it.");
    std::fs::create_dir_all(dest.join(&s.name)).unwrap();
    std::fs::write(dest.join(&s.name).join("SKILL.md"), &s.content).unwrap();
    std::fs::create_dir_all(dest.join("edited-brief")).unwrap();
    std::fs::write(dest.join("edited-brief/SKILL.md"), "changed").unwrap();
    std::fs::create_dir_all(dest.join("extra-brief")).unwrap();
    std::fs::write(dest.join("extra-brief/SKILL.md"), "x").unwrap();
    std::fs::write(dest.join("extra-brief/notes.md"), "y").unwrap();
    std::fs::write(dest.join("file-brief"), "z").unwrap();
    let names: BTreeSet<String> = [
        s.name.as_str(),
        "edited-brief",
        "extra-brief",
        "file-brief",
        "absent-brief",
        "..",
    ]
    .iter()
    .map(|x| x.to_string())
    .collect();
    let got = observe_briefs(root, REL, &DestRootState::Dir, &names).unwrap();
    assert_eq!(got[&s.name], brief_with(&s.content_hash));
    assert_eq!(got["edited-brief"], brief_with(&Hash::of_bytes(b"changed")));
    assert_eq!(
        got["extra-brief"],
        BriefObserved::Other { found: "directory" }
    );
    assert_eq!(got["file-brief"], BriefObserved::Other { found: "file" });
    assert_eq!(got["absent-brief"], BriefObserved::Missing);
    assert!(!got.contains_key(".."));
    // A destination root that is not a plain directory observes nothing.
    let none = observe_briefs(
        root,
        REL,
        &DestRootState::Missing {
            to_create: vec![".agents".into()],
        },
        &names,
    )
    .unwrap();
    assert!(none.is_empty());
}

// ---- diff_generated (M5-10) ----

#[test]
fn a_new_brief_is_written() {
    let s = spec_of("b", "Do it.");
    let d = diff_generated(
        REL,
        &input(vec![s.clone()], &[("b-brief", BriefObserved::Missing)]),
        &receipt_of(&[]),
        &none_skills(),
    );
    assert_eq!(ops(&d), vec![("write_generated", "b-brief".to_string())]);
    assert!(d.conflicts.is_empty());
    assert_eq!(d.staying, 0);
    match &d.steps[0] {
        Step::WriteGenerated {
            bundle,
            content,
            content_hash,
            ..
        } => {
            assert_eq!(bundle, "b");
            assert_eq!(content, &s.content);
            assert_eq!(content_hash, &s.content_hash);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn an_unchanged_brief_is_a_no_op_and_stays() {
    let s = spec_of("b", "Do it.");
    let d = diff_generated(
        REL,
        &input(vec![s.clone()], &[("b-brief", brief_with(&s.content_hash))]),
        &receipt_of(&[entry_for(&s)]),
        &none_skills(),
    );
    assert!(d.steps.is_empty());
    assert!(d.conflicts.is_empty());
    assert_eq!(d.staying, 1);
}

#[test]
fn a_changed_instruction_removes_then_writes() {
    let old = spec_of("b", "Old.");
    let new = spec_of("b", "New.");
    let d = diff_generated(
        REL,
        &input(
            vec![new.clone()],
            &[("b-brief", brief_with(&old.content_hash))],
        ),
        &receipt_of(&[entry_for(&old)]),
        &none_skills(),
    );
    assert_eq!(
        ops(&d),
        vec![
            ("remove_generated", "b-brief".to_string()),
            ("write_generated", "b-brief".to_string())
        ]
    );
    match &d.steps[0] {
        Step::RemoveGenerated { expect_hash, .. } => assert_eq!(expect_hash, &old.content_hash),
        other => panic!("{other:?}"),
    }
    assert_eq!(d.staying, 1);
}

#[test]
fn a_missing_receipted_brief_that_is_wanted_is_written_again() {
    let s = spec_of("b", "Do it.");
    let d = diff_generated(
        REL,
        &input(vec![s.clone()], &[("b-brief", BriefObserved::Missing)]),
        &receipt_of(&[entry_for(&s)]),
        &none_skills(),
    );
    assert_eq!(ops(&d), vec![("write_generated", "b-brief".to_string())]);
    assert!(d.conflicts.is_empty());
}

#[test]
fn a_brief_no_longer_wanted_is_removed_by_its_receipt_hash() {
    let s = spec_of("b", "Do it.");
    for obs in [
        brief_with(&s.content_hash),
        // Edited or foreign: the step still exists; the executor skips it
        // and the receipt releases the entry.
        brief_with(&Hash::of_bytes(b"edited")),
    ] {
        let d = diff_generated(
            REL,
            &input(vec![], &[("b-brief", obs)]),
            &receipt_of(&[entry_for(&s)]),
            &none_skills(),
        );
        assert_eq!(ops(&d), vec![("remove_generated", "b-brief".to_string())]);
        match &d.steps[0] {
            Step::RemoveGenerated { expect_hash, .. } => {
                assert_eq!(expect_hash, &s.content_hash)
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(d.staying, 0);
    }
}

#[test]
fn a_brief_of_another_shape_that_is_still_wanted_blocks_with_a_conflict() {
    let s = spec_of("b", "Do it.");
    for obs in [
        BriefObserved::Other { found: "directory" },
        BriefObserved::Other { found: "symlink" },
    ] {
        let d = diff_generated(
            REL,
            &input(vec![s.clone()], &[("b-brief", obs)]),
            &receipt_of(&[entry_for(&s)]),
            &none_skills(),
        );
        assert!(d.steps.is_empty());
        assert_eq!(d.conflicts.len(), 1, "{:?}", d.conflicts);
        assert_eq!(d.conflicts[0].code(), "modified-at-dest");
        assert!(d.conflicts[0].message().contains(".agents/skills/b-brief"));
        assert_eq!(d.staying, 1);
    }
}

#[test]
fn an_edited_brief_that_is_still_wanted_warns_and_never_blocks() {
    let s = spec_of("b", "Do it.");
    let d = diff_generated(
        REL,
        &input(
            vec![s.clone()],
            &[("b-brief", brief_with(&Hash::of_bytes(b"edited")))],
        ),
        &receipt_of(&[entry_for(&s)]),
        &none_skills(),
    );
    assert!(d.steps.is_empty());
    assert!(d.conflicts.is_empty(), "{:?}", d.conflicts);
    assert_eq!(d.staying, 1);
    assert_eq!(d.warnings.len(), 1);
    assert_eq!(d.warnings[0].code, "brief-modified");
    assert!(d.warnings[0].message.contains(".agents/skills/b-brief"));
}

#[test]
fn a_brief_hand_edited_to_the_desired_text_is_adopted_silently() {
    let s = spec_of("b", "Do it.");
    // The receipt hash is stale; the disk already holds the desired text.
    let mut stale = entry_for(&s);
    stale.content_hash = Hash::of_bytes(b"old").to_string();
    let d = diff_generated(
        REL,
        &input(vec![s.clone()], &[("b-brief", brief_with(&s.content_hash))]),
        &receipt_of(&[stale]),
        &none_skills(),
    );
    assert!(d.conflicts.is_empty());
    assert!(d.warnings.is_empty());
    // One write step refreshes the receipt; the executor writes no file.
    assert_eq!(ops(&d), vec![("write_generated", "b-brief".to_string())]);
}

#[test]
fn an_unwanted_receipted_brief_already_missing_emits_no_step() {
    let s = spec_of("b", "Do it.");
    let d = diff_generated(
        REL,
        &input(vec![], &[("b-brief", BriefObserved::Missing)]),
        &receipt_of(&[entry_for(&s)]),
        &none_skills(),
    );
    assert!(d.steps.is_empty(), "{:?}", d.steps);
    assert!(d.conflicts.is_empty() && d.warnings.is_empty());
}

#[test]
fn a_foreign_name_refuses() {
    let s = spec_of("b", "Do it.");
    for obs in [
        brief_with(&s.content_hash),
        BriefObserved::Other { found: "directory" },
        BriefObserved::Other { found: "file" },
    ] {
        let d = diff_generated(
            REL,
            &input(vec![s.clone()], &[("b-brief", obs)]),
            &receipt_of(&[]),
            &none_skills(),
        );
        assert!(d.steps.is_empty());
        assert_eq!(d.conflicts.len(), 1);
        assert_eq!(d.conflicts[0].code(), "foreign-at-dest");
    }
}

#[test]
fn a_brief_name_equal_to_a_desired_skill_is_a_duplicate() {
    let s = spec_of("b", "Do it.");
    let mut skills = BTreeMap::new();
    skills.insert("b-brief".to_string(), "fx:x/b-brief".to_string());
    let d = diff_generated(
        REL,
        &input(vec![s], &[("b-brief", BriefObserved::Missing)]),
        &receipt_of(&[]),
        &skills,
    );
    assert!(d.steps.is_empty());
    assert_eq!(d.conflicts.len(), 1);
    assert_eq!(d.conflicts[0].code(), "duplicate-name");
    assert!(d.conflicts[0].message().contains("fx:x/b-brief"));
}

#[test]
fn an_invalid_brief_name_is_an_invalid_name_conflict() {
    let long = "x".repeat(300);
    let b = set(vec![bundle(&long, "Long", Some("A."), None)]);
    let w = wanted(&tsel(&[long.as_str()]), &b);
    let d = diff_generated(REL, &input(w, &[]), &receipt_of(&[]), &none_skills());
    assert!(d.steps.is_empty());
    assert_eq!(d.conflicts.len(), 1);
    assert_eq!(d.conflicts[0].code(), "invalid-name");
}

#[test]
fn inconsistent_receipt_entries_are_ignored() {
    let s = spec_of("b", "Do it.");
    let mut bad = entry_for(&s);
    bad.dest = "elsewhere/b-brief".into();
    let d = diff_generated(
        REL,
        &input(vec![], &[("b-brief", brief_with(&s.content_hash))]),
        &receipt_of(&[bad]),
        &none_skills(),
    );
    assert!(d.steps.is_empty(), "{:?}", d.steps);
}

#[test]
fn removes_come_before_writes_each_sorted_by_name() {
    let a = spec_of("a", "A.");
    let b = spec_of("b", "B.");
    let old_c = spec_of("c", "old");
    let d = diff_generated(
        REL,
        &input(
            vec![b.clone(), a.clone()],
            &[
                ("a-brief", BriefObserved::Missing),
                ("b-brief", BriefObserved::Missing),
                ("c-brief", brief_with(&old_c.content_hash)),
            ],
        ),
        &receipt_of(&[entry_for(&old_c)]),
        &none_skills(),
    );
    assert_eq!(
        ops(&d),
        vec![
            ("remove_generated", "c-brief".to_string()),
            ("write_generated", "a-brief".to_string()),
            ("write_generated", "b-brief".to_string()),
        ]
    );
}
