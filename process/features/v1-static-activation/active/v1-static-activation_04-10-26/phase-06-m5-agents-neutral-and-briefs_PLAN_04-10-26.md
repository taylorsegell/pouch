---
name: plan:v1-static-activation-phase-06-m5-agents-neutral-and-briefs
description: "v1: static activation, phase 6 of 8: milestone M5, agents-neutral adapter and generated briefs; completes product Phase 1; HARD STOP after this milestone"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-06
---

# Milestone M5: agents-neutral adapter and generated briefs

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 6 of 8: milestone M5
**Status**: PLANNED
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

---

## Overview

Milestone M5 adds the second adapter, `agents-neutral` (`.agents/skills`), with shared-destination reporting, ancestor warnings, the Hermes followup, a cross-adapter duplicate check, and the generated bundle brief with opt-out (spec section 10 M5; spec 4.7, 4.8, 4.10, 5; PRD D3, 7.6, 7.9, 10.2, 10.3; AC-43). Estimated 1 to 2 days (spec 10).

M5 completes product Phase 1 in code (spec section 10: "M5 completes PRD Phase 1"). Product Phase 1 is DONE only by this definition (PRD section 12 and PRD 13.2; decision 10, 2026-10-04): AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1 unless it sits under the "Phase 2" or "Phase 3" heading) AND Taylor uses the tool daily. That second half is Taylor's confirmation, not this milestone's tests. The program HARD STOPS after this milestone.

---

## Scope

Carried from spec section 10 M5:

- Second adapter `agents_neutral` with shared-destination reporting (consumers codex, cursor, windsurf-devin, hermes named on every plan), ancestor warnings for nested projects, Hermes followup, and a cross-adapter duplicate check (spec 4.7; PRD D3, ACT-13, EDG-11).
- `WriteGenerated` bundle brief (generated brief skill, opt-out) per spec 4.8, 4.10 and PRD Q5 current answer.
- Default strategy and `strategies()` for `agents-neutral` taken from the M0 Outcome. M0 Cursor rows are labeled `Cursor (cursor-agent CLI)`: evidence for the `cursor-agent` CLI only; the Cursor editor is not tested, so an evidence string for the `cursor` consumer must name the CLI.
- Two targets in one project holding different selections (spec 4.5, PRD ACT-3).
- Performance and success-target checks (decision 11, 2026-10-04), listed under "Performance and success-target checks" below.

---

## Acceptance Criteria

Closes (spec section 10 M5): AC-20, AC-42, AC-43, AC-24 (second adapter). Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-20 | `ac_20_*` (two targets in one project hold and report different selections) | Fully-Automated |
| AC-42 | `ac_42_*` (nested project under an ancestor-scanning target warns and names the ancestor) | Fully-Automated |
| AC-43 | `ac_43_*` (every `agents-neutral` plan names codex, cursor, windsurf-devin, hermes as consumers) | Fully-Automated |
| AC-24 (second adapter) | `ac_24_*` extended: no agent config file created or modified for `agents-neutral` | Fully-Automated |

### Performance and success-target checks (exit-gate checks; spec 9, PRD 13.1)

| ID | Check | How measured |
|---|---|---|
| S9 (agents-neutral) | No adapter claim without an evidence URL or spike result | unit test `adapter_claims_have_evidence` over the `agents_neutral` adapter (spec 8 adapters; checklist A1) |
| S1 | Cold-cache time from invoking the tool to a verified-active two-skill selection under 60 s, median under 30 s | Measured ONCE at the product Phase 1 exit on Taylor's real library, read-only: register `~/Developer/01-AI/skillzone/` as a source (never written), fresh `UNAM_CACHE_DIR`, destination is a temporary project outside this repository; time the scripted sequence `source add`, add two skills with `--yes`, `status`; five runs, each under 60 s and the median under 30 s; record in the phase report |
| S6 | Estimated catalog cost of a typical selection at least 90 percent lower than the whole library | Measured ONCE at the product Phase 1 exit on the same real library, read-only: compute the spec 4.8 estimate (step 10 formula) for Taylor's typical selection and for the whole cataloged library; assert the selection estimate is at most 10 percent of the library estimate; record both numbers |
| Network none; background processes none (spec 9) | Confirmed, not assumed | (a) `cargo tree -p unam --edges normal` lists no HTTP or socket client crate; (b) source search for `std::net`, `TcpStream`, `UdpSocket`, and process spawning that outlives the command finds none outside tests; (c) test `no_background_process`: after each CLI command exits, no `unam` process remains (`pgrep -x unam` empty) |

