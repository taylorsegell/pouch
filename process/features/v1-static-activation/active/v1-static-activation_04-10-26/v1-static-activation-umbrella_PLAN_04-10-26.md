---
name: plan:v1-static-activation-umbrella
description: "v1: static activation, umbrella/orchestration plan for the 8-milestone program (M0 to M7) with a hard stop after M5"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: umbrella
---

# v1: static activation - Umbrella Plan

**Date**: 04-10-26
**Complexity**: COMPLEX
**Status**: PLANNED

- Program type: PHASE PROGRAM (8 toolkit phases, one per spec milestone M0 to M7, sequential with gated joins)
- Feature folder: `process/features/v1-static-activation/`
- Program task folder: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/`
- Vocabulary: "product Phase 1/2/3" means the PRD phases. Toolkit phases are called milestones (M0 to M7) in all prose. Product Phase 3 means dynamic routing and is not part of this program.

---

## Overview

Build `unam` v1 (the Rust CLI described in `docs/spec/unam-technical-spec.md`) milestone by milestone: M0 to M5 deliver product Phase 1 (selection and activation); M6 and M7 deliver the first part of product Phase 2 (picker, release readiness). There was a HARD STOP after M5; Taylor cleared it on 2026-10-17 (see "Product Phase 1 Gate" below), so M6 and M7 proceed. Milestones M6 and M7 are product Phase 2. Their plans carry only the check additions recorded on 2026-10-04 (decision 11); nothing else in them changes.

This plan sequences the milestones, holds the Program Goal Charter and the Stable Program Goal block, and records open items. It does not restate the spec or PRD: every milestone plan cites spec and PRD sections by number. Milestone M0 is the gate for everything: no application code and no Cargo files exist before the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter (brief rule; all-context standing rule).

Phased Delivery Plan: see "Phase Ordering" below (one milestone per toolkit phase).

Source documents (read-only for agents): the five kickoff design documents `docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`, plus `spike/symlink-discovery.md` (authoritative for the M0 procedure). Spec and PRD were edited once by Taylor on 2026-10-04 (commit ae120c5); these plans follow that version. Context: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`.

---

## Program Goal Charter

```
v1: static activation - Program Goal Charter

North star:
- Ship `unam` v1: a single Rust binary that lets Taylor choose a small set of skills from the
  external library, activate it per project and per agent, and verify, repair, and undo it, so the
  agent chooses from a few relevant skills instead of the whole library (brief: Why it exists).

Definition of done (an unattended agent must be able to do all of these, M0 to M5):
1. Run or consume the M0 spike results and set the default strategy per adapter from evidence.
2. Register sources, scan the catalog, search, and show skills without touching the source
   (spec 4.2, 4.3; PRD 7.1).
3. Create and edit Hermes-compatible bundles and a per-project selection, and resolve them to a
   flat desired set with provenance, erroring on ambiguity (spec 4.4 to 4.6).
4. Plan and apply a selection to claude-code with a journal, receipt, history, resume, and rollback,
   never overwriting foreign files (spec 4.7 to 4.10).
5. Verify with the six states, report status, reconcile, and roll back (spec 4.11, 4.12).
6. Do the same for the agents-neutral adapter with shared-destination reporting and generated briefs.
7. After the gate and Taylor's confirmation only (M6, M7): picker, then README, evidence docs,
   CHANGELOG, and release readiness.

What "verified" means (program level):
- A milestone is VERIFIED only when all of spec section 8 holds for its scope: `cargo fmt --all
  --check`, `cargo clippy --workspace --all-targets -- -D warnings` (unwrap denied outside tests),
  `cargo test --workspace`, and the `ac_NN_*` integration test for every acceptance criterion the
  milestone closes (spec section 10) are green.
- Every mutating test asserts both invariants: the source fixture tree hash (blake3) is unchanged,
  and the foreign destination fixture file is byte-identical (spec 8; PRD S3, S4).
- Regression checks against previously verified milestone surfaces pass, and validate-contract gates
  are recorded with phase gates and regression evidence. A milestone without a validate-contract (or
  documented skip reason) cannot be marked VERIFIED.
- M0 has no cargo gate: it is VERIFIED when the spike results table is filled in and the Outcome
  section records the default strategy per adapter and any contradicted adapter claim.
- Product Phase 1 done = AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1
  unless it sits under the "Phase 2" or "Phase 3" heading) AND Taylor uses the tool daily (PRD 12).
  Success criteria are PRD section 13.1; the milestone plans carry the S-checks as exit-gate checks.

Scope tiers -> milestone mapping:
- Tier 1 Gate (spike, evidence for adapter claims) -> M0
- Tier 2 Read path and desired state (product Phase 1) -> M1, M2
- Tier 3 Mutation and recovery for claude-code (product Phase 1) -> M3, M4
- Tier 4 Second adapter and generated briefs; completes product Phase 1 -> M5
- ---- HARD STOP: product Phase 1 gate, Taylor's explicit confirmation ----
- Tier 5 Interactive picker and release readiness (first part of product Phase 2) -> M6, M7
- This program retires Tiers 1-4 autonomously and Tier 5 only after the gate.

Explicitly out of scope (deferred tier):
- Product Phase 3, intent-driven exposure, dynamic routing, atlas, hooks (PRD 12; ADR-0001; gated by D8).
- v2 items in spec section 11: pin mode, isolated cursor and windsurf-devin adapters, adopt (OWN-6),
  bundle composition and export, remote sources (SRC-7), committed-selection alias indirection, native
  Hermes adapter, Windows. Milestones M6 and M7 are product Phase 2; the stop was cleared on 2026-10-17.
- Context offloading (PRD D6). A repository map for orientation (not part of any phase).
- Any plan, code, or doc work for the above beyond a pointer in `process/general-plans/backlog/`.

Hard safety constraints (non-negotiable, per milestone; from the brief's Rules):
- The skill library at ~/Developer/01-AI/skillzone/ is read-only. Inspect freely; never write.
- Never run `unam` against this repository (the toolkit owns `.claude/skills/` here and links
  `.agents/skills` to it). All `unam` tests use temporary directories and fixtures; env overrides
  (`UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`) always point at temp dirs.
- No publishing to crates.io, no Homebrew tap publication, no push to any remote without Taylor's
  explicit go-ahead.
- No license file without asking (PRD Q11, crates.io published name and license, is an M7 entry condition).
- The five kickoff design documents (docs/PROJECT-BRIEF.md, docs/KICKOFF-PROMPT.md,
  docs/spec/PRD-unambiguous.md, docs/spec/unam-technical-spec.md, docs/adr/ADR-0001-dynamic-skill-routing.md)
  are never edited, moved, renamed, or rewritten by agents without Taylor's explicit per-edit permission.
  New docs the spec calls for (docs/adapters.md, docs/json.md) are allowed and agents maintain them.
  Contradictions in the five documents are reported to Taylor, not resolved.
- No application code or Cargo files before M0's results table is filled in.
- No em dashes and no emojis in any file.
- Commit each milestone's execution changes on `main` before starting the next; keep
  process/plan/context commits separate from execution commits; never push.
```

---

## Stable Program Goal

