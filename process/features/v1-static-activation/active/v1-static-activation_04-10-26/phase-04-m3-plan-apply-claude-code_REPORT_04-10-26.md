---
phase: phase-04-m3-plan-apply-claude-code
date: 2026-10-09
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md
---

# Milestone M3 phase report: plan and apply for claude-code

TL;DR: M3 is VERIFIED (WITH_GAPS). `add`, `rm`, `use`, `deactivate`, `resume`, `rollback` work for the claude-code adapter with a journal, receipt, history, and project lock. Gates G1 to G19 are green on the independent EVL run (vc-tester, sonnet, 512 tests). You approved the high-risk evidence pack in chat on 2026-10-09 (decision `approved`). Four things stay open: Linux/CI never run, the D4 real activation (yours), five production files over the size guide, and no red-run record (D22). The M4 entry gate is met.

## What Was Done

- New core modules (`crates/unam-core/src/`): `adapter/{mod,claude_code}.rs` (trait, evidence table, strategies from the M0 outcome), `observe.rs` (what is on disk), `state.rs` (receipt, JSONL journal, history; `.unam/<target>/` layout), `plan.rs` (pure `plan::diff`), `apply.rs` (journaled apply, `resume`, `rollback`), `change.rs` (orchestration). `fsops.rs` rewritten behind a public `FsOps` seam with typed paths. `hash.rs` reused unchanged.
- Changed M1/M2 files: `error.rs` (20 variants), `paths.rs`, `source.rs`, `resolve.rs`, `catalog/mod.rs`, `project/selection.rs`, `output.rs`, plus the `write_atomic` caller migration to typed paths (no behavior change). One `rust-version = "1.89"` line in each of the three `Cargo.toml` files; `Cargo.lock` unchanged.
- CLI: `cli/change.rs` with the six commands and the global flags `--project`, `--target`, `--yes`. Plan output follows SES-1 and SES-2.
- Tests: new `ac_activation` (17), `ac_recovery` (13), `activation_cli` (36), `common/faults.rs`; snapshots 19 to 31 (12 new; among pre-existing only `usage_bare` changed). Total 512 tests.
- Acceptance criteria closed: AC-2, 10 (end to end), 16, 17, 18, 19, 22, 23, 25, 30, 31, 32, 40, 53; AC-13, 15, 21 on the applied-state half (status half in M4); AC-24 and AC-29 claude-code half; S2, S3, S4, S9 (claude-code).
- Trust-boundary and recovery gates closed with named tests: crafted state symlinks refused, destination-root re-check, torn journal tail, write-ahead dir record, project-overlaps-source refusal, `.DS_Store` removal rule.
- Evidence pack written to `harness/` (`risk-gate`, `context-snippets`, `verification`, `adversarial-validation`, `review-decision`). `validate-risk-artifacts.mjs` failures: `[]`.
- Pre-Step record: `phase-04-m3-pre-step_REF_05-10-26.md` (real directories untouched, baseline 296 tests).

## What Was Skipped/Deferred

- Dropped from M4 by the decision sheet (row 53): nested `scan_locations`, VCS-tracked warning (EDG-12), `--skip-conflicts`, case-insensitive collisions (EDG-13).
- Source `rename` (PRD SRC-4): proposed drop for v1 (sheet row 21).
- Hybrid gate D4, one real activation in the first adopting project: backlog NOTE `process/features/v1-static-activation/backlog/m3-d4-first-real-activation_NOTE_05-10-26.md`. Yours to run; stays CONDITIONAL; does not block M4 or M5.
- Status halves of AC-12, 13, 15, 21, 35 and `reconcile` zero-divergence: M4.

## Test Gate Outcomes

EVL by vc-tester (sonnet), loop log `results-m3-evl.tsv`: baseline, then `HALTED_SUCCESS` at cycle 1 with zero gaps. PVL log `results-m3.tsv`: Gate CONDITIONAL with 12 gaps, PASS after 3 fix cycles (iteration reports 001 to 004).

| Gate | Result |
|---|---|
| G1 fmt, G2 clippy (`-D warnings`, no `unwrap` in non-test code), G3 `cargo test --workspace --locked` (512 tests, none ignored) | PASS |
| G4, G4b, G4c per-AC and trust-boundary commands (each exactly 1 passed) | PASS |
| G5 M1/M2 regression (all earlier suites unmodified) | PASS |
| G6 perf, release | PASS, median 121.6 ms (limit 300 ms) |
| G7 snapshots | PASS, 31 snapshots, only `usage_bare` changed among pre-existing |
| Fault matrix (AC-31, about 62 s full run) | PASS |
| Source-tree and foreign-file invariants, protected-docs diff | PASS |
| G10b evidence pack review | PASS, `approved` by you, 2026-10-09 |
| G19 and remaining gates G8 to G18 | PASS |

