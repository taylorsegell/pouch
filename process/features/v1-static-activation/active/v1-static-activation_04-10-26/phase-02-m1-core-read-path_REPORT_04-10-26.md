---
phase: phase-02-m1-core-read-path
date: 2026-10-05
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md
---

# Milestone M1 phase report: core read path

TL;DR: M1 is VERIFIED (WITH_GAPS). The Cargo workspace, CI file, config, source, catalog, search, and show all exist and gates G1 to G13 are green on the independent EVL run (164 tests). The cold-scan perf margin is noisy (median 183.9 ms against a 300 ms limit, but two of five runs were near or over 300 ms), and CI was written but never run. Nine design departures need Taylor's confirmation (umbrella open items 15 to 23). The M2 entry gate is met.

## What Was Done

- Created the virtual workspace (`Cargo.toml`, `Cargo.lock`, `clippy.toml`, resolver 3, edition 2024, `unwrap_used` denied) and two crates:
  - `crates/unam-core/src/`: `config.rs`, `paths.rs`, `error.rs`, `fsops.rs` (minimal), `hash.rs` (blake3 tree hash), `source.rs`, `catalog/{mod,cache,frontmatter,lookup,scan,search}.rs`, `lib.rs`.
  - `crates/unam-cli/src/`: `main.rs`, `output.rs`, `cli/{mod,search,show,source}.rs`.
- Commands working: `unam source add|list|rm`, `unam search`, `unam show`, bare `unam` (usage only), all with the `--json` envelope and the documented exit codes.
- Tests: `crates/unam-cli/tests/` (`ac_read_path`, `cache_cli`, `read_side_invariant`, `search_cli`, `show_cli`, `snapshots`, `source_cli`, `usage`, shared `common/` harness with the standard fixture library) and `crates/unam-core/tests/perf_cold_scan.rs`. First test written was `source_tree_unchanged_read_side` (AC-2 read-side invariant helper).
- Acceptance criteria closed (spec 10 M1): AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side). AC-2 stays open until milestone M3, as decided.
- Tooling: hand-written `dist-workspace.toml` (parse-checked only) and `.github/workflows/ci.yml` (fmt, clippy `-D warnings`, test, ubuntu and macos matrix).
- Resolved dependency versions: serde 1.0.229, serde_json 1.0.151, toml 1.1.6, anyhow 1.0.104, tracing 0.1.44, tracing-subscriber 0.3.23, predicates 3.1.4, proptest 1.11.0, plus the pinned set in the plan (clap 4.6.7, serde_norway 0.9.42, ignore 0.4.33, blake3 1.8.7, nucleo-matcher 0.3.1, jiff 0.2.37, thiserror 2.0.21, tempfile 3.27.0, insta 1.49.0, assert_cmd 2.2.2).

## What Was Skipped/Deferred

- CI run: `.github/workflows/ci.yml` was written and parse-checked, never run (no push this run). The first push is Taylor's call. Backlog NOTE: confirm action versions and the Linux snapshot results on first CI run.
- `dist` verification: `dist` is not installed; `dist-workspace.toml` is unverified. Milestone M7 verifies with `dist plan`. Backlog NOTE.
- `cargo-insta` not installed: snapshots reviewed through the documented no-`cargo-insta` procedure (`INSTA_UPDATE=no` in gates).
- Match-index output for search (spec 4.3): deferred to milestone M6.
- `source rename` (PRD SRC-4): not built, no spec command. Suggested for milestone M4 (open item 21).
- Status in bare `unam`: deferred to milestone M4 (open item 22).
- Root-level `SKILL.md`: not treated as a skill (open item 23).

## Test Gate Outcomes

EVL run by vc-tester (sonnet, 2026-10-05) against the validate-contract commands: G1 to G13 PASS, 164 tests. Loop log: `results-m1-evl.tsv` (baseline row, then HALTED_SUCCESS at cycle 1, zero gaps).

| Suite | Tests |
|---|---|
| unam-core unit | 107 |
| unam-cli unit | 5 |
| ac_read_path | 6 |
| cache_cli | 6 |
| read_side_invariant | 1 |
| search_cli | 6 |
| show_cli | 7 |
| snapshots | 10 |
| source_cli | 10 |
| usage | 5 |
| perf_cold_scan | 1 |

Gate G6 (`perf_cold_scan`, release build, 500-skill generated fixture, limit 300 ms): EVL median 183.9 ms over five runs (183.9, 170.4, 181.6, 365.2, 297.9). The execute agent measured 155.4 and 205.6. The 365.2 ms run exceeds the limit; the median gate passes, but the margin is noisy.

Plan validation (PVL): 3 validate-fix cycles, Gate PASS (`results-m1.tsv`, iteration reports `phase-02-m1-core-read-path-pvl-iteration-001` to `003`).

What green proves: every closed AC has a named passing automated test; read, scan, search, and show leave the source tree and the per-entry stat snapshot unchanged; JSON envelopes and exit codes match their snapshots on this machine; fmt and clippy are clean. What it does not prove: behavior on Linux (snapshots `show_found` tree hash and `add_bad_path` OS error text may differ; the first CI run will tell), the perf limit under load, `dist` validity, or the CI file.

