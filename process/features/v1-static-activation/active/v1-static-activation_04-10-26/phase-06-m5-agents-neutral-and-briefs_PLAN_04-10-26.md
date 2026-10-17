---
name: plan:v1-static-activation-phase-06-m5-agents-neutral-and-briefs
description: "v1: static activation, phase 6 of 8: milestone M5, agents-neutral adapter and generated briefs; completes product Phase 1; HARD STOP after this milestone; lighter-process plan supplemented 2026-10-12"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-06
---

# Milestone M5: agents-neutral adapter and generated briefs

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-12 in one combined research, design, and plan pass; lighter process per `v1-static-activation_WAVES_09-10-26.md`)
**Complexity**: COMPLEX
**Phase:** Phase 6 of 8: milestone M5
**Status**: VERIFIED (WITH_GAPS) 2026-10-17 (EXECUTED 2026-10-17; EVL CLEAN after 1 fix cycle, 643 tests; UPDATE PROCESS done; HARD STOP at the Product Phase 1 Gate)
**Phase status:** VERIFIED (WITH_GAPS)
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.
**Execution strategy:** sequential, one sonnet execute agent (Taylor, 2026-10-09: all agents sonnet). One compile chain (`adapter` -> `observe` -> `generated` -> `plan` -> `state` -> `fsops` -> `apply` -> `change` -> CLI -> tests); no disjoint file groups for a team.

---

## Overview

M5 adds the second adapter `agents-neutral` (`.agents/skills`) with shared-destination reporting, ancestor warnings, the Hermes followup, a cross-adapter duplicate warning, and the generated bundle brief with opt-out (spec 10 M5, 4.7, 4.8 steps 7 and 9, 5; PRD D3, 7.6, 7.9, 10.2, 10.3, Q5; AC-20, 42, 43, 24, 29 second halves). It also records the evidence for product Phase 1 completion. Estimated 1 to 2 days (spec 10).

M5 completes product Phase 1 in code. Product Phase 1 is DONE only when AC-1 to AC-43 and AC-53 pass AND Taylor uses the tool daily (PRD 12, 13.2). The second half is Taylor's. The program HARD STOPS after this milestone.

Threat model (standing rule): same user, trusted local filesystem; untrusted input is skill file content and cloned-repo files. No hardening beyond it. Protections against your own mistakes stay (never delete foreign, modified, or unreceipted content; the source library is never written).

Constraints: symlink-only going forward (default strategy `symlink` from the M0 Outcome; existing copy code stays, no new copy branches). Build on the frozen M3 journal, receipt, and executor; no new journal RECORD types (new step ops inside the existing `begin` record are decision M5-8, reported). Never invoke hermes, claude, codex, or cursor-agent in tests or dry runs; the Hermes followup is printed text only.

---

## Scope

- `agents_neutral` adapter (spec 4.7) with evidence per claim; registry entry; `Scope::Ancestor`.
- Ancestor warning (EDG-11), cross-adapter duplicate warning, shared-destination consumer list, Hermes followup, undocumented-refresh wording (AC-29 second half).
- `WriteGenerated` bundle brief with per-bundle opt-out (spec 4.8 step 7; AC-17 and AC-23 behaviors must hold for generated entries).
- Two targets in one project with independent selections (AC-20).
- Product Phase 1 exit checks: S9 for both adapters, network none, background none, S1 and S6 on the real library (once), AC coverage check, completeness report, S5 follow-up note.

Not in scope (decision sheet and PRD): isolated per-vendor destinations (PRD D3, Phase 2), copy-specific work, `--adopt`, VCS-tracked warning (EDG-12, row 53), nested `scan_locations` (row 53), `docs/json.md` (M7), anything of M6 or M7.

---

## Acceptance Criteria

Closes (spec 10 M5): AC-20, AC-42, AC-43, AC-24 (second adapter). Also closes AC-29 second half (M4 report: "AC-24, 29 met for claude-code only; M5") and the second-adapter halves of the invariants. Criteria text: PRD 13.2.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-20 | `ac_20_*` (two targets in one project hold and report different selections) | Fully-Automated |
| AC-42 | `ac_42_*` (nested project under an ancestor-scanning target warns and names the ancestor) | Fully-Automated |
| AC-43 | `ac_43_*` (every `agents-neutral` plan names codex, cursor, windsurf-devin, hermes, in human and `--json`) | Fully-Automated |
| AC-24 (second adapter) | `ac_24_agents_neutral_*` (no agent config file created or modified; manual steps printed) | Fully-Automated |
| AC-29 (second half) | `ac_29_agents_neutral_*` (undocumented target: output says "not documented"; never "no restart required") | Fully-Automated |
| Briefs (spec 4.8 step 7; AC-17, AC-23 hold) | `brief_*` tests (write, no-op, opt-out, remove, modified kept, foreign refused, interruption) | Fully-Automated |

### Performance and success-target checks (exit-gate checks; spec 9, PRD 13.1)

| ID | Check | How measured |
|---|---|---|
| S9 (both adapters) | No adapter claim without an evidence URL or spike result | unit test `adapter_claims_have_evidence` over both adapters (generalized, M5-13) |
| Network none | No HTTP, TLS, or socket client | test `no_network_dependencies` (denylist over `Cargo.lock`) and `no_network_symbols` (no `std::net` symbols and no `.spawn(` in `crates/*/src`), plus `cargo tree -p unam --edges normal` read once into the report |
| Background none | No process outlives a command | test `no_background_process`: run commands with a unique temp project path, then `pgrep -f <that path>` is empty (not `pgrep -x unam`, which other tests and terminals would disturb) |
| S1 | Cold-cache scripted sequence under 60 s, median under 30 s | env-gated test `s1_cold_to_verified_active_on_real_library` (M5-14): five runs of `source add`, add two skills with `--yes`, `status`, fresh `UNAM_CACHE_DIR`, temporary project outside this repository; self-skips when `UNAM_REAL_LIBRARY` is unset; run once at the Phase 1 exit |
| S6 | Typical selection estimate at most 10 percent of the whole library estimate | env-gated test `s6_catalog_cost_reduction_on_real_library`: spec 4.8 step 10 estimate read from the refused (`--json`, no `--yes`) plan for a four-skill selection and for every cataloged skill; nothing is applied; both numbers recorded |

S5 (median active selection under 10 after four weeks) is OBSERVATIONAL, not a gate. Record it as the "S5 follow-up" in the report and in the umbrella exit notes.

---

## Entry Gate

- M0 gate met (`spike/symlink-discovery.md` Outcome: `agents-neutral` default strategy `symlink`).
- M4 VERIFIED with its report; commits `c3c9be7` (code) and `8d688e1` (process) exist; the M4 Forward Preview was read for this plan. Base for the protected-docs diff is `8d688e1`.
- The decision sheet rows marked "needs a real decision" are unmarked, so defaults apply; none of them blocks M5 (row 54, D4, stays with Taylor).

---

## Decisions

Simplest spec-faithful choices where the spec or PRD is silent. "REPORT TO TAYLOR" marks a visible interpretation for the phase report.

