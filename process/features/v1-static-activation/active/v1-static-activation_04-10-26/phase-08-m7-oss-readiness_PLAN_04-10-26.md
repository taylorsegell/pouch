---
name: plan:v1-static-activation-phase-08-m7-oss-readiness
description: "v1: static activation, phase 8 of 8: milestone M7, OSS readiness (README, evidence docs, CHANGELOG, license, release workflow; blocked by the product Phase 1 gate and PRD Q11)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-08
---

# Milestone M7: OSS readiness

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 8 of 8: milestone M7
**Status**: PLANNED (product Phase 1 gate cleared 2026-10-17; M6 VERIFIED; next loop step RESEARCH, after PRD Q11)
**Phase status:** PLANNED (product Phase 1 gate cleared; BLOCKED only by PRD Q11)
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

**PRODUCT PHASE 2 HOLD (2026-10-04).** This milestone is product Phase 2. It must not be researched, planned further, or executed until Taylor clears the stop after M5 (see the umbrella `## Product Phase 1 Gate`). The checks recorded below (decision 11) are recorded now; nothing else in this plan changes until the stop is cleared.

---

## Overview

Milestone M7 makes the tool releasable and understandable by a stranger (spec section 10 M7; PRD section 12 product Phase 2 "done when": the primary scenario is one interactive step and a stranger can install and use the tool from the README; S11 install to first activation under 10 minutes). Estimated 1 to 2 days (spec 10). It is the last milestone and contains outward-facing steps, so every publish, push, and license action is a hard stop.

---

## Scope

Carried from spec section 10 M7:

- README with the primary scenario as the first example (PRD Journey A).
- `docs/adapters.md` (created by the first milestone with content for it, M2 or M4; regenerated here from `doctor --evidence`) and `docs/json.md` documenting the `--json` schemas (spec 5). Both are new spec-called documents that agents may create and maintain; only the five kickoff design documents are protected (decision 4, 2026-10-04).
- CHANGELOG.
- License file (PRD Q11 decision required; never added without asking).
- Release workflow via `cargo-dist`; `cargo install` and Homebrew tap (spec 9 binary targets: macOS universal, Linux x86_64 and aarch64).
- Crate published name and license settled per PRD Q11 (`unam` versus `unambiguous` with a `unam` binary). Q11 covers only the crates.io published name and the license; nothing before M7 depends on it; it stays an M7 entry condition (decision 5, 2026-10-04).

- Binary targets check (spec 9 binary row; decision 11): single binary via `cargo-dist` for macOS universal, Linux x86_64, and Linux aarch64.

Notes: the five kickoff design documents (`docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`) are never edited by agents without Taylor's explicit per-edit permission. Publishing to crates.io, creating or pushing a Homebrew tap, and pushing tags or any remote are NOT part of this plan's execution; they are Taylor's explicit go-ahead only.

---

## Acceptance Criteria

Closes: none listed in spec section 10 for M7. Supports PRD success criterion S11 (install to first activation under 10 minutes for a new user following the README) and S9 (no adapter claim without evidence URL).

| Criterion | Proven by | Strategy |
|---|---|---|
| S11 (PRD 13.1) | clean-environment walkthrough of the README to first activation | Hybrid (precondition: clean macOS or Linux account or container; Taylor or reviewer follows the README) |
| S9 (PRD 13.1) | `doctor --evidence` lists a URL or spike result for every claim; `docs/adapters.md` matches it | Fully-Automated (output check) plus Agent-Probe (doc match) |
| Release workflow validity | local `cargo-dist` plan or dry run, no publish | Hybrid (precondition: `cargo-dist` installed) |
| Binary targets (spec 9): the local `cargo dist plan` output lists artifacts for macOS universal, Linux x86_64, and Linux aarch64 and a single binary `unam` | local `cargo dist plan` output inspected (no publish) | Hybrid (precondition: `cargo-dist` installed; assertion on the plan output automated) |

---

## Entry Gate

- FIRST ITEM, HARD STOP GATE: the product Phase 1 gate in the umbrella plan (`## Product Phase 1 Gate`) shows `Confirmed by Taylor` with date and evidence. If NOT CONFIRMED or AWAITING TAYLOR, do not start any step, including RESEARCH. Never auto-proceeded under /goal.
- Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- Milestone M6 VERIFIED with its report.
- PRD Q11 (crates.io published name and license) decided by Taylor before this milestone starts (brief: "before M7"). No license file is created before then.

---

## Blast Radius

