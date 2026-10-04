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

Build `unam` v1 (the Rust CLI described in `docs/spec/unam-technical-spec.md`) milestone by milestone: M0 to M5 deliver product Phase 1 (selection and activation); M6 and M7 deliver the first part of product Phase 2 (picker, release readiness). There is a HARD STOP after M5: M6 and M7 may not start until product Phase 1 is done and Taylor confirms in writing (see "Product Phase 1 Gate" below). Milestones M6 and M7 are product Phase 2: they must not be researched, planned further, or executed until Taylor clears the stop. Their plans carry only the check additions recorded on 2026-10-04 (decision 11); nothing else in them changes.

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
  Hermes adapter, Windows. Milestones M6 and M7 are product Phase 2 and are held behind the stop.
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
- M6 and M7 (product Phase 2) are NOT in this goal: no research, planning, or execution until Taylor clears the stop (PRD 12).

AUTONOMY: Before ANY subagent spawn read umbrella ## Current Execution State, then the milestone plan ## Phase Loop Progress; first unchecked box = next spawn.

PER-MILESTONE LOOP (7-step inner loop R,I,P,PVL,E,EVL,UP; never skip or reorder; SKIPS SPEC):
 1 RESEARCH 2 INNOVATE 3 PLAN-SUPPLEMENT 4 PVL 5 EXECUTE 6 EVL 7 UPDATE-PROCESS
- PVL NEVER skipped; placeholder or partial validate-contract = blocked
- Every subagent FIRST ACTION: vc-context-discovery + vc-plan-discovery
- Every phase-END: invoke vc-agent-strategy-compare for the next step
- EXECUTE runs opus; every other step sonnet

HARD STOPS (pause, wait for Taylor):
- GATE: after M5 stop. Never start M6 or M7 until Taylor confirms product Phase 1 done (AC-1 to AC-43 and AC-53 pass AND daily use). Never auto-proceed under /goal.
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

START: Milestone M0, loop step 5 EXECUTE (manual: Taylor runs the runbook; orchestrator transcribes report blocks into spike/symlink-discovery.md).
```

---

## Product Phase 1 Gate (HARD STOP after M5)

This gate sits between milestone M5 and milestone M6. It is never auto-proceeded, including under /goal.

- Why: PRD section 12 (product Phase 1 "done when" the acceptance criteria pass and the author uses the tool daily instead of the manual workflow; PRD 13.2 rule: every AC is product Phase 1 unless under the "Phase 2" or "Phase 3" heading); spec section 10 after M5 ("Phase 2 does not start before that"); brief "Out of scope" (product Phase 2 until Phase 1 is done by the PRD definition).
- What blocks: milestone M6 (TUI) and milestone M7 (OSS readiness) may not start, not even RESEARCH or any plan refresh for them, until the gate is confirmed.
- Preconditions the orchestrator checks and reports (it does not decide):
  1. Milestones M0 to M5 are VERIFIED with phase reports.
  2. AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1 unless it sits under the "Phase 2" or "Phase 3" heading). The set closed by M1 to M5 per spec section 10 is exactly this set (coverage map below; AC-2 closes in M3, AC-33 in M4).
  3. Taylor uses the tool daily (brief name of the first real adopting project, due end of M3).
  4. The S1 and S6 measurements on Taylor's real library are recorded in the M5 report (read-only against the library).
- Confirmation: only an explicit message from Taylor stating that product Phase 1 is done (AC-1 to AC-43 and AC-53 pass and daily use) and M6 may start. The orchestrator then records it in the record below and in `## Current Execution State`. No agent may fill in the record on its own judgment.
- Gate record: Confirmed by Taylor: NOT CONFIRMED. Date: n/a. Evidence message: n/a.
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
| 7 of 8 | M6 TUI | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md` | PRODUCT PHASE 2, HELD: no research, planning, or execution until Taylor clears the stop. Picker, plan view, result view, interrupted modal | Gate |
| 8 of 8 | M7 OSS readiness | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md` | PRODUCT PHASE 2, HELD: no research, planning, or execution until Taylor clears the stop. README, evidence docs, CHANGELOG, license, release workflow | M6, Q11 decided, Gate |