S5 (median active selection size under 10 after four weeks) is OBSERVATIONAL, not a gate: it is taken four weeks after product Phase 1 is in daily use. Record it as a follow-up in the umbrella plan ("S5 follow-up") and in this milestone's phase report exit notes; do not wait for it to close this milestone.

---

## Entry Gate

- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter (including `agents-neutral`).
- Milestone M4 VERIFIED with its report; M4 Forward Preview read.

---

## Blast Radius

- `crates/unam-core/src/adapter/agents_neutral.rs`, `adapter/mod.rs` (registry, detection, shared-destination reporting), `plan.rs` and `apply.rs` (generated brief write, cross-adapter duplicate check), `verify.rs` (ancestor warnings, shared-destination reporting)
- `crates/unam-cli/src/cli/` and `output.rs` (consumer list on plan, ancestor warning, Hermes followup)
- `crates/unam-cli/tests/` new and extended `ac_NN_*` tests and fixtures (nested project with an ancestor `.agents/skills`)
- Not touched: `docs/`, `spike/`, the skill library, this repository as a target (its `.agents/skills` is a toolkit symlink)

Risk class: shared destination visible to several agents (PRD R3); test fixtures must never use this repository's `.agents/skills`.

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT; spec sections cited, not copied.

### Step A: Second adapter

- [ ] A1. Implement `agents_neutral` per spec 4.7 with evidence per claim; unit test that every adapter claim has evidence (spec 8 adapters).
- [ ] A2. Implement shared-destination reporting naming the four consumers on every plan (AC-43; PRD D3).
- [ ] A3. Implement ancestor warnings for nested projects (AC-42; PRD EDG-11) and the cross-adapter duplicate check.
- [ ] A4. Implement the Hermes followup output (spec 5 plan rendering; PRD 5, Q5).

### Step B: Briefs and multi-target

- [ ] B1. Implement `WriteGenerated` bundle brief with opt-out (spec 4.8, 4.10; AC-17 and AC-23 behaviors must still hold for generated entries).
- [ ] B2. Support two targets in one project with independent selections and reporting (AC-20).

### Step C: Tests

- [ ] C1. Write `ac_20_*`, `ac_42_*`, `ac_43_*`; extend `ac_24_*` for the second adapter; snapshot new `--json` output.
- [ ] C2. Assert both invariants in every mutating test; run the full M1 to M4 regression suite.
- [ ] C3. Write the checks from the performance table: `no_background_process`, the dependency and source audit (a) and (b), and the S9 unit test for `agents_neutral`; prepare the scripted S1 and S6 measurement for the product Phase 1 exit run.

### Step D: Product Phase 1 completeness report