| # | Decision |
|---|---|
| M5-1 | New `Scope::Ancestor` (`as_str` "ancestor"). `agents_neutral.scan_locations`: project `<root>/.agents/skills` rank 0; ancestors rank 1; `~/.agents/skills` User rank 2 (spec 4.7). Ancestors are every `.agents/skills` in the parent directories of `<root>` up to and including the nearest directory holding `.git`; when no `.git` directory is found above `<root>` there are no ancestors (the walk never runs to the filesystem root). Empty when `<root>` is itself the Git root, as the spec says. REPORT TO TAYLOR: the stop rule when no Git root exists is not in the spec. |
| M5-2 | `observe` shadows today use scope (enterprise or user) without rank. Fix: a location shadows only when its rank is lower than the Project location's rank. For `claude-code` nothing changes (0 and 1 beat 2); for `agents-neutral` the user location (rank 2) never shadows a project entry, and ancestors are not shadows (spec: equal-rank same-name is a warning). |
| M5-3 | Registry order stays `[claude-code, agents-neutral]`, so nothing detected still defaults to `claude-code` (M3 decision 44); `.agents/` alone selects `agents-neutral`. Two targets in a selection still make a bare command `target-ambiguous` (exit 2); tests and users pass `--target`. Bare `unam` with an ambiguous target prints usage and a one-line hint and exits 0. The M3 unit tests that assert `agents-neutral` is unsupported (`adapter_for_registry`, `resolve_target_branches`) are planned edits. |
| M5-4 | Consumers come verbatim from spec 4.7: `["codex", "cursor", "windsurf-devin", "hermes"]`. `PlanHeader.consumers` already flows to the human header and `visible_to` in `--json`; the shared-destination sentence "Visible to: codex, cursor, windsurf-devin, hermes." appears on every `agents-neutral` plan and result. |
| M5-5 | `refresh` returns `Refresh::Unknown`; `refresh_note` has `kind: "unknown"`, no command, and text with one clause per consumer (Codex: detects changes automatically, restart if the skill does not appear; Cursor: not documented, the cursor-agent CLI needed a restart in testing; Windsurf/Devin: not documented; Hermes: run `hermes skills trust` once per repository) plus evidence URLs. It never contains "no restart or reload required". |
| M5-6 | `followups` returns `hermes skills trust <root>` (reason: Hermes lists project skills only after the repository is trusted; spike and Hermes docs) when `~/.hermes` exists, else empty. `cli/change.rs` and `output.rs` currently pass empty followups; M5 passes `adapter.followups(...)` rendered as `Then ...` lines and into `--json` `followups`. unam prints it and never runs it. |
| M5-7 | Warnings in `change::prepare` (no trait change): `ancestor-skills` for each Ancestor scan location that holds at least one skill directory, text "ancestor <path> contributes N skill(s) to this project (codex walks upward)"; `cross-adapter-duplicate`, one per skill whose destination name is desired in this plan and also receipted for the other adapter, text "duplicate for cursor and windsurf-devin via compat read of .claude/skills" naming the skill; `destination-policy-isolated` when config `destination_policy = isolated` ("not implemented in v1; using shared", PRD D3). All are warnings: exit codes unchanged. |
| M5-8 | Generated brief mechanism. New plan steps `WriteGenerated {n, bundle, name, content, content_hash}` and `RemoveGenerated {n, bundle, name, expect_hash}` (ops `write_generated`, `remove_generated`), new `JournalStep` ops of the same names inside the existing `begin` record (no new record types), two new `FsOps` methods (`write_generated`, `remove_generated`) on `DestPath`, and a receipt entry exactly as spec 2.4 shows (`kind: generated`, `generated: bundle-brief`, `bundle`, `name`, `dest`, `content_hash`, `applied_at`) carried as `Entry::Other`. The brief is the directory `<dest>/<slug>-brief/` holding only `SKILL.md`: frontmatter `name: <slug>-brief` and a one-line `description` (built with the YAML serializer, not string concatenation), then the trimmed `instruction`. `content_hash` is the `blake3:` hash of the exact bytes written. Order stays Removes, Replacements, Adds, Generated (spec 4.8 step 8). New logic lives in a new `generated.rs`; `plan.rs` and `apply.rs` gain thin arms only (both exceed the file-size guide already). REPORT TO TAYLOR: new step ops in the journal. |
| M5-9 | Opt-out is per bundle: `metadata.unambiguous.materialize_instruction` (spec 2.2, already parsed). Absent means on (PRD: "opt-out per bundle"), `false` means off, a blank `instruction` means no brief. There is no config key: the stub's "config key per spec 2.1" is wrong, spec 2.1 has none. A brief is wanted for every bundle named in the target selection, not for bundles reached only through `add` entries. |
| M5-10 | Brief ownership rules (mirror skills): observed == receipt hash and desired differs -> `WriteGenerated` replaces; observed missing and desired -> `WriteGenerated`; not desired and observed == receipt hash -> `RemoveGenerated`; not desired and observed missing -> drop from the receipt; observed present but neither receipt nor desired hash (user edited) -> kept: blocking `ForeignAtDest` conflict when desired, skipped `modified` and released from the receipt when not desired; a name already taken by a foreign directory or a desired skill -> `ForeignAtDest` / `DuplicateName` conflict. An unchanged brief is a no-op and its receipt record is carried verbatim (AC-17). Deactivate and rollback remove briefs through the same steps. |
| M5-11 | `status` and `reconcile` keep the six-state record shape (snapshots stay). A receipted brief that is missing or edited adds diagnostics `brief-missing` or `brief-modified` (info level for reconcile findings, no exit change). Receipted brief names are added to the `known` set of `list_foreign`, so a brief is never reported `foreign`. `reconcile --repair` rewrites a missing brief through `change::run`; an edited brief is left. |
| M5-12 | Human plan output: each brief is a line "generated  <slug>-brief  (bundle instruction ...)" with the instruction text shown under it, and, when any brief is written, one honesty line: briefs are discoverable, not guaranteed to be read (spec 11, PRD G12). `--json` plan steps carry `bundle`, `name`, `content_hash`, not the text. |
| M5-13 | `adapter_claims_have_evidence` hard-codes `refresh:none`, `refresh:command`, and `scan:nested`. Generalize: required keys are `destination`, `precedence` (only when the adapter has two or more scan locations), at least one `refresh:*` key, `strategy:*` per strategy, `scan:<scope>` per scan location, `consumers:<first word>` per consumer. `claude-code` keeps its entries; `agents-neutral` adds `refresh:unknown` and one `refresh:<consumer>` entry per consumer. Every entry still needs an `https://` URL or an existing spike file. |
| M5-14 | Real-library checks (S1, S6) are tests that self-skip when `UNAM_REAL_LIBRARY` is unset, not `#[ignore]` (gate G13c bans that text). They take the library path from the variable, register it in a temp `UNAM_CONFIG_DIR`, snapshot the library before and after with the existing `snapshot_source` helper (read-only proof), and print their measurements with `--nocapture`. `cargo run`-style shell commands are avoided because the repo hook blocks build-directory path segments. |
| M5-15 | A mechanical coverage test `phase1_ac_coverage` scans `crates/*/tests/*.rs` for a test named `ac_NN_` for every NN in 1 to 43 and 53 (needles built with `format!` so the test file does not match itself). The completeness report cites it; passing tests per AC come from the EVL run. |

---

## Blast Radius