### Join Conditions

- M1 MUST NOT start until M0 exit gate passes (results table filled, Outcome records default strategy per adapter). Every later milestone plan also lists this as an entry dependency.
- M2 MUST NOT start until M1 exit gate passes; M3 after M2; M4 after M3; M5 after M4.
- M6 MUST NOT start until M5 exit gate passes AND the Product Phase 1 Gate is confirmed by Taylor.
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
| M6 | M5 exit met and Gate confirmed (HELD: product Phase 2) | gates green; AC-44 test green; TUI parity test green; warm start to picker under 50 ms and memory under 50 MB (spec 9) |
| M7 | M6 exit met, Gate confirmed, Q11 decided (HELD: product Phase 2) | README, docs, CHANGELOG, license, release workflow in place; local `cargo dist plan` lists macOS universal, Linux x86_64, Linux aarch64 (spec 9 binary row); no publish performed |

---

## Per-Phase Loop

Each milestone executes the canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP`, which SKIPS SPEC (SPEC runs once in the outer program loop). The authoritative description is `process/development-protocols/phase-programs.md`; it is not restated here. Each milestone plan carries its own `## Phase Loop Progress` checklist. PVL is never skipped; a placeholder or partial `## Validate Contract` means blocked.

Re-research at every milestone entry is mandatory (read the immediately prior milestone report in full, earlier reports' Forward Preview only).

---

## Phase Loop Progress

Program-level summary. The authoritative per-step checkboxes are in each milestone plan. Orchestrator reads the milestone plan, not this table, to choose the next subagent.

| Milestone | Next loop step | Note |
|---|---|---|
| M0 | 5 EXECUTE (manual) | Steps 1 to 4 done (PVL Gate PASS 2026-10-04 after 2 fix cycles; results.tsv). Next: Taylor runs the runbook by hand; the orchestrator transcribes his report blocks into `spike/symlink-discovery.md` |
| M1 to M5 | not started | Blocked behind M0 |
| M6, M7 | not started | Product Phase 2, held: no research, planning, or execution until Taylor clears the stop after M5 |

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
| M6 | UI surface (HELD: product Phase 2) | sequential, opus EXECUTE |
| M7 | outward-facing release mechanics (HELD: product Phase 2) | sequential; publish actions stay hard stops |

Plan authoring for this program was done by one agent: each plan owns a distinct file, and blast radii are recorded in `phase-blast-radius-registry.md`, so no mid-draft coordination was needed.

---

## Autonomous Execution Rules (During /goal)

- The agent self-decides at all V5 gates. CONDITIONAL net gate: proceed with fixes and gaps on record. BLOCKED net gate: write a backlog note, continue.
- Hard stops (must pause for Taylor): the Product Phase 1 Gate (M6 and M7 are product Phase 2: no research, planning, or execution before Taylor clears it); the M0 spike (Taylor runs it); push, publish, crates.io, Homebrew tap, license file; any outward-facing or irreversible action not named in the validate-contract; contradictions between documents (report, do not resolve).
- Under /goal the Product Phase 1 Gate is NEVER auto-proceeded: the goal ends at M5 UPDATE-PROCESS.
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
| M0 | none (evidence for adapter claims; PRD S9 support) |
| M1 | AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side) |
| M2 | AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-35 (desired side) |
| M3 | AC-2, AC-13, AC-15, AC-16, AC-17, AC-18, AC-19, AC-21, AC-22, AC-23, AC-24, AC-25, AC-29, AC-30, AC-31, AC-32, AC-40, AC-53 |
| M4 | AC-14, AC-26, AC-27, AC-28, AC-33, AC-34, AC-36, AC-37, AC-39, AC-41 |
| M5 | AC-20, AC-42, AC-43, AC-24 (second adapter) |
| M6 | AC-44 |
| M7 | none listed in spec 10 |
| not in this program | AC-45 is product Phase 2 pinning (spec 11 v2 theme); AC-46 to AC-52, AC-54 to AC-56 are product Phase 3 and out of scope |