```
SESSION GOAL: v1-static-activation: v1: static activation (milestones M0 to M7, STOP after M5)
Ref: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

TARGET: Complete milestones M0 to M5, then STOP. Done when:
- each milestone Exit Gate passes and the spec 10 acceptance criteria it closes are green as ac_NN_* tests
- M0 is done only when spike/symlink-discovery.md table is filled and Outcome names the default strategy per adapter
- Test tiers: automated (iterate-until-green) / hybrid (fix-if-in-blast-radius) / agent-probe (record-judgment)
- M6 and M7 (product Phase 2): the stop was cleared by Taylor on 2026-10-17 (PRD 12); they proceed under the same loop.

AUTONOMY: Before ANY subagent spawn read umbrella ## Current Execution State, then the milestone plan ## Phase Loop Progress; first unchecked box = next spawn.

PER-MILESTONE LOOP (7-step inner loop R,I,P,PVL,E,EVL,UP; never skip or reorder; SKIPS SPEC):
 1 RESEARCH 2 INNOVATE 3 PLAN-SUPPLEMENT 4 PVL 5 EXECUTE 6 EVL 7 UPDATE-PROCESS
- PVL NEVER skipped; placeholder or partial validate-contract = blocked
- Every subagent FIRST ACTION: vc-context-discovery + vc-plan-discovery
- Every phase-END: invoke vc-agent-strategy-compare for the next step
- EXECUTE runs opus; every other step sonnet

HARD STOPS (pause, wait for Taylor):
- GATE: CLEARED by Taylor on 2026-10-17 (daily use, D4, S5 deferred until after M7). M6 and M7 proceed.
- M0 gate: no application code or Cargo files until spike table filled. Taylor runs the spike by hand.
- Any push, publish, crates.io, Homebrew tap, license file
- Doc contradictions: report to Taylor, never resolve
- Net gate BLOCKED with no backlog path

SAFETY (never override):
- ~/Developer/01-AI/skillzone/ read-only
- Never run unam against this repo; tests use temp dirs and env overrides only
- 5 kickoff design docs (brief, kickoff prompt, PRD, spec, ADR-0001) never edited without Taylor's per-edit OK; new spec-called docs allowed
- No em dashes, no emojis in any file
- Commit each milestone on main before advancing; process and execution commits separate; no push

TEST GATES (every milestone exit; cargo gates from M1):
 cargo fmt --all --check
 cargo clippy --workspace --all-targets -- -D warnings
 cargo test --workspace
 cargo test -p unam --test <file> ac_NN
 node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs
 node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs
 node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs
 node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <plan>
 git diff --check
 Mutating tests assert: source tree hash unchanged; foreign dest file byte-identical.

VALIDATE CONTRACT: per milestone, written by vc-validate-agent into each plan before EXECUTE.

START: Milestone M4 (phase 5 of 8), loop step 1 RESEARCH.
```

---

## Product Phase 1 Gate (HARD STOP after M5)

This gate sits between milestone M5 and milestone M6. It is never auto-proceeded, including under /goal.

- Why: PRD section 12 (product Phase 1 "done when" the acceptance criteria pass and the author uses the tool daily instead of the manual workflow; PRD 13.2 rule: every AC is product Phase 1 unless under the "Phase 2" or "Phase 3" heading); spec section 10 after M5 ("Phase 2 does not start before that"); brief "Out of scope" (product Phase 2 until Phase 1 is done by the PRD definition).
- What blocked: milestone M6 (TUI) and milestone M7 (OSS readiness) until the gate was confirmed. It is now confirmed (record below).
- Preconditions the orchestrator checks and reports (it does not decide):
  1. Milestones M0 to M5 are VERIFIED with phase reports.
  2. AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1 unless it sits under the "Phase 2" or "Phase 3" heading). The set closed by M1 to M5 per spec section 10 is exactly this set (coverage map below; AC-2 closes in M3, AC-33 in M4).
  3. Taylor uses the tool daily (brief name of the first real adopting project, due end of M3).
  4. The S1 and S6 measurements on Taylor's real library are recorded in the M5 report (read-only against the library).
- Confirmation: only an explicit message from Taylor stating that product Phase 1 is done (AC-1 to AC-43 and AC-53 pass and daily use) and M6 may start. The orchestrator then records it in the record below and in `## Current Execution State`. No agent may fill in the record on its own judgment.
- Gate record: Confirmed by Taylor: CLEARED. Date: 2026-10-17. Evidence message (given directly in session, recorded by the orchestrator): Taylor chose "Clear gate, continue M6+M7" and said "Daily will begin once we finish all of the milestones". Deferred until all milestones are done: daily use, D4 (the first adopting project), and S5. Open items for after M7 are listed in `## Current Execution State`.
- Where it is enforced: this section, the Stable Program Goal HARD STOPS, the charter, the M5 plan Exit Gate (final item), and the M6 plan Entry Gate (first item).

---

## Phase Ordering

One toolkit phase per milestone. Phase numbers are 1-indexed; milestone numbers start at M0.

| Phase | Milestone | Plan file | Scope summary | Depends on |
|---|---|---|---|---|
| 1 of 8 | M0 Gating spike (gate for everything) | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_PLAN_04-10-26.md` | Manual symlink discovery spike; fill results table; record default strategy per adapter | none |
| 2 of 8 | M1 Core read path | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md` | Workspace, CI, config, source, catalog, frontmatter, search, show | M0 |
| 3 of 8 | M2 Bundles and selection | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_PLAN_04-10-26.md` | Bundles, project root and selection IO, resolve | M1 |
| 4 of 8 | M3 Plan and apply for claude-code | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md` | Adapter trait, claude_code, fsops, plan::diff, apply, add/rm/use/deactivate | M2 |
| 5 of 8 | M4 Verify, status, reconcile, rollback | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md` | Six-state verify, status, reconcile, rollback, doctor | M3 |
| 6 of 8 | M5 agents-neutral and generated briefs | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_PLAN_04-10-26.md` | Second adapter, shared-destination reporting, bundle brief. Completes product Phase 1 | M4 |
| GATE | Product Phase 1 gate | this plan, section "Product Phase 1 Gate" | Taylor's explicit confirmation | M5 VERIFIED |
| 7 of 8 | M6 TUI | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md` | PRODUCT PHASE 2, gate cleared 2026-10-17. Picker, plan view, result view, interrupted modal | Gate |
| 8 of 8 | M7 OSS readiness | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md` | PRODUCT PHASE 2, gate cleared 2026-10-17. README, evidence docs, CHANGELOG, license, release workflow | M6, Q11 decided, Gate |

### Join Conditions

- M1 MUST NOT start until M0 exit gate passes (results table filled, Outcome records default strategy per adapter). Every later milestone plan also lists this as an entry dependency.
- M2 MUST NOT start until M1 exit gate passes; M3 after M2; M4 after M3; M5 after M4.
- M6 MUST NOT start until M5 exit gate passes AND the Product Phase 1 Gate is confirmed by Taylor (met 2026-10-17).
- M7 MUST NOT start until M6 exit gate passes, the Product Phase 1 Gate is confirmed, and PRD Q11 (crates.io published name and license) is decided by Taylor. Q11 is only an M7 entry condition; nothing before M7 depends on it.

---

## Per-Phase Entry / Exit Gates

| Phase | Entry | Exit gate |
|---|---|---|
| M0 | Program start | `spike/symlink-discovery.md` results table filled (tested rows with versions and dates, or "not tested"); Outcome section records default strategy per adapter and any contradicted spec claim; Taylor confirms |
| M1 | M0 exit met | fmt, clippy, test green; AC-1, 3, 4, 5, 6, 38 (read-side) tests green; first test `source_tree_unchanged_read_side` green (PRD S3 read-side); cold scan of a generated 500-skill fixture under 300 ms (spec 9; median of five release runs, `perf_cold_scan`) |
| M2 | M1 exit met | gates green; AC-7 to AC-12, AC-35 (desired side) tests green; Hermes slugify parity decided |
| M3 | M2 exit met | gates green; AC-2, 13, 15, 16, 17, 18, 19, 21, 22, 23, 24, 25, 29, 30, 31, 32, 40, 53 tests green; S2 (`s2_one_command_add`), S3, S4 (invariants in every mutating test), S9 for claude-code (`adapter_claims_have_evidence`) green; interruption test proves consistency by comparing destination and receipt to expected state; first adopting project named |
| M4 | M3 exit met | gates green; AC-14, 26, 27, 28, 33, 34, 36, 37, 39, 41 tests green; S7 (`s7_interruption_zero_divergence`) and S8 (`s8_divergence_always_reported`) green; interruption test also asserts `reconcile` reports zero divergence |
| M5 | M4 exit met | gates green; AC-20, 42, 43, 24 (second adapter) tests green; S9 for agents-neutral green; S1 and S6 measured once on Taylor's real library (read-only) and recorded; network none and background none confirmed (`cargo tree`, source audit, `no_background_process`); product Phase 1 completeness report written (AC-1 to AC-43 and AC-53); S5 follow-up recorded; STOP at the gate |
| M6 | M5 exit met and Gate confirmed (met 2026-10-17; product Phase 2) | gates green; AC-44 test green; TUI parity test green; warm start to picker under 50 ms and memory under 50 MB (spec 9) |
| M7 | M6 exit met, Gate confirmed (met 2026-10-17), Q11 decided (product Phase 2) | README, docs, CHANGELOG, license, release workflow in place; local `cargo dist plan` lists macOS universal, Linux x86_64, Linux aarch64 (spec 9 binary row); no publish performed |

