---
phase: phase-06-m5-agents-neutral-and-briefs
date: 2026-10-17
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_PLAN_04-10-26.md
---

# Milestone M5 phase report: agents-neutral adapter and generated briefs

TL;DR: M5 is VERIFIED (WITH_GAPS). The execute run was green and the independent EVL run (vc-tester) passed after one fix cycle (a real resume bug in brief replace, fixed in 5ba6888; 643 tests now). Execute-run numbers: gates G1 to G3 pass (642 tests at that point, 0 failed, 0 ignored), all 44 product Phase 1 criteria (AC-1 to AC-43 and AC-53) have a named, passing test, and S1 and S6 pass on Taylor's real library (median 0.65 s against a 30 s limit; four skills cost 1.1 percent of the whole library). Product Phase 1 is NOT done: that needs Taylor's daily use, the first real project (D4), and Linux/CI, none of which an agent can supply. The program stops at the product Phase 1 gate (state: AWAITING TAYLOR).

## What Was Done

- New core, `crates/unam-core/src/`: `adapter/agents_neutral.rs` (adapter, 16 evidence entries; the ancestor walk is `observe::ancestors_to_git_root`), `generated.rs` (brief content, opt-out rule, brief observation, the generated side of the diff), `generated/tests.rs` (20 unit tests).
- Changed core: `adapter/mod.rs` (`Scope::Ancestor`; registry `[claude-code, agents-neutral]`; `adapter_claims_have_evidence` generalized over both adapters), `observe.rs` (a location shadows only when it outranks the Project location), `plan.rs` (`Step::WriteGenerated`, `Step::RemoveGenerated`, `DiffInput.generated`, receipt arms, `removes_something`), `state.rs` (journal step ops `write_generated` and `remove_generated` inside the `begin` record; `GeneratedEntry` typed view of `Entry::Other`), `fsops.rs` (`FsOps::write_generated`, `FsOps::remove_generated`, `observe_brief`, `BriefObserved`), `apply.rs` (journal conversions, execute, resume, rollback arms; the moved-aside copies are deleted once the apply is final), `change.rs` (brief inputs from the selection's bundles; the shared-destination warnings), `verify.rs` and `reconcile.rs` (`brief-missing`, `brief-modified`; receipted briefs are never `foreign`), `lib.rs`.
- CLI: `output.rs` (shared-destination sentence, brief lines with the instruction under them, honesty line, followup lines, brief counts), `cli/change.rs` (adapter followups in the human result and `--json`; `Visible to` line on `agents-neutral` results, recover flows included), `cli/status.rs` (bare `unam` with two targets prints usage plus a one-line hint, exit 0).
- Tests: new `ac_agents_neutral.rs` (10), `ac_briefs.rs` (12, including fault matrices over create, deactivate, and replace for both targets), `phase1_checks.rs` (4), `phase1_exit.rs` (2, self-skip without `UNAM_REAL_LIBRARY`); unit tests in `agents_neutral.rs`, `generated/tests.rs`, `fsops/tests.rs`, `state/tests.rs`, `plan/tests.rs`, `observe.rs`, `output.rs`; three new snapshots (`plan_agents_neutral_json`, `status_agents_neutral_json`, `add_brief_json`; 39 in all). 642 tests in all (567 at M4).
- Docs: `docs/adapters.md` gained the `agents-neutral` evidence table, notes, and a generated-brief section; the two M2 sections and the M4 sections are kept. The five protected documents are untouched.
- No new dependency (`Cargo.lock` unchanged); no `#[ignore`; no em dash or emoji in any changed file.

## Mid-phase progress note

Written at closeout, not at 50 percent: the note was not saved to disk while the work ran (a process miss, recorded as such). State at about the halfway point: A1 to A3 done; `generated.rs` stubbed with its 20 unit tests red, then green once implemented; C1 to C4 compiled; the workspace green apart from the planned M3 edits. The one bug found after that point was in `rollback` (a never-started `WriteGenerated` of a replacement was judged against the old brief still at the destination); the fault matrix caught it and it is fixed (a step that never started is skipped).

## Product Phase 1 completeness

Agent half of PRD section 12. Every criterion has a named test (`phase1_ac_coverage`; red-first, see Red-first record) and every test passed in the execute run. The EVL column is the independent vc-tester run at 5ba6888 (643 tests, 0 failed, 0 ignored, full fault matrix included): pass for all 44 criteria. Nothing is red or missing.

| Criterion | Closed in | Test (file::name) | Execute run | EVL |
|---|---|---|---|---|
| AC-1 | M1 | `ac_read_path::ac_01_register_non_git_directory_and_skills_discoverable` | pass | pass |
| AC-2 | M3 | `ac_activation::ac_02_source_tree_unchanged_after_full_activate_modify_deactivate_cycle` | pass | pass |
| AC-3 | M1 | `ac_read_path::ac_03_three_level_nested_categories_real_paths` | pass | pass |
| AC-4 | M1 | `ac_read_path::ac_04_git_and_node_modules_yield_no_skills` | pass | pass |
| AC-5 | M1 | `ac_read_path::ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic` | pass | pass |
| AC-6 | M1 | `ac_read_path::ac_06_search_spans_three_categories_one_result_set` | pass | pass |
| AC-7 | M2 | `ac_bundles::ac_07_hermes_only_bundle_with_bare_names_loads_unchanged` | pass | pass |
| AC-8 | M2 | `ac_bundles::ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates` | pass | pass |
| AC-9 | M2 | `ac_bundles::ac_09_bundle_spans_two_categories_and_two_sources` | pass | pass |
| AC-10 | M2 | `ac_activation::ac_10_saved_bundle_applied_to_empty_project_yields_original_selection`; `ac_selection::ac_10_saved_bundle_in_empty_project_yields_original_selection` | pass | pass |
| AC-11 | M2 | `ac_bundles::ac_11_removing_one_entry_leaves_every_other_line_byte_identical` | pass | pass |
| AC-12 | M2 | `ac_selection::ac_12_bundle_plus_add_plus_remove_resolves_with_provenance` | pass | pass |
| AC-13 | M3 | `ac_activation::ac_13_two_skills_from_two_categories_both_applied_with_no_category_step` | pass | pass |
| AC-14 | M4 | `ac_verify::ac_14_status_shows_source_path_destination_strategy_provenance_state_per_entry` | pass | pass |
| AC-15 | M3 | `ac_activation::ac_15_yes_still_prints_the_plan_and_the_plan_matches_the_filesystem` | pass | pass |
| AC-16 | M3 | `ac_activation::ac_16_second_identical_activation_makes_zero_changes_and_says_so` | pass | pass |
| AC-17 | M3 | `ac_activation::ac_17_remove_one_add_one_changes_exactly_two_entries_and_a_third_record_is_verbatim` (generated entries: `ac_briefs::brief_unchanged_is_a_no_op_and_record_verbatim`) | pass | pass |
| AC-18 | M3 | `ac_activation::ac_18_replace_and_add_from_the_same_start_give_different_end_states` | pass | pass |
| AC-19 | M3 | `ac_activation::ac_19_operations_in_project_a_leave_project_b_unchanged_by_hash` | pass | pass |
| AC-20 | M5 | `ac_agents_neutral::ac_20_two_targets_hold_and_report_different_selections` | pass | pass |
| AC-21 | M3 | `ac_activation::ac_21_returning_to_a_project_shows_its_prior_selection_unchanged` | pass | pass |
| AC-22 | M3 | `ac_activation::ac_22_collision_with_a_foreign_skill_refuses_names_both_paths_and_changes_nothing` | pass | pass |
| AC-23 | M3 | `ac_activation::ac_23_deactivate_all_removes_exactly_the_two_owned_and_leaves_foreign_and_directory` (generated entries: `ac_briefs::brief_removed_on_deactivate_and_foreign_left`) | pass | pass |
| AC-24 | M3, M5 | `ac_activation::ac_24_no_agent_config_file_is_created_or_modified`; `ac_agents_neutral::ac_24_agents_neutral_no_agent_config_file_is_created_or_modified` | pass | pass |
| AC-25 | M3 | `ac_activation::ac_25_every_created_entry_is_in_the_receipt_with_source_id_and_strategy` | pass | pass |
| AC-26 | M4 | `ac_verify::ac_26_broken_destination_symlink_is_orphaned_and_status_exits_8` | pass | pass |
| AC-27 | M4 | `ac_verify::ac_27_shadowed_skill_reports_destination_and_shadowing_path` | pass | pass |
| AC-28 | M4 | `ac_verify::ac_28_no_output_claims_an_agent_read_loaded_or_applied_a_skill` | pass | pass |
| AC-29 | M3, M5 | `ac_activation::ac_29_claude_code_output_states_no_restart_required_with_evidence`; `ac_agents_neutral::ac_29_agents_neutral_says_undocumented_and_prints_the_hermes_followup` | pass | pass |
| AC-30 | M3 | `ac_activation::ac_30_every_removal_prints_the_loaded_instructions_caveat` | pass | pass |
| AC-31 | M3 | `ac_recovery::ac_31_interrupted_apply_leaves_the_journal_and_the_next_command_detects_it_without_acting`; `ac_recovery::ac_31_resume_and_rollback_reach_a_consistent_state_for_every_injected_failure` | pass | pass |
| AC-32 | M3 | `ac_recovery::ac_32_one_of_three_entries_fails_the_other_two_remain_applied_and_the_failure_is_named` | pass | pass |
| AC-33 | M4 | `ac_reconcile::ac_33_rollback_from_history_restores_prior_destination_and_receipt` | pass | pass |
| AC-34 | M4 | `ac_reconcile::ac_34_hand_deleted_entry_reconcile_reports_exit_8_and_repair_restores` | pass | pass |
| AC-35 | M2 | `ac_selection::ac_35_hand_edited_selection_is_desired_not_applied` | pass | pass |
| AC-36 | M4 | `ac_verify::ac_36_renamed_source_skill_is_orphaned_with_candidates_and_blocked_by_default` | pass | pass |
| AC-37 | M4 | `ac_reconcile::ac_37_source_relocate_repairs_every_affected_project` | pass | pass |
| AC-38 | M1 | `ac_read_path::ac_38_unreachable_source_reported_no_mutation` | pass | pass |
| AC-39 | M4 | `ac_verify::ac_39_source_edit_under_follow_is_visible_at_destination_without_drift_warning` | pass | pass |
| AC-40 | M3 | `ac_activation::ac_40_skill_in_two_bundles_appears_once_with_two_provenance_records` | pass | pass |
| AC-41 | M4 | `ac_verify::ac_41_contradictory_skills_activate_and_no_semantic_conflict_claim_appears` | pass | pass |
| AC-42 | M5 | `ac_agents_neutral::ac_42_nested_project_warns_and_names_the_ancestor` | pass | pass |
| AC-43 | M5 | `ac_agents_neutral::ac_43_every_plan_names_the_four_consumers` | pass | pass |
| AC-53 | M3 | `ac_recovery::ac_53_second_mutating_command_refuses_while_locked_and_changes_nothing` | pass | pass |

Success targets (PRD 13.1):

| ID | Result | Evidence |
|---|---|---|
| S1 | PASS | `s1_cold_to_verified_active_on_real_library` on `~/Developer/01-AI/skillzone` (743 skills), release build, fresh cache per run: five runs 0.621, 0.674, 0.648, 0.629, 0.651 s; median 0.648 s (limits: each under 60 s, median under 30 s). The measured sequence is `source add`, `add` of two skills with `--yes` into a temporary project, `status`. One machine, a fresh cache directory (not a cold OS page cache) |
| S2 | PASS | `ac_activation::s2_one_command_add` |
| S3 | PASS | the source-tree invariant (`fx.done()`) ends every mutating test; S1 and S6 snapshot the real library before and after (identical) |
| S4 | PASS | the planted-foreign invariant (`fx.done()`) ends every mutating test, the new briefs included |
| S5 | OBSERVATIONAL | see the S5 follow-up below |
| S6 | PASS | `s6_catalog_cost_reduction_on_real_library`: a four-skill selection is about 990 tokens, all 743 skills about 92,938 tokens: 1.1 percent (limit 10 percent). Read from the refused (`--json`, no `--yes`) plans; nothing applied; the library has 743 skills, so the library-size caveat does not apply |
| S7 | PASS | `ac_recovery::s7_interruption_zero_divergence`, plus the new brief fault matrices (create, deactivate, replace) for both targets |
| S8 | PASS | `ac_reconcile::s8_divergence_always_reported` |
| S9 | PASS | `adapter::tests::adapter_claims_have_evidence` over both adapters (generalized, M5-13) |
| Network none | PASS | `no_network_dependencies` (denylist over `Cargo.lock`), `no_network_symbols` (no `std::net`, socket types, or process spawn in `crates/*/src`); `cargo tree -p unam --edges normal` read once (130 lines; the direct dependencies are anyhow, blake3, clap, ignore, jiff, nucleo-matcher, serde, serde_json, serde_norway, tempfile, thiserror, toml, tracing, tracing-subscriber, unam-core): no HTTP, TLS, async runtime, or socket client |
| Background none | PASS | `no_background_process`: eight commands over both targets in a project with a unique path, then `pgrep -f <path>` is empty; the probe is proven live by a control process (`sh -c 'sleep 30; true' <path>`) that `pgrep` does see |

S5 follow-up (not a gate, recorded here and in the umbrella exit notes): four weeks after Taylor starts using `unam` daily, count the `active` records of `unam status --json` in each project he uses (the median should be under 10, PRD S5). Nothing is automated; it needs his real projects.

Open, and reported as open (not passed): Taylor's daily use; the first adopting project and the D4 real activation (backlog NOTE `m3-d4-first-real-activation_NOTE_05-10-26.md`); Linux and CI (the CI file has never run, no remote); the Cursor editor and Windsurf were never tested (the Cursor rows rest on documentation plus the `cursor-agent` CLI spike); the seven decision-sheet rows Taylor has not marked.

## What Was Skipped or Deferred

- Per the plan: isolated per-vendor destinations (PRD D3, product Phase 2), copy-specific work, `--adopt`, VCS-tracked warning (EDG-12), nested `scan_locations`, `docs/json.md` (M7), M6 and M7.
- No pty test for the interactive prompt (R8, accepted at M3).
- The fault matrices run the strided sample by default (call 1, every third call, last call); `UNAM_FULL_FAULT_MATRIX=1` runs every call. The full set was not run this session for the new brief matrices (each takes about a minute strided).
- The specialist review calls (vc-code-reviewer, vc-code-simplifier) and the vc-security and vc-scenario skills could not be invoked: this execute agent has no tool to spawn another agent or skill. Replaced by a self-review (below). The orchestrator's independent vc-tester run and any reviewer spawn remain to do.

## Test Gate Outcomes

Commands run with `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no`.

| Gate | Command | Result |
|---|---|---|
| G1 | `cargo fmt --all --check` | exit 0 |
| G2 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| G3 | `cargo test --workspace --locked --no-fail-fast` | 642 passed, 0 failed, 0 ignored, 26 result lines; 642 tests listed. 39 snapshots; the 36 existing are unchanged |
| G4 | perf only if `crates/unam-core/src/catalog` changed | not applicable: no catalog file was edited. The literal command `git diff --name-only 8d688e1 -- crates/unam-core/src/catalog` prints six file names, but only because of a repo-wide mode change (see Plan Deviations, D9); the blobs are identical. Run anyway as extra evidence: `cargo test --release -p unam-core --test perf_cold_scan` median 90.3 ms (limit 300 ms) |
| G5 (a) | `(^|::)ac_20_`, `ac_42_`, `ac_43_`, `ac_24_agents_neutral_`, `ac_29_agents_neutral_` in `cargo test --workspace --locked -- --list` | 1 match each |
| G5 (b) | `adapter_claims_have_evidence`, `no_network_dependencies`, `no_network_symbols`, `no_background_process`, `phase1_ac_coverage`, `s1_cold_to_verified_active_on_real_library`, `s6_catalog_cost_reduction_on_real_library` | all listed (1 each) |
| G5 (c) | names matching `(^|::)brief_[a-z_]+: test$` | 13 (at least 10 required) |
| G5 (d) | `! grep -rEq '#\[ignore' crates` | ok |
| G5 (e) | `git diff --diff-filter=MD --exit-code 8d688e1 -- crates/unam-cli/tests/snapshots` | the literal command exits 1 on mode-only changes (644 to 755, commit `4956007`); blob-by-blob comparison of all 36 snapshots against `8d688e1` finds 0 content differences, `git diff --diff-filter=MD --exit-code HEAD -- <snapshots>` exits 0, and `git status` shows no modified or deleted snapshot, only three new files |
| G6 | invariants: every new mutating test ends with `fx.done()` and calls the target-aware zero-divergence helper after apply or recovery (`assert_zero_divergence_in` / `_for`); S1 and S6 compare `snapshot_source` before and after | ok |
| G6 | `git diff --exit-code 8d688e1 -- <the five protected docs>` | the literal command exits 1 on the same mode-only change (blob `57f126f` is identical for `PRD-unambiguous.md`, and every protected doc compares equal blob by blob); `git diff --exit-code HEAD -- <the five docs>` exits 0 |
| extra | `git diff --check` | exit 0 |
| extra | E2 (real library) `UNAM_REAL_LIBRARY=$HOME/Developer/01-AI/skillzone cargo test --release -p unam --test phase1_exit -- --nocapture` | 2 passed, library snapshot identical after both |

One flake seen once in about eight full runs: `activation_cli::history_is_capped_at_twenty_entries` failed with `Locked` (the in-process lock of its own project). It passed on every rerun (three binary reruns, two full suite runs). Cause (a reading, not proven): a concurrent test thread's `Command::spawn` forks while the in-process lock file descriptor is open, so the child briefly holds the `flock` until it execs. It predates M5 (no spawn was added to that test binary) and is a test-harness race, not a product bug. Not fixed (out of scope).

## Red-first record

The plan asks whether M5 ran tests red first (M3 and M4 did not).

- Red-first, recorded: the 20 unit tests of `generated.rs` and the `diff_generated` cases ran against stub bodies first: 20 failed, 0 passed; then green after the implementation. `phase1_ac_coverage` ran red with `ac_agents_neutral.rs` moved away: it failed with `no test is named for acceptance criteria [20, 42, 43]`; green once the file returned (EX4).
- Not red-first (written together with, or after, the code): the `agents_neutral.rs` and `observe.rs` unit rows, the `ac_agents_neutral.rs` and `ac_briefs.rs` CLI tests, the fsops/state/plan unit rows, the snapshots, `phase1_checks.rs` except the coverage test. One real defect was found by a test written after the code (the fault matrix, see the progress note), which is the evidence those tests can fail.

## Plan Deviations

Within the validate-contract blast radius; none touches auth, billing, schema, a public API, or the container. Also written into the plan under `## Deviations`.

| # | Deviation | Reason | Impact |
|---|---|---|---|
| D1 | A wanted brief that exists with edited content gives `modified-at-dest`, not `foreign-at-dest` (M5-10 words it as `ForeignAtDest`). `foreign-at-dest` is used for a name occupied by something unam never wrote | the message of `foreign-at-dest` says "not managed by unam", wrong for a receipted brief the user edited; this is the existing skill rule | both block with exit 6; tests assert the code |
| D2 | A changed brief is two steps, `remove_generated` then `write_generated`; the removal moves the old brief into the operation trash (EX5) and rollback restores it exactly | EX5 (supersedes the C4 "restore from hash" fallback); the journal holds only a hash | new `FsOps` methods stay at two (rollback restores through the existing `restore_owned`) |
| D3 | The new warnings (`ancestor-skills`, `cross-adapter-duplicate`, `destination-policy-isolated`) are in `plan.warnings` (printed with the plan, in `data.plan.warnings`), not the envelope `warnings`. `brief-missing` and `brief-modified` are envelope warnings (severity warning, not info), and `reconcile` also lists them as `findings` | M5-7 and M5-11 do not say where; a warning belongs next to the plan the user confirms | exit codes unchanged |
| D4 | `--json` `receipt.entries` counts every receipt entry (was skill entries only) | a receipt holding a brief must not report one fewer | unchanged output when a receipt holds only skills; the 36 snapshots are unchanged |
| D5 | The brief `description` text is mine: `Instructions for working with the <bundle name> bundle: <first instruction line, at most 160 characters>` | spec 4.8 step 7 says only "a one-line description" | the wording can be changed in `generated.rs` without touching anything else |
| D6 | `no_background_process` also runs a positive control (a process that does name the path must be seen by `pgrep`) | without it a sandbox that hides processes would pass vacuously (it did once: `sleep 30 <path>` is an error on macOS and exits at once) | none |
| D7 | The mid-phase progress note was written at closeout | see the note above | none |
| D8 | The specialist review and security skills were not invoked | no agent or skill tool in this execute session | self-review and an inline STRIDE below; EVL and a reviewer spawn remain |
| D9 | RESOLVED: mode change reverted in `e4bf5b2`, literal gates exit 0. Was: the validate-contract commands G4, G5 (e), and G6 compare against `8d688e1`, but commit `4956007` ("update file permissions", 644 to 755 across the repository, protected docs, catalog sources, and all 36 snapshots included) makes the literal commands report mode-only differences | an environment change since the contract was written; content is identical (blob hashes equal) | the contract commands as written fail for a reason unrelated to M5; an EVL run must compare against `HEAD` or by blob. Taylor should know the chmod happened |
| D10 | S1 passes `--yes`, into a temporary project outside this repository | the plan's S1 definition requires `add` with `--yes` to measure "verified active"; S6 never passes it (EX12). The spawn instruction said "never pass `--yes` there" for the real library | the library is read-only in S1 (only symlinks pointing into it are created in the temp project); the before/after `snapshot_source` equality is asserted |
| D11 | RESOLVED in `476b81c` (2026-10-17). Was: an unwanted receipted brief already missing from disk became a `RemoveGenerated` step. Now: no step and no SES-2 caveat; `apply::finish` releases the receipt entry of a missing brief that no step touches (follow-up `0d5c56b`: a plan with zero steps also releases the stale row, through a receipt-only rewrite in `change::run`) | finding R3 | none outstanding |

EX1 to EX14 (all binding notes):

| EX | Result |
|---|---|
| EX1 | Applied first. `copy_fixture_bundle_no_brief` appends the opt-out to a copy; `Fx::new_with` uses it; `fixtures/bundles/hermes-only.yaml` is byte-identical; only the brief tests (and `add_brief_json`) copy the unmodified file. No existing snapshot changed |
| EX2 | Applied: `adapter_for_registry`, `resolve_target_branches` (two assertions), and `unsupported_agent_flag_value_exit_2` rewritten (now `agents_neutral_flag_value_is_a_supported_target`); `invalid_agent_flag_value_exit_2` kept; `TargetUnsupported` stays |
| EX3 | Applied: all `FsOps` implementors got delegating methods (RealFs, FailingFs, Faulty, RecordingFs, FinalCrash, CrashOnRmdir, LogFs, Crash, FailRemove); `cargo test --workspace --no-run --locked` ran right after C3 and again green |
| EX4 | Applied: needles `format!("fn ac_{n:02}_")`; red run `[20, 42, 43]`, then green |
| EX5 | Applied: remove moves the directory into the operation trash; rollback restores it exactly; resume treats "gone but in trash" as done; the trash copy is deleted once the apply is final; when the copy is missing the rollback result says `content-not-recoverable` and the receipt is restored without the file |
| EX6 | Applied: `dest_dirs_to_create` counts a `WriteGenerated`; `write_brief` calls `ensure_dirs` (execute and resume); receipted briefs that stay count toward the `RemoveDir` decision |
| EX7 | Applied: `is_removal` and `removes_something` count `RemoveGenerated` (SES-2 caveat prints for a brief-only removal; tested); `Step::id()` is `None`; `render_plan` arms and unit tests |
| EX8 | Applied: `request_for`, `assert_zero_divergence_for`, `assert_ordering_invariant_for` (old names delegate); the brief tests also assert the file and the receipt entry directly |
| EX9 | Applied: replay verifies the content hash first, writes only when the destination is missing, treats an equal-hash directory as done, otherwise skips as `dest-occupied`; test `brief_resume_refuses_journal_content_that_does_not_match_its_hash` |
| EX10 | Done: in the For Taylor block |
| EX11 | Applied: one `cross-adapter-duplicate` warning per skill name, whichever target changes; an unreadable other receipt means none (tested) |
| EX12 | Applied: library path only from `UNAM_REAL_LIBRARY`; isolated `TestEnv`; project under the system temp dir; before/after `snapshot_source`; S6 reads `plan.estimate` from a refused plan and never passes `--yes`; no `#[ignore`; the Hermes followup test creates an empty `~/.hermes` in the temp HOME; no agent CLI is run or looked up |
| EX13 | Applied: with no Project location nothing shadows; `shadows_from_enterprise_and_user_only` kept; the `agents-neutral` case added beside it |
| EX14 | Applied: no `cargo-insta`; the three new `.snap` files were written once with `INSTA_UPDATE=always` for those three tests only, read, then every gate runs `INSTA_UPDATE=no` |

## Test Infra Gaps Found

- `faults.rs` hooks for the two new `FsOps` methods exist; the strided matrix now covers the brief steps (create, deactivate, replace; both targets). Full-matrix (`UNAM_FULL_FAULT_MATRIX=1`) run of the new brief matrices still to do.
- `Fx` has no helper for a Git-root fixture with an ancestor `.agents/skills`; `ac_42` builds it inline with `fx.project` plus `write_file`.
- The in-process lock versus concurrent `Command::spawn` race (see Test Gate Outcomes).
- `pgrep` is required by `no_background_process`; the test self-skips (named residual) where it is missing.
- `ac_briefs` takes about a minute on this machine (two threaded fault matrices).

## Review, security, and edge cases (self-review standing in for the specialist spawns)

- STRIDE for the new write path (a brief is written and removed by the frozen executor): Tampering: the destination is a validated child name (`join_child`), the file name is fixed, removal happens only when the directory holds exactly `SKILL.md` with the expected hash, a symlink or foreign file is never followed or removed, the journal content is hash-checked on replay and the journal names are validated. Repudiation: every step is journaled and ends in history. Information disclosure: a brief holds the user's own bundle instruction and lives in the project, like the skill links; whether the project is under version control is EDG-12, out of scope. Denial of service: bounded by the instruction size. Elevation: a cloned repository's own `<x>-brief` is foreign and is never touched. Spoofing: not applicable (single local user, per the threat model). No hardening beyond the threat model was added.
- Edge cases covered by named tests: opt-out true, false, absent, no instruction; unknown and duplicated bundle names; special characters, a `---` line, and control characters in the instruction; an over-long slug; a brief name equal to a skill name; a foreign name; an edited brief (wanted and not wanted); a missing brief; a changed instruction; an unreadable other-adapter receipt; ancestors that are the Git root, below it, absent, or empty; a user-scope skill that must not shadow.
- Not done: a second look by a reviewer agent. `generated.rs`, `apply.rs` (new arms), and `fsops.rs` (two new functions) are the places to read first.

## Closeout Packet

- Selected plan: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_PLAN_04-10-26.md`.
- Finished: checklist A1 to E4; F1 at the commit step. Classification: `Keep in active/testing` until the independent EVL run passes; then `Ready for UPDATE PROCESS archival`.
- Verified in this run: G1 to G3, G5, G6 (content), E2, E3, perf (extra). Unverified: the independent re-run; Linux and CI; the Cursor editor and Windsurf; any agent actually reading a brief (PRD G12).
- Remaining at execute time: EVL, UPDATE PROCESS (context docs), then the hard stop. Done since: EVL (2 cycles, CLEAN) and UPDATE PROCESS (context docs updated for M5, 643 tests); only the hard stop remains.
- Follow-up plan stubs created: none. CONTEXT_PARTIAL: `CONTEXT_PARTIAL: process/context/tests/all-tests.md` and `all-context.md` (milestone M5 facts: test count, `agents_neutral.rs`, `generated.rs`, new test files, 39 snapshots) are stale until UPDATE PROCESS; execute may not edit `process/context/`.
- Single best next state: spawn vc-tester for the EVL confirmation run (use `HEAD` or blob comparison for the protected-docs and snapshot checks, see D9).

## For Taylor

ANSWERED 2026-10-17: Taylor confirmed readings 1 to 3 (M5-1, M5-8, M5-9/EX10, see item 0 below), approved the review fixes (R1 to R4 plus the adopt-rollback and zero-step receipt gaps, `476b81c`, `0d5c56b`), had the D9 mode change reverted (`e4bf5b2`), and CLEARED the Product Phase 1 Gate ("Clear gate, continue M6+M7"; daily use, D4 and S5 deferred until all milestones are done: "Daily will begin once we finish all of the milestones"). Items 5 to 7 and 9 below are read and carried as open items after M7 where they were Taylor's (see the umbrella `## Current Execution State`).

What is verified (execute run and the independent EVL run at `5ba6888`): everything under Product Phase 1 completeness above; the S1 and S6 numbers on your library; the new adapter and briefs behave as the plan says, including interruption at every sampled step. What is not: Linux/CI, the Cursor editor and Windsurf, and whether any agent reads a brief.

To confirm product Phase 1 you still need to: use `unam` daily on a real project (D4, name the project), look at `unam doctor --evidence` once, and say explicitly that product Phase 1 is done and M6 may start. No agent fills the gate record.

Interpretations to read (the plan asked for these; I did not resolve any document tension):

1. M5-1: the ancestor walk for `agents-neutral` stops at the nearest directory holding `.git` (included), and there are no ancestors when the project is itself the Git root or when no `.git` exists above it. The spec does not say what to do when no Git root exists; the walk never runs to the filesystem root.
2. M5-8: the brief mechanism adds two new step ops, `write_generated` and `remove_generated`, inside the existing journal `begin` record (no new record type), two `FsOps` methods, and a receipt entry exactly as spec 2.4 shows.
3. M5-9 and EX10: an absent `metadata.unambiguous.materialize_instruction` means the brief is written (PRD 5.3 says "opt-out per bundle"), while spec 4.8 step 7 reads "with `materialize_instruction` true". The stub's "config key per spec 2.1" does not exist (spec 2.1 has none), so there is no config key.
4. D9: commit `4956007` changed the mode of every tracked file from 644 to 755, protected docs included. Content is unchanged, but the contract's `git diff --exit-code 8d688e1 -- ...` commands now fail on mode alone. Intended or not is yours to say.
5. D1, D3, D5: the conflict code for an edited brief, where the new warnings print, and the brief description wording (see Plan Deviations).
6. D10: S1 used `--yes` into a temporary project (the library was only read).
7. EVL found and fixed one real bug (`5ba6888`: a replaced brief could be lost after an interrupted replace and resume). Details in the EVL outcome section.
0. Confirmed by Taylor on 2026-10-17: the three readings M5-1 (ancestor stop rule), M5-8 (brief journal ops), and M5-9/EX10 (brief written by default unless the bundle opts out). D9 mode change reverted (commit `e4bf5b2`); the literal `8d688e1` diff gates exit 0 again.
8. Code-review findings R1 to R4 are now fixed (`476b81c`; see the review table). Originally recorded as: R1, a `.DS_Store` in a brief directory blocks that target with exit 6 (a one-line-class fix: ignore `SKILL_SKIP_NAMES` in `brief_at` and removal); R2, a hand-edited brief blocks the whole plan, even when the edit equals the desired content. R3 is D11; R4 is negligible.
9. The open M3 and M4 items stay with you: the seven decision-sheet rows, D4 and the first adopting project (row 54), the M4 interpretations in the M4 report.

## Forward Preview

#### Test Infra Found

- `phase1_checks.rs` and `phase1_exit.rs` (self-skipping without `UNAM_REAL_LIBRARY`); `common`: `copy_fixture_bundle_no_brief`, `Fx::write_bundle_with`, `core_apply_for`, `request_for`, `assert_zero_divergence_in`, `assert_zero_divergence_for`, `assert_ordering_invariant_for`.
- M6 must keep `phase1_ac_coverage` green (it counts AC-1 to AC-43 and AC-53 by name; AC-44 is M6's).

#### Blast Radius Changes

- The registry now has two adapters: any M6 code that lists adapters or targets sees both. `Scope::Ancestor` is a new scope value (exhaustive matches in M6 UI code must handle it).
- Receipts can hold `kind: generated` entries; the TUI must show or skip them without treating them as skills (`Receipt::generated_entries`, `verify` never lists them as records).
- `--json` `receipt.entries` counts all entries.

#### Commands to Stay Green

```
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
UNAM_REAL_LIBRARY=$HOME/Developer/01-AI/skillzone cargo test --release -p unam --test phase1_exit -- --nocapture   # once, at the Phase 1 exit
```

#### Dependency Changes

- None. `Cargo.lock` is unchanged.

## EVL fix cycle 1

Failing gate: full fault matrix on `ac_briefs` (`UNAM_FULL_FAULT_MATRIX=1`), 2 of 12 failed at `ac_briefs.rs:716` (scenario `replace claude-code`, `FailBefore(7)`, resume).

### Root cause (product bug, not a matrix expectation error)

Call 7 of the replace sequence is `remove_generated` of the old `mx-brief` (calls 1-6 are the selection write, journal write, journal append, the `deploy-checklist` symlink, and two journal appends). Evidence from the journal after `FailBefore(7)`: step 2 `failed io-error`, step 3 (`write_generated`) `skipped dest-occupied` (the old brief was still there), then a `partial` receipt marker.

`resume` treats `Skipped` as terminal, so after the remove step re-ran and succeeded, the dependent write was never retried. The brief was gone, its receipt row was released, and resume reported Recovered. A brief silently dropped after a "recovered" replace is a product defect.

Fixing that exposed a second defect on the same path, `FailAfter(7)` (the old brief moved aside, the error returned afterwards; the new brief written by step 3): on resume the failed remove step saw the new brief at the destination, found its hash differed from the expected old hash, and skipped as modified. The receipt was right but the moved-aside copy was never purged (trash left).

### Fix (`crates/unam-core/src/apply.rs`, `resume` and `resume_step`)

- `resume`: a `WriteGenerated` journaled `Skipped{dest-occupied}` is re-evaluated (not treated as terminal) when the same-name `RemoveGenerated` in the plan is not terminal. The write replay stays hash-checked (EX9 unchanged: `resume_step` still verifies the observed hash against the journaled content hash before treating it as done).
- `resume_step` `RemoveGenerated`: when the destination holds a brief whose hash differs from the step's expected (old) hash and the operation trash already holds the moved-aside copy, the step is Done (observed), so the purge runs. The trash move-aside (EX5) is untouched.

### Regression test

`brief_replace_interrupted_at_call_seven_resumes_to_clean_state` in `crates/unam-cli/tests/ac_briefs.rs`: runs without the full-matrix env var; asserts call 7 is `remove_generated`, then for `FailBefore(7)` and `FailAfter(7)` resumes and checks Recovered, end state equals the clean run (brief file plus `generated mx-brief` receipt row), empty trash, zero divergence. Both cases failed before the fix.

### Gate output

- `UNAM_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs`: 13 passed, 0 failed (the 12 plus the new test).
- `UNAM_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_recovery`: 14 passed. `ac_briefs` and `ac_recovery` are the only suites that read the env var.
- G1 `cargo fmt --all --check`: pass. G2 clippy `-D warnings`: pass. G3 `INSTA_UPDATE=no cargo test --workspace --locked`: all pass (core 390 unit tests, ac_briefs 13).
- `git diff --stat HEAD -- crates/unam-cli/tests/snapshots`: empty.

Closeout state at the end of the fix cycle: Keep in active/testing until the orchestrator's EVL re-run (vc-tester) confirms. It confirmed, see below.

## EVL outcome (UPDATE PROCESS, 2026-10-17)

Two cycles, loop status HALTED_SUCCESS (`results-m5-evl.tsv`; reports `phase-06-m5-agents-neutral-and-briefs-evl-iteration-001_REPORT_17-10-26.md` and `-002_`).

| Cycle | Result |
|---|---|
| 1 (vc-tester at the execute commit) | G1 to G6 pass (G5 e and G6 mode-only). The full fault matrix (`UNAM_FULL_FAULT_MATRIX=1`) failed 2 brief tests: replace on claude-code, `FailBefore(7)`, resume dropped the new brief. Product bug. Fix cycle 1 (above), commit `5ba6888` |
| 2 (vc-tester at `5ba6888`) | G1, G2 exit 0; G3 643 passed, 0 failed, 0 ignored; G4 not applicable; G5 (a) to (d) pass (14 `brief_*` names); G5 (e) and G6 mode-only (numstat 0 0 and 0 insertions, 0 deletions); full fault matrix `ac_briefs` 13 and `ac_recovery` 14 pass; `git diff --check` clean |

Known gap carried forward: the literal G5 (e) and G6 diff commands exit 1 only because commit `4956007` changed every tracked file mode from 644 to 755 (D9); content is identical.

Lesson: the strided fault sample (call 1, every third call, last call) did not hit call 7 and missed a real resume bug. The full matrix found it. Run the full matrix in EVL by default for any change to the apply, resume, or rollback path (about 150 s).

Independent code review (vc-code-reviewer, static, no material bug; the four safety properties hold: source never written, only receipted entries removed, foreign files untouched, names validated before they become paths). Findings are recorded, not fixed; Taylor decides on R1 and R2:

| # | Severity | Finding |
|---|---|---|
| R1 to R4 | FIXED in `476b81c` (2026-10-17, Taylor: "Fix findings"). R1: `.DS_Store` and `.git` ignored when observing a brief; removal takes a `.DS_Store`, a `.git` makes the brief Modified and is never deleted. R2: an edited, still-wanted brief gives a `brief-modified` plan warning and never blocks (receipt entry kept); content equal to the desired text is adopted through a write step that writes nothing and refreshes the receipt hash (follow-up `0d5c56b`: the write step is journaled as `adopt`, so rollback leaves the user's file byte-identical and the receipt returns to its prior entry; resume ends clean; tested by crashing after every call). R3: see D11. R4: scratch is `.unam-tmp-rollback`, rejected by `validate_dest_name`, asserted in a unit test. Two earlier tests that encoded the old blocking behavior (`brief_edited_is_kept_and_released`, `brief_missing_is_reported_and_repaired`) were updated to the new behavior; D1 now applies only to briefs of another shape (extra entries). The original findings follow | |
| R1 | low (macOS daily use) | `fsops.rs` `brief_at` requires the brief directory to hold exactly `SKILL.md`, while skill hashing ignores `.git` and `.DS_Store` (`catalog/scan.rs` `SKILL_SKIP_NAMES`). If Finder adds a `.DS_Store`, a wanted brief becomes a blocking `modified-at-dest` conflict (exit 6) on every add, use, and `reconcile --repair` for that target. An unwanted brief is skipped, unreceipted, and later reported as foreign. Fails safe. Suggested fix: ignore `SKILL_SKIP_NAMES` in `brief_at` and in removal |
| R2 | low | An edited, still-wanted brief blocks the whole plan, including unrelated adds and `reconcile --repair`, even when it was hand-edited to exactly the desired content (`generated.rs` about lines 235 to 253). Broader than M5-11's "an edited brief is left" |
| R3 | low (unlisted deviation) | Recorded as D11 |
| R4 | negligible | The rollback scratch path `trash.join("rollback")` could collide with a removed skill literally named `rollback` |

Closeout state: Ready for UPDATE PROCESS archival (EVL CLEAN). The plan stays in the program task folder like M0 to M4.

## EVL iteration 3 (post-review fixes, 2026-10-17)

Independent vc-tester run at HEAD `4832e31` after the review-fix round (`e4bf5b2` modes restored, `476b81c` R1 to R4, `0d5c56b` adopted brief survives rollback and zero-step plans drop stale brief rows; report commits `d9c357b`, `4832e31`). Report: `phase-06-m5-agents-neutral-and-briefs-evl-iteration-003_REPORT_17-10-26.md`; TSV row in `results-m5-evl.tsv`. Loop status HALTED_SUCCESS, classification CLEAN.

- G1, G2 exit 0; G3 655 passed, 0 failed, 0 ignored.
- G5 (a) to (d) pass (17 `brief_*` names); G5 (e) literal and G6 literal exit 0 (the D9 mode noise is gone).
- Full fault matrix: `ac_briefs` 19 and `ac_recovery` 14 pass. `git diff --check` clean.

Design note (not a bug): a zero-step plan releases stale brief receipt rows without a prompt, because there is no step to confirm. `release_stale_brief_rows` has one call site (`change.rs`), runs under the project lock, only on a zero-step plan, and only drops receipt rows for briefs already missing on disk. Read-only commands never reach it.

The "Known gap carried forward" about D9 above is closed by this iteration. Product Phase 1 Gate: CLEARED by Taylor on 2026-10-17.