Check (2026-10-04): every AC-1 to AC-43 and AC-53 is mapped exactly once, except AC-24, which appears in M3 (claude-code) and M5 (agents-neutral) by design (second adapter). AC-2 closes in M3 (needs apply for the full activate, modify, deactivate cycle); M1 supplies the source-tree hash helper and the read-side test as the first test. AC-33 closes in M4 (rollback from history). AC-37 closes in M4 (`source relocate`). AC-44 is M6 (product Phase 2, held).

---

## Contradictions and Open Items

Taylor's twelve decisions of 2026-10-04 are applied below (spec and PRD edits are in commit ae120c5). Items still open are marked STILL OPEN.

1. RESOLVED 2026-10-04 (decision 1). AC-2 closes in M3 (needs apply for the full cycle). M1 writes the source-tree hash helper and the read-side test (register, scan, search leave the source tree hash unchanged) as the FIRST test; the invariant is asserted in every mutating test from M3 on. The M1 "AC-2 open item" is replaced by these tasks.
2. RESOLVED 2026-10-04 (decision 2). `source relocate` is built in M4 (needs receipts and the project index). M4 closes AC-37.
3. RESOLVED 2026-10-04 (decision 3). Journal-based `resume` and `rollback` belong to M3 (spec 4.10). Rollback from history and `reconcile` stay in M4. AC-31 and AC-32 stay in M3; AC-33 moves to M4. In M3 the interruption test proves consistency by comparing destination and receipt to the expected state; M4 adds the "reconcile reports zero divergence" assertion to that test. The M3/M4 RESEARCH flags on this are removed.
4. RESOLVED 2026-10-04 (decision 4). Only the five kickoff design documents are protected (docs/PROJECT-BRIEF.md, docs/KICKOFF-PROMPT.md, docs/spec/PRD-unambiguous.md, docs/spec/unam-technical-spec.md, docs/adr/ADR-0001-dynamic-skill-routing.md): never edited by agents without Taylor's explicit per-edit permission. New spec-called docs (docs/adapters.md, docs/json.md) may be created and maintained by agents (M4 creates docs/adapters.md, M7 regenerates it and writes docs/json.md). The M0 Outcome may be recorded in docs/adapters.md once it exists.
5. RESOLVED 2026-10-04 (decision 5). Proceed with the spec: crates `unam-core` and `unam-cli`, package and binary `unam`. PRD Q11 is only the crates.io published name and the license; nothing before M7 depends on it. Q11 stays an M7 entry condition. The M1 conflict note is removed.
6. RESOLVED 2026-10-04 (decision 6). `spike/symlink-discovery.md` is authoritative for the M0 procedure; spec 10 M0 now points to it. The spec/spike contradiction is closed.
7. STILL OPEN (partly resolved). Spec 1.3 lists `hash.rs`, `state.rs`, `error.rs`, `fsops.rs` but spec section 10 does not assign each to a milestone. `hash.rs` is now created in M1 (decision 1: the hash helper). The first-needed milestone for the rest is decided at each milestone's RESEARCH (M1 creates the error skeleton; M3 plans `fsops.rs` and `state.rs`).
8. RESOLVED 2026-10-04 (decision 11). Spec section 9 targets and PRD S1 to S9 are exit-gate checks in the milestone plans: M1 cold scan under 300 ms on a generated 500-skill fixture; M3 S2, S3, S4, S9 (claude-code); M4 S7, S8; M5 S9 (agents-neutral), S1 and S6 measured once on Taylor's real library at the product Phase 1 exit (read-only), network none and background none confirmed; M6 warm start under 50 ms and memory under 50 MB (held); M7 binary targets (held). S5 is observational (see "Follow-ups (not gates)").
9. RESOLVED 2026-10-04 (decision 10). Product Phase 1 done = AC-1 to AC-43 and AC-53 pass (PRD 13.2 rule: every AC is product Phase 1 unless under the "Phase 2" or "Phase 3" heading) AND Taylor uses the tool daily. No PRD section 7 computation is needed; the AC-2 caveat is gone.
10. STILL OPEN. Toolkit shape notes: phase-stub template links the umbrella as `{program-slug}-umbrella_{date}/` (a folder) while the flat layout puts it in the program folder; template says "Phase Sequence" while validators and protocol say "Phase Ordering"; both phase and umbrella validators auto-scan only files directly in `active/` (not task folders), so they must be run with explicit paths; templates contain em dashes and emoji status values (replaced here); the template's "Phase 0 pre-program" row clashes with milestone M0 (this program maps M0 to phase 1 of 8); `validate-plan-artifact.mjs` requires Acceptance Criteria and Phase Completion Rules that the phase-stub template lacks (added).
11. RESOLVED 2026-10-04 (decision 7). The spike run order (hide one folder per run for Claude Code and Cursor) is accepted; marked resolved in the M0 plan.
12. RESOLVED 2026-10-04 (decision 8). PRD 12 now says RTE-1 to RTE-12 for product Phase 3 "In:". No plan in this program references the old range; closed.
13. RESOLVED 2026-10-04 (decision 9). OWN-6 (`adopt`) and SRC-7 (remote sources) are product Phase 2 and now listed in spec 11. The charter out-of-scope list matches; closed.
14. RESOLVED 2026-10-04 (decision 12). The Hermes trust write in the M0 runbook is accepted as planned (backup, single write, remove in cleanup, show the diff before deleting); marked resolved in the M0 plan; no runbook change.

