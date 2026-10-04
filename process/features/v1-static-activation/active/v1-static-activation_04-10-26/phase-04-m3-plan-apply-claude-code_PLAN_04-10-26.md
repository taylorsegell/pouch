---
name: plan:v1-static-activation-phase-04-m3-plan-apply-claude-code
description: "v1: static activation, phase 4 of 8: milestone M3, plan and apply for the claude-code adapter (journal, receipt, history)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-04
---

# Milestone M3: Plan and apply for claude-code

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 4 of 8: milestone M3
**Status**: PLANNED
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-04-m3-plan-apply-claude-code_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

---

## Overview

Milestone M3 is the first milestone that mutates user-owned destinations. It adds the adapter trait and the `claude_code` adapter, typed-path filesystem operations, `plan::diff`, and the journaled `apply` with receipt and history, plus the commands `add`, `rm`, `use`, `deactivate` with SES-1 and SES-2 output (spec section 10 M3; spec 4.7 to 4.10, 2.4, 2.5, 5, 7; PRD 7.3, 7.6, 7.7, 7.8, 11). Estimated 3 to 4 days (spec 10). It is the highest-risk milestone of product Phase 1 (destructive writes, ownership, interruption).

Product Phase 1, tier 3 (mutation and recovery for claude-code).

---

## Scope

Carried from spec section 10 M3:

- `adapter` trait and `claude_code` adapter, destination `.claude/skills` (spec 4.7), with an evidence URL or spike result for every claim (spec 4.7; an adapter without evidence fails a unit test, spec 8).
- `fsops` with typed paths `SourcePath`, `DestPath`, `StatePath`; no write function accepts a `SourcePath` (spec 4.9).
- `plan::diff` including the `proptest` idempotency and carry-forward properties (spec 4.8, 8).
- `apply` with journal, receipt, history capped at 20, advisory project lock via `fs4`, resume and rollback in core (spec 4.10, 2.4, 2.5).
- `add`, `rm`, `use <bundle> --replace|--add`, `deactivate [--all]` (spec 5).
- SES-1 and SES-2 output (PRD 7.8; spec 5 plan rendering).
- Default strategy and `strategies()` for the adapter taken from the M0 Outcome (spec 10 M0).

---

## Acceptance Criteria

Closes (spec section 10 M3): AC-13, AC-15, AC-16, AC-17, AC-18, AC-19, AC-21, AC-22, AC-23, AC-24, AC-25, AC-29, AC-30, AC-31, AC-32, AC-33, AC-40, AC-53. Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-13 | `ac_13_*` | Fully-Automated |
| AC-15 | `ac_15_*` | Fully-Automated |
| AC-16 | `ac_16_*` plus `plan::diff` proptest | Fully-Automated |
| AC-17 | `ac_17_*` plus proptest carry-forward | Fully-Automated |
| AC-18 | `ac_18_*` | Fully-Automated |
| AC-19 | `ac_19_*` (hash of project B unchanged) | Fully-Automated |
| AC-21 | `ac_21_*` | Fully-Automated |
| AC-22 | `ac_22_*` | Fully-Automated |
| AC-23 | `ac_23_*` (foreign file byte-identical) | Fully-Automated |
| AC-24 | `ac_24_*` (no agent config file created or modified; claude-code side) | Fully-Automated |
| AC-25 | `ac_25_*` | Fully-Automated |
| AC-29 | `ac_29_*` (output text and evidence line) | Fully-Automated |
| AC-30 | `ac_30_*` (SES-2 caveat on `rm` and `deactivate`) | Fully-Automated |
| AC-31 | `ac_31_*` (interruption test `FsOps` fails at step N) | Fully-Automated |
| AC-32 | `ac_32_*` (one of three entries fails) | Fully-Automated |
| AC-33 | `ac_33_*` (rollback restores prior destination and receipt) | Fully-Automated |
| AC-40 | `ac_40_*` | Fully-Automated |
| AC-53 | `ac_53_*` (second mutating command refuses while locked; exit 11) | Fully-Automated |

