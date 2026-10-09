---
name: plan:v1-static-activation-phase-05-m4-verify-status-reconcile-rollback
description: "v1: static activation, phase 5 of 8: milestone M4, verify, status, reconcile, rollback from history, doctor, source relocate; lighter-process plan supplemented 2026-10-09"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-05
---

# Milestone M4: Verify, status, reconcile, rollback

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-09 in one combined research, design, and plan pass; lighter process per `v1-static-activation_WAVES_09-10-26.md`)
**Complexity**: COMPLEX
**Phase:** Phase 5 of 8: milestone M4
**Status**: PLANNED (plan supplemented 2026-10-09; PVL cycle 1 passed 2026-10-09, ready for EXECUTE)
**Phase status:** PLANNED
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.
**Execution strategy:** sequential, one sonnet execute agent (Taylor, 2026-10-09: all agents sonnet). The work is one compile chain (`error` -> `plan` -> `change` -> `verify` -> `reconcile` -> `source` -> CLI -> tests); no disjoint file groups exist for a team.

---

## Overview

M4 makes state honest: classify every entry into the six states, report them with `status`, detect and repair divergence with `reconcile`, restore a previous applied state with `rollback` from history, diagnose the environment with `doctor`, and repair projects after `source relocate` (spec 4.2, 4.10 to 4.12, 5, 7, 10 M4; PRD 4, 7.4, 7.7, 7.9, 11.5). It also fixes the M3 residual R10: the `source rm` guard must count bundle membership (decision sheet rows 26 and 55).

Threat model (standing rule): same user, trusted local filesystem; untrusted input is only skill file content and files from cloned repos. This plan adds no hardening beyond it. Protections against your own mistakes (never delete foreign, modified, or unreceipted content; preview before every repair) stay.

Constraints: v1 is symlink-only going forward (existing copy code stays; no new copy-specific branches; copy entries are checked only by the existing tree-hash comparison). Build on the frozen M3 journal, receipt, and executor; no new journal record types. Reconcile repair and relocate repair run through `change::run`, so they inherit lock, journal, history, and `resume`/`rollback`.

---

## Scope

In:

- `verify` six-state classification, shadow analysis, candidates for a renamed source skill (spec 4.11).
- `status` (human table and `--json`), exit 8 on any `orphaned` (spec 5).
- `reconcile [--repair]` (spec 4.12), exit 8 on divergence without `--repair`.
- `rollback` from history when no journal exists (spec 4.10, STA-5). Journal-based `resume` and `rollback` are M3 and stay as built.
- `source relocate <alias> <path>` with repair of dependent projects (spec 4.2; closes AC-37).
- `doctor [--evidence]`, exit 9 on any failed check (spec 5).
- `source rm` guard counts bundle members (sheet rows 26 and 55); `bundle rm` still never blocks.
- Bare `unam` prints usage, then the project status table when the resolved root holds `.unam/` (sheet row 22; always exit 0).
- Status halves of AC-12, 13, 15, 21, 35 added to the existing M2 and M3 tests.
- `docs/adapters.md` extended with the M0 Outcome and the `doctor --evidence` listing.

Out of scope (dropped by the decision sheet or deferred): source `rename` (row 21), nested `scan_locations`, VCS-tracked warning (EDG-12), case-insensitive collisions (EDG-13), `--skip-conflicts` (row 53), interactive repoint prompts, a pty test (R8), Linux/CI runs, splitting the oversized M3 files, `docs/json.md` (M7), any change to the five protected design documents.

### Success-target checks (PRD 13.1)

| ID | Check | How measured |
|---|---|---|
| S7 | Zero unreconcilable states after simulated interruption | `s7_interruption_zero_divergence`: the AC-31 fault matrix with the zero-divergence assertion after every `resume` and every `rollback` (strided sample by default, full set with `UNAM_FULL_FAULT_MATRIX=1`) |
| S8 | Zero diverged projects go unreported | `s8_divergence_always_reported`: one fixture project per divergence kind; `status` and `reconcile` both report each (table in Test Plan) |

---

## Acceptance Criteria

Closes (spec 10 M4): AC-14, 26, 27, 28, 33, 34, 36, 37, 39, 41; status halves of AC-12, 13, 15, 21, 35. Criterion text: PRD 13.2. Each row names its proving test (REQ-TEST-LINK); the full map is in "AC-to-Test Map".

| Criterion | proven by | strategy |
|---|---|---|
| AC-14 | `ac_14_*` | Fully-Automated |
| AC-26 | `ac_26_*` | Fully-Automated |
| AC-27 | `ac_27_*` | Fully-Automated |
| AC-28 | `ac_28_*` | Fully-Automated |
| AC-33 | `ac_33_*` | Fully-Automated |
| AC-34 | `ac_34_*` | Fully-Automated |
| AC-36 | `ac_36_*` | Fully-Automated |
| AC-37 | `ac_37_*` | Fully-Automated |
| AC-39 | `ac_39_*` | Fully-Automated |
| AC-41 | `ac_41_*` | Fully-Automated |
| AC-12, 13, 15, 21, 35 (status halves) | status assertions appended to the existing `ac_12`, `ac_13`, `ac_15`, `ac_21`, `ac_35` tests | Fully-Automated |

---

## Entry Gate

- M0 gate met (`spike/symlink-discovery.md` Outcome recorded).
- M3 VERIFIED (WITH_GAPS) 2026-10-09 and committed (d34bcb5, 7086c6d, 9e1c4e9 on `main`). D4 stays CONDITIONAL and does not block M4 (sheet row 54).
- Baseline before any edit: the three cargo gates below are green (512 tests, strided fault sample).

---

## Decisions