---

## Follow-ups (not gates)

- S5 follow-up (PRD 13.1, observational): take the median active selection size four weeks after product Phase 1 is in daily use; target under 10. This is a record, not a gate for any milestone or for the product Phase 1 Gate. Also noted in the M5 plan exit notes.

---

## Program Status Table

| Phase | Status |
|---|---|
| 0 (pre-program, plan creation) | PLANNED (this plan set written 04-10-26) |
| 1 of 8: M0 Gating spike | PLANNED (steps 1 to 4 done, PVL Gate PASS 2026-10-04; manual EXECUTE pending) |
| 2 of 8: M1 Core read path | PLANNED |
| 3 of 8: M2 Bundles and selection | PLANNED |
| 4 of 8: M3 Plan and apply for claude-code | PLANNED |
| 5 of 8: M4 Verify, status, reconcile, rollback | PLANNED |
| 6 of 8: M5 agents-neutral and generated briefs | PLANNED |
| Product Phase 1 Gate | NOT CONFIRMED |
| 7 of 8: M6 TUI | PLANNED (product Phase 2, held behind the gate) |
| 8 of 8: M7 OSS readiness | PLANNED (product Phase 2, held behind the gate) |

Status values: PLANNED | CODE DONE | TESTING | VERIFIED | BLOCKED | COMPLETE

---

## Touchpoints