- [ ] D1. Write a report section listing every acceptance criterion closed by M1 to M5 with its test name and status, for the product Phase 1 set AC-1 to AC-43 and AC-53 (AC-2 is closed by milestone M3 and AC-33 by milestone M4), flagging any criterion not yet green, plus the S1 and S6 measurements and the S5 follow-up note. This is evidence for Taylor, not a decision.
- [ ] D2. Run the S1 and S6 measurements once on Taylor's real library (read-only; temporary destination project; never this repository) and record the results in the report.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass
cargo test -p unam --test <file> ac_20   # repeat for ac_42, ac_43, ac_24
# Expected: pass
cargo test -p unam-core adapter_claims_have_evidence
# Expected: pass for both adapters (PRD S9)
cargo test -p unam --test <file> no_background_process
# Expected: pass; plus `cargo tree -p unam --edges normal` shows no HTTP or socket client crate (spec 9)
```

- All checklist items checked; AC-20, 42, 43, 24 (second adapter) tests green; both invariants asserted.
- Regression: M1 to M4 criteria tests green.
- Product Phase 1 completeness report written (checklist step D1); S1 and S6 measured once on the real library and recorded (D2); network and background-process checks green; S5 follow-up recorded (observational, not a gate).
- Phase report written to the report destination above.
- FINAL ITEM, HARD STOP: the program STOPS here. Do not start research, plan refresh, or any step of milestone M6 or M7. Update the umbrella `## Current Execution State` to "Product Phase 1 Gate: AWAITING TAYLOR" and report to Taylor. The gate requires Taylor's explicit confirmation that product Phase 1 is done: AC-1 to AC-43 and AC-53 pass and Taylor uses the tool daily (PRD section 12 and 13.2; spec section 10 after M5; brief "Out of scope"). It is never auto-proceeded under /goal.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- A VERIFIED M5 does NOT mean product Phase 1 is done. Product Phase 1 done is a separate condition confirmed only by Taylor.
- Commit execution changes on `main` before the stop; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met, or M4 not VERIFIED.
- M0 Outcome lacks a result for the `agents-neutral` destination for any consumer that the plan must name: report; consumers with no evidence are output as "not documented" (spike decision rule).
- Fixture design would require writing to this repository's `.agents/skills`: stop (hard safety constraint).

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: M4 report read; test context loaded; plan drift checked
- [ ] 2. INNOVATE: innovate-agent: approach decided; Decision Summary written
- [ ] 3. PLAN-SUPPLEMENT: plan-agent: existing phase plan updated; Inner Loop Refresh Note if sections changed (or "n/a: research clean")
- [ ] 4. PVL: vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [ ] 5. EXECUTE: all checklist items done; per-section test gates run and green (or gaps documented)
- [ ] 6. EVL: all EVL gates green; follow-up stubs registered; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, commit done, then HARD STOP at the product Phase 1 gate

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Touchpoints

- Files listed in Blast Radius
- Read: spec sections 4.5, 4.7, 4.8, 4.10, 5, 8; PRD sections 0 (D3), 5, 7.6, 7.9, 10.2, 10.3, 12, 13.2
- Umbrella `## Product Phase 1 Gate` and `## Current Execution State` (updated at UPDATE-PROCESS)

---

## Public Contracts

- Adapter id `agents-neutral`, destination `.agents/skills`, and the consumer list in plan output (PRD D3).
- Generated brief skill and its opt-out switch (config key per spec 2.1).
- Two-target selections per project (spec 2.3).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `ac_20_*` two targets, different selections | Fully-Automated | AC-20 |
| `ac_42_*` ancestor warning names the ancestor | Fully-Automated | AC-42 |
| `ac_43_*` consumers named on every `agents-neutral` plan | Fully-Automated | AC-43 |
| `ac_24_*` no agent config file touched, second adapter | Fully-Automated | AC-24 |
| `adapter_claims_have_evidence` over `agents_neutral` | Fully-Automated | PRD S9 |
| `no_background_process` plus `cargo tree` and source audit | Fully-Automated | spec 9 network none, background none |
| S1 and S6 scripted measurement on Taylor's real library (read-only, temporary destination) | Hybrid (precondition: the real library present; once, at the product Phase 1 exit) | PRD S1, S6 |
| Hermes followup wording and shared-destination notice read for honesty (no overclaim) | Agent-Probe (reviewer judgment against PRD G12) | PRD G12 honesty; AC-28 consistency |
| Product Phase 1 completeness report cross-checked against spec section 10 | Agent-Probe | PRD 12 done condition (evidence for the gate) |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: after M4 is VERIFIED, spawn vc-research-agent for RESEARCH (step 1). After UPDATE PROCESS, STOP: do not spawn anything for milestone M6 until the umbrella gate record shows Taylor's confirmation.

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

After M4 is VERIFIED, run RESEARCH for this milestone (RIPER-5), then PLAN-SUPPLEMENT and PVL before any EXECUTE. Stop at the product Phase 1 gate afterwards.