- `README.md`, `CHANGELOG.md`, license file (only after Q11 decision)
- `docs/adapters.md`, `docs/json.md` (new spec-called documents; agents may create and maintain them; see Notes)
- Release workflow file and `cargo-dist` config; crate and package metadata in `Cargo.toml` files (name, license, description) if Q11 renames anything
- Not touched: skill library, `spike/`, toolkit directories; no remote, registry, or tap is modified

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT after RESEARCH; spec sections cited, not copied.

### Step A: Verify gates

- [ ] A1. Read the umbrella gate record; stop if not confirmed.
- [ ] A2. Confirm Q11 (crates.io published name and license) is decided and recorded.

### Step B: Documentation

- [ ] B1. Write README with the primary scenario (PRD Journey A) as the first example and the honesty caveats (PRD G12, SES-2).
- [ ] B2. Regenerate `docs/adapters.md` (created by milestone M2 or M4) from `unam doctor --evidence` run against temporary directories only; never run `unam` against this repository. Cursor entries state that the evidence is for the `cursor-agent` CLI only and the Cursor editor is not tested.
- [ ] B3. Write `docs/json.md` from the `insta` snapshots (spec 5 envelope).
- [ ] B4. Write CHANGELOG for v1.

### Step C: Release mechanics (local only)

- [ ] C1. Apply Q11 naming to package metadata; add the license file chosen by Taylor.
- [ ] C2. Finalize the `cargo-dist` release workflow (targets: macOS universal, Linux x86_64, Linux aarch64; spec 9) and the install story (`cargo install`, Homebrew tap formula as a file only).
- [ ] C3. Run a local `cargo dist plan` or build and check the output lists the three spec 9 binary targets and a single binary; do not publish, push, tag remotely, or create a tap repository.

### Step D: Tests and walkthrough

- [ ] D1. Re-run the full cargo gates and the M1 to M6 regression suite.
- [ ] D2. Clean-environment README walkthrough timed against S11.
- [ ] D3. Prepare a release handoff list for Taylor (what to publish, in what order). Taylor executes it, not an agent.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0
cargo test --workspace
# Expected: all tests pass
cargo run -p unam -- doctor --evidence   # against temp dirs via env overrides only
# Expected: every adapter claim listed with a URL or spike result
```

- README, CHANGELOG, `docs/adapters.md`, `docs/json.md`, license, and release workflow exist (the license is written only after Taylor decides Q11; no edit to any of the five protected design documents).
- Local `cargo dist plan` lists macOS universal, Linux x86_64, and Linux aarch64 and a single binary (spec 9).
- README walkthrough meets S11 or the shortfall is recorded.
- No publish, push, or tag was performed by an agent.
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means artifacts exist with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- Program completion: when M7 is VERIFIED, the program is complete as scoped; later work (product Phase 3, v2 items) moves to a new feature folder or backlog, not to this program.
- Commit execution changes on `main`; no push.

---

## Blockers That Would Justify BLOCKED Status

- Product Phase 1 gate not confirmed by Taylor.
- PRD Q11 (crate name and license) undecided: no license file, no rename; report to Taylor.
- M0 gate not met, or M6 not VERIFIED.
- Any step that needs publish, push, crates.io, or Homebrew tap action: stop and hand to Taylor.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: gate record and Q11 verified FIRST; M6 report read; test context loaded; plan drift checked
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
- Read: spec sections 5, 9, 10 M7, 11; PRD sections 6 (Journey A), 12, 13.1, 15 (Q11), 16

---

## Public Contracts

- README install and first-use instructions, `docs/adapters.md`, `docs/json.md`: public documentation of contracts established in earlier milestones.
- Crate and package names and the license: public and hard to reverse once published (publishing itself is out of this plan).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `cargo run -p unam -- doctor --evidence` lists evidence for every claim | Fully-Automated | S9 |
| `docs/adapters.md` matches `doctor --evidence` output | Agent-Probe (reviewer diff judgment) | S9 |
| Clean-environment README walkthrough to first activation, timed | Hybrid (clean account or container) | S11 |
| Local `cargo-dist` plan or build without publish | Hybrid (precondition: `cargo-dist` installed) | spec 10 M7 release workflow |
| M1 to M6 regression suite | Fully-Automated | AC-1, AC-3 to AC-43 (except AC-2), AC-44, AC-53 still green |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`
- Next step: DO NOT START until the umbrella gate record shows Taylor's confirmation, M6 is VERIFIED, and PRD Q11 is decided. Then spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

Verify the gate record and Q11 first; only then run RESEARCH for this milestone (RIPER-5).