---

## Per-Phase Loop

Each milestone executes the canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP`, which SKIPS SPEC (SPEC runs once in the outer program loop). The authoritative description is `process/development-protocols/phase-programs.md`; it is not restated here. Each milestone plan carries its own `## Phase Loop Progress` checklist. PVL is never skipped; a placeholder or partial `## Validate Contract` means blocked.

Re-research at every milestone entry is mandatory (read the immediately prior milestone report in full, earlier reports' Forward Preview only).

---

## Phase Loop Progress

Program-level summary. The authoritative per-step checkboxes are in each milestone plan. Orchestrator reads the milestone plan, not this table, to choose the next subagent.

| Milestone | Next loop step | Note |
|---|---|---|
| M0 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-05. Report: `phase-01-m0-symlink-spike_REPORT_04-10-26.md`. Default strategy: symlink for both adapters |
| M1 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-05. Report: `phase-02-m1-core-read-path_REPORT_04-10-26.md`. 164 tests, G1 to G13 green |
| M2 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-05. Report: `phase-03-m2-bundles-and-selection_REPORT_04-10-26.md`. G1 to G14, G12b, E1 green; 13 design departures awaiting Taylor: items 24 to 36 |
| M3 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-09. Report: `phase-04-m3-plan-apply-claude-code_REPORT_04-10-26.md`. G1 to G19 green, 512 tests; review decision approved by Taylor; D4 real activation pending |
| M4 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-12. Report: `phase-05-m4-verify-status-reconcile-rollback_REPORT_12-10-26.md`. G1 to G6 green, 567 tests; commit pending in the same wave |
| M5 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-17. Report: `phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md`. EVL CLEAN (3 iterations; 655 tests at 4832e31), full fault matrix. Product Phase 1 Gate cleared 2026-10-17 |
| M6 | complete (steps 1 to 7 done) | VERIFIED (WITH_GAPS) 2026-10-17. Report: `phase-07-m6-tui_REPORT_17-10-26.md`. EVL PASS in 2 iterations, 699 tests; Taylor's manual AC-44 terminal run outstanding |
| M7 | 1 RESEARCH (not started) | Product Phase 2, gate cleared; PRD Q11 (crate name and license) needed first |

---

## Pre-PVL Conflict Resolution

Written as a placeholder by vc-plan-agent; the orchestrator fills it before outer PVL begins.

Blast-radius overlap between milestones is intentional and sequential (later milestones extend crates created earlier: `unam-core`, `unam-cli`, `crates/unam-cli/tests/`). Because milestones run strictly one after another, there is no concurrent write conflict. Classification for the shared packages `crates/unam-core` and `crates/unam-cli`: sequential-owner by milestone order (the earlier milestone owns creation; the later milestone owns its additions). Orchestrator to confirm or amend before outer PVL.

---

## Execution Strategy (per milestone, short)

Recorded per `vc-agent-strategy-compare`. All milestones form a dependency chain, so the between-milestone strategy is sequential. Model policy: EXECUTE runs opus; research, innovate, plan, validate, EVL, update-process run sonnet. Within a milestone, parallel subagents or an agent team are allowed only for disjoint file groups (for example M2: `bundle.rs` versus `project.rs`); the milestone's PLAN-SUPPLEMENT step decides.

| Milestone | Signals present | Recommended |
|---|---|---|
| M0 | none (manual) | sequential; no EXECUTE agent for the spike itself |
| M1 | multi-package, 5+ files | sequential, opus EXECUTE |
| M2 | 5+ files | sequential, optional 2-way parallel on disjoint modules |
| M3 | 5+ files, high-risk class (destructive filesystem writes) | sequential, opus EXECUTE, hybrid gates |
| M4 | 5+ files, high-risk class | sequential, opus EXECUTE |
| M5 | 5+ files, shared-destination behavior | sequential, opus EXECUTE |
| M6 | UI surface (product Phase 2) | sequential, opus EXECUTE |
| M7 | outward-facing release mechanics (product Phase 2) | sequential; publish actions stay hard stops |

Plan authoring for this program was done by one agent: each plan owns a distinct file, and blast radii are recorded in `phase-blast-radius-registry.md`, so no mid-draft coordination was needed.

---

## Autonomous Execution Rules (During /goal)

- The agent self-decides at all V5 gates. CONDITIONAL net gate: proceed with fixes and gaps on record. BLOCKED net gate: write a backlog note, continue.
- Hard stops (must pause for Taylor): (the Product Phase 1 Gate was cleared on 2026-10-17); the M0 spike (Taylor runs it); push, publish, crates.io, Homebrew tap, license file; any outward-facing or irreversible action not named in the validate-contract; contradictions between documents (report, do not resolve).
- The Product Phase 1 Gate was cleared by Taylor on 2026-10-17; the earlier goal ended at M5 UPDATE-PROCESS.
- The phase report is the communication channel for conflicts, errors, and learnings.
- Subagent delegation (no inline execution) remains mandatory.

---

## Global Constraints

- No application code or Cargo files before the M0 results table is filled in.
- Spec and PRD are authoritative; a plan that disagrees with them loses.
- Test paths: every test points `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`, and any user-scope scan dir at `tempfile` directories; never at real config, `~/.claude/skills`, `~/.agents/skills`, the skill library, or this repository.
- Integration tests live in `crates/unam-cli/tests/` (package `unam`), named `ac_NN_*`.
- `unwrap` denied outside tests by clippy configuration.
- Run `node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs` and the plan validators after any change to plan files.
- Commit each milestone's execution changes before starting the next; process commits stay separate.

---

## Durable Report Destinations

All inside the program task folder, flat.

| Phase | Report path |
|---|---|
| M0 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_REPORT_04-10-26.md` |
| M1 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_REPORT_04-10-26.md` |
| M2 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_REPORT_04-10-26.md` |
| M3 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-04-m3-plan-apply-claude-code_REPORT_04-10-26.md` |
| M4 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_REPORT_04-10-26.md` |
| M5 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md` |
| M6 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_REPORT_04-10-26.md` |
| M7 | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_REPORT_04-10-26.md` |

---

## Acceptance Criteria Coverage Map

Source: spec section 10 ("Closes" lines, as edited in commit ae120c5); criteria text in PRD section 13.2 (not restated). Strategy for every row: Fully-Automated integration test `ac_NN_*` in `crates/unam-cli/tests/`, except where the milestone plan names a Hybrid or Agent-Probe gate.

| Milestone | Closes (per spec 10) |
|---|---|
| M0 | none (evidence for adapter claims; PRD S9 support; Cursor rows are labeled `Cursor (cursor-agent CLI)`, evidence for that CLI only, the Cursor editor is not tested) |
| M1 | AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side) |
| M2 | AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-35 (desired side) |
| M3 | AC-2, AC-13, AC-15, AC-16, AC-17, AC-18, AC-19, AC-21, AC-22, AC-23, AC-24, AC-25, AC-29, AC-30, AC-31, AC-32, AC-40, AC-53 |
| M4 | AC-14, AC-26, AC-27, AC-28, AC-33, AC-34, AC-36, AC-37, AC-39, AC-41 |
| M5 | AC-20, AC-42, AC-43, AC-24 (second adapter) |
| M6 | AC-44 |
| M7 | none listed in spec 10 |
| not in this program | AC-45 is product Phase 2 pinning (spec 11 v2 theme); AC-46 to AC-52, AC-54 to AC-56 are product Phase 3 and out of scope |

Check (2026-10-04): every AC-1 to AC-43 and AC-53 is mapped exactly once, except AC-24, which appears in M3 (claude-code) and M5 (agents-neutral) by design (second adapter). AC-2 closes in M3 (needs apply for the full activate, modify, deactivate cycle); M1 supplies the source-tree hash helper and the read-side test as the first test. AC-33 closes in M4 (rollback from history). AC-37 closes in M4 (`source relocate`). AC-44 is M6 (product Phase 2).

