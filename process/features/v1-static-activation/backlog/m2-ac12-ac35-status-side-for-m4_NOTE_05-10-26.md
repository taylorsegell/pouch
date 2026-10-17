---
name: note:m2-ac12-ac35-status-side-for-m4
description: "Backlog note from M2: the status side of AC-12 and AC-35 must be closed by M4 tests"
date: 05-10-26
feature: v1-static-activation
---

# M2 follow-up: status side of AC-12 and AC-35 (for milestone M4)

Priority: required for M4 exit (these are product Phase 1 criteria).

Problem: PRD AC-12 ("`status` shows provenance") and AC-35 ("hand-edited selection shows `selected`, not `active`") both name `status`, which is built in milestone M4. Milestone M2 closed only the desired side, through core-API integration tests (umbrella open item 24, decision D1 of the M2 plan).

Already proven in M2 (do not duplicate):

- `ac_12_bundle_plus_add_plus_remove_resolves_with_provenance` in `crates/unam-cli/tests/ac_selection.rs`: the resolved set carries provenance (`Bundle(slug)` and `Adhoc`, winner is the last record), and removed entries drop with their provenance.
- `ac_35_hand_edited_selection_is_desired_not_applied` in the same file: a hand-written `selection.yaml` (with and without `version: 1`) resolves to desired entries while no `.unam/applied.json` and no destination exist, and loading registers nothing in the project index.

M4 must add (to the M4 plan, exit gate, and tests):

1. Extend the `ac_12_*` test: `unam status` for the AC-12 selection prints, per entry, its provenance (bundle slug or ad hoc).
2. Extend the `ac_35_*` test: `unam status` on a hand-edited selection with no receipt prints `selected` and never `active`.

Fix options: add both assertions to the existing tests once `status` exists (preferred, keeps one named test per criterion); or add `ac_12_status_*` and `ac_35_status_*` siblings if the M4 harness differs.

Pointers: M2 report `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_REPORT_04-10-26.md` (SPEC Gaps and Forward Preview); M4 plan `phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md` (not edited by M2; M4 RESEARCH or PLAN-SUPPLEMENT picks this up). Related: the `DesiredEntry::winner()` reading (open item 27) decides what `status` shows as the effective source of an entry.