Simplest spec-faithful choices where the spec or PRD is silent. "REPORT TO TAYLOR" marks a visible departure or interpretation for the phase report.

| # | Decision |
|---|---|
| M4-1 | Exit 8 (`status`, `reconcile`) and exit 9 (`doctor`) are carried as new `Error` variants `Diverged` and `DoctorFailed` through `Failed::with_data`, the same pattern as exit 10: the human table prints first, the `--json` envelope has `ok: false` and the full report in `data`. New `NoHistory` (exit 4). A corrupt history file reuses the closest existing variant. REPORT TO TAYLOR: `ok: false` on exits 8 and 9 is visible in the JSON contract that M7 documents. |
| M4-2 | Classification follows spec 4.11 plus the PRD "missing or altered" wording. `orphaned` reasons: `missing` (destination absent), `broken-link` (symlink does not resolve), `modified` (symlink text differs from the receipt, a non-directory where a link was recorded, or a copy whose tree hash differs from the receipt), `source-skill-missing` (receipt ID no longer in the catalog while its source is reachable). Altered-but-resolving symlinks count as `orphaned` (spec table silent; PRD wins). |
| M4-3 | `foreign` rows: non-hidden directories, or symlinks to directories, inside the destination root whose name is not in the receipt. Plain files and dot-names (`.DS_Store`, `.unam-tmp-*`) are ignored. A desired entry colliding with a foreign name yields both a `selected` row and a `foreign` row. |
| M4-4 | `verify` and `status` are read-only: no lock, no write. A leftover journal adds the warning `interrupted-apply` (remedy: `unam resume` or `unam rollback`); it does not change the exit code. |
| M4-5 | `shadowed` uses the existing `observe` shadows (enterprise and user scope outrank project scope; spec 4.7). Shadowed is reported by `status` and listed by `reconcile` as an informational finding, not a divergence (exit 0). |
| M4-6 | Divergence causes (reconcile report groups): `not-applied` (desired, no receipt entry), `hand-deleted`, `hand-modified`, `source-moved`, `source-unreachable`, `source-skill-missing`, `deselected` (receipted, no longer desired). Orphaned reasons map by M4-2 and the catalog: `missing` to hand-deleted; `modified` to hand-modified; `broken-link` to source-unreachable (alias unreachable), source-skill-missing (ID not in catalog), source-moved (ID in catalog at a path other than the receipt `source_path`), else hand-modified. |
| M4-7 | `reconcile --repair` is `change::run` with new `Edit::Reconcile` (selection unchanged). `plan::diff` gains `DiffInput.repoint_moved: bool`, set only for `Edit::Reconcile`: an owned symlink whose live source path differs from the receipt `source_path` becomes a `Remove` (expect the old link text) plus an `Add` instead of "unchanged plus info". `add`, `rm`, `use`, `deactivate` pass `false`, so M3 behavior is unchanged. Modified entries stay conflicts (`ModifiedAtDest`, exit 6): user content is never overwritten. Foreign entries are never touched. |
| M4-8 | Renamed or removed source skill (AC-36, EDG-1): `status` shows `orphaned` with reason `source-skill-missing` and `candidates` from `resolve::resolve_text(catalog, id)` NotFound candidates. "Block by default" is the existing M3 behavior (the unresolved selection ref blocks `add`, `rm`, and `--repair` with the not-found error and changes nothing). The "offer repoint" is a printed remedy (`unam rm <old>` then `unam add <candidate>`), not an interactive prompt. REPORT TO TAYLOR: PRD EDG-1 says "offer repoint"; this is the printed form. |
| M4-9 | Rollback from history: `rollback` with no journal reads the newest `history/*.json` of the target (op IDs sort by time), takes the target's `selection_before` (absent means the target did not exist), and runs `change::run` with new `Edit::Restore(TargetSelection)` (or `Edit::Deactivate` when absent), command `rollback`, normal confirmation. Only this target's selection is restored, never other targets. A second `rollback` undoes the first (it wrote its own history entry). Unchanged entries keep their receipt record verbatim; re-created entries get new `applied_at`. With a journal present the M3 recover path runs as before. |
| M4-10 | `source relocate <alias> <path>`: validate the new path like `source add` (canonical, directory, readable, not registered under another alias; unknown alias is exit 4); build an in-memory config with the new path; for each indexed project whose receipt holds an entry from the alias, compute the repair plan (`Edit::Reconcile`); print all plans; one confirmation (`--yes` or TTY; no TTY without `--yes` is exit 2 and changes nothing); then save the config, delete the alias cache file, and apply each project's plan. A project that cannot be planned (skill missing at the new path, project directory gone) is reported and left `orphaned`; the config is still updated and the exit code is 0 unless an apply failed (exit 7). Selections need no rewrite (alias based). |
| M4-11 | `source rm` guard: a project counts as referencing an alias when a literal `add`/`remove` ref starts with `<alias>:` (as now) or when any bundle named in its selection resolves, through the catalog, to a skill of the alias (`resolve::resolve_entries`). Unresolvable bare bundle members are not counted. `--force` unchanged. |
| M4-12 | `doctor` checks (each `ok`, `fail`, or `info`): `symlink-capability` and `destination-writable` (a `tempfile` directory created in the nearest existing directory at or above the destination root, containing a probe symlink; removed before returning, so the project is byte-identical afterward), `adapter-detection` (info), `source-reachability` (fail per unreachable source), `receipt-integrity` (receipt loads and every entry passes `SkillEntry::consistent`), `journal-presence` (fail while a journal exists). `--evidence` adds every adapter evidence entry (claim key, claim, URL or spike path, verified or assumption). The `tempfile` probe is the one place core creates files outside `FsOps`; it writes nothing the user keeps. |
| M4-13 | Bare `unam` = usage text, then the status table when the resolved root holds `.unam/`; the "status arrives in a later milestone" line is removed; exit 0. The `usage_bare` snapshot changes. |
| M4-14 | AC-39 and AC-37 deliberately change the source tree (an edit, a move). Those tests retake the source snapshot after the deliberate change (or assert the new location), and `Fx` gains `done_at(&self, lib: &Path)` for the relocated library. All other tests keep `fx.done()`. |
| M4-15 | The `scan:nested` evidence claim in `adapter/claude_code.rs` says "not enumerated until milestone M4"; M4 does not enumerate nested locations, so the wording becomes "not enumerated in v1". |