---

## Contradictions and Open Items

Taylor's twelve decisions of 2026-10-04 are applied below (spec and PRD edits are in commit ae120c5). Items still open are marked STILL OPEN. Items 15 to 21 were added 2026-10-05 at the milestone M1 PLAN-SUPPLEMENT (reports to Taylor, resolved provisionally by INNOVATE under autopilot). Items 22 and 23 were added 2026-10-05 at milestone M1 PVL cycle 1. Items 24 to 36 were added 2026-10-05 at the milestone M2 PLAN-SUPPLEMENT (reports to Taylor, resolved provisionally by INNOVATE under autopilot). As of 2026-10-09 the remaining open items (7, 10, 15 to 55) are being settled through `open-items-decision-sheet_REF_09-10-26.md`: proposed answers apply unless Taylor marks rows. Items 37 to 54 were added 2026-10-05 at the milestone M3 PLAN-SUPPLEMENT (reports to Taylor, resolved provisionally by INNOVATE under autopilot; they come from the 14 INNOVATE decisions plus the spec-text mismatches found when the spec and PRD were read, because INNOVATE did not re-read them).

1. RESOLVED 2026-10-04 (decision 1). AC-2 closes in M3 (needs apply for the full cycle). M1 writes the source-tree hash helper and the read-side test (register, scan, search leave the source tree hash unchanged) as the FIRST test; the invariant is asserted in every mutating test from M3 on. The M1 "AC-2 open item" is replaced by these tasks.
2. RESOLVED 2026-10-04 (decision 2; built and tested in M4 2026-10-12, AC-37 met). `source relocate` is built in M4 (needs receipts and the project index). M4 closes AC-37.
3. RESOLVED 2026-10-04 (decision 3). Journal-based `resume` and `rollback` belong to M3 (spec 4.10). Rollback from history and `reconcile` stay in M4. AC-31 and AC-32 stay in M3; AC-33 moves to M4. In M3 the interruption test proves consistency by comparing destination and receipt to the expected state; M4 adds the "reconcile reports zero divergence" assertion to that test. The M3/M4 RESEARCH flags on this are removed.
4. RESOLVED 2026-10-04 (decision 4). Only the five kickoff design documents are protected (docs/PROJECT-BRIEF.md, docs/KICKOFF-PROMPT.md, docs/spec/PRD-unambiguous.md, docs/spec/unam-technical-spec.md, docs/adr/ADR-0001-dynamic-skill-routing.md): never edited by agents without Taylor's explicit per-edit permission. New spec-called docs (docs/adapters.md, docs/json.md) may be created and maintained by agents (M4 creates docs/adapters.md, M7 regenerates it and writes docs/json.md). The M0 Outcome may be recorded in docs/adapters.md once it exists.
5. RESOLVED 2026-10-04 (decision 5). Proceed with the spec: crates `unam-core` and `unam-cli`, package and binary `unam`. PRD Q11 is only the crates.io published name and the license; nothing before M7 depends on it. Q11 stays an M7 entry condition. The M1 conflict note is removed.
6. RESOLVED 2026-10-04 (decision 6). `spike/symlink-discovery.md` is authoritative for the M0 procedure; spec 10 M0 now points to it. The spec/spike contradiction is closed.
7. STILL OPEN (partly resolved). Spec 1.3 lists `hash.rs`, `state.rs`, `error.rs`, `fsops.rs` but spec section 10 does not assign each to a milestone. `hash.rs` is now created in M1 (decision 1: the hash helper). The first-needed milestone for the rest is decided at each milestone's RESEARCH (M1 creates the error skeleton; M3 plans `fsops.rs` and `state.rs`).
8. RESOLVED 2026-10-04 (decision 11). Spec section 9 targets and PRD S1 to S9 are exit-gate checks in the milestone plans: M1 cold scan under 300 ms on a generated 500-skill fixture; M3 S2, S3, S4, S9 (claude-code); M4 S7, S8; M5 S9 (agents-neutral), S1 and S6 measured once on Taylor's real library at the product Phase 1 exit (read-only), network none and background none confirmed; M6 warm start under 50 ms and memory under 50 MB; M7 binary targets. S5 is observational (see "Follow-ups (not gates)").
9. RESOLVED 2026-10-04 (decision 10). Product Phase 1 done = AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1 unless under the "Phase 2" or "Phase 3" heading) AND Taylor uses the tool daily. No PRD section 7 computation is needed; the AC-2 caveat is gone.
10. STILL OPEN. Toolkit shape notes: phase-stub template links the umbrella as `{program-slug}-umbrella_{date}/` (a folder) while the flat layout puts it in the program folder; template says "Phase Sequence" while validators and protocol say "Phase Ordering"; both phase and umbrella validators auto-scan only files directly in `active/` (not task folders), so they must be run with explicit paths; templates contain em dashes and emoji status values (replaced here); the template's "Phase 0 pre-program" row clashes with milestone M0 (this program maps M0 to phase 1 of 8); `validate-plan-artifact.mjs` requires Acceptance Criteria and Phase Completion Rules that the phase-stub template lacks (added).
11. RESOLVED 2026-10-04 (decision 7). The spike run order (hide one folder per run for Claude Code and Cursor) is accepted; marked resolved in the M0 plan.
12. RESOLVED 2026-10-04 (decision 8). PRD 12 now says RTE-1 to RTE-12 for product Phase 3 "In:". No plan in this program references the old range; closed.
13. RESOLVED 2026-10-04 (decision 9). OWN-6 (`adopt`) and SRC-7 (remote sources) are product Phase 2 and now listed in spec 11. The charter out-of-scope list matches; closed.
14. RESOLVED 2026-10-04 (decision 12). The Hermes trust write in the M0 runbook is accepted as planned (backup, single write, remove in cleanup, show the diff before deleting); marked resolved in the M0 plan; no runbook change.
15. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; resolved provisionally by INNOVATE under autopilot). Spec 1.4 says "Library on macOS" for the `directories` crate, which contradicts spec section 2 and PRD section 9 (both give `~/.config/unam`, `~/.cache/unam`, `~/.local/share/unam` on macOS and Linux). PRD 9 followed; the `directories` crate is dropped and `paths.rs` resolves the paths. Reported, not resolved in the design documents.
16. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). The data dir (project index, `projects.json`) has no `UNAM_*` override in spec section 2 or PRD 9. Not added in M1 (the three documented overrides are unchanged); tests isolate it through `HOME` and `XDG_DATA_HOME`. Taylor to confirm whether a fourth override is wanted.
17. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). The spec has no `Diagnostic` type and no catalog-level diagnostics, yet spec 4.2, 4.3, and AC-5 require named diagnostics. M1 adds `Diagnostic { code, severity, message, path, remedy }`, per-skill diagnostics, and a catalog-level list carrying the source alias; JSON arrays are always present. Code names are recorded in the M1 plan as stable contract.
18. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). M1 gets a temporary `lookup(ref)` in the catalog module that handles all three spec 4.6 reference forms (exit 4 with candidates, exit 5 listing every candidate, never picking) so `show` works. It is explicitly temporary: milestone M2's `resolve.rs` supersedes it. M1 also reads a provisional `<data dir>/projects.json` for the `source rm` exit-3 check (index absent means no references); milestone M2's `project.rs` owns the real index format and replaces that reader.
19. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). `dist-workspace.toml` is hand-written and unverified (`dist` is not installed; milestone M7 verifies with `dist plan`). Spec section 9 names a macOS universal binary, but the research evidence does not support a universal target, so M1 lists per-arch targets (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`) and makes no universal claim. The M7 entry in the Per-Phase Exit Gate table still says "macOS universal"; to be reconciled when milestone M7 is researched.
20. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). M1 CLI search uses `nucleo-matcher` (Pattern::parse, match_list) instead of the full `nucleo` crate named in spec 1.4. The picker in milestone M6 may adopt full `nucleo` if it needs incremental matching. Match-index output (spec 4.3) is deferred to milestone M6.
21. DROPPED for v1 (decision sheet row 21 proposed answer; M4 did not build it; workaround `source rm` then `source add`; Taylor may override). PRD requirement SRC-4 lists "rename" among source operations, but the spec CLI (section 5) has no `source rename` command. M1 does not build it; sources are keyed by alias in the data model as the spec says. Suggested milestone: M4 (alongside `source relocate`, which already updates references). Taylor to confirm the milestone.
22. RESOLVED 2026-10-12 (M4 built it: bare `unam` prints usage, then the status table when the root holds `.unam/`, exit 0; original note follows). Spec 5 says bare `unam` prints status and usage. M1 prints usage only (plus a line that status arrives later) because the `status` command is built in milestone M4; bare `unam` does not load the config. Taylor to confirm; milestone M4 adds status to bare `unam`.
23. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional; added at M1 PVL cycle 1). PRD DIS-1 says any directory containing `SKILL.md` is a skill. M1 ignores a `SKILL.md` placed directly at a source root (warning `root-skill-skipped`; the walk still scans the root's children) because a skill needs a non-empty relative path for its ID (`alias:rel/path`). Taylor to confirm, or decide how a root-level skill should be identified.
24. RESOLVED 2026-10-12 (M4 added the status halves of AC-12, 13, 15, 21, 35 to the existing tests; original note follows). AC-12 ("`status` shows provenance") and AC-35 ("shows `selected`, not `active`") both name `status`, which is built in milestone M4. Milestone M2 closes the desired side of each through core-API integration tests (the resolved set carries provenance; a hand-edited selection resolves to desired entries with no receipt and no destination); the status side closes in milestone M4. The M2 plan records two tracked M4 test notes (status prints per-entry provenance; status prints `selected`, never `active`, for a hand-edited selection) and a backlog NOTE is written at M2 UPDATE PROCESS. No new command outside spec 5 is added.
25. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Spec 4.4 describes `slugify` as "any run of non-`[a-z0-9]` becomes `-`" (an ASCII rule) and says it approximates Hermes. Milestone M2 read the Hermes source (`agent/skill_commands.py` `slugify_skill_name`, used by `agent/skill_bundles.py`) and ports it exactly: lowercase, space and underscore to `-`, delete characters that are not Unicode `\w` or `-`, collapse hyphen runs, trim hyphens. The result differs from spec 4.4 for non-ASCII names (`Caf\u00e9` keeps the accent; CJK is kept) and for punctuation (`foo.bar` gives `foobar`, not `foo-bar`). Parity with Python `\w` is made exact with a generated exclusion table (Rust `is_alphanumeric` also accepts combining marks and some symbols Python excludes). Outcome recorded in `docs/adapters.md`. Spec 4.4 text is not edited (protected document); Taylor to confirm or amend the spec.
26. RESOLVED 2026-10-12 (M4: the `source rm` guard counts bundle members through the catalog; `bundle rm` still never blocks; original note follows). `source rm` does not consult bundle membership: a bundle that lists `alias:...` entries does not block removing that source (spec 4.2 refuses only for project selections). The PRD BND-10 report "every project referencing a bundle" is deferred; `bundle rm` lists the projects whose selections reference the bundle but never blocks (BND-6: deleting never deactivates). The project index stays the M1 string list, now read by `project::index`.
27. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Provenance: spec 2.3 says "deduplicate by `SkillId` keeping all provenance records"; PRD 5.3 says "last-write-wins at the skill ID level, provenance recorded per entry". Milestone M2 keeps every provenance record (deduplicated, first-seen order) and documents the "effective winner" as the last record (`DesiredEntry::winner`). Nothing in M2 or M3 varies behavior by winner; Taylor to confirm the reading. Two selected skills that share a name are not flagged at resolution (EDG-2 conflict belongs to M3 `plan::diff`).
28. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Bundle command scope: BND-6 "rename" is deferred (duplicate plus delete is the workaround; `bundle edit` refuses a name change that alters the slug); BND-9 "validate" is folded into `bundle show` (per-entry resolution status) and `bundle edit` (problems reported after a commit) instead of a new command; the overwrite flag on `bundle save` and `bundle duplicate` is named `--force` (spec 5 names none); spec 5 lists `rm` both as manifest CRUD and as the in-place skill remover, so `bundle rm <name>` deletes the manifest and `bundle rm <name> <ref>...` removes entries (a user who meant to remove a skill but forgot the reference deletes the bundle file; the output names the path and there is no confirmation prompt, as in Hermes).
29. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Bundle load order: spec 2.2 says "files are loaded in alphabetical order". Hermes processes `*.yaml` sorted, then `*.yml` sorted (not one merged alphabetical order); the first duplicate slug wins, so the order is observable (`a.yml` loses to `b.yaml` when both name the same slug). Milestone M2 mirrors Hermes (D7). Spec 2.2 and 4.4 ("sorted by filename") are not edited.
30. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Blank bundle name: spec 2.2 and PRD 5.1 treat a blank name as an error; milestone M2 reports an error diagnostic (`bundle-name-blank`) and skips the bundle. Hermes' loader also skips such a bundle (with a log warning); the difference is that an explicitly empty `name: ""` falls back to the file stem in Hermes (falsy value) while M2 treats an explicitly empty or whitespace-only string as blank. An absent or null `name` falls back to the file stem in both.
31. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). The M1 `lookup` ordered candidates by edit distance then by ID string. Spec 4.6 says source precedence "only orders the candidate list". Milestone M2 changes the order to (edit distance, source precedence, ID) for not-found candidates and (source precedence, ID) for ambiguous candidates. With one source nothing changes; with several, the error candidate order follows `source add` order (precedence).
32. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). The global flags `--project` and `--target` (spec 5) are not built in M2 (no M2 command needs them beyond `bundle save`); milestone M3 declares them with its first command that uses them, as M1 did for its flags.
33. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Without `--project` and `--target`, `bundle save` takes the project root from the working directory (nearest `.git` file, directory, or symlink, else the working directory itself, spec 4.5) and the target from the selection file's single target. When the selection has several targets the command fails with exit 2 and a remedy that says the `--target` flag arrives in milestone M3, so in M2 a multi-target selection cannot be saved as a bundle. Taylor to confirm this interim behavior.
34. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; research contradiction). PRD 5.1 and spec 2.2 describe a blank bundle name as raising (`ValueError("Bundle name is required")`). In the Hermes source that exception exists only in `save_bundle`; the loader (`_load_bundle_file`) skips a nameless bundle with a log warning ("has no name"). The PRD 5.1 table row mixes the two. Not edited (protected documents); M2 follows the loader's observable behavior (skip and report) with an error-severity diagnostic.
35. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; process note). `docs/adapters.md` is created by milestone M2 with the slugify parity outcome and the loader differences, and extended by M4. Spec 10 M7 says `docs/adapters.md` is generated from `doctor --evidence`; the M7 plan must preserve the two M2 sections ("Bundle slugify parity with Hermes" and "Bundle loader: differences from Hermes") when it regenerates the adapter-evidence part.
36. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Further Hermes loader differences recorded in `docs/adapters.md`: null manifest entries are dropped here (Python `str(None)` would give the text `None`); mapping or list entries are dropped with a `bundle-entry-ignored` warning (Python gives their repr text); booleans become `true` and `false` (Python `True` and `False`); YAML 1.1 booleans such as `yes` are not mirrored (`serde_norway` follows the YAML 1.2 core schema); a non-UTF-8 manifest is skipped with an error diagnostic (the Hermes loader does not catch the decode error); no mtime cache (the directory is reloaded per command); hidden files are loaded as in Hermes; `bundle save` writes only `name` and `skills`. The `\w` table is tied to the Unicode versions of the generating Python and rustc (Python 3.14.5 with Unicode 16.0.0 and rustc 1.99.0 at plan time).
37. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional; added at M3 PLAN-SUPPLEMENT). Failure semantics. Spec 4.10 step 3 says that on a step error the executor records the failure and breaks. PRD STA-4 ("one failure does not abort already-applied entries") and AC-32 ("One of three entries fails: the other two remain applied") are not met by a break when the first of three fails. M3 continues past a failed step, records `failed` in the journal, writes the receipt with the applied subset, keeps the journal when any step failed (the next mutating command exits 10 until `resume` or `rollback`), and exits 7 for any failed or skipped step (a skip-only run finalizes the journal and still exits 7). Spec 4.10 is not edited; Taylor to confirm.
38. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). State layout and field name. Spec 2.1, 2.4, 2.5, and 4.10 name singular `.unam/applied.json`, `.unam/journal.json`, and `.unam/history/<timestamp>-<target>.json`; the 2.4 receipt has a single `target` field, so one file cannot hold two targets (AC-19, AC-20). M3 uses `.unam/<target>/applied.json`, `journal.jsonl` (a JSON-lines write-ahead log instead of the spec 2.5 single document with a `completed` list), `history/<op_id>.json` (`op_id` is `{unix_millis:013}-{8 hex}`), `trash/`, and one project-wide `.unam/lock`. INNOVATE said each file carries `schema_version`; spec 2.4, 2.5, and every existing unam file spell it `version`, so M3 uses `version`. The offered `.gitignore` lines become `.unam/lock`, `.unam/*/applied.json`, `.unam/*/journal.jsonl`, `.unam/*/history/`, `.unam/*/trash/`, and the offer now fires when the selection file is first created (the lock creates `.unam/` earlier). Taylor to confirm or amend spec 2.1, 2.5, and 4.5.
39. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Lock implementation. Spec 1.4 lists the `fs4` crate. M3 uses std `File::try_lock` (stable since Rust 1.89; probe on rustc 1.99.0: two handles of one file in one process conflict, the lock is released on drop, and the OS gives no holder PID), so no new crate. The lock file holds one JSON line `{pid, started_at, command}` written by the holder after it locks and is never deleted; a contender reads it best-effort for `Locked { pid }` (spec 7). `rust-version = "1.89"` is added to the workspace manifest. Taylor to confirm dropping `fs4` and the 1.89 floor.
40. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). "Changes nothing" and the lock. AC-22 and AC-53 say the refused command changes nothing, but the lock is taken first and creates `.unam/` and `.unam/lock`. Those assertions therefore exclude the `.unam/lock` path (and an otherwise empty `.unam/` directory). The lock file content is written only by the holder, never by a refused contender.
41. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Selection write timing and recovery. `selection.yaml` is written atomically after the confirmation and before the apply, under the lock (desired state is the user's intent and stays after a partial failure). Refused, declined, no-TTY-refused, conflicted, and unresolved-reference commands write no selection. An empty plan with a changed selection writes the selection and exits 0 "no changes". A selection that did not change is never rewritten (a rewrite drops comments, the M2 limitation). Journal `rollback` restores `selection.yaml` to its pre-command text only when its current bytes still match what the command wrote; otherwise it leaves the file and warns `selection-not-restored`. Spec 4.10 does not say. Taylor to confirm.
42. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Selection semantics. `add` and `rm` store qualified `SkillId` texts. `rm X` drops X from `add` and, when a bundle still supplies X, appends X to `remove`; `rm` of an unselected skill is exit 4 `not-selected` with no change. `use B --replace` sets `bundles` to `[B]` and clears `add` and `remove` (the target `strategy` is kept); `use B --add` appends B once. `deactivate` removes the target key from `targets`. Spec 5 lists `deactivate [--all]` without defining `--all`, so M3 treats `--all` as all targets, identical to the default while one adapter exists. Taylor to confirm, notably that `use --replace` also clears ad hoc `add` and `remove`.
43. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Confirmation rules. Spec 5 and PRD ACT-9 say `--yes` skips the prompt and never the plan, and are silent when there is no TTY. M3 refuses (exit 2 `confirmation-required`, the plan in the payload, nothing changed, remedy "pass --yes") when a mutating plan has no TTY or `--json` is set without `--yes`. An empty plan exits 0 without a prompt; a declined prompt exits 0 with status `declined`; the lock is held across the prompt; `resume` and `rollback` print their steps and need the same confirmation (PRD 11.5: every repair path previews first). Taylor to confirm the exit codes.
44. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Detection default and created directories. When detection finds neither `<root>/.claude` nor `~/.claude`, M3 defaults the target to `claude-code` and the plan header says "not detected; .claude/skills will be created" (PRD ACT-2 says default to the single detected target; the undetected case is unspecified). The receipt records `created_dirs`; `deactivate` removes them with `rmdir` only when empty (PRD OWN-4). Spec 4.7 `refresh` is `Command("/reload-skills")` when the apply created the directory.
45. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Destination names and conflicts. Names come from skill frontmatter and M1 does not validate them. M3 validates at plan time (non-empty, not whitespace-only, at most 255 bytes, no `/`, `\`, NUL, or control character, not `.` or `..`, no `.unam-tmp-` prefix) and reports a failure as the plan conflict `invalid-name`. INNOVATE called that conflict "non-blocking"; spec 3 and 4.8 make every conflict blocking unless overridden and PRD STA-8 forbids a partial apply, so M3 blocks (exit 6, nothing written) for every conflict and does not build `--skip-conflicts` (named in spec 4.8 and in a spec 7 remedy, absent from the spec 5 flag list). Taylor to confirm.
46. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Ownership and removal. M3 `remove_owned(dest, &OwnedExpectation{strategy, source, tree_hash})` replaces spec 4.9 `remove_owned(dst, &Ownership)` (`Ownership` is an M4 observation type). A symlink is unlinked only when its link text equals the receipt; a copy is removed only after its tree hash matches, by rename into `.unam/<target>/trash/<op_id>/`, re-verification, a manifest-walk delete, and a bottom-up `rmdir` (never `remove_dir_all` on a destination; unrecorded content such as a `.git` directory inside a copy stays in trash and is reported). Anything else (modified, foreign, missing) deletes nothing and releases ownership (the receipt entry is dropped and the plan reports it). This is stricter than spec 4.8 item 4, which also removes an orphaned symlink whose target is under a registered source. Taylor to confirm.
47. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Hash prefix and provenance shape. INNOVATE proposed the `b3:` prefix for `selection_hash` and `{kind, name}` provenance records; spec 2.4 shows `blake3:` (the existing `Hash` type) and `{"bundle": "<slug>"}`. M3 keeps the spec forms: `selection_hash` is blake3 of the compact canonical JSON of the target selection; receipt provenance is sorted (bundles by slug, then ad hoc), deduplicated, and uses the M2 serialization `{"bundle": slug}` and `{"adhoc": true}` (the reading of item 27 is unchanged). Spec 2.4 does not show the ad hoc form.
48. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Enterprise paths and refresh timing. Spec 4.7 reads "the enterprise managed-settings location" at rank 0 without giving a path. M3 uses `/Library/Application Support/ClaudeCode/.claude/skills` (macOS) and `/etc/claude-code/.claude/skills` (Linux), labeled Assumption in the adapter evidence table (URL code.claude.com/docs/en/skills), held in `Dirs` and overridable by the environment variable `UNAM_ENTERPRISE_SKILLS_DIRS` (a test seam and an undocumented override, like item 16). Plan shadow warnings cover enterprise and user scope only; nested scan locations wait for M4. `refresh` follows spec 4.7 and PRD 10.1 (`None` when `.claude/skills` existed, `/reload-skills` when this apply created it); the output says "no restart or reload required" with the evidence URL for AC-29 and never claims an effect inside a running session (SES-3). Taylor to confirm the paths and the variable.
49. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). `write_atomic` and typed paths. `write_atomic` becomes generic over a sealed `Writable` trait (`DestPath`, `StatePath`, `ToolPath`; `SourcePath` is not writable, proved by a compile-fail doctest) and also fsyncs the parent directory (it fsynced only the file). The five M1 and M2 callers (`config.rs`, `catalog/cache.rs`, `project/index.rs`, `bundle/crud.rs`, selection IO) migrate to `ToolPath` or `StatePath` with no behavior change, and `SourcePath::as_path` becomes `pub(crate)`. Spec 4.9 shows `write_atomic(p: &StatePath)`; `ToolPath` for user-scope files is an addition.
50. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Rename onto an empty directory. INNOVATE assumed a non-empty destination makes the rename fail. Probe (rustc 1.99.0, macOS): `rename` of a directory onto an EMPTY directory succeeds and replaces it; only a non-empty destination fails (`DirectoryNotEmpty`); `symlink(2)` onto an existing path fails (`AlreadyExists`). M3 checks the destination is absent immediately before the rename and treats a foreign empty directory as a plan-time conflict; the window in which a user creates an empty directory between that check and the rename is a named residual (an atomic no-replace rename needs `unsafe` or a new crate).
51. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Spec-text mismatches found at M3 PLAN-SUPPLEMENT: (a) spec 7 remedies name `--strategy copy` and `--skip-conflicts`, neither in the spec 5 flag list; M3 builds neither and words the remedies around the selection or config key and "resolve the conflict"; (b) PRD 11.2 says copy is selected automatically when symlinks fail; M3 does not fall back automatically, a failing symlink is the step failure `symlink-unsupported` with a remedy (spec 7 treats the switch as an explicit user choice); (c) AC-13 ("reach `active`"), AC-15, and AC-21 ("shows its prior selection") need `status`, which is M4: M3 proves the applied state (destination, receipt, selection) and M4 adds the `active` and `status` halves to the same tests; AC-24 and AC-29 close the claude-code half only; (d) the spec 2.5 journal (one document with `completed`) versus the JSON-lines write-ahead log (item 38); (e) the M2 error remedies that said `--target` and `unam use` arrive in milestone M3 are reworded now that they exist.
52. RESOLVED 2026-10-09 (Taylor approved the evidence pack in chat; vocabulary used: `approved`, as `validate-risk-artifacts.mjs` accepts; `.claude/` not edited; the script mismatch remains a backlog item). High-risk evidence tooling. The two scripts in `.claude/skills/vc-risk-evidence-pack/scripts/` disagree: `validate-risk-artifacts.mjs` requires `task`, `planPath`, `riskLevel`, `riskClasses`, `mustStopBeforeFinalize`, `humanApprovalRequired`, and `why` in `risk-gate.json` and a `decision` of `approved`, `approved-with-concerns`, or `rejected`, while SKILL.md and `validate-evidence-pack.mjs` require exactly `APPROVE` or `REJECT`; no single value passes both. The M3 pack carries the union of fields and gates on `validate-risk-artifacts.mjs`; `.claude/` is not edited. Under the manual-first rule `review-decision.json` is left for Taylor and autopilot STOPS after EVL: no UPDATE PROCESS, no commit, and no M4 start until he records the decision.
53. STILL OPEN (REPORT TO TAYLOR, 2026-10-05; provisional). Deferred from M3 (tracked for M4 unless noted): nested `scan_locations` (needs the `ignore` walk); the VCS-tracked destination warning (EDG-12, needs a `git` subprocess); `--skip-conflicts`; case-insensitive filesystem collisions (EDG-13, priority P2; only the apply-time absent-check guards it); `status`, rollback from history, and `doctor --evidence`; recording the claude-code evidence table and the M0 Outcome in `docs/adapters.md` (the spike assigned it to whichever milestone created the file first; M2 created it without that content, M3 leaves `docs/` untouched, M4 records it from `Adapter::evidence()`).
54. PARTLY RESOLVED 2026-10-09 (review decision recorded by Taylor, so the M3 close and M4 start are unblocked; the first adopting project is still unnamed and D4 stays CONDITIONAL per decision sheet row 54). First adopting project and held commit. The brief's first adopting project (outside this repository) is still unnamed; Hybrid gate D4 (one real activation there) is recorded with a backlog NOTE and owned by Taylor. Because M3 is high-risk, the M3 execution commit, UPDATE PROCESS, and the start of M4 also wait for his review decision (item 52).

55. RESOLVED 2026-10-12 (M4 fixed it together with item 26; M3 residual R10 is closed; original note follows). Source rm guard blind spot. With M3 activation, a project whose selection reaches a source only through bundles (for example after `use <bundle> --replace`) is invisible to the `source rm` guard, because the M2 `projects_referencing_source` counts only literal `add` and `remove` entries starting with `<alias>:`. `source rm` can therefore unregister a source that live symlinks still point at (M3 named residual R10). Recommend revisiting item 26 for M4: count bundle membership, or resolve selections, in the guard.

---

## Follow-ups (not gates)

- S5 follow-up (PRD 13.1, observational): take the median active selection size four weeks after product Phase 1 is in daily use; target under 10. This is a record, not a gate for any milestone or for the product Phase 1 Gate. Also noted in the M5 plan exit notes and in the M5 phase report (`phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md`, "Product Phase 1 completeness").

---

## Program Status Table

| Phase | Status |
|---|---|
| 0 (pre-program, plan creation) | PLANNED (this plan set written 04-10-26) |
| 1 of 8: M0 Gating spike | VERIFIED (WITH_GAPS) 2026-10-05 (gates G1 to G6 PASS; Windsurf and Cursor editor not tested) |
| 2 of 8: M1 Core read path | VERIFIED (WITH_GAPS) 2026-10-05 (G1 to G13 PASS, 164 tests; perf margin noisy, CI not run, 9 design departures awaiting Taylor: items 15 to 23) |
| 3 of 8: M2 Bundles and selection | VERIFIED (WITH_GAPS) 2026-10-05 (G1 to G14, G12b, E1 PASS; AC-10, 12, 35 desired side only; 13 design departures awaiting Taylor: items 24 to 36) |
| 4 of 8: M3 Plan and apply for claude-code | VERIFIED (WITH_GAPS) 2026-10-09 (G1 to G19 PASS, 512 tests; review decision `approved` by Taylor; Linux/CI never run, D4 pending; residuals R1 to R10 accepted) |
| 5 of 8: M4 Verify, status, reconcile, rollback | VERIFIED (WITH_GAPS) 2026-10-12 (G1 to G6 PASS, 567 tests, deletion-safety evidence check PASS; Linux/CI never run, D4 pending; three interpretations for Taylor in the M4 report) |
| 6 of 8: M5 agents-neutral and generated briefs | VERIFIED (WITH_GAPS) 2026-10-17 (G1 to G3 PASS, 655 tests after EVL fix cycle 1 and the review-fix round, full fault matrix PASS, S1 and S6 PASS on the real library; literal G5 e and G6 diff commands mode-only (D9); Linux/CI never run, D4 pending; review findings R1 and R2 for Taylor) |
| Product Phase 1 Gate | CLEARED by Taylor 2026-10-17 (daily use, D4, S5 deferred until after M7) |
| 7 of 8: M6 TUI | VERIFIED (WITH_GAPS) 2026-10-17 (G1 to G6 PASS, 699 tests, EVL PASS in 2 iterations; warm start 45.0 to 46.8 ms of 50, RSS 9.1 MB; termion backend instead of crossterm; Taylor's manual AC-44 real-terminal run outstanding; Linux/CI never run) |
| 8 of 8: M7 OSS readiness | CODE DONE 2026-10-17 (EXECUTE: G1 to G3 green, 710 tests; dist plan, generate --check, and the profile check exit 0; Linux/CI, the first release run, and Taylor's manual runs outstanding; EVL next) |

Status values: PLANNED | CODE DONE | TESTING | VERIFIED | BLOCKED | COMPLETE

---

## Touchpoints

- M0: `spike/symlink-discovery.md` (filled in by Taylor); no code.
- M1: `Cargo.toml`, `Cargo.lock`, `crates/unam-core/**`, `crates/unam-cli/**`, CI workflow, `cargo-dist` config.
- M2 to M5: additions under `crates/unam-core/src/` and `crates/unam-cli/src/cli/`, plus `crates/unam-cli/tests/`.
- M2 and M4: `docs/adapters.md` (new spec-called document) is created by whichever of M2 (Hermes slugify outcome) or M4 first has content for it; the other extends it. M6: `crates/unam-cli/src/tui/`. M7: `README.md`, `CHANGELOG.md`, license, release workflow, `docs/adapters.md`, `docs/json.md` (new spec-called documents; agents may create and maintain them).
- Process: this task folder, `process/context/**` updates at each UPDATE-PROCESS.

---

## Public Contracts

- The CLI surface in spec section 5 (commands, exit codes 0 to 11, `--json` envelope) is the product contract; milestones add commands, never change earlier ones.
- File formats in spec section 2 (config, bundle, selection, receipt, journal) are contracts with Hermes compatibility for bundles (PRD 5).
- Unchanged by this program: the skill library, the five kickoff design documents under `docs/`, the toolkit directories (`.claude/`, `.codex/`, `.agents/`).

---

## Blast Radius

Files directly modified or created across the program:

- `spike/symlink-discovery.md` (M0, by Taylor)
- Rust workspace: `Cargo.toml`, `Cargo.lock`, `crates/unam-core/`, `crates/unam-cli/` (M1 to M6)
- CI and release config (M1 CI and `cargo-dist` config; M7 release workflow)
- `README.md`, `CHANGELOG.md`, license file (M7); `docs/adapters.md` (created by M2 or M4, whichever first; regenerated M7), `docs/json.md` (M7); new spec-called docs, no permission gate
- `process/context/**` and this task folder (every milestone)

Risk class: destructive filesystem writes in user-owned destinations (M3 to M5; hybrid test gates required), and outward-facing release mechanics (M7).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| M0 results table filled and Outcome recorded (manual, Taylor) | Hybrid (precondition: agents installed; Taylor runs) | PRD A1, R1 (symlink discovery); gates adapter defaults (spec 10 M0) |
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` at each milestone exit | Fully-Automated | spec 8 quality gates |
| `ac_NN_*` per criterion in the coverage map | Fully-Automated | AC-1 to AC-43, AC-53 (AC-2 in M3, AC-33 in M4) |
| M1 `source_tree_unchanged_read_side` and `perf_cold_scan` (under 300 ms, generated 500-skill fixture) | Fully-Automated (perf: Hybrid, release build, local SSD) | S3 read-side; spec 9 cold scan |
| M3 S2, S9 (claude-code); M4 S7, S8; M5 S9 (agents-neutral), network and background none | Fully-Automated | PRD S2, S9, S7, S8; spec 9 |
| M5 S1 and S6 measured once on Taylor's real library (read-only, temporary destination) | Hybrid (precondition: the real library present) | PRD S1, S6 |
| M6 warm start under 50 ms and memory under 50 MB; M7 binary targets (recorded only) | Hybrid | spec 9 |
| Source-tree hash and foreign-file invariants in every mutating test (from M3 on) | Fully-Automated | S3, S4 (AC-2, AC-23) |
| Gate record check before M6 | Agent-Probe (orchestrator reads the gate record) | PRD 12 product Phase 1 done condition |

No criterion is carried by Known-Gap. If a milestone discovers an untestable behavior it writes a backlog stub and keeps that gate CONDITIONAL.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md`
- Last completed phase: phase 7 of 8 (milestone M6), VERIFIED (WITH_GAPS) 2026-10-17; phase report written
- Validate-contract status: M0 to M6 written (Gate PASS) and closed; M7 not written (vc-validate-agent writes it at the M7 PVL step)
- Supporting context files loaded: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, `docs/PROJECT-BRIEF.md`, spec sections 8 to 12, PRD sections 0, 3, 12, 13, 15, `spike/symlink-discovery.md`, `process/development-protocols/phase-programs.md`
- Next step for a fresh agent: read this plan and the M5 report (Forward Preview). M0 to M6 are done. Taylor cleared the Product Phase 1 Gate on 2026-10-17 and approved M6 and M7; continue with M7 (RESEARCH next) once PRD Q11 (crate name and license) is decided.
- Execute-agent start instruction: never start EXECUTE for M1 or later until the M0 gate is met; the Product Phase 1 Gate record shows Taylor's confirmation (2026-10-17), so M7 may proceed through the normal loop.

---

## Current Execution State

Last updated: 2026-10-17 (M7 EXECUTE complete, code done; EVL next)
Current phase: 8 of 8 (milestone M7, OSS readiness)
Phase 8 name: M7 OSS readiness
Phase 8 status: CODE DONE (EXECUTE 2026-10-17; code commit 9897bfe; gates G1 to G3 green with 710 tests; EVL and UPDATE PROCESS not yet run)
Phase 8 report: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_REPORT_17-10-26.md
Phase 7 status: VERIFIED (WITH_GAPS) 2026-10-17
Phase 7 EVL: PASS in 2 iterations (HALTED_SUCCESS, `results-m6-evl.tsv`). Iteration 1 at 0425429: G1 to G6 green, warm start 47.4 ms. Iteration 2 at 9658536 after the review-fix round (bd97432): G1 to G6 green, 699 tests, warm start 45.0 to 46.8 ms of 50, peak RSS 9.1 MB, 39 CLI snapshots and 6 TUI snapshots unchanged, `git diff --check` clean; classification WITH_GAPS. Code review: no Critical or High findings; findings 1 to 6 fixed; three Low items not fixed
Phase 7 report: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_REPORT_17-10-26.md
Phase 7 commits: 37292c2 (code), 0425429 (process, execute report), bd97432 (review fixes), 9658536 (process, review-fix round and EVL 1); the UPDATE PROCESS commit follows
Phase 6 report: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md
Next phase: phase 8 of 8 (M7 OSS readiness), loop step EVL (independent vc-tester re-run of G1 to G6)
Completed phases: phase 0 (plan creation); phases 1 to 7 of 8 (M0 to M6), each VERIFIED (WITH_GAPS)
Current loop step: EVL (milestone M7, step 6; code done, EXECUTE finished)
Validate-contract status: M0 to M6 written (Gate PASS) and closed; M7 written 2026-10-17 (Gate PASS, inner-pvl: phase-8)
Product Phase 1 Gate: CLEARED by Taylor on 2026-10-17 (record in `## Product Phase 1 Gate`). Daily use, D4, and S5 are deferred until all milestones are done
Program Net Gate: PENDING
Open items (all with Taylor unless noted):
- M6 manual AC-44 run in a real terminal (steps in the M6 report "For Taylor"; backlog stub `m6-ac44-manual-terminal-run_NOTE_17-10-26.md`) and the M6 decisions to confirm (mixed add/remove rule, key routing M6-12, enter applies in the plan view, the `b` overlay, `r`/`u` modal keys)
- Warm-start margin of about 3 to 5 ms (backlog stub `m6-warm-start-margin_NOTE_17-10-26.md`)
- Linux and CI never run (no remote)
- PRD Q11 (crates.io name and license): needed before M7 starts
- M7 docs must carry the M6 spec deviations (termion instead of crossterm, no `e` key, static loading frame, no match highlighting, lock only during apply, D-R1 ambiguous-target fallback, D-R2 warning classes); M7's release profile must not set `panic = "abort"`
- Daily use, the D4 real activation, and naming the first adopting project (backlog NOTE `m3-d4-first-real-activation_NOTE_05-10-26.md`); then the S5 follow-up four weeks after daily use begins
- The seven decision-sheet rows marked "Needs a real decision" in open-items-decision-sheet_REF_09-10-26.md
- The Cursor editor and Windsurf were never tested
- Whether `.claude/scheduled_tasks.lock` should be untracked (pending Taylor)
Latest validator run: 2026-10-17, M6 UPDATE PROCESS (results in the commit message)

Loop step values: RESEARCH | INNOVATE | PLAN-SUPPLEMENT | PVL | EXECUTE | EVL | UPDATE-PROCESS
Orchestrator rule: read "Current loop step" and "Validate-contract status" before spawning any subagent. Never spawn execute-agent when loop step is RESEARCH, INNOVATE, PLAN-SUPPLEMENT, or PVL.

Note: The Stable Program Goal above is fixed. This section is the only part that changes; update-process-agent rewrites it after every milestone closeout (overwrite, not append).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

M0 to M6 are VERIFIED (WITH_GAPS) and Taylor cleared the Product Phase 1 Gate on 2026-10-17. Next: M7 (OSS readiness, phase 8 of 8), loop step 1 RESEARCH, once PRD Q11 (crate name and license) is decided.
