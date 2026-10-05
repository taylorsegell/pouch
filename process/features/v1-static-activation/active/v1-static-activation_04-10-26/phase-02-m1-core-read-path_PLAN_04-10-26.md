---
name: plan:v1-static-activation-phase-02-m1-core-read-path
description: "v1: static activation, phase 2 of 8: milestone M1, core read path (workspace, CI, config, source, catalog, search, show)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-02
---

# Milestone M1: Core read path

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: COMPLEX
**Phase:** Phase 2 of 8: milestone M1
**Status**: PLANNED
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

---

## Overview

Milestone M1 creates the Cargo workspace and the read-only half of the product: register sources, scan and cache the catalog, parse frontmatter, search, and show (spec section 10 M1; spec 4.1 to 4.3, 5, 7; PRD 7.1 and 7.2 discovery requirements). Nothing in M1 writes to a source or a project destination. Estimated 2 to 3 days (spec 10). It ends in something usable: a `unam` binary that can register a source and search it.

Product Phase 1, tier 2 (read path). Program charter: see umbrella.

---

## Scope

Carried from spec section 10 M1:

- Workspace, crates, CI (fmt, clippy, test), `cargo-dist` config (workspace per spec 12 and 1.3; dependencies per spec 1.4, versions chosen by `cargo add`, lockfile committed).
- `config` (spec 4.1, 2.1), `source add|list|rm` (spec 4.2, 5), `catalog` scan with cache (spec 4.3), `frontmatter` (spec 4.3 and the unit cases in spec 8), `search` (spec 4.3, `nucleo`), `show` (spec 5).
- Error model skeleton with remedy strings (spec 7); clippy configuration denying `unwrap` outside tests.
- Test harness: `crates/unam-cli/tests/` fixtures (nested categories, duplicate names, broken-YAML skill, supporting files, nested `.git` to skip; spec 8 Integration).

---

## Acceptance Criteria

Closes (spec section 10 M1): AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side). Criteria text: PRD section 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-1 | `ac_01_*` | Fully-Automated |
| AC-3 | `ac_03_*` | Fully-Automated |
| AC-4 | `ac_04_*` | Fully-Automated |
| AC-5 | `ac_05_*` | Fully-Automated |
| AC-6 | `ac_06_*` | Fully-Automated |
| AC-38 (read-side) | `ac_38_*` (unmounted or missing source reported unreachable, no mutation) | Fully-Automated |
| S3 (read-side; first test, not an AC closure) | `source_tree_unchanged_read_side` (register, scan, search leave the source tree hash unchanged) | Fully-Automated |
| Spec 9 cold scan under 300 ms (exit-gate check) | `perf_cold_scan` | Hybrid (see Exit Gate) |

### AC-2 and the source-tree invariant (resolved 2026-10-04, decision 1)

AC-2 (source tree hash identical before and after a full activate, modify, deactivate cycle) closes in milestone M3, because the full cycle needs `apply`. Milestone M1 does two things for it (spec 12 "First test to write"; spec 8; PRD S3):

- writes the source-tree hash helper (blake3 tree hash, spec 8), and
- writes the read-side test as the FIRST test of the milestone: register, scan, and search leave the source tree hash unchanged (proves PRD S3 for read operations; does NOT close AC-2).

The invariant is then asserted in every mutating test from milestone M3 on. M1 does not close AC-2 and does not list it in its Closes line.

---

## Entry Gate

- [x] Milestone M0 gate met (2026-10-05, symlink default for both adapters; see `spike/symlink-discovery.md` Outcome): the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter. No application code and no Cargo files before this.
- [x] Milestone M0 report written (`phase-01-m0-symlink-spike_REPORT_04-10-26.md`) and its Forward Preview read.
- [x] Taylor's confirmation that M0 is complete (confirmed 2026-10-05).

---

## Blast Radius

