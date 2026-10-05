---
name: plan:v1-static-activation-phase-05-m4-verify-status-reconcile-rollback
description: "v1: static activation, phase 5 of 8: milestone M4, verify, status, reconcile, rollback, doctor"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-05
---

# Milestone M4: Verify, status, reconcile, rollback

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 5 of 8: milestone M4
**Status**: PLANNED
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

---

## Overview

Milestone M4 makes state honest: classify every entry into the six states, report status, detect and repair divergence, roll back from history, and diagnose the environment (spec section 10 M4; spec 4.11, 4.12, 4.10 rollback, 5, 7; PRD 4 (six states), 7.4, 7.7, 7.9, 11.5). Estimated 2 days (spec 10).

Product Phase 1, tier 3 (mutation and recovery). Reconcile is report-first; repair goes through the same journaled executor built in M3.

---

## Scope

Carried from spec section 10 M4:

- `verify` six-state classification and shadow analysis (spec 4.11).
- `status` (spec 5; exit 8 if any `orphaned`).
- `reconcile [--repair]` (spec 4.12; exit 8 on divergence without repair).
- `rollback` from history (spec 5, 4.10). Journal-based `resume` and `rollback` are built in milestone M3 (decision 3, 2026-10-04) and are not rebuilt here.
- `source relocate` (spec 4.2, 5), built here because it needs receipts and the project index (decision 2, 2026-10-04). Closes AC-37.
- `doctor [--evidence]`: symlink capability probe, destination writability, adapter detection, source reachability, receipt integrity, journal presence; evidence listing for every adapter claim (spec 5; exit 9).

Scope decisions (resolved 2026-10-04): `source relocate` lands here (decision 2); journal-based `resume` and `rollback` landed in milestone M3 and AC-33 (rollback from history) moves here (decision 3). This milestone also adds the "reconcile reports zero divergence" assertion to the M3 interruption test (spec 8 Interruption).

### Performance and success-target checks (exit-gate checks; PRD 13.1)

| ID | Check | How measured |
|---|---|---|
| S7 | Zero unreconcilable states after simulated interruption | `s7_interruption_zero_divergence`: the M3 interruption test, run with the test `FsOps` failing at every step index N of an apply, then after `resume` and (separately) after `rollback` assert `reconcile` reports zero divergence |
| S8 | Zero projects diverged from desired without being reported | `s8_divergence_always_reported`: fixture projects diverged each way (hand-deleted entry, broken symlink, modified copy, foreign shadow, renamed source skill); assert `status` and `reconcile` report every one (exit 8) |

---

## Acceptance Criteria

Closes (spec section 10 M4): AC-14, AC-26, AC-27, AC-28, AC-33, AC-34, AC-36, AC-37, AC-39, AC-41. Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-14 | `ac_14_*` (status shows source path, destination, strategy, provenance, state per entry) | Fully-Automated |
| AC-26 | `ac_26_*` (broken destination symlink is `orphaned`, never `active`) | Fully-Automated |
| AC-27 | `ac_27_*` (shadowed skill reports both paths) | Fully-Automated |
| AC-28 | `ac_28_*` (no output asserts an agent read, loaded, or applied a skill; text assertion across commands) | Fully-Automated |
| AC-33 | `ac_33_*` (`rollback` from history restores prior destination and receipt) | Fully-Automated |
| AC-34 | `ac_34_*` (hand-deleted entry: reconcile reports, `--repair` restores) | Fully-Automated |
| AC-36 | `ac_36_*` (renamed source skill is `orphaned` with candidates; blocked by default) | Fully-Automated |
| AC-37 | `ac_37_*` (`source relocate` repairs every affected project) | Fully-Automated |
| AC-39 | `ac_39_*` (source edit under follow visible at destination, no drift warning) | Fully-Automated |
| AC-41 | `ac_41_*` (contradictory skills activate; no semantic conflict claim anywhere) | Fully-Automated |

---

## Entry Gate

- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- Milestone M3 VERIFIED with its report; M3 Forward Preview read; first real adopting project named (brief: by end of M3).
- Milestone M3 report records that journal-based `resume` and `rollback` and the interruption test (consistency compared directly to expected state) are done.

---

## Blast Radius

- `crates/unam-core/src/verify.rs`, `reconcile.rs`; additions to `apply.rs` (rollback from history), `source.rs` (relocate), `adapter/claude_code.rs` (doctor probes, `refresh`), `state.rs`
- `crates/unam-cli/src/cli/` (status, reconcile, rollback from history, doctor, source relocate), `output.rs` (six-state table, SES caveats)
- `crates/unam-cli/tests/` new `ac_NN_*` files and fixtures (broken symlinks, shadowing user-scope directory via env override, renamed source skill)
- `docs/adapters.md`: created here if absent (milestone M2 may already have created it for the Hermes slugify outcome), otherwise extended; a new spec-called document (not one of the five protected design documents; decision 4, 2026-10-04) holding the M0 Outcome and the `doctor --evidence` listing; milestone M7 regenerates and maintains it
- Not touched: the five kickoff design documents (`docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`), the skill library, this repository as a target

Risk class: repair paths mutate destinations; they reuse the M3 journaled executor and must keep both invariants.

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT; spec sections cited, not copied.

### Step A: Verify and status