- Core, new: `crates/unam-core/src/adapter/agents_neutral.rs`, `crates/unam-core/src/generated.rs`
- Core, edited: `adapter/mod.rs` (register, `Scope::Ancestor`, evidence test), `observe.rs` (shadow rank, brief observation, `list_foreign`), `plan.rs` (step variants, `diff` hook, `build_receipt`), `state.rs` (`JournalStep` ops), `fsops.rs` (two `FsOps` methods, `RealFs`), `apply.rs` (journal conversion, execute, resume, rollback arms), `change.rs` (generated input, warnings, `resolve_target` support), `verify.rs` and `reconcile.rs` (brief diagnostics), `doctor.rs` if the evidence listing needs the new scope, `lib.rs`
- CLI: `crates/unam-cli/src/cli/change.rs`, `cli/mod.rs` (bare `unam` ambiguity), `output.rs` (brief lines, followups, consumers sentence)
- Tests: new `ac_agents_neutral.rs`, `ac_briefs.rs`, `phase1_checks.rs`, `phase1_exit.rs`; edits in `common/mod.rs`, `common/faults.rs` (fault hooks for the two new `FsOps` methods), `snapshots.rs` plus new snapshots, M3 unit tests named in M5-3
- Docs (not protected): `docs/adapters.md` gets the `agents-neutral` rows (claims, evidence, Cursor editor and Windsurf not tested)
- Not touched: the five protected documents, `spike/`, `catalog/` (so no perf run), the skill library, this repository as a target (its `.agents/skills` is a toolkit symlink; every fixture uses temp directories)

Risk class: shared destination visible to several agents (PRD R3); new write path (briefs) through the frozen executor; journal step ops.

---

## Test Plan

### AC-to-Test Map

| Criterion | Test (file::name prefix) | Layer |
|---|---|---|
| AC-20 | `ac_agents_neutral.rs::ac_20_two_targets_hold_and_report_different_selections`: `add --target` to each, `status --target` per target, two receipts under `.unam/`, each destination holds its own set | CLI |
| AC-42 | `ac_agents_neutral.rs::ac_42_nested_project_warns_and_names_the_ancestor`: temp Git root with `.agents/skills/x`, project passed with `--project <root>/sub`; warning `ancestor-skills` names the ancestor path; a Git-root project gets none | CLI |
| AC-43 | `ac_agents_neutral.rs::ac_43_every_plan_names_the_four_consumers`: `add` plan (human and `--json` `visible_to`), `deactivate` plan, and the no-change plan | CLI |
| AC-24 | `ac_agents_neutral.rs::ac_24_agents_neutral_no_agent_config_file_is_created_or_modified`: planted `AGENTS.md`, `.codex/config.toml`, `.cursor/rules`, `.hermes`-style files are byte-identical after add and deactivate | CLI |
| AC-29 | `ac_agents_neutral.rs::ac_29_agents_neutral_says_undocumented_and_prints_the_hermes_followup`: output contains "not documented" and `hermes skills trust`; with no `~/.hermes` the followup is absent; `refresh.kind` is `unknown` | CLI |

### Other tests

| Test | Layer | Notes |
|---|---|---|
| `agents_neutral.rs` unit: `detect` rows, `scan_locations` (ancestor walk: Git root, below Git root, no Git root), `followups`, `refresh_note`, consumers verbatim | unit | Fixtures in temp dirs only |
| `adapter_claims_have_evidence`, `adapter_for_registry` (edited) | unit | Both adapters, M5-13 |
| `observe` shadow rank: user location does not shadow under `agents-neutral`, still shadows under `claude-code` | unit | M5-2 |
| `generated.rs` unit: content bytes (frontmatter via serializer, special characters in the instruction), hash, name from slug, opt-out rules, observation of Missing / Match / Edited | unit | M5-8 to M5-10 |
| `plan::diff` generated cases (new, replace, no-op carried verbatim, remove, modified kept, foreign refused, duplicate name) | unit | Property test not needed |
| `state` journal round trip for the two new step ops | unit | |
| `ac_briefs.rs::brief_written_on_add_and_listed_in_receipt`, `brief_unchanged_is_a_no_op_and_record_verbatim` (AC-17), `brief_opt_out_writes_none`, `brief_removed_on_deactivate_and_foreign_left` (AC-23), `brief_edited_is_kept_and_released`, `brief_foreign_name_refuses`, `brief_changed_instruction_rewrites`, `brief_never_reported_foreign`, `brief_missing_is_reported_and_repaired`, `brief_interruption_resume_and_rollback` | CLI + fault `FsOps` | Run for both targets where cheap |
| `cross_adapter_duplicate_warns_once_per_skill`, `destination_policy_isolated_warns` | CLI | M5-7 |
| `bare_unam_with_two_targets_exits_zero` | CLI | M5-3 |
| `phase1_checks.rs`: `no_network_dependencies`, `no_network_symbols`, `no_background_process`, `phase1_ac_coverage` | integration | M5-14, M5-15 |
| `phase1_exit.rs`: `s1_cold_to_verified_active_on_real_library`, `s6_catalog_cost_reduction_on_real_library` | Hybrid (self-skip) | Needs the real library |
| New snapshots: `plan_agents_neutral_json`, `status_agents_neutral_json`, `add_brief_json` | snapshot | Existing 36 snapshots must not change |

Every mutating test asserts both invariants (source tree hash unchanged; planted foreign file byte-identical) and calls `assert_zero_divergence` after apply or recovery (M4 helper).

TDD note: write the unit tests for `generated.rs`, the ancestor walk, and the diff cases red-first (a failing stub per Fully-Automated row), then the code. M3 and M4 recorded no red-first run; record whether M5 did.

---

## Implementation Checklist

Spec sections cited, not copied. Order is the compile order.

### Step A: Adapter, scope, registry (core)

- [x] A1. Add `Scope::Ancestor` and its `as_str` in `adapter/mod.rs`; fix every exhaustive `match` the compiler reports.
- [x] A2. Write `adapter/agents_neutral.rs` per spec 4.7 and decisions M5-1, M5-4 to M5-6, with the evidence table (below) and unit tests; register it in `adapters()`; update the M3 unit tests named in M5-3 and generalize `adapter_claims_have_evidence` (M5-13).
- [x] A3. `observe.rs`: apply the rank rule (M5-2); add the ancestor walk helper used by `scan_locations` (reuse the `.git` test of `project::detect_root`).

Evidence table to implement (every claim keyed; source is a URL from PRD section 16 or the spike):

| Claim key | Source | Status |
|---|---|---|
| `destination` (`.agents/skills`), `scan:project` | Codex https://learn.chatgpt.com/docs/build-skills.md and Cursor https://cursor.com/docs/skills | Verified |
| `scan:ancestor` (Codex reads every `.agents/skills` from cwd up to the repository root) | Codex docs URL | Verified |
| `scan:user` (`~/.agents/skills`) | Codex docs URL | Verified |
| `precedence` (project over user; ancestor order not documented) | https://agentskills.io/client-implementation/adding-skills-support.md | Verified (ancestor order Assumption, stated in the claim) |
| `strategy:symlink`, `strategy:copy` | `spike/symlink-discovery.md` (Codex 0.160.0, cursor-agent CLI 2026.10.01, Hermes v0.21.5; run 2026-10-05) | Verified |
| `refresh:unknown` and `refresh:codex` (automatic; restart fallback) | Codex docs URL and spike | Verified |
| `refresh:cursor` (not documented; the cursor-agent CLI needed a restart; Cursor editor not tested) | spike | Verified for the CLI only, claim names the CLI |
| `refresh:windsurf-devin` (not documented; Windsurf not tested) | https://docs.devin.ai/desktop/cascade/skills | Assumption (absence of documentation) |
| `refresh:hermes` (trust once, restart for mid-session additions) | Hermes docs https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md and spike | Verified |
| `consumers:codex`, `consumers:cursor`, `consumers:windsurf-devin`, `consumers:hermes` | the same URLs; Cursor claim cites the docs, not the editor, with the CLI spike as supporting evidence | Verified |

