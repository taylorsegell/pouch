---
name: plan:v1-static-activation-phase-03-m2-bundles-and-selection
description: "v1: static activation, phase 3 of 8: milestone M2, bundles, project selection, and resolve"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-03
---

# Milestone M2: Bundles and selection

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 3 of 8: milestone M2
**Status**: PLANNED
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

---

## Overview

Milestone M2 adds the desired-state side: Hermes-compatible bundle manifests, project root detection and `selection.yaml` IO, and resolution of references to a flat desired set with provenance, erroring on ambiguity (spec section 10 M2; spec 4.4 to 4.6, 2.2, 2.3, 5; PRD 7.2, 7.3, 5). Nothing here writes to a destination or applies anything. Estimated 2 days (spec 10).

Product Phase 1, tier 2 (read path and desired state).

---

## Scope

Carried from spec section 10 M2:

- `bundle` load, save, and CRUD with Hermes compatibility and in-place edits (spec 4.4, 2.2; CLI `bundle save|ls|show|edit|duplicate|rm`, `bundle add|rm` per spec 5).
- `slugify` table-driven unit tests, with the Hermes-parity case `#[ignore]` until parity is confirmed (spec 8 Unit).
- `project` root detection and `selection.yaml` IO (spec 4.5, 2.3).
- `resolve` of every `SkillRef` form with ambiguity errors that name candidates (spec 4.6; exit code 5 per spec 5).
- Resolution to a flat desired set with provenance (consumed by M3 `plan::diff`).

---

## Acceptance Criteria

Closes (spec section 10 M2): AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-35 (desired side). Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-7 | `ac_07_*` | Fully-Automated |
| AC-8 | `ac_08_*` | Fully-Automated |
| AC-9 | `ac_09_*` | Fully-Automated |
| AC-10 | `ac_10_*` | Fully-Automated |
| AC-11 | `ac_11_*` | Fully-Automated |
| AC-12 | `ac_12_*` | Fully-Automated |
| AC-35 (desired side) | `ac_35_*` (hand-edited selection shows `selected`; the `active` half needs M4 verify) | Fully-Automated |

---

## Entry Gate

- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- Milestone M1 VERIFIED with its report; M1 Forward Preview read.
- Hermes `slugify` parity decision available or explicitly deferred with the `#[ignore]` case in place (see Blockers).

---

## Blast Radius

- `crates/unam-core/src/bundle.rs`, `project.rs`, `resolve.rs`; extensions to `catalog/mod.rs` (lookup for resolution) only as needed
- `crates/unam-cli/src/cli/` (bundle commands, selection-related plumbing)
- `crates/unam-cli/tests/` new `ac_07` to `ac_12`, `ac_35` tests; fixtures: Hermes-only bundle with bare names, duplicate names across categories and sources
- Not touched: destinations, adapters, `docs/`, `spike/`, the skill library

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT; spec sections cited, not copied.

### Step A: Bundles

- [ ] A1. Implement manifest load and save per spec 4.4 and 2.2, loading a Hermes-only bundle unchanged (absent `description` yields the Hermes default form).
- [ ] A2. Implement line-oriented in-place edits of the `skills:` block so other entries keep text and order (spec 11 YAML trade-off; AC-11).
- [ ] A3. Implement `slugify` with table-driven tests; keep the Hermes-parity case `#[ignore]` until confirmed against the Hermes source named in PRD section 16.
- [ ] A4. Add bundle CLI commands per spec 5.

### Step B: Project and selection

- [ ] B1. Implement project root detection (walk for `.git`, spec 1.4) and `selection.yaml` load and save per spec 4.5 and 2.3.
- [ ] B2. Resolve bundle plus single adds and removes to a flat desired set with provenance (AC-12).

### Step C: Resolve

- [ ] C1. Implement every `SkillRef` form per spec 4.6; ambiguous bare names error with all candidate IDs and nothing activates (AC-8).
- [ ] C2. Cover the unit matrix in spec 8 Unit (resolve against duplicates across categories and sources).

### Step D: Tests

- [ ] D1. Write `ac_07_*` to `ac_12_*` and `ac_35_*` integration tests; snapshot every new command's `--json` output with `insta`.
- [ ] D2. Confirm no test writes outside `tempfile` directories.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass; the Hermes-parity slugify case is the only #[ignore] allowed, and only if parity is undecided
cargo test -p unam --test <file> ac_07   # repeat for ac_08 to ac_12 and ac_35
# Expected: pass
```

- All checklist items checked; AC-7 to AC-12 and AC-35 (desired side) tests green.
- Regression: M1 criteria tests (AC-1, 3, 4, 5, 6, 38) still green.
- Hermes slugify parity: confirmed and aligned, or the documented difference recorded in the phase report.
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- Commit execution changes on `main` before starting milestone M3; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met, or M1 not VERIFIED.
- Hermes `slugify` parity (brief open question; needed by M2): read the Hermes implementation (`agent/skill_bundles.py`, PRD section 16) and align, or document the difference. If neither is possible, ship with the case `#[ignore]` and report to Taylor.
- A bundle fixture reveals a Hermes format fact that contradicts spec 2.2: report to Taylor; spec and PRD stay authoritative.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: M1 report read; Hermes slugify source read; test context loaded; plan drift checked
- [ ] 2. INNOVATE: innovate-agent: approach decided; Decision Summary written
- [ ] 3. PLAN-SUPPLEMENT: plan-agent: existing phase plan updated; Inner Loop Refresh Note if sections changed (or "n/a: research clean")
- [ ] 4. PVL: vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [ ] 5. EXECUTE: all checklist items done; per-section test gates run and green (or gaps documented)
- [ ] 6. EVL: all EVL gates green; follow-up stubs registered; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Touchpoints

- Files listed in Blast Radius
- Read: spec sections 2.2, 2.3, 4.4 to 4.6, 5, 8; PRD sections 5, 7.2, 7.3, 13.2, 16 (Hermes sources)

---

## Public Contracts

- Bundle manifest format (Hermes-compatible) and `selection.yaml` format (spec 2.2, 2.3): contracts for users and for M3 onward.
- CLI: bundle commands (spec 5); exit code 5 for ambiguity.
- Provenance structure of the desired set consumed by M3 `plan::diff`.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `ac_07_*` (Hermes-only bundle, bare names, default description form) | Fully-Automated | AC-7 |
| `ac_08_*` (bare name in two categories errors with both IDs; nothing activates) | Fully-Automated | AC-8 |
| `ac_09_*` (bundle across two categories and two sources) | Fully-Automated | AC-9 |
| `ac_10_*` (saved bundle applied to empty project yields original selection, desired side) | Fully-Automated | AC-10 |
| `ac_11_*` (remove one entry; others unchanged in text and order) | Fully-Automated | AC-11 |
| `ac_12_*` (bundle plus add plus remove resolves; provenance) | Fully-Automated | AC-12 |
| `ac_35_*` (hand-edited selection reads `selected`, desired side) | Fully-Automated | AC-35 |
| slugify parity against Hermes source | Hybrid (precondition: Hermes source readable) | brief open question: Hermes slugify parity |

AC-10 final end-to-end form depends on apply (M3); the M2 test covers the resolution half only and the M3 plan re-verifies it. No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: after M1 is VERIFIED, spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

After M1 is VERIFIED, run RESEARCH for this milestone (RIPER-5), then PLAN-SUPPLEMENT and PVL before any EXECUTE.