- [ ] A1. Implement observe and classify into the six states and shadow analysis (spec 4.11; PRD section 4).
- [ ] A2. Implement `status` with `--json` full records and exit 8 on any `orphaned` (spec 5).
- [ ] A3. Ensure no output claims an agent read, loaded, or applied a skill (PRD VER-5; AC-28).

### Step B: Reconcile, rollback, resume

- [ ] B1. Implement `reconcile` report and `--repair` via the M3 executor (spec 4.12).
- [ ] B2. Implement `rollback` from history when no journal is present (spec 5; AC-33). Journal-based `resume` and `rollback` already exist from milestone M3.
- [ ] B3. Extend the M3 interruption test (`ac_31_*`, `ac_32_*`) with the assertion that `reconcile` reports zero divergence after `resume` and after `rollback` (spec 8 Interruption), and write `s7_interruption_zero_divergence` over every failure step index; record the result in the phase report.

### Step C: Source edge cases

- [ ] C1. Implement renamed-source detection with candidates and default block (AC-36; spec 4.11, PRD EDG-1).
- [ ] C2. Implement `source relocate` repair of dependent projects (AC-37; spec 4.2, 5; PRD EDG-4). Requires receipts and the project index (spec 11: cache, not source of truth).
- [ ] C3. Verify follow-mode visibility of source edits without drift warnings (AC-39).

### Step D: Doctor

- [ ] D1. Implement `doctor` checks and `--evidence` listing (spec 5; exit 9; spec 4.7 evidence rule).
- [ ] D2. Create `docs/adapters.md` if absent, otherwise extend it (keep the M2 slugify record), with the M0 Outcome and `unam doctor --evidence` run against temporary directories only (never against this repository); state that M0 Cursor results are evidence for the `cursor-agent` CLI only and the Cursor editor is not tested; do not edit any of the five protected design documents.

### Step E: Tests

- [ ] E1. Write `ac_14_*`, `ac_26_*`, `ac_27_*`, `ac_28_*`, `ac_33_*`, `ac_34_*`, `ac_36_*`, `ac_37_*`, `ac_39_*`, `ac_41_*`; snapshot all new `--json` outputs.
- [ ] E2. Assert both invariants in every mutating test; add a regression pass over M3 tests.
- [ ] E3. Write the success-target checks `s7_interruption_zero_divergence` and `s8_divergence_always_reported`.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass
cargo test -p unam --test <file> ac_14   # repeat for ac_26, ac_27, ac_28, ac_33, ac_34, ac_36, ac_37, ac_39, ac_41
# Expected: pass
cargo test -p unam --test <file> s7_interruption_zero_divergence
# Expected: pass (PRD S7)
cargo test -p unam --test <file> s8_divergence_always_reported
# Expected: pass (PRD S8)
```

- All checklist items checked; AC-14, 26, 27, 28, 33, 34, 36, 37, 39, 41 tests green; S7 and S8 checks green; both invariants asserted.
- Regression: M1 to M3 criteria tests green; M3 interruption tests also assert zero divergence via `reconcile`.
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- Commit execution changes on `main` before starting milestone M5; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met, or M3 not VERIFIED.
- First real adopting project still unnamed (brief open question, due end of M3): report; does not block code but blocks the product Phase 1 gate later.
- Any repair path that cannot be made journaled: stop and report.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: M3 report read; test context loaded; plan drift checked; project index shape for `source relocate` confirmed
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
- Read: spec sections 4.2, 4.7, 4.10 to 4.12, 5, 7, 8; PRD sections 4, 7.4, 7.7, 7.9, 11.5, 13.2

---

## Public Contracts

- Commands `status`, `reconcile`, `rollback` (from history, in addition to the M3 journal-based rollback), `doctor`, `source relocate` and exit codes 8, 9, 10 (spec 5).
- The six-state vocabulary and `status --json` record shape.
- `doctor --evidence` output is the source for `docs/adapters.md` (created here if absent, otherwise extended; maintained in M7).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `ac_14_*` status record fields | Fully-Automated | AC-14 |
| `ac_26_*` broken symlink is `orphaned` | Fully-Automated | AC-26 |
| `ac_27_*` shadowed with both paths | Fully-Automated | AC-27 |
| `ac_28_*` no output asserts an agent read or loaded a skill | Fully-Automated | AC-28 |
| `ac_33_*` rollback from history restores prior destination and receipt | Fully-Automated | AC-33 |
| `ac_34_*` hand-deleted entry, reconcile and `--repair` | Fully-Automated | AC-34 |
| `ac_36_*` renamed source skill orphaned with candidates | Fully-Automated | AC-36 |
| `ac_37_*` relocate repairs every affected project | Fully-Automated | AC-37 |
| `ac_39_*` follow-mode edit visible, no drift warning | Fully-Automated | AC-39 |
| `ac_41_*` contradictory skills activate without conflict claim | Fully-Automated | AC-41 |
| `s7_interruption_zero_divergence` every failure step, resume and rollback, then reconcile reports zero divergence | Fully-Automated (test `FsOps`) | PRD S7 |
| `s8_divergence_always_reported` every divergence kind reported by status and reconcile | Fully-Automated | PRD S8 |
| `doctor` symlink capability and writability probes on a real filesystem | Hybrid (precondition: real macOS or Linux filesystem; temp dirs) | spec 5 doctor; PRD R1 mitigation |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: after M3 is VERIFIED, spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

After M3 is VERIFIED, run RESEARCH for this milestone (RIPER-5), then PLAN-SUPPLEMENT and PVL before any EXECUTE.