### Step B: Warnings, followups, refresh (core and CLI)

- [x] B1. `change::prepare`: ancestor, cross-adapter duplicate, and isolated-policy warnings (M5-7). Cross-adapter reads the other adapter's receipt through `Receipt::load` and treats an unreadable receipt as no duplicate (diagnostic already exists in `project::index`).
- [x] B2. `cli/change.rs` and `output.rs`: pass `adapter.followups(...)` into result views and `--json`; keep `claude-code` output byte-identical (its followups are empty); render the shared-destination sentence.
- [x] B3. `cli/mod.rs`: bare `unam` with an ambiguous target (M5-3).

### Step C: Generated briefs (core and CLI)

- [x] C1. `generated.rs`: spec struct, content builder, hash, observation, and the diff helper that returns steps and conflicts for M5-10.
- [x] C2. `state.rs`: the two `JournalStep` ops and the receipt value builder; `plan.rs`: two `Step` variants (all `n`, `set_n`, `name`, `op`, `to_json` arms), `DiffInput.generated`, the `diff` hook after Adds, `build_receipt` adds and drops generated entries.
- [x] C3. `fsops.rs`: `write_generated` (create the brief directory, atomic write of `SKILL.md`, both under a `DestPath`) and `remove_generated` (only when the directory holds exactly `SKILL.md` with the expected hash; else a skip), with `RealFs` and the test `FsOps` wrapper in `common/faults.rs`.
- [x] C4. `apply.rs`: `journal_step`, `step_from_journal`, `exec_step`, `resume_step`, and `rollback` arms (rollback: written brief is removed; removed brief is restored from the journal's hash only if its content is in the receipt copy, otherwise the receipt's prior state is restored without the file and the result says so).
- [x] C5. `change.rs`: build the generated inputs from the target selection's bundles (M5-9); `verify.rs` and `reconcile.rs` diagnostics and `list_foreign` known set (M5-11); `output.rs` brief lines and honesty sentence (M5-12).

### Step D: Tests (after the code they exercise)

- [x] D1. Unit tests named in the Test Plan, then `ac_agents_neutral.rs` and `ac_briefs.rs` through the existing `Fx` fixture (temp HOME so `~/.hermes` and `~/.agents` are fixtures; never this repository).
- [x] D2. `phase1_checks.rs` and `phase1_exit.rs`; new snapshots; confirm the 36 existing snapshots are unchanged.
- [x] D3. Run the full M1 to M4 suites; fix only regressions this change caused.

### Step E: Docs and Phase 1 evidence

- [x] E1. `docs/adapters.md`: add the `agents-neutral` rows (claims, URLs, spike reference, "Cursor editor and Windsurf not tested", the M5-1 stop rule); keep the two M2 sections and the M4 sections.
- [x] E2. Run `s1_*` and `s6_*` once on Taylor's real library (`UNAM_REAL_LIBRARY=$HOME/Developer/01-AI/skillzone`, release build, `--nocapture`), read-only; record the five S1 times, the median, and both S6 numbers. If the library has fewer than 40 skills, still record the numbers and flag S6 as a library-size limit, not a failure of the mechanism.
- [x] E3. Read `cargo tree -p unam --edges normal` once and record that no HTTP or socket client appears.
- [x] E4. Write the phase report with the "Product Phase 1 completeness" section (below) and the S5 follow-up.

### Step F: Stop

- [x] F1. Commit execution changes on `main` (code first, then process); no push. Update the umbrella `## Current Execution State` to "Product Phase 1 Gate: AWAITING TAYLOR". Do NOT fill the gate record. Stop.

---

## Product Phase 1 Completion: how it is checked and reported

1. **Mechanical (agents):** `phase1_ac_coverage` proves a named test exists for AC-1 to AC-43 and AC-53; the EVL run (independent `vc-tester`) of `cargo test --workspace --locked` proves each passes. The report lists all 44 criteria in one table: criterion, milestone that closed it (AC-2 in M3, AC-33 in M4, AC-20/42/43 in M5), test name, EVL result. Any criterion not green is flagged at the top of the report, not buried.
2. **Success targets:** S1 and S6 numbers (E2), S2 to S4, S7 to S9 from their existing tests, network none and background none (E3 and the two tests), S5 as a follow-up four weeks after daily use begins.
3. **Taylor (not an agent):** the daily-use half, and the first adopting project plus the D4 real activation (backlog NOTE `m3-d4-first-real-activation_NOTE_05-10-26.md`). Linux and CI have never run; the first push shows Linux behavior. The report states this as open, not as passed.
4. **Reporting:** the report ends with a "For Taylor" block: what is verified, what is unverified, what he must do to confirm. The orchestrator records `Product Phase 1 Gate: AWAITING TAYLOR` and stops. Only Taylor's explicit message that product Phase 1 is done and M6 may start clears the gate; no agent writes the gate record.

---

## Exit Gate

```bash
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check                                          # G1 Expected: exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings   # G2 Expected: exit 0 (no unwrap in non-test code)
cargo test --workspace --locked                                  # G3 Expected: all pass, including every test named in the Test Plan
```

