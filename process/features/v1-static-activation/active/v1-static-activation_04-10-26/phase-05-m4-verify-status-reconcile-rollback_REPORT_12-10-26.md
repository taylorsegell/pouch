---
phase: phase-05-m4-verify-status-reconcile-rollback
date: 2026-10-12
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md
---

# Milestone M4 phase report: verify, status, reconcile, rollback, doctor

TL;DR: M4 is VERIFIED (WITH_GAPS). `status`, `reconcile [--repair]`, `doctor [--evidence]`, rollback from history, and `source relocate` work, and `source rm` now counts bundle members. The independent EVL run is clean (G1 to G6 green, 567 tests, full fault matrix passes), and the deletion-safety evidence check passed. Open for you: three interpretations to read (below), the seven decision-sheet rows, the first adopting project (D4), and Linux/CI, which have still never run. The commit is pending (a separate git step: code commit, then process commit).

## What Was Done

- New core modules in `crates/unam-core/src/`: `verify.rs` (six-state classification, shadow analysis, candidates for a renamed skill), `reconcile.rs` (divergence report by cause, preview plan, `--repair` through `change::run`), `doctor.rs` (environment checks and the adapter evidence listing).
- Changed core: `error.rs` (variants `NoHistory` exit 4, `Diverged` exit 8, `DoctorFailed` exit 9), `plan.rs` (`DiffInput.repoint_moved`), `state.rs` (`read_latest_history`), `change.rs` (`Edit::Reconcile`, `Edit::Restore`, `Edit::FromHistory`, `history_edit`, `has_journal`), `observe.rs` (`list_foreign`), `source.rs` (`relocate`, `remove` guard), `project/index.rs` (`projects_with_receipt_from`, `projects_using_source`), `adapter/claude_code.rs` (one evidence wording line), `lib.rs`.
- CLI: new `cli/status.rs` (`status`, `reconcile`) and `cli/doctor.rs`; `cli/mod.rs` (new commands, `source relocate`, bare `unam` prints usage then the status table when the root holds `.unam/`, always exit 0); `cli/source.rs` (relocate flow, `rm` guard); `cli/change.rs` (`rollback` with no journal restores from history); `output.rs` (six-state table, spec 4.11 footer).
- Tests: new `ac_verify.rs` (7), `ac_reconcile.rs` (12), `doctor_cli.rs` (5); `ac_recovery.rs` now 14 (adds `s7_interruption_zero_divergence`); additive status halves in `ac_selection.rs`, `ac_activation.rs`; five new snapshots (`status_json`, `reconcile_json`, `rollback_history_json`, `doctor_json`, `source_relocate_json`; 36 in total); `common/mod.rs` and `common/faults.rs` gain helpers. 567 tests in all (512 at M3).
- Docs: `docs/adapters.md` gained the claude-code evidence table and the M0 Outcome; the two M2 sections are kept. The five protected documents are untouched.

## AC Coverage

All strategies Fully-Automated except one Hybrid residual (doctor unwritable destination, self-skips as root).

| Criterion | Result |
|---|---|
| AC-14, 26, 27, 28 | met: `ac_14_*`, `ac_26_*`, `ac_27_*`, `ac_28_*` in `ac_verify.rs` |
| AC-36, 39, 41 | met: `ac_36_*`, `ac_39_*`, `ac_41_*` in `ac_verify.rs` |
| AC-33, 34, 37 | met: `ac_33_*`, `ac_34_*`, `ac_37_*` in `ac_reconcile.rs` |
| AC-12, 13, 15, 21, 35 status halves | met: assertions appended to the existing tests; the M2 backlog note `m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md` is now satisfied |
| S7 | met: `s7_interruption_zero_divergence` (strided sample by default, full set with `UNAM_FULL_FAULT_MATRIX=1`; the full set passed at EVL) |
| S8 | met: `s8_divergence_always_reported` |
| Row 26/55 (R10) | fixed: `source rm` counts bundle members; `bundle rm` still never blocks |

Not closed here, by design: AC-24 and AC-29 second-adapter halves (M5).

## What Was Skipped/Deferred

- Dropped by decision-sheet row 53: nested `scan_locations`, VCS-tracked warning (EDG-12), `--skip-conflicts`, case-insensitive collisions (EDG-13). Source `rename` (row 21): not built.
- No interactive repoint prompt, no pty test (R8), `docs/json.md` (M7), no split of the oversized M3 files.
- D4 real activation (Hybrid, yours): backlog NOTE `m3-d4-first-real-activation_NOTE_05-10-26.md`.