---

## Blast Radius

- New core modules: `crates/unam-core/src/verify.rs`, `reconcile.rs`, `doctor.rs` (registered in `lib.rs`).
- Modified core: `error.rs` (3 variants), `plan.rs` (`repoint_moved`), `state.rs` (`read_latest_history`), `change.rs` (`Edit::Reconcile`, `Edit::Restore`, `history_edit`), `observe.rs` (`list_foreign`), `source.rs` (`relocate`, `remove` guard), `project/index.rs` (`projects_with_receipt_from`, `projects_using_source`), `adapter/claude_code.rs` (M4-15 wording only). Their inline test modules and `plan/tests.rs`, `apply/tests.rs`, `change/tests.rs` (4 `DiffInput` call sites add `repoint_moved: false`).
- CLI: `crates/unam-cli/src/cli/mod.rs` (Status, Reconcile, Doctor, `source relocate`, bare `unam`), new `cli/status.rs` and `cli/doctor.rs`, `cli/source.rs`, `cli/change.rs` (rollback branch; helpers become `pub(super)`), `output.rs` (state table, footer).
- Tests: new `crates/unam-cli/tests/ac_verify.rs`, `ac_reconcile.rs`, `doctor_cli.rs`; additive edits to `ac_selection.rs`, `ac_activation.rs`, `ac_recovery.rs`, `common/mod.rs`, `common/faults.rs`; new insta snapshots.
- Docs: `docs/adapters.md` (extend; keep the two M2 sections).
- Not touched: the five protected design documents, `catalog/` (so the perf gate is conditional), the skill library, this repository as a target, the journal record types, `fsops.rs` and `apply.rs` (read and call only).

Risk class: `reconcile --repair`, relocate repair, and rollback from history delete through the M3 executor (`remove_owned` re-checks ownership at call time). Short evidence check below.

---

## Test Plan

Context loaded: `process/context/tests/all-tests.md` and its chain (cargo runner, `TestEnv` and `Fx` fixtures, `FailingFs` and `StepClock` in `common/faults.rs`, insta snapshots, the two invariants). Existing suites that must stay green: `ac_activation` (17), `ac_recovery` (13), `activation_cli` (36), `snapshots` (31), M1/M2 suites. Strategies are Fully-Automated unless noted; there are no Known-Gap rows.

### AC-to-Test Map (lowest layer that proves it, plus one CLI test where CLI-visible)

| AC | Core layer test (`unam-core`, inline) | CLI test (`crates/unam-cli/tests/`) |
|---|---|---|
| AC-14 | `verify::tests::record_has_id_name_category_source_dest_strategy_provenance_state` | `ac_verify.rs`: `ac_14_status_shows_source_path_destination_strategy_provenance_state_per_entry`; snapshot `status_json` |
| AC-26 | `verify::tests::broken_symlink_is_orphaned_never_active` | `ac_verify.rs`: `ac_26_broken_destination_symlink_is_orphaned_and_status_exits_8` |
| AC-27 | `verify::tests::shadowed_by_user_scope_reports_both_paths` | `ac_verify.rs`: `ac_27_shadowed_skill_reports_destination_and_shadowing_path` (user-scope skill under the test `HOME`) |
| AC-28 | none (text property across commands) | `ac_verify.rs`: `ac_28_no_output_claims_an_agent_read_loaded_or_applied_a_skill`: runs `status`, `reconcile`, `doctor`, `add`, `rm`, `deactivate`, human and `--json`; asserts the exact spec 4.11 footer on `status` (human line and `data.footer`) and that no output contains any of: "has read", "was read", "agent read", "has loaded", "was loaded by", "is using", "now using". The SES-2 caveat sentence ("already loaded") is allowed. |
| AC-33 | `change::tests::history_edit_restores_target_selection_before` and `state::tests::latest_history_is_newest_op_id` | `ac_reconcile.rs`: `ac_33_rollback_from_history_restores_prior_destination_and_receipt` (add A, add B, `rollback`: destination listing and receipt entries equal the A-only state, A entry verbatim; second `rollback` undoes the first; first-ever apply rolled back removes created dirs) |
| AC-34 | `reconcile::tests::hand_deleted_is_reported_and_repair_plan_re_adds` | `ac_reconcile.rs`: `ac_34_hand_deleted_entry_reconcile_reports_exit_8_and_repair_restores` |
| AC-36 | `verify::tests::renamed_source_skill_is_orphaned_with_candidates` | `ac_verify.rs`: `ac_36_renamed_source_skill_is_orphaned_with_candidates_and_blocked_by_default` (rename a skill directory in the library; `status` shows candidates; `add other` and `reconcile --repair` exit with the not-found code and the project is byte-identical) |
| AC-37 | `plan::tests::repoint_replaces_a_moved_symlink`, `source::tests::relocate_plans_each_dependent_project`, `project::index::tests::receipt_scan_finds_dependent_projects` | `ac_reconcile.rs`: `ac_37_source_relocate_repairs_every_affected_project` (two projects; library directory moved; both `orphaned` before; after `source relocate fx <new> --yes` both `status` exit 0 with all entries `active`; the library at the new path equals the snapshot) |
| AC-39 | `verify::tests::follow_mode_source_edit_stays_active` | `ac_verify.rs`: `ac_39_source_edit_under_follow_is_visible_at_destination_without_drift_warning` (edit `SKILL.md` body; destination file shows the edit; `status` all `active`; no diagnostic code containing `drift`; `add` again reports no changes) |
| AC-41 | none (no semantic analysis exists to unit test) | `ac_verify.rs`: `ac_41_contradictory_skills_activate_and_no_semantic_conflict_claim_appears` (two fixture skills with opposite instructions; `add` exit 0; both `active`; outputs contain none of "contradict", "incompatible", "semantic") |
| AC-12, 13, 15, 21, 35 status halves | n/a | assertions appended to `ac_12`, `ac_13`, `ac_15`, `ac_21`, `ac_35`: `status` shows provenance (12), both `active` (13), states equal the printed plan (15), prior selection and states unchanged (21), `selected` not `active` (35) |