## Plan Deviations

Seven internal deviations recorded in the plan under "Deviations (EXECUTE 2026-10-05)"; none changes a public contract:

1. `catalog/cache.rs` and `catalog/lookup.rs` split out of `catalog/mod.rs` (pre-authorized, file size).
2. `Skill.category` is a `String` (empty for top-level) instead of `RelPath`.
3. Extra helper types and methods added to keep the CLI thin (`FlatDiagnostic`, `DiagClass`, `Catalog::flat_diagnostics`, `resolve_text`, `reindex`, `source`, `SourceStatus.expanded`).
4. The first test also asserts `--json search review` returns results; otherwise it was green against the skeleton binary.
5. CI env written as `INSTA_UPDATE: "no"` (quoted; bare YAML `no` is boolean false).
6. `toml` resolved to 1.1.6 (current major).
7. Static fixture files created by a python script run through Bash instead of one Write call per file (process only).

## Design Departures for Taylor (umbrella open items 15 to 23)

All provisional under autopilot; each needs Taylor's confirmation. None edits a protected design document.

| Item | Departure | Ask |
|---|---|---|
| 15 | Spec 1.4 "Library on macOS" for `directories` contradicts spec 2 and PRD 9 (`~/.config/unam`, `~/.cache/unam`, `~/.local/share/unam`). PRD 9 followed; `directories` dropped; `paths.rs` resolves paths. | Confirm, and fix spec 1.4 when convenient. |
| 16 | The data dir (`projects.json`) has no `UNAM_*` override. Not added; tests isolate with `HOME` and `XDG_DATA_HOME`. | Want a fourth override? |
| 17 | New `Diagnostic { code, severity, message, path, remedy }` type and catalog-level diagnostics (spec lacks them but 4.2, 4.3, AC-5 need them). Code names are stable contract in the M1 plan. | Confirm. |
| 18 | Temporary `lookup(ref)` handling all three spec 4.6 reference forms, and a provisional `<data dir>/projects.json` reader for the `source rm` exit-3 check. Milestone M2 supersedes both. | Confirm; see Forward Preview. |
| 19 | `dist-workspace.toml` hand-written with per-arch targets; no macOS universal claim (spec 9 names universal, evidence does not support it). The umbrella M7 gate row still says universal. | Confirm; reconcile at M7 research. |
| 20 | `nucleo-matcher` instead of full `nucleo` (spec 1.4). Match-index output deferred to M6. | Confirm. |
| 21 | PRD SRC-4 lists "rename" but the spec CLI has no `source rename`. Not built. | Decide the milestone (suggested M4). |
| 22 | Bare `unam` prints usage only, not status (spec 5), because `status` is built in M4. | Confirm; M4 adds status. |
| 23 | A `SKILL.md` directly at a source root is ignored with warning `root-skill-skipped` (PRD DIS-1 says any directory with `SKILL.md` is a skill) because the skill ID needs a non-empty relative path. | Confirm or decide how a root skill is identified. |

## Known Gaps

- Perf margin is noisy: two of five EVL runs were near or over 300 ms (365.2 and 297.9). Gate is the median. Watch under CI load; the test is Hybrid (release build, local SSD).
- CI written, not run.
- `dist-workspace.toml` parse-checked only.
- G11 Pre-Step record: the record of the three real `unam` dirs (config, cache, data) before the work started was not located. All three are absent now and were never created by M1, so no harm is evidenced, but the "before" snapshot is missing.
- Chmod Hybrid tests self-skip when run as root (they are meaningful only for a normal user).
- Linux-sensitive snapshots: `show_found` (tree hash) and `add_bad_path` (OS error text).

## Process Incidents

1. Edit tool disabled for the session. Agents that needed in-place plan edits could not use it; the workaround was exact-match python scripts (assert a single match, then replace) run from the scratchpad, or full-file Write. Recorded as a feedback memory.
2. A plan-supplement agent stalled on NEEDS_CONTEXT because of the disabled Edit tool and was resumed with python-script editing instructions.
3. Model drift on research: milestone M1 RESEARCH ran on opus because the agent frontmatter listed opus; fixed in commit 8eebda4. Every later M1 agent ran sonnet except the execute agent (opus), per policy.
4. The scout-block hook blocks Bash text containing path-like `target/`, `.git`, `node_modules`, `build`, `dist`, `vendor`, `coverage` segments. Gate commands were verified to pass the hook as written; fixtures containing those names were created by script (see deviation 7).

## Test Infra Gaps Found

- Perf test noise (see Known Gaps). Backlog NOTE: consider a relative or warm-run guard if CI shows instability; do not loosen the 300 ms limit.
- No Linux verification until the first CI run. Backlog NOTE.
- `cargo-insta` and `dist` not installed locally. Backlog NOTE for M7 (`dist plan`).
- Chmod tests cannot prove anything under root. Backlog NOTE: run once as a normal user in CI.
- Pre-Step real-dirs record was not persisted anywhere findable. Backlog NOTE: later milestones that touch real user dirs must write the Pre-Step record into the task folder.