Deletion paths were reviewed in the evidence pack with no departures from the plan, and no deviation weakens deletion safety.

## Plan Deviations

Twenty-three recorded in the plan (D1 to D23). Six matter for later milestones:

| # | Deviation | Effect |
|---|---|---|
| D7 | Journal `Begin` record gains `selection_hash` (journal format) | `resume` writes the same receipt the interrupted apply would have. M4 builds on this frozen format. |
| D8 | `open_lock_file` also refuses a lock path whose parent `.unam` is a symlink | Stricter than the plan; no cost. |
| D9 | `read_journal` rejects a `receipt_before` entry failing the receipt consistency rule | Crafted journals cannot escape the destination root. |
| D10 | `resume` re-evaluates a kept `remove_dir` step | Found by the fault matrix; without it `resume` could not reach the clean state. |
| D13 | Failure payload travels in `DataError` inside `Failed.error` | Avoids edits outside the blast radius; `--json` output identical. |
| D16 | Edit made before the command: ownership released, exit 0, nothing deleted; exit 7 only for an edit in the confirm window | Matches P7. You approved it (sheet row D16). |

The other seventeen are naming, layout, or test-mechanics details (D1 to D6, D11, D12, D14, D15, D17 to D20, D23) or process notes (D21 heredoc/script used for two non-source writes; D22 below). D4 is the file-size item under Known Gaps.

## Test Infra Gaps Found

- FsOps fault injection exists only in `crates/unam-cli/tests/common/faults.rs`; the full fault matrix takes about 62 s. The wave plan moves it to a strided sample by default, with `UNAM_FULL_FAULT_MATRIX=1` for the full run.
- No pty test for the interactive prompt (R8). Confirmation is tested only through an injected reader.
- No kernel-level torn-write test; crashes are simulated in process.
- `mkfifo`, chmod, and read-only data-directory rows self-skip when the environment cannot support them.

## SPEC Achievement

The umbrella SPEC governs this phase. Every closed criterion above is scored met by a passing automated gate (Fully-Automated; `perf_cold_scan` is Hybrid and passed). Unmet or partial, each with a backlog or M4 note:

| Criterion | Score | Where it closes |
|---|---|---|
| AC-13, 15, 21 | met on applied state, status half unmet | M4 (`status`) |
| AC-24, 29 | met for claude-code, second-adapter half unmet | M5 |
| AC-12, 35 | unchanged from M2: status half unmet | M4 (backlog note `m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md`) |
| D4 real activation | unmet (Hybrid, yours) | backlog NOTE above |

## Closeout Packet

1. Plan: `phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md`.
2. Classification: Ready for UPDATE PROCESS archival of the phase as a milestone step; the plan file stays in `active/` with the task folder (the program archives the folder as a unit, per earlier milestones).
3. Finished: see What Was Done.
4. Verified: G1 to G19. Unverified: Linux, real Claude Code discovery (M0 spike did it once), D4, Taylor's real library.
4b. Validate-contract: present, Gate PASS (cycle 4), `generated-by: inner-pvl: phase-4`.
5. Cleanup: report, plan ticks, umbrella state, registry, context done here. Still needed: nothing for M3.
6. Next valid state: milestone M4 (phase 5 of 8), loop step 1 RESEARCH.
7. Commit: process-only changes here (`process/` artifacts); the orchestrator makes the commits (execution first, then process). No commit was made by this step.
8. Regression: M1 and M2 suites unmodified and green (G5); 19 pre-existing snapshots unchanged except `usage_bare`.
9. SPEC: see SPEC Achievement.

Drift score: HIGH (4 signals: 10+ files touched, task-folder artifacts added, backlog NOTE written, validate-contract deviations D1 to D23). Strongly recommend UPDATE PROCESS -- harness/protocol files touched. (Here no harness files changed; the HIGH band comes from file count and deviations, and this report is that UPDATE PROCESS.)

Autonomy record: no Phase 3 prompt was shown. Your approval of the evidence pack was recorded in chat; the context and plan edits are inside the blast radius and reversible.

## Known Gaps