| Gate | Check |
|---|---|
| G1 to G3 | The three commands above |
| G4 | Perf: `cargo test --release -p unam-core --test perf_cold_scan` only if anything under `crates/unam-core/src/catalog/` changed (not expected); otherwise "not applicable" |
| G5 | Presence: `ac_20_`, `ac_42_`, `ac_43_`, `ac_24_agents_neutral_`, `ac_29_agents_neutral_` each match at least one test; `adapter_claims_have_evidence`, `no_network_dependencies`, `no_background_process`, `phase1_ac_coverage` run (a filter matching zero tests fails the gate); `! grep -rEq '#\[ignore' crates` (G13c) |
| G6 | Invariants and protected docs: both invariants asserted in every new mutating test; the source library snapshot unchanged around S1 and S6; `git diff --exit-code 8d688e1 -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0 |

- All checklist items checked; AC-20, 42, 43, 24 and 29 (second adapter) tests green; brief tests green; the 36 existing snapshots unchanged.
- Regression: M1 to M4 criteria tests green; `claude-code` human and JSON output unchanged except where a planned test edit says so.
- Phase 1 evidence recorded (Step E); phase report written to the report destination.
- FINAL ITEM, HARD STOP (Step F1): the program STOPS here. Do not start research, plan refresh, or any step of milestone M6 or M7. Never auto-proceed under /goal.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- A VERIFIED M5 does NOT mean product Phase 1 is done. That is a separate condition confirmed only by Taylor.
- Commit execution changes on `main` before the stop; no push.

---

## Blockers That Would Justify BLOCKED Status

- M4 not VERIFIED (it is) or the M0 gate not met.
- A fixture that would need to write to this repository's `.agents/skills`, or a test that would invoke a real agent CLI: stop (hard safety constraint).
- The brief mechanism cannot be added without a new journal RECORD type (not just new step ops): stop and ask, since "no new record types" is a Taylor-approved rule.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH (2026-10-12): combined pass (lighter process); M4 report and Forward Preview read; test context loaded; adapter, plan, apply, state, observe, change, verify, CLI surfaces read
- [x] 2. INNOVATE (2026-10-12): folded into this pass; design choices recorded as Decisions M5-1 to M5-15 (no separate Decision Summary)
- [x] 3. PLAN-SUPPLEMENT (2026-10-12): existing phase plan rewritten into an executable plan; Inner Loop Refresh Note written
- [x] 4. PVL (2026-10-12, cycle 1 of at most 2): Gate PASS, 0 FAIL-class gaps, execute-time notes EX1 to EX14 in the Validate Contract. Was: vc-validate-agent: at most 2 cycles; only FAIL-class gaps (wrong requirement mapping, steps that cannot compile in order) trigger a cycle; nits go to execute time; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md`
- [x] 5. EXECUTE (2026-10-17): checklist A1 to E4 done, F1 done at the commit step; G1 to G3 green (642 tests), G4 to G6 recorded (mode-only diff note in the report); phase report written; EVL (independent vc-tester) not yet run
- [x] 6. EVL (2026-10-17): independent vc-tester re-run, 2 cycles (cycle 1 found the replace/FailBefore(7)/resume brief-loss bug, fixed in 5ba6888; cycle 2 all green, 643 tests, full fault matrix); no follow-up stubs; EVL HANDOFF SUMMARY CLEAN
- [x] 7. UPDATE PROCESS (2026-10-17): phase report completed (EVL outcome, D11, review findings R1 to R4), umbrella state updated, context docs updated, committed; HARD STOP at the Product Phase 1 Gate (AWAITING TAYLOR)

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Inner Loop Refresh Note: 2026-10-12 - changed sections: header (Status, Execution strategy), Overview, Scope, Acceptance Criteria (AC-29 second half added, briefs row, success checks made concrete), Entry Gate, Decisions (new), Blast Radius (adds `generated.rs`, `fsops.rs`, `state.rs`, `docs/adapters.md`, new test files), Test Plan (new), Implementation Checklist (rewritten in compile order with the evidence table), Product Phase 1 Completion (new), Exit Gate (six gates, `--locked`), Phase Loop Progress, Touchpoints, Public Contracts, Verification Evidence, Resume and Execution Handoff. Research found: no `WriteGenerated` or brief code exists anywhere (only the bundle metadata parse), so the brief needs new plan steps, journal step ops, and `FsOps` methods; `observe` shadows ignore rank, which would make `~/.agents/skills` wrongly shadow project entries for `agents-neutral`; `Scope` has no ancestor value; `adapter_claims_have_evidence` hard-codes claude-code refresh and nested keys; followups are never passed to the CLI result views; two M3 unit tests assert `agents-neutral` is unsupported; AC-29 second half is owned by M5 per the M4 report; the stub's "config key per spec 2.1" does not exist; gate G13c bans `#[ignore]`, so the real-library checks self-skip instead. No validate-contract existed to invalidate.

---

## Touchpoints

- Files listed in Blast Radius.
- Read only: spec 2.2, 2.4, 4.5, 4.7, 4.8, 4.10, 5, 8, 9, 10, 11; PRD 0 (D3), 5, 7.6, 7.9, 10, 12, 13.1, 13.2, 16; `spike/symlink-discovery.md` Outcome; M4 report Forward Preview; decision sheet; WAVES file.
- Umbrella `## Product Phase 1 Gate` and `## Current Execution State` (updated at UPDATE PROCESS only).

---

## Public Contracts