## Test Gate Outcomes

EVL by vc-tester (sonnet), loop log `results-m4-evl.tsv`: `HALTED_SUCCESS` at cycle 1, zero gaps. PVL log `results-m4.tsv`: Gate PASS, cycle 1.

| Gate | Result |
|---|---|
| G1 `cargo fmt --all --check` | PASS |
| G2 `cargo clippy --workspace --all-targets -- -D warnings` | PASS (no `unwrap` in non-test code) |
| G3 `INSTA_UPDATE=no cargo test --workspace --locked` | PASS, 567 tests, none ignored |
| G4 perf | not applicable (no change under `catalog/`) |
| G5 presence: ten `ac_NN_` names, `s7_`, `s8_`, exactly 7 `evidence_` tests, `doctor_` tests | PASS |
| G6 protected-docs diff | PASS |
| Full fault matrix (`UNAM_FULL_FAULT_MATRIX=1`) | PASS |
| Deletion-safety evidence check | PASS, see `phase-05-m4-verify-status-reconcile-rollback_EVIDENCE_12-10-26.md` |

Evidence check result: `verify.rs`, `reconcile.rs`, and `doctor.rs` contain no production deletion. Every delete runs through the M3 executor and `remove_owned` (receipt ownership by link text or tree hash, typed `DestPath`); foreign and modified entries are skipped or dropped from the receipt only; the source library has no write path. The seven `evidence_*` rows ran and passed.

## Plan Deviations

Nine recorded in the plan (D1 to D9), all inside the validate-contract blast radius:

| # | Summary |
|---|---|
| D1 | Two existing assertions edited as planned (`usage.rs` status line; `activation_cli.rs` rollback without journal now `no-history`); `NoJournal` remedy text refreshed |
| D2 | New `Edit::FromHistory`, resolved after the lock is taken, so a locked project answers `locked` (exit 11), not `no-history`; plus `change::has_journal` |
| D3 | `reconcile::report` takes a `&dyn Clock`; `assert_zero_divergence` takes `(&CoreCtx, context)` |
| D4 | Interrupted-repair evidence row: `resume` ends with zero divergence; `rollback` returns the project byte-identical to its pre-repair state (the same hand deletions still reported). The S7 matrix asserts zero divergence after both recoveries |
| D5 | The `source rm` guard also counts an alias-qualified bundle entry even when it does not resolve |
| D6 | The five new snapshots live in `snapshots.rs`; only `usage_bare` changed among existing ones |
| D7 | The `docs/adapters.md` claim table was generated from the `EVIDENCE` table in code, not by running the binary |
| D8 | Tests written after the code; no red-first record (same gap as M3 D22) |
| D9 | The core doctor unit test is named `listing_has_every_adapter_claim_with_its_source` so G5 counts exactly 7 `evidence_` tests |

## Test Infra Gaps Found

- No red-first run for the new tests (D8).
- No pty test for the interactive prompt (R8); confirmation is tested with `--yes` and the no-TTY refusal.
- Strided fault sample by default; the full matrix needs `UNAM_FULL_FAULT_MATRIX=1` (about 62 s).
- The doctor unwritable-destination row self-skips as root.
- Linux and CI never run.

## SPEC Achievement

The umbrella SPEC governs. Every M4 criterion above is scored met by a passing automated gate. Partial or unmet:

| Criterion | Score | Where it closes |
|---|---|---|
| AC-24, 29 | met for claude-code only | M5 |
| D4 real activation | unmet (Hybrid, yours) | backlog NOTE above |

## Interpretations Taylor Should See

1. **Exits 8 and 9 return `"ok": false` with the full report.** `status` and `reconcile` exit 8 on divergence (`status` only on an `orphaned` row; `reconcile` on any divergence except informational shadowed or foreign findings), and `doctor` exits 9 on a failed check. The human table prints first; the `--json` envelope carries `ok: false` and the whole report in `data`, the same pattern as exit 10. M7's `docs/json.md` must document this.
2. **"Offer repoint" for a renamed skill is a printed remedy, not a prompt.** PRD EDG-1 says offer repoint; `status` shows the row `orphaned` with reason `source-skill-missing` and candidates, and prints `unam rm <old>` then `unam add <candidate>`. `add`, `rm`, and `reconcile --repair` stay blocked by the not-found error until you do.
3. **Rollback after `reconcile --repair` or a relocate repair is a no-op (EX8).** Those history entries hold equal before and after selections, and history rollback restores only the selection, so it matches the old destination only while bundles and sources are unchanged. Use `rollback` to undo an `add`, `rm`, `use`, or `deactivate`, not a repair.