- **Linux and CI never run.** `.github/workflows/ci.yml` is written but unpushed. Snapshots `show_found` (tree hash) and `add_bad_path` (OS error text) may differ on Linux.
- **D4 real activation pending.** First adopting project unnamed; yours to run.
- **File-size guide exceeded (D4).** Production lines: `apply.rs` 1331, `fsops.rs` 1115, `state.rs` 1063, `plan.rs` 942, `change.rs` 617, `cli/change.rs` 602. Not split. The M3 journal and hardening depth is frozen, not refactored.
- **No red-run record (D22).** Tests were written with each module; no separate failing run was captured for the first new core test.
- **Residuals R1 to R10**, all accepted by you as named known gaps:

| # | Residual |
|---|---|
| R1 | Microsecond window between the second look and `remove_file` of a symlink; copies are renamed aside and re-hashed, so a changed copy is restored |
| R2 | Path component of `.claude/skills` swapped for a symlink after the destination-root check (std only, no `openat`) |
| R3 | Empty directory created just before `rename(temp, dst)` is replaced (no data loss) |
| R4 | Lock-file open races inside the project (planted dangling symlink, swapped `.unam`) |
| R5 | `.unam-tmp-<op_id>-<name>` visible in the destination root during a copy and after a crash until `resume` or `rollback` |
| R6 | Hostile checked-in `.unam/` state counts as a claim of ownership; cannot reach outside `.claude/skills`, modified, or foreign entries |
| R7 | Crash between the `dir` intent record and `create_dir` with a concurrent creator: `rollback` may remove that empty directory |
| R8 | Interactive prompt covered only through an injected reader; no pty test |
| R9 | Removing an owned copy with read-only subdirectories fails midway and leaves residue in `trash/` (reported, not silent) |
| R10 | `source rm` guard counts only literal `add`/`remove` entries; bundle-only selections are invisible (umbrella items 26 and 55; fix in M4) |

Under your 2026-10-09 threat model (same user, trusted local filesystem; untrusted input is skill file content and cloned-repo files) R1, R2, R3, R4, R7 need a hostile concurrent process and are outside it. R6 and R9 stay relevant.

## Over-Engineering Audit (2026-10-09), context for M4

- Finding: about 15 to 20 percent excess, mainly slugify parity (the Unicode word table and generator, removal scheduled next in the waves) and journal/hardening depth. The journal and hardening code is frozen, not refactored; M4 builds on it as is.
- Process overhead dominated elapsed time, not code. The lighter process is in effect: plans capped near 400 lines, PVL at most 2 cycles with only FAIL-class gaps triggering a cycle, a short gate list, symlink-only going forward. See `v1-static-activation_WAVES_09-10-26.md` and the Threat Model section of `process/context/all-context.md`.
- The seven open decisions for you are in `open-items-decision-sheet_REF_09-10-26.md`; proposed answers apply unless you mark rows.

## Forward Preview

#### Test Infra Found

- Suites to keep green: `ac_activation`, `ac_recovery`, `activation_cli`, `snapshots` (31), `common/faults.rs` (strided fault sample by default after the wave change).
- Add `status` halves to the existing `ac_13`, `ac_15`, `ac_21` tests rather than new files where possible.

#### Blast Radius Changes

- M4 owns: `verify.rs`, `reconcile.rs` (new); additions to `apply.rs` (rollback from history), `source.rs` (relocate), `adapter/claude_code.rs` (doctor probes, `refresh`), `state.rs`; CLI `status`, `reconcile --repair`, rollback from history, `doctor --evidence`, `source relocate`; `docs/adapters.md` (record the claude-code evidence table and M0 Outcome from `Adapter::evidence()`).
- M4 closes: AC-14, 26, 27, 28, 33, 34, 36, 37, 39, 41, plus the status halves of AC-12, 13, 15, 21, 35. Verify reports the six states.
- M4 must include the sheet's row 26/55 fix: the `source rm` guard resolves the selection so bundle members count; `bundle rm` still never blocks.
- Lighter process applies: plan near 400 lines, PVL at most 2 cycles, short evidence check on `reconcile --repair` deletes, symlink-only (add no new copy-specific branches), build on the frozen M3 journal without refactoring it.

#### Commands to Stay Green

```
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --release -p unam-core --test perf_cold_scan   # only if catalog code changes
```

Plus the source-tree and foreign-file invariants and the protected-docs diff (`docs/PROJECT-BRIEF.md`, kickoff prompt, PRD, spec, ADR-0001 unchanged).

#### Dependency Changes

None in M3 (`Cargo.lock` unchanged). `rust-version = "1.89"` added to the three manifests for std `File::try_lock`. M4: none expected.

## Wave-Plan Position

M3 close satisfies the Wave 1 carry-over. Not done here, and not started: slugify machinery removal, fault-matrix sampling, and M4. Next wave: `v1-static-activation_WAVES_09-10-26.md` Wave 2.