Note (reported to Taylor, umbrella items 2 and 3): AC-33 needs a `rollback` entry point and the spec 8 interruption tests assert `reconcile` reports zero divergence, but the CLI `rollback` and `reconcile` are scoped in milestone M4. RESEARCH must decide how M3 proves AC-31 to AC-33 without taking M4 scope silently (core-level rollback and resume are in spec 1.3 `apply.rs`); record the decision and surface it to Taylor if it moves scope.

---

## Entry Gate

- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- Milestone M2 VERIFIED with its report; M2 Forward Preview read.
- Strategy default and supported strategies for `claude-code` known from the M0 Outcome.

---

## Blast Radius

- `crates/unam-core/src/adapter/mod.rs`, `adapter/claude_code.rs`, `fsops.rs`, `plan.rs`, `apply.rs`, `state.rs`; `hash.rs` (needed for the invariants in every mutating test if not already created)
- `crates/unam-cli/src/cli/` (add, rm, use, deactivate), `output.rs` (plan rendering, SES-1, SES-2)
- `crates/unam-cli/tests/` new `ac_NN_*` files, interruption fixtures, foreign-file fixtures, `insta` snapshots
- Runtime files created by the product in TEMPORARY project directories only: `<root>/.unam/` (applied receipt, journal, history, lock)
- Not touched: this repository as a target, the skill library, `docs/`, `spike/`

Risk class: destructive filesystem writes in user-owned destinations and ownership boundaries. Hybrid gate minimum and manual-first evidence handoff (`vc-risk-evidence-pack`) apply.

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT; spec sections cited, not copied.

### Step A: Types and filesystem layer

- [ ] A1. Define typed paths and `fsops` (atomic write, symlink, copy) per spec 4.9; compile-time guarantee that no write function accepts `SourcePath`.
- [ ] A2. Implement `state` IO for receipt, journal, history, and the project lock (spec 2.4, 2.5, 4.10).
- [ ] A3. Implement `hash` (blake3 tree hashing) if not present; wire the two non-negotiable invariants into the shared test helper.

### Step B: Adapter and plan

- [ ] B1. Define the adapter trait and registry (spec 4.7); implement `claude_code` with `destination`, `scan_locations`, `strategies`, `refresh`, evidence per claim; unit test that an adapter lacking evidence fails.
- [ ] B2. Implement `plan::diff` per spec 4.8 with the two `proptest` properties from spec 8.

### Step C: Apply and commands

- [ ] C1. Implement `apply` executor per spec 4.10: journal before mutation, receipt after, history cap 20, advisory lock, partial-failure reporting, resume, rollback (core).
- [ ] C2. Implement `add`, `rm`, `use --replace|--add`, `deactivate [--all]` (spec 5; exit codes 5, 6, 7, 10, 11) with `--yes` still printing the plan.
- [ ] C3. Implement SES-1 and SES-2 output (PRD 7.8); refuse collisions with foreign skills (ownership rules, spec 4.9, 4.10).

### Step D: Tests