- M0: `spike/symlink-discovery.md` (filled in by Taylor); no code.
- M1: `Cargo.toml`, `Cargo.lock`, `crates/unam-core/**`, `crates/unam-cli/**`, CI workflow, `cargo-dist` config.
- M2 to M5: additions under `crates/unam-core/src/` and `crates/unam-cli/src/cli/`, plus `crates/unam-cli/tests/`.
- M4: also creates `docs/adapters.md` (new spec-called document). M6: `crates/unam-cli/src/tui/`. M7: `README.md`, `CHANGELOG.md`, license, release workflow, `docs/adapters.md`, `docs/json.md` (new spec-called documents; agents may create and maintain them).
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
- `README.md`, `CHANGELOG.md`, license file (M7); `docs/adapters.md` (created M4, regenerated M7), `docs/json.md` (M7); new spec-called docs, no permission gate
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
| M6 warm start under 50 ms and memory under 50 MB; M7 binary targets (held, recorded only) | Hybrid | spec 9 |
| Source-tree hash and foreign-file invariants in every mutating test (from M3 on) | Fully-Automated | S3, S4 (AC-2, AC-23) |
| Gate record check before M6 | Agent-Probe (orchestrator reads the gate record) | PRD 12 product Phase 1 done condition |

No criterion is carried by Known-Gap. If a milestone discovers an untestable behavior it writes a backlog stub and keeps that gate CONDITIONAL.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md`
- Last completed phase: phase 0 (plan set written 04-10-26); milestone M0 loop steps 1 to 4 (PVL Gate PASS 2026-10-04 after 2 fix cycles); plans updated for Taylor's twelve decisions of 2026-10-04
- Validate-contract status: M0 written (Gate PASS 2026-10-04); M1 to M7 pending (vc-validate-agent writes per milestone)
- Supporting context files loaded: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, `docs/PROJECT-BRIEF.md`, spec sections 8 to 12, PRD sections 0, 3, 12, 13, 15, `spike/symlink-discovery.md`, `process/development-protocols/phase-programs.md`
- Next step for a fresh agent: read this plan, then the M0 plan. M0 step 5 EXECUTE is Taylor's manual spike (he runs the runbook); do not write code and do not spawn an execute agent for M0. The orchestrator transcribes his report blocks into `spike/symlink-discovery.md`, then EVL and UPDATE PROCESS. After the table is filled, spawn research for M1. Do not research, plan, or execute M6 or M7 (product Phase 2, held).
- Execute-agent start instruction: never start EXECUTE for M1 or later until the M0 gate is met; never start research, planning, or EXECUTE for M6 or M7 until the Product Phase 1 Gate record shows Taylor's confirmation.

---

## Current Execution State

Last updated: 2026-10-04 (Taylor's twelve decisions applied to the plans)
Current phase: 1 of 8 (milestone M0)
Phase 1 name: M0 Gating spike
Phase 1 status: PLANNED (steps 1 to 4 done; PVL Gate PASS 2026-10-04 after 2 fix cycles; results.tsv)
Phase 1 EVL: n/a
Phase 1 report: not written
Next phase: 2 of 8 (milestone M1), blocked until M0 gate met
Completed phases: phase 0 (plan creation)
Current loop step: EXECUTE (step 5; manual: Taylor runs the runbook; orchestrator transcribes report blocks into spike/symlink-discovery.md)
Validate-contract status: M0 written (Gate PASS 2026-10-04)
Program Net Gate: PENDING
Product Phase 1 Gate: NOT CONFIRMED
Milestones M6 and M7: product Phase 2, held (no research, planning, or execution until Taylor clears the stop)
Latest validator run: 2026-10-04, see the final report of the decisions-update session

Loop step values: RESEARCH | INNOVATE | PLAN-SUPPLEMENT | PVL | EXECUTE | EVL | UPDATE-PROCESS
Orchestrator rule: read "Current loop step" and "Validate-contract status" before spawning any subagent. Never spawn execute-agent when loop step is RESEARCH, INNOVATE, PLAN-SUPPLEMENT, or PVL. M0 EXECUTE is manual: spawn no execute agent for it.

Note: The Stable Program Goal above is fixed. This section is the only part that changes; update-process-agent rewrites it after every milestone closeout (overwrite, not append).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

Taylor runs the M0 runbook by hand (step 5 EXECUTE) and pastes the report blocks; the orchestrator transcribes them into `spike/symlink-discovery.md`, then EVL and UPDATE PROCESS for M0 (RIPER-5).