- `Cargo.toml` (virtual workspace, resolver "3"), `Cargo.lock`
- `crates/unam-core/Cargo.toml`, `src/lib.rs`, `error.rs`, `config.rs`, `source.rs`, `catalog/{mod,scan,frontmatter,search}.rs`; `hash.rs` (the blake3 source-tree hash helper, created in M1 as the first-test dependency; reused by M3 and later)
- `crates/unam-cli/Cargo.toml` (package `unam`), `src/main.rs`, `src/cli/` (source, search, show), `src/output.rs`
- `crates/unam-cli/tests/` and `tests/fixtures/`
- CI workflow file and `cargo-dist` configuration (paths chosen at RESEARCH; spec 10 names them without paths)
- clippy configuration (unwrap denied outside tests)
- Not touched: `docs/`, `spike/`, the skill library, `.claude/`, `.codex/`, `.agents/`

---

## Implementation Checklist

Refined at PLAN-SUPPLEMENT after RESEARCH; spec sections are cited, not copied.

### Step A: Workspace and gates

- [ ] A1. Create the workspace and both crates per spec 12 and 1.3 (package `unam` for the binary); add dependencies per spec 1.4 via `cargo add`; commit `Cargo.lock`.
- [ ] A2. Configure clippy so `unwrap` is denied outside tests (spec 7); confirm `cargo clippy --workspace --all-targets -- -D warnings` passes on the empty skeleton.
- [ ] A3. Add CI running fmt, clippy, and test on macOS and Linux (all-tests.md "CI target"); add `cargo-dist` config (crate and package names per spec 1.3: `unam-core`, `unam-cli`, package and binary `unam`).
- [ ] A4. Create the error enum with remedy strings for the variants M1 needs (spec 7).

### Step B: Config and sources

- [ ] B1. Implement `config` per spec 4.1 and 2.1, including env overrides (`UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`).
- [ ] B2. Implement `source add|list|rm` per spec 4.2 and 5 (exit codes 0, 2, 3).

### Step C: Catalog, frontmatter, search, show

- [ ] C1. Implement scan with `ignore` per spec 4.3 (skip `.git`, `node_modules`; nested categories with real paths) and the scan cache.
- [ ] C2. Implement frontmatter parsing per spec 4.3, covering the unit cases listed in spec 8 (valid, missing description, unparseable, colon-in-value retry, name/dir mismatch warning).
- [ ] C3. Implement `search` (fuzzy and field search, `--category`, `--source`) and `show` (spec 5; M1 fills only fields available without projects or bundles; later milestones add the rest).
- [ ] C4. Implement the bare `unam` stub (print usage; spec 5 says status arrives with later milestones).

### Step D: Tests

- [ ] D0. FIRST TEST, written before any other test: implement the source-tree hash helper (`hash.rs`, blake3 tree hash per spec 8) and write `source_tree_unchanged_read_side` in `crates/unam-cli/tests/`: register a fixture source, scan, and search, then assert the tree hash of the source is identical before and after (spec 12; PRD S3). Keep the helper reusable so milestone M3 asserts it in every mutating test.
- [ ] D1. Build fixtures per spec 8 Integration; point every env override at `tempfile` directories.
- [ ] D2. Write `ac_01_*`, `ac_03_*`, `ac_04_*`, `ac_05_*`, `ac_06_*`, `ac_38_*` as integration tests in `crates/unam-cli/tests/`; write the frontmatter and search unit tests inline.
- [ ] D3. Performance check (exit gate): generate a 500-skill fixture in a `tempfile` directory (nested categories, valid frontmatter) and write `perf_cold_scan` in `crates/unam-cli/tests/`: run a cold scan (no cache) five times in a release build and assert the median wall-clock time is under 300 ms (spec 9 "Cold scan, 500 skills, local SSD"). Record the measured median in the phase report. Warm start is a milestone M6 check, not M1.

---

## Exit Gate

```bash
cargo fmt --all --check
# Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# Expected: exit 0, no unwrap findings outside tests
cargo test --workspace
# Expected: all tests pass
cargo test -p unam --test <file> ac_01   # repeat for ac_03, ac_04, ac_05, ac_06, ac_38
# Expected: pass
cargo test -p unam --test <file> source_tree_unchanged_read_side
# Expected: pass (first test; PRD S3 read-side)
cargo test -p unam --release --test <file> perf_cold_scan
# Expected: pass; median cold scan of the generated 500-skill fixture under 300 ms (spec 9)
```