- [ ] D1. Write the `ac_NN_*` tests listed above; interruption tests use a test `FsOps` that errors at step N (spec 8 Interruption).
- [ ] D2. Assert both invariants (source tree hash unchanged; foreign fixture file byte-identical) in every mutating test.
- [ ] D3. Snapshot all new `--json` outputs with `insta`; assert exit codes per spec 5.
- [ ] D4. Hybrid gate: Taylor names the first real project to adopt `unam` by the end of this milestone and runs one real activation in it (outside this repository).

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass
cargo test -p unam --test <file> ac_13   # repeat for each criterion in the table above
# Expected: pass
```

- All checklist items checked; the 18 criteria above green; both invariants asserted in every mutating test.
- Regression: M1 and M2 criteria tests still green; M1 and M2 CLI snapshots unchanged.
- High-risk evidence pack produced per `vc-risk-evidence-pack` (manual-first).
- First real adopting project named by Taylor (brief open question, due end of this milestone).
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- Interrupted or failed `apply` states must reconcile to a consistent state in tests before this milestone can be VERIFIED.
- Commit execution changes on `main` before starting milestone M4; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met, or M2 not VERIFIED.
- First real adopting project not named by the end of this milestone (brief open question): not a code blocker; report to Taylor and keep M4 start pending his naming it.
- M0 Outcome says symlinks are not discovered for the adapter: strategy default changes to `copy`; spec claims affected are reported to Taylor, not edited.
- The AC-31 to AC-33 scope ambiguity above cannot be resolved without moving M4 scope: report to Taylor.
- Any test run would touch this repository, the skill library, or real user config: stop (hard safety constraint).

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: M2 report read; M0 Outcome applied; test context loaded; plan drift checked; AC-31 to AC-33 scope question analyzed
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
- Read: spec sections 2.4, 2.5, 4.7 to 4.10, 5, 7, 8; PRD sections 7.3, 7.6 to 7.8, 11, 13.2

---

## Public Contracts

- Commands `add`, `rm`, `use`, `deactivate`, exit codes 5, 6, 7, 10, 11, and `--json` envelope (spec 5).
- Receipt, journal, history formats (spec 2.4, 2.5) and the `<root>/.unam/` layout.
- Adapter trait (spec 4.7), extended by M5 without breaking `claude_code`.
- Ownership guarantees: never overwrite or delete foreign files (PRD G11, S4).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `ac_13_*` two skills from two categories reach active | Fully-Automated | AC-13 |
| `ac_15_*` `--yes` still prints the plan; plan matches filesystem | Fully-Automated | AC-15 |
| `ac_16_*` and `plan::diff` proptest idempotency | Fully-Automated | AC-16 |
| `ac_17_*` and proptest carry-forward | Fully-Automated | AC-17 |
| `ac_18_*` `--replace` versus `--add` end states | Fully-Automated | AC-18 |
| `ac_19_*` project A operations leave project B unchanged by hash | Fully-Automated | AC-19 |
| `ac_21_*` return to project shows prior selection | Fully-Automated | AC-21 |
| `ac_22_*` collision with foreign skill refuses | Fully-Automated | AC-22 |
| `ac_23_*` `deactivate --all` removes exactly the two owned | Fully-Automated | AC-23 |
| `ac_24_*` no agent config file created or modified | Fully-Automated | AC-24 (claude-code) |
| `ac_25_*` every created entry in receipt with source ID and strategy | Fully-Automated | AC-25 |
| `ac_29_*` no-restart statement with evidence | Fully-Automated | AC-29 |
| `ac_30_*` SES-2 caveat on every removal | Fully-Automated | AC-30 |
| `ac_31_*` interruption leaves journal; next command detects (exit 10) | Fully-Automated (test `FsOps`) | AC-31 |
| `ac_32_*` partial failure keeps two of three applied and names failure | Fully-Automated | AC-32 |
| `ac_33_*` rollback restores prior destination and receipt | Fully-Automated | AC-33 |
| `ac_40_*` skill in two bundles appears once with two provenance records | Fully-Automated | AC-40 |
| `ac_53_*` lock refuses concurrent mutation (exit 11) | Fully-Automated | AC-53 |
| First real activation in Taylor's adopting project (outside this repository) | Hybrid (precondition: project named; real agent installed) | PRD 12 daily-use path; brief open question |
| Evidence-pack review of destructive-write paths | Agent-Probe (orchestrator and reviewer judgment) | PRD S4, G11 |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: after M2 is VERIFIED, spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

After M2 is VERIFIED, run RESEARCH for this milestone (RIPER-5), then PLAN-SUPPLEMENT and PVL before any EXECUTE.
