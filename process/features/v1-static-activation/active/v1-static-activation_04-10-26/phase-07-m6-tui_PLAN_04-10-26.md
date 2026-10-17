---
name: plan:v1-static-activation-phase-07-m6-tui
description: "v1: static activation, phase 7 of 8: milestone M6, TUI picker (first part of product Phase 2; blocked by the product Phase 1 gate)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-07
---

# Milestone M6: TUI

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 7 of 8: milestone M6
**Status**: PLANNED
**Phase status:** PLANNED (product Phase 1 gate cleared by Taylor 2026-10-17; RESEARCH next)
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

**PRODUCT PHASE 2 HOLD (2026-10-04).** This milestone is product Phase 2. It must not be researched, planned further, or executed until Taylor clears the stop after M5 (see the umbrella `## Product Phase 1 Gate`). The checks recorded below (decision 11) are recorded now; nothing else in this plan changes until the stop is cleared.

---

## Overview

Milestone M6 adds the interactive picker so the primary scenario becomes one interactive step (spec section 10 M6; spec 6; PRD UX-2, UX-3, S10, AC-44). It is the first milestone of product Phase 2 and may not start until product Phase 1 is done by the PRD definition and Taylor confirms (see Entry Gate). Estimated 3 days (spec 10); the TUI is the largest discretionary item.

---

## Scope

Carried from spec section 10 M6:

- Picker, plan view, result view, and the interrupted modal (spec 6: layout, keys, markers, state machine; `ratatui` and `crossterm`).
- Bare `unam` opens the picker for the current project (spec 5).
- Parity test: every TUI action calls a core function that is also reachable from the CLI; the TUI never calls `fsops` directly (spec 6, 1.1).
- Performance checks (spec 9; decision 11, 2026-10-04), as exit-gate checks: warm start to the picker under 50 ms for a 500-skill catalog, and resident memory under 50 MB. Measured as in "Performance checks" below.

---

## Acceptance Criteria

Closes (spec section 10 M6): AC-44. Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-44 | `ac_44_*` (bare `unam`, type a query, space on a result, enter twice yields an active skill; driven through a test backend or pseudo-terminal) | Hybrid (precondition: pseudo-terminal harness or `ratatui` test backend); automated assertions on the resulting filesystem state |
| TUI parity (spec section 10 M6) | `tui_parity_*` | Fully-Automated |

### Performance checks (exit-gate checks; spec 9)

| Check | How measured |
|---|---|
| Warm start to picker under 50 ms (500-skill catalog) | Generated 500-skill fixture in a temp directory with a warm cache (one prior scan); time from process start to first picker render using the `ratatui` test backend; median of five runs in a release build on local SSD; assert under 50 ms; test `perf_warm_start_picker` |
| Memory under 50 MB resident | Run the picker on the same fixture in a release build under `/usr/bin/time -l` (macOS) or `/usr/bin/time -v` (Linux) and read the peak resident set size; assert under 50 MB; test `perf_memory_picker` |

---

## Entry Gate

- FIRST ITEM, HARD STOP GATE: the product Phase 1 gate in the umbrella plan (`## Product Phase 1 Gate`) shows `Confirmed by Taylor` with a date and the evidence message. If it reads NOT CONFIRMED or AWAITING TAYLOR, do not start any step of this milestone, including RESEARCH. This gate is never auto-proceeded under /goal. Basis: PRD section 12 (product Phase 1 "done when" AC-1 to AC-43 and AC-53 pass and Taylor uses the tool daily), spec section 10 after M5, brief "Out of scope".
- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- Milestone M5 VERIFIED with its report and the product Phase 1 completeness report.

---

## Blast Radius

- `crates/unam-cli/src/tui/mod.rs`, `app.rs`, `picker.rs`, `plan_view.rs` (and a result view module if RESEARCH splits it), `main.rs` (bare `unam` routing)
- `crates/unam-cli/tests/` new `ac_44_*` and parity tests
- Core changes only if a TUI action lacks a core entry point that the CLI can also call (record each)
- Not touched: `docs/`, `spike/`, the skill library, adapters, receipt formats

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT after RESEARCH; spec sections cited, not copied.

### Step A: Verify the gate

- [ ] A1. Read the umbrella gate record; stop if not confirmed.

### Step B: Picker

- [ ] B1. Implement the state machine `Loading, Picker, Plan, Applying, Result, Picker` and the `Interrupted` modal per spec 6.
- [ ] B2. Implement the three-pane picker (category tree, search, preview) with the keys and markers in spec 6, using `nucleo` search from M1.
- [ ] B3. Implement plan view (same content as CLI plan rendering; `y`, `n`, `e` per spec 6) and result view (six-state list, refresh guidance, followups, SES-2 caveat when anything was removed).
- [ ] B4. Route bare `unam` to the picker; keep non-interactive behavior (status and usage) for non-TTY use (spec 5).

### Step C: Tests

- [ ] C1. Write `ac_44_*` through a test backend or pseudo-terminal in a temp project; assert the destination state.
- [ ] C2. Write the parity test: enumerate TUI actions and assert each maps to a core function also reachable from the CLI.
- [ ] C3. Write `perf_warm_start_picker` and `perf_memory_picker` per the "Performance checks" table; record the measured values in the phase report.
- [ ] C4. Run the full M1 to M5 regression suite and the two invariants.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass
cargo test -p unam --test <file> ac_44
# Expected: pass
cargo test -p unam --release --test <file> perf_warm_start_picker
# Expected: pass; median warm start to picker under 50 ms (spec 9)
cargo test -p unam --release --test <file> perf_memory_picker
# Expected: pass; peak resident set size under 50 MB (spec 9)
```

- All checklist items checked; AC-44, the parity test, and both performance checks green; both invariants asserted in mutating tests.
- Regression: M1 to M5 criteria tests green.
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- Commit execution changes on `main` before starting milestone M7; no push.

---

## Blockers That Would Justify BLOCKED Status

- Product Phase 1 gate not confirmed by Taylor (this is the standing block for this milestone).
- M0 gate not met, or M5 not VERIFIED.
- No reliable way to drive the TUI in tests: fall back to the `ratatui` test backend plus a Hybrid manual run by Taylor; do not mark AC-44 Known-Gap (backlog stub and CONDITIONAL gate if unavoidable).

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: gate record verified FIRST; M5 report read; test context loaded; plan drift checked
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
- Read: spec sections 1.1, 5, 6, 9; PRD sections 7.10 (UX-2, UX-3), 12, 13.2 (AC-44), S10

---

## Public Contracts

- Bare `unam` behavior: picker on a TTY (spec 5, 6).
- Keys and markers in spec 6 are user-visible contracts.
- No new state formats; TUI is a renderer over core.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `ac_44_*` query, space, enter, enter yields an active skill | Hybrid (pseudo-terminal or test backend; assertions automated) | AC-44, S10 |
| `tui_parity_*` every TUI action reaches a core function also on the CLI | Fully-Automated | spec 10 M6 parity requirement |
| Real-terminal manual run by Taylor of the primary scenario | Hybrid (precondition: TTY, temp project) | PRD 12 product Phase 2 done condition (one interactive step) |
| `perf_warm_start_picker` median under 50 ms | Hybrid (precondition: release build, local SSD, 500-skill fixture; assertion automated) | spec 9 warm start |
| `perf_memory_picker` peak resident under 50 MB | Hybrid (precondition: release build, `/usr/bin/time`; assertion automated) | spec 9 memory |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: DO NOT START until the umbrella product Phase 1 gate record shows Taylor's confirmation. Then spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

Verify the gate record first; only then run RESEARCH for this milestone (RIPER-5).