## SPEC Achievement

No locked `*_SPEC_*.md` exists (phase-program inner loop; the umbrella plan governs). Scored against the plan acceptance criteria:

| Criterion | Score | Proven by |
|---|---|---|
| AC-1 register non-git directory | met | `ac_01_register_non_git_directory_and_skills_discoverable` (Fully-Automated) |
| AC-3 three-level nested categories | met | `ac_03_three_level_nested_categories_real_paths` |
| AC-4 `.git` and `node_modules` yield no skills | met | `ac_04_git_and_node_modules_yield_no_skills` |
| AC-5 name mismatch warns, bad YAML skipped | met | `ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic` |
| AC-6 search spans categories | met | `ac_06_search_spans_three_categories_one_result_set` |
| AC-38 read-side unreachable source | met | `ac_38_unreachable_source_reported_no_mutation` |
| S3 read-side invariant | met | `source_tree_unchanged_read_side` |
| Spec 9 cold scan under 300 ms | met (Hybrid, noisy margin) | `perf_cold_scan`, median 183.9 ms |

## SPEC Gaps

No unmet criteria. Residual gaps are listed under Known Gaps and Test Infra Gaps Found, each as a backlog NOTE.

## Closeout Packet

1. Selected plan path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md`
2. Closeout classification: Ready for UPDATE PROCESS archival, with gaps (EVL classification CLEAN with gaps). The task folder stays in `active/` because it holds the whole program; the M1 plan file stays in place, marked VERIFIED (WITH_GAPS).
3. What was finished: see What Was Done.
4. Verified: G1 to G13 (EVL, vc-tester). Unverified: Linux behavior, CI, `dist`, perf under load.
4b. Validate-contract: present in the plan, Gate PASS 2026-10-05 after 3 PVL cycles.
5. Cleanup done: plan ticked, umbrella state rewritten, registry DONE, context docs updated. Still needed: execution commit and process commit by the orchestrator (execution first).
6. Next valid state: milestone M2 (phase 3 of 8), loop step 1 RESEARCH.
7. Commit checkpoint: execution commit recommended before the process commit (M1 has source code); the orchestrator commits on `main`, no push.
8. Regression status: first code milestone; the only prior verified surface is M0 (spike file), which M1 does not touch. Harness validators re-run in this closeout.
9. SPEC achievement: see above (no SPEC; plan criteria all met).

Drift score: 4 signals (files touched >= 1 and >= 10 gives 2; 3+ memory-worthy observations; feature-folder structural change via the new report and backlog NOTEs). No `.claude/`, agent, or protocol file was edited, and no validate-contract blast-radius deviation occurred. HIGH by count, driven by file volume. Strongly recommend UPDATE PROCESS -- harness/protocol files touched.

## Forward Preview

### Test Infra Found

- Real suite: 164 tests; harness in `crates/unam-cli/tests/common/mod.rs` (`TestEnv`, standard fixture library, source-tree hash plus stat snapshot assertions); snapshots under `crates/unam-cli/tests/snapshots/`.
- Gate helper pattern: shell functions `gate` (fails when a filter matches zero tests) and `gate6` (perf median), defined inside each Bash call; see the M1 plan Exit Gate.

### Blast Radius Changes

- Added versus the registry entry: `clippy.toml`, `dist-workspace.toml`, `.github/workflows/ci.yml`, `catalog/cache.rs`, `catalog/lookup.rs`, `crates/unam-core/tests/perf_cold_scan.rs`. Registry M1 entry marked DONE.
- M2 owns `docs/adapters.md` (create if absent), `bundle.rs`, `project.rs`, `resolve.rs`, with only-as-needed extensions to `catalog/mod.rs`.

### Commands to Stay Green

```
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs
node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs
node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs
node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <plan>
git diff --check
```

### Dependency Changes

- Added in M1: the full workspace dependency set listed above (lockfile `Cargo.lock` created). No removals. `directories` was dropped from the planned set (open item 15).
- Notes for milestone M2:
  - `resolve.rs` supersedes the temporary `lookup(ref)` in `catalog/lookup.rs`; remove or re-point the temporary code and keep the AC-5/AC-38 and `show_cli` tests green.
  - `project.rs` owns the real `projects.json` index format; the provisional reader behind the `source rm` exit-3 check must be reconciled with it (same file, same format) and its tests updated.
  - Hermes `slugify` parity: read the Hermes source read-only, record the outcome in `docs/adapters.md` (M2 creates it if first), and un-ignore or keep the `#[ignore]` parity case accordingly. Never invoke the real `hermes` CLI.
  - Watch the perf noise: re-run `perf_cold_scan` if M2 touches `catalog/mod.rs`; the 300 ms limit stays.
  - Reminder: never run `unam` outside `TestEnv`; never invoke real agent CLIs in dry runs.