### Other tests

| Test | Layer | Notes |
|---|---|---|
| `s7_interruption_zero_divergence` (`ac_recovery.rs`) | CLI/core, `FailingFs` | Extract the AC-31 loop body into a helper; AC-31 runs scenarios a, b, c, S7 runs scenario a; both call `assert_zero_divergence(env, project)` (new in `common/mod.rs`, calls `reconcile::report`) after every `resume` and every `rollback` |
| `s8_divergence_always_reported` (`ac_reconcile.rs`) | CLI | One project per kind: hand-deleted (`hand-deleted`), broken link after a library rename (`source-skill-missing`), modified copy via a selection with `strategy: copy` (`hand-modified`), user-scope shadow (`shadowed`, informational), hand-edited selection (`not-applied`), deselected entry (`deselected`). Each: `status` shows the expected state; `reconcile` lists the cause; exit 8 for all but shadowed (exit 0, listed under `findings`) |
| `doctor_*` (`doctor_cli.rs`) | CLI | all checks `ok` on a clean project (Fully-Automated); unreachable source fails (exit 9); leftover journal fails (exit 9); `--evidence` lists every `Adapter::evidence()` key with its URL; unwritable destination fails exit 9 (Hybrid: needs a non-root user, self-skips otherwise like the M3 chmod rows); project byte-identical after `doctor` |
| `source_rm_counts_bundle_members` (`ac_reconcile.rs`) and `project::index::tests::guard_counts_bundle_members` | CLI, core | A project that selects a bundle whose members come from `fx`: `source rm fx` exits 3; `--force` removes; `bundle rm` still succeeds |
| `bare_unam_prints_status_for_a_project` (`doctor_cli.rs` or `usage.rs`) | CLI | exit 0 even with an orphaned entry |
| Snapshots | insta | new: `status_json`, `reconcile_json`, `rollback_history_json`, `doctor_json`, `source_relocate_json`; changed: `usage_bare` only. Create with `INSTA_UPDATE=always` once, review the diff, then gate with `INSTA_UPDATE=no`. |

Every mutating test calls `fx.done()` (or `done_at`) and asserts the project snapshot helpers where it claims "changes nothing" (invariants below).

### Evidence check (reconcile --repair, relocate repair, and history rollback delete)

| Adversarial row | Test | Expected |
|---|---|---|
| Foreign dir, file-in-dir, and symlink in the destination | `evidence_repair_never_touches_foreign` | byte-identical after `--repair` (via `Fx::plant_foreign` + `fx.done()`) |
| Owned entry the user edited (copy file changed; symlink repointed by hand) | `evidence_repair_never_overwrites_modified_entries` | `--repair` exits 6, entry bytes and link text unchanged, nothing deleted |
| Deselected entry whose link text no longer matches the receipt | `evidence_deselected_modified_entry_is_dropped_from_receipt_only` | filesystem entry kept, receipt record dropped (existing `DropOrphan` rule) |
| Repair interrupted mid-run | `evidence_repair_interrupted_resume_and_rollback_reach_zero_divergence` | `FailingFs` fails the second repair step; `resume` and (separately) `rollback` end with `assert_zero_divergence` |
| Report preview versus applied repair | `evidence_repair_preview_equals_applied_steps` | step names and order in `reconcile` JSON `data.plan` equal the apply results |
| Second `--repair` | `evidence_repair_is_idempotent` | "no changes", exit 0, project byte-identical |
| Relocate onto a path that lacks some skills | `evidence_relocate_to_incomplete_library_reports_and_deletes_nothing` | affected project reported and left `orphaned`; config still updated; nothing removed |

---

## Implementation Checklist

Compile order. Spec sections are cited, not copied; reuse the named functions.

### Step A: Errors, plan hook, change edits (core, compile first)

- [ ] A1. `error.rs`: add `NoHistory { target }` (exit 4, code `no-history`, remedy), `Diverged { count, kind }` (exit 8, code `diverged`), `DoctorFailed { failed }` (exit 9, code `doctor-failed`); extend `code`, `exit_code`, `remedy`, and the variant tests (M4-1).
- [ ] A2. `plan.rs`: add `DiffInput.repoint_moved`; in the owned-symlink "unchanged" branch, when set and `catalog.abs_path(id)` differs from `r.source_path`, push `Step::Remove` (expectation from `expectation_of`) plus `make_add`; add `false` at the 4 existing `DiffInput` call sites (`change.rs`, `plan/tests.rs` x2, `apply/tests.rs`); add `plan::tests::repoint_replaces_a_moved_symlink` and a test that `false` keeps the old "unchanged plus `source-moved` info" result.
- [ ] A3. `state.rs`: add `read_latest_history(state: &StateDir) -> Result<Option<HistoryEntry>>` (newest valid `<op_id>.json`, same name filter as `prune_history`).
- [ ] A4. `change.rs`: add `Edit::Reconcile` and `Edit::Restore(TargetSelection)` handled in `apply_edit` (Reconcile: unchanged; Restore: replace the target selection, `changed` when different); `prepare` sets `repoint_moved` from `req.edit == Edit::Reconcile`; add `pub fn history_edit(root, target) -> Result<Edit>` (`NoHistory`, or `Edit::Deactivate` when `selection_before` has no such target); unit tests in `change/tests.rs`.