- Adapter id `agents-neutral`, destination `.agents/skills`, consumers `codex, cursor, windsurf-devin, hermes`, default strategy `symlink`, `refresh.kind` `unknown`.
- Warning codes `ancestor-skills`, `cross-adapter-duplicate`, `destination-policy-isolated`, `brief-missing`, `brief-modified`; followup text `hermes skills trust <root>`.
- Receipt entry `kind: generated` (spec 2.4); journal step ops `write_generated`, `remove_generated`; plan step ops of the same names in `--json`.
- Per-bundle opt-out `metadata.unambiguous.materialize_instruction`; no new config key.
- Library API consumed by M6: `adapter::adapters()` now returns both adapters; `Scope::Ancestor`.
- Unchanged: the six-state record shape, exit codes, journal record types, `Adapter` trait.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` | Fully-Automated | spec 8 gates |
| `ac_20_*` two targets, different selections | Fully-Automated | AC-20 |
| `ac_42_*` ancestor warning names the ancestor | Fully-Automated | AC-42 |
| `ac_43_*` four consumers on every `agents-neutral` plan | Fully-Automated | AC-43 |
| `ac_24_agents_neutral_*` no agent config file touched | Fully-Automated | AC-24 (second adapter) |
| `ac_29_agents_neutral_*` undocumented refresh, Hermes followup | Fully-Automated | AC-29 (second half) |
| `brief_*` write, no-op, opt-out, remove, edited kept, foreign refused, interruption | Fully-Automated (test `FsOps`) | spec 4.8 step 7; AC-17, AC-23 for generated entries |
| `adapter_claims_have_evidence` over both adapters | Fully-Automated | PRD S9 |
| `no_network_dependencies`, `no_network_symbols`, `no_background_process` | Fully-Automated | spec 9 network none, background none |
| `phase1_ac_coverage` plus the EVL run | Fully-Automated | PRD 12 done condition, agent half (AC-1 to AC-43, AC-53 named and green) |
| `s1_*` and `s6_*` on Taylor's real library (read-only, temp destination) | Hybrid (precondition: `UNAM_REAL_LIBRARY` set; once, at the Phase 1 exit) | PRD S1, S6 |
| Hermes followup, refresh text, and brief honesty line read for overclaim | Agent-Probe (reviewer judgment against PRD G12) | PRD G12; AC-28 and AC-29 consistency |
| Completeness report cross-checked against spec 10 (AC list per milestone) | Agent-Probe | PRD 12 done condition (evidence for the gate) |

No Known-Gap rows. Residual gaps are named, not hidden: daily use, D4, Linux and CI, and the Cursor editor and Windsurf (never tested) are Taylor's or untested, and the report says so.

---

## Test Infra Improvement Notes

- `common/faults.rs` needs hooks for `write_generated` and `remove_generated`; the strided fault matrix (M4) should include the brief steps once they exist.
- `Fx` needs a helper that builds a Git-root fixture with an ancestor `.agents/skills` (a `.git` directory created by Rust code, not by shell).
- No `pty` test for the interactive prompt (R8, accepted).

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-06-m5-agents-neutral-and-briefs_PLAN_04-10-26.md`
- Last completed step: 5 EXECUTE (2026-10-17, one sonnet execute agent; gates G1 to G3 green, 642 tests)
- Validate-contract status: written 2026-10-12 (inner-pvl: phase-6, PASS, cycle 1; honor G5 presence checks and EX1 to EX14)
- Supporting context files: `process/context/all-context.md` (Threat Model, standing rules), `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, `phase-05-m4-verify-status-reconcile-rollback_REPORT_12-10-26.md` (Forward Preview), `open-items-decision-sheet_REF_09-10-26.md`, `v1-static-activation_WAVES_09-10-26.md`, `spike/symlink-discovery.md`, `docs/adapters.md`
- Next step: EVL (independent vc-tester re-runs G1 to G6), UPDATE PROCESS, then STOP at the product Phase 1 gate: no research, plan, or execution for M6 or M7 until Taylor clears it.
- Items that stay with Taylor: D4 first real activation and naming the adopting project (row 54); the seven decision-sheet rows he has not marked; Linux and CI; the product Phase 1 gate and daily-use confirmation; the S5 four-week follow-up; the three REPORT TO TAYLOR interpretations (M5-1 stop rule, M5-8 journal step ops, M5-9 no config key).

---

## Validate Contract

Status: PASS
Date: 12-10-26
date: 2026-10-12
generated-by: inner-pvl: phase-6

Parallel strategy: sequential
Rationale: score 4/7 (S2 new journal step ops, receipt `generated` entries, and CLI JSON; S4 phase program; S6 shared destination plus a new write path through the executor; S7 more than 5 files; no S1 because only two crates, no S3, no S5). One compile chain (adapter, observe, generated, plan, state, fsops, apply, change, CLI, tests), so no disjoint file group exists for a team and no independent direction exists for parallel subagents. EXECUTE: one sonnet execute agent (Taylor, 2026-10-09: all agents sonnet), then an independent sonnet vc-tester EVL run. Agent counts: sequential 1; parallel subagents 4 + 7 + 3 = 14 with nothing to split; agent team 3 x 2 = 6, no benefit; workflow 7 x 1 x 1 = 7, not warranted. Cost guard not triggered. Cycle 1 of at most 2 (light lane, WAVES file): worked inline by the validate agent, no fan-out, no helper agent, no `unam` run, no agent CLI, skill library untouched. Only FAIL-class gaps were counted. Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so none is written here. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Requirement mapping check (spec 10 M5, 4.7, 4.8 steps 7 and 9; PRD D3, 7.6, 7.9, 10.2, 10.3, 13.2, Q5): every M5 criterion maps to a named test. AC-20, 42, 43, 24 (second adapter) are the spec 10 list; AC-29 second half is added from the M4 report; AC-17 and AC-23 are carried to generated entries; OWN-5, ACT-13, EDG-11, SES-1, SES-4 are covered by the same tests. No M5 criterion is missed and no decision contradicts the PRD or spec. The tensions that need reporting are EX10 and the three REPORT TO TAYLOR items already in the plan.

Reuse claims checked against the real code (all hold):

| Claim | Evidence read | Verdict |
|---|---|---|
| New step ops are step ops, not a new journal record type | `JournalStep` is the element of `Begin.plan` (state.rs, tag `op`); `Record` (tag `t`) is untouched; `validate_begin` only checks step number, `step.name()` via `validate_dest_name`, and `RemoveDir` prefixes, so two new variants with a `name` pass; ops serialize as `write_generated` and `remove_generated` | Holds |
| Registry flip is safe | `resolve_target` picks the first Present adapter in registry order, else `claude-code`; `agents-neutral` detect is limited to the spec list, so temp-HOME fixtures still default to `claude-code` | Holds |
| Observe rank rule | `observe` is the only producer of `Shadow`; `plan::diff` and `verify` only consume it; claude-code ranks 0, 1, 2 keep enterprise and user shadowing; agents-neutral project 0 and user 2 never shadows | Holds |
| Generated receipt entry | `Receipt::from_value` already routes any non-`skill` kind to `Entry::Other`; `build_receipt` starts from `before.entries` so an unchanged brief is carried verbatim; `to_value` re-emits `Other` unchanged | Holds |
| Bundle inputs | `BundleMetadata.materialize_instruction: Option<bool>` and `instruction: Option<String>` (blank gives `None`) are already parsed | Holds |
| Followups never reach the CLI | `cli/change.rs` passes `followups: &[]` at four sites and `json!([])` once; B2 is accurate | Holds |
| Network and background tests can pass today | no `.spawn(` and no `std::net` in `crates/*/src`; `Command::new(..).status()` only in `bundle/crud.rs`; `Cargo.lock` holds no HTTP, TLS, or socket crate (only `libc`) | Holds |
| `phase1_ac_coverage` can pass after M5 | grep of `fn ac_NN_` over `crates`: every NN in 1 to 43 and 53 exists except 20, 42, 43 once the needle is zero padded (EX4) | Holds with EX4 |
| Evidence URLs | Codex, Cursor, Devin, Hermes, and agentskills.io URLs in the evidence table are all in PRD section 16; Cursor and Hermes spike rows exist in `spike/symlink-discovery.md` | Holds |
| Hard stop present | Exit Gate final item, F1, Blockers, and the umbrella `## Product Phase 1 Gate` (record NOT CONFIRMED, no agent writes it) | Holds |

Compile order A to F traced: A1 (`Scope::Ancestor`) before A2 (adapter file and registry; the three M3-era tests that assert "unsupported" must be edited here, EX2); A3 (observe) after A1; B1 uses `Scope::Ancestor` and the registry; B2 and B3 touch CLI only; C1 (`generated.rs`) before C2 (`state.rs` ops, `plan.rs` steps and `DiffInput.generated`); C3 (`FsOps` methods) before C4 (apply arms); C5 last. No forward reference. Structural validator: 0 failures, 0 warnings.

Gates (hook-safe: every command text below was fed to `scout-block.cjs`, every one exit 0):
- G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets --locked -- -D warnings`; G3 `INSTA_UPDATE=no cargo test --workspace --locked` (set `PATH` and `INSTA_UPDATE` first, as in the plan's Exit Gate block).
- G4 perf, only if `git diff --name-only 8d688e1 -- crates/unam-core/src/catalog` prints anything (none expected); otherwise record "not applicable".
- G5 presence plus snapshot and ignore checks (G3 alone stays green if a named test is never written). Run `cargo test --workspace --locked -- --list` and require: (a) each of `(^|::)ac_20_`, `(^|::)ac_42_`, `(^|::)ac_43_`, `(^|::)ac_24_agents_neutral_`, `(^|::)ac_29_agents_neutral_` matches; (b) `adapter_claims_have_evidence`, `no_network_dependencies`, `no_network_symbols`, `no_background_process`, `phase1_ac_coverage`, `s1_cold_to_verified_active_on_real_library`, `s6_catalog_cost_reduction_on_real_library` are listed; (c) at least 10 names match `(^|::)brief_[a-z_]+: test$`; (d) `! grep -rEq '#\[ignore' crates`; (e) `git diff --diff-filter=MD --exit-code 8d688e1 -- crates/unam-cli/tests/snapshots` exits 0 (no existing snapshot modified or deleted; the three new snapshots are additions).
- G6 invariants and protected docs: every new mutating test ends with `fx.done()` or `done_at` (source tree hash unchanged, planted foreign file byte-identical) and calls the target-aware zero-divergence helper after apply or recovery; `git diff --exit-code 8d688e1 -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0 (this form compares the base to the working tree, so staged and unstaged edits both count).

AC-to-test map check (plan Test Plan against PRD 13.2 text):

| Criterion | PRD text | Plan test | Verdict |
|---|---|---|---|
| AC-20 | two targets in one project hold and report different selections | `ac_20_two_targets_hold_and_report_different_selections` (`add --target` each, `status --target` each, two receipts) | Maps |
| AC-42 | nested project under an ancestor-scanning target warns and names the ancestor | `ac_42_nested_project_warns_and_names_the_ancestor` (plus a Git-root project gets none) | Maps |
| AC-43 | every plan against `agents-neutral` names the four consumers | `ac_43_every_plan_names_the_four_consumers` (add, deactivate, no-change; human and `--json`) | Maps |
| AC-24 (second adapter) | no agent config file created or modified; manual steps printed | `ac_24_agents_neutral_no_agent_config_file_is_created_or_modified` | Maps |
| AC-29 (second half) | undocumented target says it is undocumented | `ac_29_agents_neutral_says_undocumented_and_prints_the_hermes_followup` | Maps |
| AC-17, AC-23 for generated entries | untouched record verbatim; foreign left | `brief_unchanged_is_a_no_op_and_record_verbatim`, `brief_removed_on_deactivate_and_foreign_left` | Maps |
| AC-1 to 43, 53 (agent half of PRD 12) | each criterion has a named passing test | `phase1_ac_coverage` plus the EVL run | Maps with EX4 |

Test gates (C3 table; legacy form is the Verification Evidence table of this plan):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 gates | fmt, clippy, workspace tests green | Fully-Automated | G1, G2, G3 | B |
| AC-20 | two targets hold and report different selections | Fully-Automated | `ac_20_two_targets_hold_and_report_different_selections` | B |
| AC-42 | nested project warns and names the ancestor; Git-root project gets no warning | Fully-Automated | `ac_42_*` plus the `agents_neutral` ancestor-walk unit rows | B |
| AC-43 | every plan and result names codex, cursor, windsurf-devin, hermes (human and `--json`) | Fully-Automated | `ac_43_every_plan_names_the_four_consumers` | B |
| AC-24 | no agent config file created or modified; manual steps printed | Fully-Automated | `ac_24_agents_neutral_*` | B |
| AC-29 | undocumented refresh is stated; never "no restart required"; Hermes followup printed only, only when `~/.hermes` exists | Fully-Automated | `ac_29_agents_neutral_*` | B |
| spec 4.8 step 7, AC-17, AC-23 | brief written, unchanged is a no-op, opt-out writes none, remove, edited kept, foreign refused, rewrite on change, never reported foreign, missing repaired, interruption resume and rollback | Fully-Automated (test `FsOps`) | the ten `brief_*` tests plus `generated.rs` and `plan::diff` unit rows | B |
| M5-2 | user scope never shadows under `agents-neutral`; still shadows under `claude-code` | Fully-Automated | `observe` unit rows | B |
| M5-3, M5-7 | bare `unam` with two targets exits 0; cross-adapter duplicate once per skill; isolated policy warns | Fully-Automated | `bare_unam_with_two_targets_exits_zero`, `cross_adapter_duplicate_warns_once_per_skill`, `destination_policy_isolated_warns` | B |
| S9 | no claim without a URL or spike result, both adapters | Fully-Automated | `adapter_claims_have_evidence`, `adapter_for_registry` | B |
| network none, background none | no network crate or symbol; no process outlives a command | Fully-Automated | `no_network_dependencies`, `no_network_symbols`, `no_background_process` | B |
| PRD 12, agent half | a named test exists for AC-1 to 43 and 53 | Fully-Automated | `phase1_ac_coverage` plus the EVL run | B |
| S1, S6 | cold-to-verified under 60 s (median under 30 s); selection estimate at most 10 percent of the library | Hybrid (precondition: `UNAM_REAL_LIBRARY` set; run once at the Phase 1 exit; self-skips otherwise) | `s1_*`, `s6_*` in `phase1_exit.rs` | D (named residual) |
| PRD G12, AC-28 consistency | Hermes followup, refresh text, and brief honesty line do not overclaim | Agent-Probe | EVL reviewer reads the human output against PRD G12 | A |
| PRD 12 completeness | the completeness report matches spec 10 AC lists per milestone | Agent-Probe | EVL reviewer cross-checks the report | A |

Failing stubs: each named test is written red-first by the execute agent from the Scenario text in the AC-to-Test Map; per-row stub blocks are omitted under the lighter process. Record whether the red-first run happened (the plan's TDD note asks for it).

Dimension findings:
- Infra fit: PASS - one compile chain on the frozen M3 and M4 executor; every reuse claim above holds; the extra `FsOps` implementors (EX3) and the fixture collision (EX1) are execute-time work, not structure.
- Test coverage: PASS - every M5 criterion and every brief rule maps to a named test; G5 presence closes the vacuous-green path; no developed behavior rests on a Known-Gap; two Hybrid or Agent-Probe residuals named.
- Breaking changes: PASS - the new ops are step ops inside the `begin` record, not record types; the receipt shape is already tolerated; three existing tests assert "unsupported" (EX2); claude-code output stays byte-identical once EX1 holds.
- Security surface: PASS - brief writes go only to a validated child name of the destination with a fixed `SKILL.md` name; removal only of a directory holding exactly `SKILL.md` with the expected hash; journal content is hash-checked on replay (EX9); no `SourcePath` reaches a write function; no agent CLI is invoked and the Hermes followup is text; nothing beyond the threat model is demanded.
- Sections A (adapter, scope, registry), B (warnings, followups), C (briefs), D (tests), E (docs and Phase 1 evidence), F (stop): PASS each; 0 FAILs / 0 CONCERNs.

Execute-time notes (nits, binding on the execute agent, not gaps; list each applied or deviated in the phase report):
- EX1 Fixture collision with default-on briefs (do this before any other test work). `fixtures/bundles/hermes-only.yaml` carries an `instruction` and no opt-out, and about 17 existing call sites activate it against `claude-code` (`ac_activation`, `ac_reconcile`, `ac_recovery`, `activation_cli`, `snapshots`, through `Fx::new_with` and `snapshots.rs`). Under M5-9 each would gain a `hermes-only-brief` directory, receipt entry, and plan step, and the snapshots `use_replace`, `status_json`, and `reconcile_json` would change, which breaks the gate "36 existing snapshots unchanged". Keep the fixture file byte-identical (AC-07 reads it) and add a test helper that copies it and appends `metadata: unambiguous: materialize_instruction: false` for the suites that apply; only the new brief tests use the unmodified copy. If any existing snapshot still has to change, stop and report; do not accept it. The `commented.yaml` fixture is only used by bundle commands and needs nothing.
- EX2 Three existing assertions that `agents-neutral` is unsupported, not two: `adapter::tests::adapter_for_registry`, `change::tests::resolve_target_branches` (two asserts), and `activation_cli.rs::unsupported_agent_flag_value_exit_2` (`--target agents-neutral add` now succeeds; rewrite it as a plan or success test, keep `invalid_agent_flag_value_exit_2`, which uses `cursor`). The `TargetUnsupported` variant stays (variant-table test).
- EX3 `FsOps` has eight implementors, not two: `RealFs`, `common/faults.rs` `FailingFs`, and the unit-test impls `Faulty`, `RecordingFs`, `FinalCrash`, `CrashOnRmdir` (`apply/tests.rs`), `LogFs`, `Crash` (`change/tests.rs`), and `FailRemove` (`state/tests.rs`). Adding required methods breaks all of them; add delegating methods to each (`FailingFs` counts them). Run `cargo test --workspace --no-run --locked` right after C3.
- EX4 `phase1_ac_coverage` needle: existing tests for AC-1 to 9 use zero-padded names (`ac_01_` to `ac_09_`). Build needles with `format!("fn ac_{n:02}_")` for every NN in 1 to 43 and 53. Before the new files exist only 20, 42, 43 are missing (verified); the test must fail first (red) and pass at the end.
- EX5 `RemoveGenerated` and rollback: the receipt and the journal hold only a hash, so a delete-only step cannot be undone. Make `remove_generated` take the op trash and move the brief directory aside (rename), and make rollback rename it back (exact restore) and resume treat "gone but in trash" as done, as `Remove` does. The C4 fallback ("restored without the file, and the result says so") applies only when the trash copy is missing. This keeps the deactivate-interruption scenario of `brief_interruption_resume_and_rollback` consistent.
- EX6 Directory creation and release: `plan::diff` sets `dest_dirs_to_create` only when a step is an `Add`; include `WriteGenerated`, and call `ensure_dirs` in the `WriteGenerated` exec and resume arms (a brief can be the only new entry). When counting `remaining_valid` for the `RemoveDir` decision on deactivate, count receipted generated entries that stay, so a created directory is not planned for removal while a brief remains.
- EX7 Removals and rendering: `is_removal` and `Plan::removes_something` must count `RemoveGenerated` so a brief-only removal still prints the SES-2 caveat (AC-30); `Step::id()` is `None` for both new variants; `output.rs` needs arms in `render_plan` and in its step tests.
- EX8 Test helpers hard-code `claude-code`: `common/faults.rs` `assert_zero_divergence` uses `TargetId::ClaudeCode` and `assert_ordering_invariant` uses `.unam/claude-code`. Add a target parameter (keep the old names delegating) before the `agents-neutral` recovery tests. A missing receipted brief is a finding, not a divergence (M5-11), so `assert_zero_divergence` alone does not catch it: the brief tests also assert the brief file and the receipt entry directly.
- EX9 Journal replay: `WriteGenerated` in the journal carries the content; on resume verify `blake3(content) == content_hash` before writing, write only when the child is `Missing`, treat an equal-hash directory as done (`observed`), and otherwise skip as `DestOccupied`, mirroring `place`. Use `join_child` for the name; the file name is always `SKILL.md`.
- EX10 REPORT TO TAYLOR (standing rule: report document tension, do not resolve silently): M5-9 reads an absent `materialize_instruction` as on (PRD 5.3 "opt-out per bundle"), while spec 4.8 step 7 says "with `materialize_instruction` true". Add it to the "For Taylor" block with M5-1, M5-8, and the no-config-key reading.
- EX11 Cross-adapter duplicate: one warning per skill name (spec 4.7 "reported once"), whichever target is being changed; an unreadable other receipt means no duplicate.
- EX12 Real-library and process hygiene: `s1_*` and `s6_*` read the path only from `UNAM_REAL_LIBRARY`, use the isolated `TestEnv` variables (temp HOME, config, cache) with the project under the system temp directory, and never write to the library (snapshot before and after). An S6 whole-library `add` may be blocked by duplicate names (exit 6); the `--json` output still carries `plan.estimate`, so read it there and never pass `--yes`. No new file may contain the literal `#[ignore` (gate G13c). The Hermes followup test creates an empty `~/.hermes` in the temp HOME; no agent CLI is run or looked up.
- EX13 `observe` rank rule: find the Project location's rank from `scan_locations`; with no Project location (tests pass `&[]`) nothing shadows. Keep `shadows_from_enterprise_and_user_only` as is and add the `agents-neutral` case beside it.
- EX14 New snapshots without `cargo-insta`: follow the no-cargo-insta procedure in the M1 plan (write the three new `.snap` files once with `INSTA_UPDATE=always`, read them, then gate with `INSTA_UPDATE=no`). G5(e) proves no existing snapshot changed.

Open gaps: none FAIL-class. Named residuals (not proving): Linux and CI unrun (known M3 gap); no pty test (R8); strided fault sample unless `UNAM_FULL_FAULT_MATRIX=1`; S1 and S6 run once on one machine and self-skip elsewhere; Cursor editor and Windsurf never tested; daily use, D4 real activation, and S5 are Taylor's.
What This Coverage Does NOT Prove:
- G1 to G3: nothing about Linux, CI, or a real terminal prompt; confirmation paths use `--yes` and the no-TTY refusal only.
- AC-43, AC-29, AC-24: output text and file effects only, never that Codex, Cursor, Windsurf, Devin, or Hermes discovered a skill; the Cursor claims rest on the cursor-agent CLI spike and the editor is untested.
- Briefs: that a brief is discoverable by a real agent or ever read (PRD G12); content is checked with unam's own frontmatter parser only.
- Ancestor warning: the stop-at-nearest-Git-root rule is a reading (M5-1); Codex ancestor precedence is undocumented.
- S1 and S6: one machine, one library, a fresh cache directory (not a cold OS page cache); S6 depends on library size.
- Network none and background none: a lockfile denylist, a source symbol scan, and a process sample after commands; not a runtime trace.
- Brief interruption: the strided sample of injected failures unless the full matrix variable is set.
- Product Phase 1 done: daily use, the first adopting project, D4, S5, and Linux are Taylor's and are reported as open.
Net result: PASS (0 FAIL-class gaps, 0 CONCERNs; no developed behavior rests on a Known-Gap alone)
Gate: PASS
Accepted by: not required (no CONCERN remains); the named residuals above are recorded as known-gaps. EXECUTE is legal: honor G5 presence checks and EX1 to EX14. The program STOPS at the product Phase 1 gate after UPDATE PROCESS.

---

## Deviations

Written at EXECUTE (2026-10-17). All are within the validate-contract blast radius; none changes a public contract. Full list with reasons: the phase report, `## Plan Deviations`.

- Edited receipted brief that is still wanted: the conflict is `modified-at-dest` (the skill rule), not `foreign-at-dest` as M5-10 words it; `foreign-at-dest` is kept for a name occupied by something unam never wrote. Both block with exit 6.
- A changed brief is two steps (`remove_generated` then `write_generated`), per EX5, so a rollback can restore the old bytes exactly from the operation trash.
- The new warnings (`ancestor-skills`, `cross-adapter-duplicate`, `destination-policy-isolated`) are in `plan.warnings` (the plan text and `data.plan.warnings`), not the envelope `warnings`; `brief-missing` and `brief-modified` are in the envelope `warnings` (severity warning) and, for `reconcile`, also `findings`.
- `--json` `receipt.entries` now counts every receipt entry (skills and briefs); for `claude-code` without briefs the number is unchanged.

---

## Next Step

EXECUTE is done (2026-10-17). Run the independent EVL confirmation run (vc-tester) with the validate-contract gates, then UPDATE PROCESS. Stop at the product Phase 1 gate afterwards.