Other visible choices: an altered but still resolving symlink counts as `orphaned` (PRD "missing or altered" wording wins); bare `unam` now ends with the status table inside a project and always exits 0.

## Closeout Packet

1. Plan: `phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md`.
2. Classification: Ready for UPDATE PROCESS archival of the phase as a milestone step; the plan stays in `active/` with the task folder (the program archives the folder as a unit, as for M1 to M3).
3. Finished: see What Was Done.
4. Verified: G1 to G6, evidence check, full fault matrix. Unverified: Linux, a real terminal prompt, D4, Taylor's real library.
4b. Validate-contract: present, Gate PASS (cycle 1), `generated-by: inner-pvl: phase-5`.
5. Cleanup done: report, plan ticks, umbrella state, registry, waves progress, context. Still needed: commits.
6. Next valid state: milestone M5 (phase 6 of 8), loop step 1 RESEARCH, light lane.
7. Commit checkpoint: execution commit first (code, tests, snapshots, `docs/adapters.md`), then the process commit (this report and process/context edits). No commit was made by this step.
8. Regression: M1 to M3 suites unmodified except the two planned assertion edits (D1) and the additive status halves; 31 earlier snapshots unchanged except `usage_bare`.
9. SPEC: see SPEC Achievement.

Drift score: HIGH (4 signals: 10+ files touched, task-folder artifacts added, 9 validate-contract deviations, 3+ memory-worthy observations). Strongly recommend UPDATE PROCESS -- harness/protocol files touched. (No harness files changed; the band comes from file count and deviations, and this report is that UPDATE PROCESS.)

Autonomy record: no Phase 3 prompt was shown (light lane, standing autonomy). All edits are reversible process and context documents inside the blast radius. No memory notes written (instructed). `validate-context-discovery` and `validate-plan-inventory` results are in the orchestrator summary.

## Known Gaps

- **Linux and CI never run.** There is no remote; `.github/workflows/ci.yml` is written and unpushed. Snapshots `show_found` and `add_bad_path` may differ on Linux.
- **D4 real activation pending.** First adopting project unnamed; yours to run.
- **File-size guide.** M4 added `verify.rs` (781 lines) and `doctor.rs` (533) in core; the M3 oversized files are not split.
- **Residuals R1 to R10 from M3** stay accepted; R10 (`source rm` bundle blind spot) is closed by M4. R8 (no pty test) remains.
- **No red-first record** (D8).

## Open for Taylor

- The seven "Needs a real decision" rows in `open-items-decision-sheet_REF_09-10-26.md` (defaults apply unless you mark rows). Row 26/55 is now built, row 22 (bare `unam` status) is built, row 21 (source `rename`) stays dropped, row 53 deferrals stay dropped.
- Name the first adopting project and run the D4 real activation (M3 backlog NOTE).
- Linux and CI have never run; the first push will show Linux behavior.

## Forward Preview

#### Test Infra Found

- Suites to keep green: `ac_verify` (7), `ac_reconcile` (12), `doctor_cli` (5), `ac_recovery` (14), `ac_activation` (17), `activation_cli` (36), `snapshots` (36).
- `assert_zero_divergence` (`common/mod.rs`) is the helper for any new apply or recovery test; M5 shared-destination tests should call it.

#### Blast Radius Changes

- M5 owns: new `adapter/agents_neutral.rs`; additions to `adapter/mod.rs`, `plan.rs`, `apply.rs`, `verify.rs`; CLI output; `crates/unam-cli/tests/`. `verify.rs` and `reconcile.rs` now exist and take a second adapter without a trait change if destination roots stay adapter-driven.
- M5 closes AC-20, 42, 43, 24 (second adapter); S9 for agents-neutral; S1 and S6 on Taylor's real library; network none and background none; the product Phase 1 completeness report. STOP at the Product Phase 1 Gate.
- Symlink-only; light lane (plan near 400 lines, PVL at most 2 cycles).

#### Commands to Stay Green

```
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
cargo test --release -p unam-core --test perf_cold_scan   # only if catalog code changes
```

Plus the source-tree and foreign-file invariants and the protected-docs diff.

#### Dependency Changes

None (`Cargo.lock` unchanged).