### Step B: Observation, verify, reconcile (core)

- [ ] B1. `observe.rs`: add `list_foreign(root, rel, known: &BTreeSet<String>) -> Result<Vec<(String, ObservedKind)>>` per M4-3 (shallow `read_dir`, `observe_one` per child).
- [ ] B2. `verify.rs` (+ `pub mod` in `lib.rs`): `EntryState { Selected, Applied, Active, Shadowed, Foreign, Orphaned }`, `Reason`, `Record` (fields per AC-14 plus `reason`, `candidates`, `shadowed_by`), `VerifyReport`, and `verify(&VerifyRequest) -> Result<VerifyReport>` using `resolve::resolve_selection`, `Receipt::load`, `observe::observe`, `catalog::frontmatter::parse_skill_md` (Applied when it fails), `resolve::resolve_text` candidates, and the journal-present warning. Read-only. Unit tests named in the AC map.
- [ ] B3. `reconcile.rs`: `report(&VerifyRequest) -> Result<ReconcileReport>` = `verify` + divergences grouped by the M4-6 causes + `findings` (shadowed, foreign) + a preview `Plan` from `change::prepare` with `Edit::Reconcile` (a `prepare` error becomes `repair_blocked` with the error text, not a failure). `repair` is `change::run` with `Edit::Reconcile` (no new code beyond a thin wrapper). Unit tests.

### Step C: Source edge cases (core)