- All checklist items checked; AC-1, 3, 4, 5, 6, 38 (read-side) tests green; `source_tree_unchanged_read_side` green; `perf_cold_scan` green (measured median recorded in the phase report).
- No test touches the real `~/.config/unam`, `~/.claude/skills`, `~/.agents/skills`, the skill library, or this repository.
- Regression: M0 table and Outcome unchanged.
- Phase report written to the report destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- A phase can be VERIFIED while the program is incomplete.
- Commit execution changes on `main` before starting milestone M2; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met (table unfilled or Outcome missing).
- Frontmatter parser crate choice (`serde_norway`) fails the spec 8 cases: fall back per spec 1.4 note (`serde-saphyr`) via a recorded decision.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [ ] 1. RESEARCH: research-agent: M0 report and Outcome read; test context loaded; plan drift checked; first-test and perf-check tasks (D0, D3) confirmed
- [ ] 2. INNOVATE: innovate-agent: approach decided; Decision Summary written
- [ ] 3. PLAN-SUPPLEMENT: plan-agent: existing phase plan updated; Inner Loop Refresh Note if sections changed (or "n/a: research clean")
- [ ] 4. PVL: vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [ ] 5. EXECUTE: all checklist items done; per-section test gates run and green (or gaps documented)
- [ ] 6. EVL: all EVL gates green; follow-up stubs registered; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Touchpoints

- Workspace and crate files listed in Blast Radius (created)
- Read: `docs/spec/unam-technical-spec.md` sections 1.3, 1.4, 2.1, 4.1 to 4.3, 5, 7, 8, 9, 12; PRD sections 7.1, 13.2
- `process/context/all-context.md` and `process/context/tests/all-tests.md` updated at UPDATE-PROCESS (tests become runnable)

---

## Public Contracts

- Introduces the CLI commands `source add|list|rm`, `search`, `show` and the `--json` envelope shape (spec 5); later milestones must not break them.
- Introduces `config.toml` format and env overrides (spec 2.1).
- Exit codes 0, 2, 3, 4 as defined in spec 5.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` | Fully-Automated | spec 8 gates |
| `cargo test -p unam --test <file> ac_01` (non-Git directory registers; skills discoverable) | Fully-Automated | AC-1 |
| `ac_03_*` (three-level nested categories, real paths) | Fully-Automated | AC-3 |
| `ac_04_*` (`.git/` and `node_modules/` yield no skills) | Fully-Automated | AC-4 |
| `ac_05_*` (name/dir mismatch warns; bad YAML skipped with diagnostic) | Fully-Automated | AC-5 |
| `ac_06_*` (search across three categories, one result set) | Fully-Automated | AC-6 |
| `ac_38_*` (unmounted source unreachable, no mutation) | Fully-Automated (simulated missing path) | AC-38 read-side |
| `source_tree_unchanged_read_side` (register, scan, search; source tree hash identical) | Fully-Automated | PRD S3 (read-side); prerequisite for AC-2 closing in M3 |
| `perf_cold_scan`: median cold scan of a generated 500-skill fixture under 300 ms (release build, five runs) | Hybrid (precondition: local SSD, release build; assertion automated) | spec 9 cold scan target |

No Known-Gap rows. Snapshot tests (`insta`) cover `--json` output of each M1 command.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md`
- Last completed step: not started
- Validate-contract status: pending
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, spec sections cited above
- Next step: after M0 is VERIFIED, spawn vc-research-agent for RESEARCH (step 1).

---

## Validate Contract

(placeholder - vc-validate-agent writes this section before EXECUTE)

---

## Next Step

After the M0 gate, run RESEARCH for this milestone (RIPER-5), then PLAN-SUPPLEMENT and PVL before any EXECUTE.