- [ ] C1. `project/index.rs`: `projects_with_receipt_from(dirs, alias)` (reads each indexed root's receipts via `Receipt::load`; missing project directories become diagnostics) and `projects_using_source(dirs, catalog, bundles, alias)` (M4-11); unit tests.
- [ ] C2. `source.rs`: change `remove` to take `catalog` and `bundles` and use `projects_using_source`; add `relocate` pieces per M4-10 (path validation shared with `add`, in-memory config, `plan_relocations`, then `commit_relocation`: save config, delete the alias cache file); unit tests.

### Step D: Doctor (core)

- [ ] D1. `doctor.rs`: the M4-12 checks and evidence listing returning a typed report; unit tests with temp dirs.
- [ ] D2. `adapter/claude_code.rs`: M4-15 wording only.

### Step E: CLI

- [ ] E1. `cli/mod.rs`: add `Command::Status`, `Command::Reconcile { repair }`, `Command::Doctor { evidence }`, `SourceCmd::Relocate { alias, path }`; extend `command_name`; replace `usage_text` per M4-13.
- [ ] E2. `cli/status.rs`: `status` and `reconcile` (report prints the table and divergences, returns `Diverged` via `Failed::with_data` when divergent; `--repair` reuses the `cli/change.rs` confirmation and outcome rendering, so those helpers become `pub(super)`). `output.rs`: six-state table with source path per entry, the spec 4.11 footer, `data.footer` in JSON.
- [ ] E3. `cli/doctor.rs`: render checks and optional evidence; `DoctorFailed` on any `fail`.
- [ ] E4. `cli/source.rs`: `relocate` (M4-10 flow) and `rm` (load catalog and bundles for the guard). `cli/change.rs` `rollback`: no journal selects the M4-9 history path via `change::history_edit` and `change::run`; journal present keeps the M3 path.

### Step F: Tests (after the code they exercise)

- [ ] F1. `common/mod.rs`: `Fx::done_at`, `assert_zero_divergence`, helpers for a user-scope shadow skill, library rename, and library move. `common/faults.rs`: reuse as is.
- [ ] F2. Write `ac_verify.rs`, `ac_reconcile.rs`, `doctor_cli.rs` per the Test Plan; extract the AC-31 loop in `ac_recovery.rs` and add `s7_interruption_zero_divergence`.
- [ ] F3. Append the status halves to `ac_12`, `ac_13`, `ac_15`, `ac_21`, `ac_35`.
- [ ] F4. Create new insta snapshots (`INSTA_UPDATE=always` once), review that only `usage_bare` changed among existing ones, then run with `INSTA_UPDATE=no`.

### Step G: Docs and report

- [ ] G1. `docs/adapters.md`: add the claude-code evidence table from `unam doctor --evidence` (run only against temporary `UNAM_CONFIG_DIR`/`HOME`, never this repository) and the M0 Outcome; state that M0 Cursor results are evidence for the `cursor-agent` CLI only and the Cursor editor is not tested; keep both M2 sections; remove the "M4 extends" wording.
- [ ] G2. Run the Exit Gate, write the phase report (decisions M4-1 to M4-15 as deviations from this plan if any change, REPORT TO TAYLOR items, evidence-check results, S7 and S8 results).

---

## Exit Gate

```bash
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
# G1 Expected: exit 0
cargo clippy --workspace --all-targets -- -D warnings
# G2 Expected: exit 0 (no unwrap in non-test code)
cargo test --workspace --locked
# G3 Expected: all tests pass, including every ac_NN, s7, s8, doctor_, and evidence_ test named above
```

- G4 perf: run `cargo test --release -p unam-core --test perf_cold_scan` only if `git diff --name-only` shows a file under `crates/unam-core/src/catalog/`; the plan expects none.
- G5 invariants: every mutating test asserts the source-unchanged and foreign-file invariants (`fx.done()` or `done_at`); `cargo test --workspace --locked evidence_` shows the seven evidence rows ran (non-zero count).
- G6 protected docs: `git diff --quiet -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0.

Regression: M1 to M3 suites unmodified except the additive status assertions in F3; only the `usage_bare` snapshot changes among existing snapshots. Phase report written to the destination above.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green, the independent EVL run, and the recorded validate-contract.
- Commit execution changes on `main` before M5 (code commit, then process commit); no push.
- Linux/CI stays unrun (known M3 gap).

## Blockers That Would Justify BLOCKED Status

- M3 not committed or its gates red at baseline.
- A repair path that cannot go through `change::run` and the journaled executor: stop and report.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH (2026-10-09): combined pass (lighter process); M3 report and Forward Preview read; test context loaded; M3 code surface read (`observe`, `state`, `plan`, `apply`, `change`, `source`, `adapter`, `project/index`, `cli/`)
- [x] 2. INNOVATE (2026-10-09): folded into this pass; design choices recorded as Decisions M4-1 to M4-15 (no separate Decision Summary)
- [x] 3. PLAN-SUPPLEMENT (2026-10-09): existing phase plan rewritten into an executable plan; Inner Loop Refresh Note written
- [x] 4. PVL (2026-10-09, cycle 1, no FAIL-class gap; contract below): vc-validate-agent: at most 2 cycles; only FAIL-class gaps (wrong requirement mapping, steps that cannot compile in order) trigger a cycle; nits go to execute time; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md`
- [ ] 5. EXECUTE: all checklist items done; per-section test gates run and green (or gaps documented)
- [ ] 6. EVL: independent vc-tester re-run of the Exit Gate; follow-up stubs registered; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, evidence check recorded, commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first.

---

## Inner Loop Refresh Note: 2026-10-09 - changed sections: header (Status, Execution strategy), Overview, Scope, Acceptance Criteria, Entry Gate, Decisions (new), Blast Radius, Test Plan (new, with AC-to-Test Map and evidence check), Implementation Checklist (rewritten in compile order), Exit Gate (six gates), Phase Loop Progress, Touchpoints, Public Contracts, Verification Evidence, Test Infra Improvement Notes, Resume and Execution Handoff. Research found: `observe` only inspects named entries (a directory listing is needed for `foreign`); `plan::diff` already re-adds a hand-deleted receipted entry, so repair needs no new step type, but it treats a moved-source symlink as "unchanged plus info", so `source relocate` needs the `repoint_moved` hook; history entries already hold `selection_before` and `receipt_before`, so rollback from history needs no new journal or history format. No validate-contract existed to invalidate.

---

## Touchpoints

- Files listed in Blast Radius.
- Read only: spec 4.2, 4.7, 4.10 to 4.12, 5, 7, 8; PRD 4, 7.4, 7.7, 7.9, 11.5, 13.2; M3 report Forward Preview; decision sheet rows 21, 22, 26, 53, 55.

## Public Contracts

- Commands `status`, `reconcile [--repair]`, `doctor [--evidence]`, `source relocate`, `rollback` from history; exit codes 8, 9 now used (10 and 11 unchanged).
- `status --json` record fields: `id, name, category, source_path, dest, strategy, provenance, state` plus `reason`, `candidates`, `shadowed_by` when relevant; `data.footer` carries the spec 4.11 sentence. `reconcile --json`: `records`, `divergences` (cause codes of M4-6), `findings`, `plan`, `repair_blocked`. `doctor --json`: `checks[]`, optional `evidence[]`.
- Library API (consumed by M6 TUI): `verify::verify`, `reconcile::report`, `change::history_edit`, `Edit::{Reconcile, Restore}`.
- Unchanged: journal and receipt formats, `Adapter` trait.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `INSTA_UPDATE=no cargo test --workspace --locked` | Fully-Automated | spec 8 gates |
| `ac_14_*` status record fields | Fully-Automated | AC-14 |
| `ac_26_*` broken symlink is `orphaned` | Fully-Automated | AC-26 |
| `ac_27_*` shadowed with both paths | Fully-Automated | AC-27 |
| `ac_28_*` no output claims an agent read or loaded a skill | Fully-Automated | AC-28 |
| `ac_33_*` rollback from history | Fully-Automated | AC-33 |
| `ac_34_*` hand-deleted entry, reconcile and `--repair` | Fully-Automated | AC-34 |
| `ac_36_*` renamed source skill, candidates, blocked | Fully-Automated | AC-36 |
| `ac_37_*` relocate repairs every affected project | Fully-Automated | AC-37 |
| `ac_39_*` follow-mode edit visible, no drift warning | Fully-Automated | AC-39 |
| `ac_41_*` contradictory skills activate, no conflict claim | Fully-Automated | AC-41 |
| status assertions in `ac_12`, `ac_13`, `ac_15`, `ac_21`, `ac_35` | Fully-Automated | AC-12, AC-13, AC-15, AC-21, AC-35 (status halves) |
| `s7_interruption_zero_divergence` | Fully-Automated (test `FsOps`) | PRD S7 |
| `s8_divergence_always_reported` | Fully-Automated | PRD S8 |
| `evidence_*` seven adversarial rows | Fully-Automated | STA-6, PRD 11.5 (never delete unreceipted content) |
| `doctor_*` unwritable destination probe | Hybrid (precondition: non-root user; self-skips otherwise) | spec 5 doctor; PRD VER-6 |

No Known-Gap rows.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-05-m4-verify-status-reconcile-rollback_PLAN_04-10-26.md`
- Last completed step: 4 PVL (2026-10-09, cycle 1)
- Validate-contract status: written 2026-10-09 (inner-pvl: phase-5, PASS, cycle 1)
- Supporting context files: `process/context/all-context.md` (Threat Model), `process/context/tests/all-tests.md`, `phase-04-m3-plan-apply-claude-code_REPORT_04-10-26.md` (Forward Preview), `open-items-decision-sheet_REF_09-10-26.md`, `v1-static-activation_WAVES_09-10-26.md`
- Next step: EXECUTE (step 5) with one sonnet execute agent and this file path; honor the execute-time notes EX1 to EX8 and the G5 presence checks in the Validate Contract.

---

## Validate Contract

Status: PASS
Date: 09-10-26
date: 2026-10-09
generated-by: inner-pvl: phase-5

Parallel strategy: sequential
Rationale: score 4/7 (S2 new CLI and JSON contracts, S4 phase program, S6 deletion through the executor, S7 more than 5 files; no S1, S3, S5). The work is one compile chain (error, plan, change, verify, reconcile, source, CLI, tests), so no disjoint file group exists for a team and no independent direction exists for parallel subagents. EXECUTE: one sonnet execute agent (Taylor, 2026-10-09: all agents sonnet), then an independent sonnet vc-tester EVL run. Agent counts: sequential 1; parallel subagents 4 + 7 + 3 = 14 with nothing to split; agent team 3 x 2 = 6, no benefit; workflow 7 x 1 x 1 = 7, not warranted. Cost guard not triggered. Cycle 1 of at most 2 (lighter process, WAVES file): worked inline by the validate agent, no fan-out, no helper agent, no `unam` run, no agent CLI, skill library untouched. Only FAIL-class gaps were counted. Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so none is written here. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Reuse claims checked against the real code (all hold):

| Claim | Evidence read | Verdict |
|---|---|---|
| `change::run` takes `Edit::Reconcile` and `Edit::Restore` | `apply_edit` is the only semantic match on `Edit`; `Edit` derives `Eq` and `TargetSelection` derives `Eq`; `run` takes lock, checks journal, writes the selection only when changed, then `apply::execute` (journal, history, receipt) | Holds |
| `repoint_moved` at `DiffInput` sites | exactly 4 construction sites (change.rs:395, plan/tests.rs:82 and :393, apply/tests.rs:252); the "Remove then Add of one name" pair already exists (plan.rs:621-631); `remove_owned_at` compares link text only, so a dangling owned symlink is removable with the receipt `source_path` | Holds |
| History `selection_before` reuse | `HistoryEntry.selection_before` is `{existed, text}` (whole selection file); written on every Applied outcome (apply.rs:585); op ids are ms plus entropy and sort by time; `parse_selection` is public | Holds |
| `source rm` guard counts bundle members | the only scanner is literal `add`/`remove` (index.rs:148); `resolve_entries(bundle, catalog)` yields `SkillId` whose `source` is the alias; only caller of `source::remove` is cli/source.rs:102; `bundle rm` untouched | Holds |
| Exit 8/9 carried like exit 10 | `Failed::with_data` and `DataError` carry data into the envelope; `exit_code` is an exhaustive match, so A1 must touch `code`, `exit_code`, `remedy`, and the variant table test | Holds |
| Relocate repair | catalog cache is keyed on `source_path` (a moved path is a silent miss); `abs_path` is canonical root plus relative path; an unreachable source only yields a `source-unreachable` diagnostic, so `status` works before relocate | Holds |
| Doctor and verify inputs | `tempfile` is a normal dependency of `unam-core`; `Adapter::evidence()`, `parse_skill_md(bytes, dir_name)`, `SkillEntry::consistent`, `Receipt::load` exist | Holds |

Compile order A to G traced: A2 adds `false` at the change.rs site before A4 derives it from `req.edit`; B3 follows A4; C2 changes `source::remove` before its only caller in E4; tests follow code. No forward reference. Structural validator: 0 failures, 0 warnings.

Gates (hook-safe: all command texts were fed to `scout-block.cjs`, every one exit 0):
- G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets -- -D warnings`; G3 `INSTA_UPDATE=no cargo test --workspace --locked`.
- G4 perf, only if `git diff --name-only` shows a path under `crates/unam-core/src/catalog/` (none expected).
- G5 invariants plus presence (strengthened by this contract, because G3 alone stays green if a named test is never written, and `cargo test evidence_` also matches the old `refresh_note_texts_cite_the_evidence_url`): run `cargo test --workspace --locked -- --list` and require (a) distinct AC numbers 14, 26, 27, 28, 33, 34, 36, 37, 39, 41 each matched by `(^|::)ac_NN_` (10 distinct), (b) `s7_interruption_zero_divergence` and `s8_divergence_always_reported` both listed, (c) exactly 7 names matching `(^|::)evidence_[a-z_]+: test$`, (d) at least one `doctor_` test. Every new mutating test ends with `fx.done()` or `done_at`.
- G6 protected docs: `git diff --quiet HEAD -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` (use `HEAD` so staged changes count).

Test gates (C3 table; legacy form is the Verification Evidence table of this plan):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 gates | fmt, clippy, workspace tests green | Fully-Automated | G1, G2, G3 | B |
| AC-14, 26, 27, 28 | six-state record fields, dangling link is orphaned (exit 8), shadowed shows both paths, no output claims an agent read or loaded a skill | Fully-Automated | `ac_14_*`, `ac_26_*`, `ac_27_*`, `ac_28_*` in `ac_verify.rs` plus the `verify::tests` rows | B |
| AC-36, 39, 41 | renamed source skill orphaned with candidates and blocked by default; follow-mode edit visible, no drift warning; contradictory skills activate, no conflict claim | Fully-Automated | `ac_36_*`, `ac_39_*`, `ac_41_*` in `ac_verify.rs` | B |
| AC-33, 34, 37 | rollback from history restores destination and receipt; hand-deleted entry reported then repaired; relocate repairs every dependent project | Fully-Automated | `ac_33_*`, `ac_34_*`, `ac_37_*` in `ac_reconcile.rs` plus `change::tests::history_edit_*`, `plan::tests::repoint_*`, `source::tests::relocate_*` | B |
| AC-12, 13, 15, 21, 35 (status halves) | status shows provenance, both active, states equal the plan, prior selection unchanged, `selected` not `active` | Fully-Automated | assertions appended to the existing `ac_12`, `ac_13`, `ac_15`, `ac_21`, `ac_35` tests; EVL reads the diff to confirm each | B |
| S7, S8 | zero divergence after every resume and rollback; every divergence kind reported by `status` and `reconcile` | Fully-Automated | `s7_interruption_zero_divergence`, `s8_divergence_always_reported` | B |
| STA-6 deletion safety | repair and relocate never touch foreign, modified, or unreceipted entries | Fully-Automated | the 7 `evidence_*` tests | B |
| doctor | unwritable destination fails exit 9 | Hybrid (non-root user; self-skips as root) | `doctor_*` unwritable row | D (named residual) |
| guard and bare `unam` | `source rm` counts bundle members; bare `unam` exits 0 with a status table | Fully-Automated | `source_rm_counts_bundle_members`, `bare_unam_prints_status_for_a_project` | B |

Failing stubs: each named test is written red-first by the execute agent from the Scenario text in the AC-to-Test Map; per-row stub blocks are omitted under the lighter process.

Dimension findings:
- Infra fit: PASS - one compile chain, reuses the M3 executor; all new items resolve against real code.
- Test coverage: PASS - every AC maps to a named test; G5 presence check closes the vacuous-green path; one Hybrid residual, no Known-Gap row.
- Breaking changes: PASS - `ok: false` JSON on exits 8 and 9 and `usage_bare` are declared; two existing-test edits are execute-time notes EX1 and EX2.
- Security surface: PASS - every deletion goes through `remove_owned` (re-checks the link text or tree hash at call time); foreign entries are never targets; relocate writes only the config and one cache file; doctor removes only its own temp directory.
- Sections A (errors, plan, change), B (observe, verify, reconcile), C (source, index), D (doctor), E (CLI), F (tests), G (docs, report): PASS each; 0 FAILs / 0 CONCERNs.

Execute-time notes (nits, binding on the execute agent, not gaps):
- EX1 Existing tests that M4 decisions change: `usage.rs` asserts "status arrives in a later milestone" (removed by M4-13); `activation_cli.rs` `resume_and_rollback_without_a_journal_exit_4` expects `no-journal` and a "milestone M4" remedy for `rollback` (now `no-history` when neither journal nor history exists; `resume` stays `no-journal`; refresh the remedy text). Update these assertions and list them as deviations in the report.
- EX2 AC-28: the spec 4.11 footer itself contains "has read". Strip the exact footer sentence (and the SES-2 caveat) before scanning for banned phrases, and assert the footer verbatim separately.
- EX3 AC-36 (rename) also changes the source tree; retake the snapshot like AC-37 and AC-39 (M4-14 names only those two).
- EX4 `repoint_moved`: test it before the provenance branch in `diff` so a moved link with changed provenance still becomes Remove plus Add; do not also push the entry to `unchanged`.
- EX5 Reason precedence: `source-skill-missing` when the alias is reachable and the ID is absent; otherwise `broken-link` with cause `source-unreachable`. `status` exits 8 only on an orphaned row; `reconcile` exits 8 on any divergence (the s8 wording "exit 8 for all but shadowed" applies to `reconcile`).
- EX6 CLI `rollback` without a journal must resolve the target the way `edit_run` does before `history_edit`, then call `edit_run` with command `rollback`.
- EX7 `doctor`: run `classify_dest_root` first and report instead of probing when the root resolves outside the project.
- EX8 REPORT TO TAYLOR: `rollback` after `reconcile --repair` or relocate repair is a no-op (those history entries hold equal before and after selections), and history rollback restores the selection, so it matches the old destination only while bundles and sources are unchanged.

Open gaps: none FAIL-class. Named residuals (not proving): Linux and CI unrun (known M3 gap); no pty test (R8); strided fault sample unless `UNAM_FULL_FAULT_MATRIX=1`; unwritable-destination row self-skips as root.
What This Coverage Does NOT Prove:
- G1 to G3: nothing about Linux, CI, or a real terminal prompt; confirmation paths are exercised with `--yes` and the no-TTY refusal only.
- AC-28 and AC-27: output text and scan rules only, never that a live agent discovered or loaded a skill.
- AC-33 and S7: the strided sample, not every injected failure point; rollback equality holds only while bundle and source contents are unchanged (EX8).
- AC-37: relocation to a library that kept the same relative skill paths; skills renamed during the move stay orphaned by design.
- evidence_* rows: the stated adversarial rows within the threat model (same user, trusted local filesystem), not a hostile concurrent process.
- doctor Hybrid row: unproven when the suite runs as root.
Net result: PASS (0 FAIL-class gaps, 0 CONCERNs; no developed behavior rests on a Known-Gap alone)
Gate: PASS (cycle 1; EXECUTE legal; honor G5 presence checks and EX1 to EX8)
Accepted by: not required (no CONCERN remains); the named residuals above are recorded as known-gaps.

---

## Next Step

Run PVL (Wave 2 stops at the PVL verdict), then EXECUTE in Wave 3.
