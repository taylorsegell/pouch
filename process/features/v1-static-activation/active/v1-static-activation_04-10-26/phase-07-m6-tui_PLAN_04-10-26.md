---
name: plan:v1-static-activation-phase-07-m6-tui
description: "v1: static activation, phase 7 of 8: milestone M6, TUI picker (first part of product Phase 2); gate cleared 2026-10-17; light-lane plan supplemented 2026-10-17 (termion backend)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-07
---

# Milestone M6: TUI

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-17; light lane per `v1-static-activation_WAVES_09-10-26.md`: plan about 400 lines, PVL at most 2 cycles, only FAIL-class gaps count)
**Complexity**: COMPLEX
**Phase:** Phase 7 of 8: milestone M6
**Status**: VERIFIED (WITH_GAPS) 2026-10-17 (EXECUTED 2026-10-17; EVL PASS in 2 iterations, 699 tests; UPDATE PROCESS done; Taylor's manual AC-44 real-terminal run outstanding)
**Phase status:** VERIFIED (WITH_GAPS); manual AC-44 run by Taylor outstanding (backlog stub); product Phase 1 gate cleared by Taylor 2026-10-17
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none. Feasibility VERDICT (cited, read it): `phase-07-m6-tui_FEASIBILITY_17-10-26.md` (VIABLE with the termion backend).
**Execution strategy:** sequential, one sonnet execute agent (Taylor 2026-10-09: all agents sonnet; this overrides the umbrella's "opus EXECUTE" line for M6). One compile chain (deps -> `tui` modules -> routing -> tests). INNOVATE skipped: Taylor and the feasibility probe resolved every design choice (Decisions D-TERM, D-EKEY, D-TEST, D-LANE below).

---

## Overview

M6 adds the interactive picker so the primary scenario is one interactive step: in bare `unam`, type a query, press space on a result, press enter twice, and the skill is active (AC-44; PRD UX-2, UX-3, S10 at four or fewer keystrokes; spec 6, 10 M6). The TUI is a renderer over `unam-core`: it calls the same core functions as the CLI and never writes files itself. Estimated 3 days (spec 10).

Threat model (standing): same user, trusted local filesystem. Never run `unam` against this repository; temp dirs only; the skill library is read-only; no agent CLIs; no em dashes or emojis; protected docs untouched; no license file; commit on `main`, no push.

---

## Scope

- Picker (three panes), plan view, result view, `Interrupted` modal, state machine `Loading, Picker, Plan, Applying, Result, Picker` (spec 6).
- Bare `unam` on a TTY opens the picker; non-TTY keeps usage plus status table, exit 0 (`cli/mod.rs` bare branch, `usage` tests, `usage_bare` snapshot).
- Parity: every TUI action maps to a named core function that the CLI also calls.
- Exit-gate perf checks: warm start under 50 ms on a 500-skill catalog; peak memory under 50 MB (spec 9, 734, 738).
- Not in scope: strategy toggle `e` (D-EKEY), match highlighting (M6-2), composing mixed add+remove in one apply (M6-5), anything of M7.

---

## Acceptance Criteria

Closes (spec 10 M6): AC-44 (Phase 2). PRD 13.2: AC-44 sits under "Phase 2"; `phase1_ac_coverage` lists AC-1 to AC-43 and AC-53 explicitly and must not gain AC-44.

| Criterion | Proven by | Strategy |
|---|---|---|
| AC-44 (UX-2, UX-3, S10) | `ac_44_*` in-crate (synthetic key events, `TestBackend`, temp project; asserts destination symlink exists, `verify` reports Active, source hash and foreign file unchanged, key count at most 4 after the query); plus one manual real-terminal run by Taylor recorded in the report (D-TEST) | Hybrid (never Known-Gap) |
| TUI parity (spec 10 M6) | `tui_parity_*` (mechanical: `Action::ALL` enum, each mapped to a named core function that the CLI source also calls) | Fully-Automated |
| Spec 6 layout, markers, plan and result views, Interrupted modal | `tui_snapshot_*` | Fully-Automated |
| Bare `unam` non-TTY unchanged | `bare_unam_non_tty_*` in `tests/usage.rs` plus existing `usage_bare` snapshot | Fully-Automated |
| Warm start under 50 ms (spec 9) | `perf_warm_start_picker` | Hybrid (release build, local SSD; assertion automated) |
| Memory under 50 MB (spec 9) | `perf_memory_picker` | Hybrid (release build, `/usr/bin/time`; assertion automated) |

### Performance checks (exit-gate; how measured, M6-9)

| Check | How measured |
|---|---|
| `perf_warm_start_picker` | Generated 500-skill fixture in a temp dir; one priming `Catalog::load` (writes the cache); then five timed runs of `Catalog::load` (warm) + `verify` + `App::new` + one `draw` to `TestBackend`; assert median under 50 ms. In-process, so exec/dynamic-link time is excluded; stated in the report. |
| `perf_memory_picker` | The test re-runs the current test binary via `std::process::Command::output()` (never `.spawn(`, which `no_network_symbols` bans) under `/usr/bin/time -l` (macOS) or `-v` (Linux), with `UNAM_PERF_CHILD=<fixture path>`. The child test `perf_memory_picker_child` does load + `App::new` + one draw and self-skips when the variable is unset (no `#[ignore]`, G13c). Parse peak RSS from the time output; assert under 50 MB. |

---

## Entry Gate

- Product Phase 1 gate: CONFIRMED by Taylor 2026-10-17 (umbrella `## Product Phase 1 Gate`). The earlier PRODUCT PHASE 2 HOLD is lifted.
- M0 gate met (symlink default for both adapters); M5 VERIFIED (643 tests; report `phase-06-m5-..._REPORT_04-10-26.md`); base commit for diffs: HEAD at EXECUTE start, expected `9a4e9df` or later (EXECUTE records the actual hash).

---

## Decisions

"REPORT TO TAYLOR" marks a visible interpretation for the phase report.

| # | Decision |
|---|---|
| D-TERM (Taylor) | `ratatui = { version = "0.30.2", default-features = false, features = ["termion"] }` in `[workspace.dependencies]`; `unam-cli` uses `ratatui.workspace = true`; no `crossterm`, no direct `termion` line (use `ratatui::termion`). Departs from spec 1.4 and all-context.md ("ratatui + crossterm"); recorded as a deviation for M7 docs; context text updated at UPDATE PROCESS; protected spec untouched. The VERDICT shows "turn off crossterm events" is impossible (ratatui-crossterm forces `events`, which pulls `mio`; negative control failed `no_network_dependencies`). `no_network_dependencies` and `no_network_symbols` stay unchanged and must pass. Lockfile 141 -> 180 packages (39 added, verified in the PVL scratch copy), none denied. Blocking `stdin().keys()`, single thread, never `async_stdin`, no `.spawn(` text in product source. Known limits (documented, not fixed): resize applies on the next keypress; external SIGTERM/SIGINT leaves the terminal raw. Required: a panic hook that writes `ToMainScreen` + `cursor::Show` then calls the saved default hook, plus the `RawTerminal`/`AlternateScreen` drop guards. Never `panic = "abort"` in any profile including the dist release profile: test `no_panic_abort_profile`. |
| D-EKEY (Taylor) | The `e` strategy toggle is dropped (v1 is symlink-only). Spec 6 deviation, REPORT TO TAYLOR and M7 docs. |
| D-TEST (Taylor) | AC-44 is Hybrid: automated synthetic keys + `TestBackend` in a temp project, plus one manual real-terminal run by Taylor in the report. No pty dependency (R8). |
| D-LANE | Light lane. |
| M6-1 | Test seam: `unam-cli` is bin-only, so the TUI tests are in-crate `#[cfg(test)]` modules (`tui/tests.rs`), driving a pure `App::handle_key(Key) -> Effect` state machine with injected `ratatui::termion::event::Key` values and rendering to `ratatui::backend::TestBackend`. No `[lib]` target. The terminal I/O (raw mode, alternate screen, blocking key loop) lives only in `tui/term.rs` and is covered by the manual run. In-crate tests run with `cargo test -p unam --bin unam <filter>`. |
| M6-2 | No match highlighting in v1: `nucleo-matcher` search returns `Hit { id, score }` only (`search.rs:15`). No core change. Recorded as a scope choice. |
| M6-3 | Category tree is derived in the TUI from `Skill.category` (path parent); `3/12` counts selected-over-total; selecting a category filters via `search(catalog, query, Some(category), None)`. No core change. |
| M6-4 | The picker holds the project lock only during apply (inside `change::run`), never while open. Spec 701's "close the open picker" remedy is moot; the `Locked` error is shown as a message in the footer. |
| M6-5 | `change::Edit` carries one kind per apply. Enter with only additions -> `Edit::Add`; only removals -> `Edit::Remove`; a mix -> footer message "apply additions and removals one at a time; esc clears" and no plan. REPORT TO TAYLOR (a combined edit would need a new core entry; deferred). |
| M6-6 | Bundle actions `b`: overlay listing bundles (`bundle::load_dir`); enter on one yields `Edit::UseReplace(name)` straight to the plan view; `a` yields `Edit::UseAdd`; `s` prompts a name and saves the pending-selected skills with `bundle::crud::save`. No bundle editing. |
| M6-7 | Interrupted modal keys: `r` resume (`change::resume`), `u` rollback (`change::rollback`), `esc` quits. The key press is the decision (`Decision::Proceed`). Shown when `change::has_journal(root, None)` is true, before the first Picker draw. |
| M6-8 | Cold scan: single-threaded, so `Loading` draws a "Loading catalog..." frame, then calls the synchronous `Catalog::load`. No thread, no spinner animation (spec 6 "spinner" is satisfied by the static frame; REPORT TO TAYLOR). |
| M6-9 | Perf checks use test-only in-crate harnesses (table above); no env var or flag is added to product code. |
| M6-10 | Bare `unam` opens the picker only when stdin and stdout are terminals (`IsTerminal`), `--json` is absent, and a project root resolves; otherwise the current usage plus status output, exit 0. Two targets: picker starts on the first detected target; `t` cycles `adapter::adapters()`. `Scope::Ancestor` entries and `kind: generated` receipt rows render as informational lines in the plan view (via `output::render_plan`) and never as selectable skills. |
| M6-11 | Rendering reuse: plan view = `output::render_plan(&plan, refresh)`; result view = `output::render_result(&ResultView{..})` (same crate, no duplication). Markers from `verify::verify` records: `[x]` pending add, `[●]` active, `[○]` active and pending removal, `[!]` conflict/foreign-at-dest. |
| M6-12 (PVL P3) | Key routing: the search input has focus at start. Every printable character except space is appended to the query (so `b`, `t`, `?` can be typed in a query such as "pytest"); `space` toggles the focused result from any pane; `backspace` edits the query; Ctrl-C quits like `esc` at the top level (raw mode delivers it as a key). `b`, `t`, and `?` act only when focus is on the category or results pane (`tab` cycles panes). REPORT TO TAYLOR (spec 6 lists `b`, `t`, `?` and "type to filter" without saying how they coexist). |

---

## Blast Radius

- Deps: workspace `Cargo.toml` (one line), `crates/unam-cli/Cargo.toml` (one line), `Cargo.lock` (about 39 new packages, reviewed against the deny list)
- New: `crates/unam-cli/src/tui/{mod,term,app,actions,picker,views}.rs`, in-crate `tui/tests.rs`, `tui/snapshots/` (new `.snap` files only)
- Edited: `crates/unam-cli/src/main.rs` (`mod tui;`), `cli/mod.rs` (bare branch routing at about lines 370-376), `crates/unam-cli/tests/usage.rs` (non-TTY regression test), `crates/unam-cli/tests/phase1_checks.rs` (`no_panic_abort_profile`; the AC list stays 1 to 43 and 53)
- Core: none expected. If a TUI action lacks a CLI-reachable core entry, record each addition (M6-5 deliberately avoids one).
- Not touched: `docs/`, `spike/`, the skill library, adapters, receipt/journal formats, the five protected documents, the 39 existing snapshots, catalog code (so no `perf_cold_scan` rerun unless catalog files change)

Risk class: new dependency tree (39 crates); raw-terminal handling; new caller of the frozen apply path.

---

## Test Plan

TEST-SCENARIO-DISCOVERY: `process/context/tests/all-tests.md` read; suite is cargo (643 tests after M5), snapshots via `insta` without `cargo-insta`, no pty (R8), package name `unam` (use `--test`/`--bin`, not `-p unam-cli`). Full fault-matrix env var applies only if apply paths change (they do not).

### AC-to-Test Map (all in `tui/tests.rs` unless noted)

| Criterion | Test | Notes |
|---|---|---|
| AC-44 | `ac_44_query_space_enter_enter_yields_active_skill` | temp HOME/project/library; keys: query chars, Space, Enter, Enter (the second Enter applies, same as `y`); asserts symlink at `.claude/skills/<name>`, `verify` Active, `assert_zero_divergence`-style check, source tree hash unchanged, foreign file byte-identical; also asserts keystrokes after the query <= 4 (S10) |
| AC-44 | `ac_44_second_target_and_ancestor_rows_render` | two targets (`t`), `Scope::Ancestor` and generated rows are informational |
| Parity | `tui_parity_every_action_maps_to_a_core_function_the_cli_calls`, `tui_parity_tui_never_writes_files` | enumerates `Action::ALL`; the second scans `src/tui/*.rs` for `std::fs::` writes and any `fsops::` token except `RealFs` |
| Layout | `tui_snapshot_picker_layout_22_48_30`, `tui_snapshot_markers_and_category_counts` | 100x30 `TestBackend` |
| Views | `tui_snapshot_plan_view`, `tui_snapshot_result_view`, `tui_snapshot_interrupted_modal` | |
| State machine | `tui_state_*` (Loading -> Picker, Plan n returns to Picker, Result enter returns to Picker, esc clears filter then quits, mixed pending message, Locked error shown, resume/rollback via modal) | pure, no terminal |
| Non-TTY | `tests/usage.rs::bare_unam_non_tty_prints_usage_and_status_exit_0` | assert_cmd runs without a TTY; existing `usage_bare` snapshot unchanged |
| Profile | `tests/phase1_checks.rs::no_panic_abort_profile` | scans `Cargo.toml`, `dist-workspace.toml`, and any `.cargo/config.toml` for `panic` set to abort |
| Perf | `perf_warm_start_picker`, `perf_memory_picker` (+ child) | M6-9 |
| Network | `no_network_dependencies`, `no_network_symbols` | unchanged, must pass |

Every mutating test asserts both invariants. New snapshots (five) are written once with `INSTA_UPDATE=always`, read, then gated with `INSTA_UPDATE=no` (no-cargo-insta procedure). TDD: write the `App` state-machine unit tests and the parity table red-first; record whether done.

Terminal-restore proof is manual only (panic hook, raw mode): Agent-Probe/Hybrid in the manual run; a Known-Gap is not claimed for any developed behavior.

---

## Implementation Checklist

Order is the compile order: A1, then Step B (B1, B2, B3), then A2 and A3 (the terminal loop and the routing call `App` and the views, so they cannot compile before Step B), then Step C. Start Step B with the `tui/mod.rs` module list and `mod tui;` in `main.rs` (without `run`) so each file is compiled as it is added; A2 then adds `term.rs` and `run`. The A/B/C labels name themes, not the sequence (PVL fix P1). Spec sections cited, not copied.

### Step A: Dependency and routing skeleton

- [x] A1. Add the `ratatui` line (D-TERM) to workspace `Cargo.toml` and `ratatui.workspace = true` to `crates/unam-cli/Cargo.toml`; a new lock cannot use `--locked`, so run `cargo check` once (unlocked) to update `Cargo.lock`, then read the new package list against the `no_network_dependencies` deny list and run that test.
- [x] A2. `tui/term.rs`: panic hook (restore main screen + cursor, then the saved default hook), `RawTerminal` + `AlternateScreen` guards, blocking `stdin().keys()` loop calling `App::handle_key`, redraw each key (resize on next key). `tui/mod.rs`: `pub fn run(...) -> anyhow::Result<()>`, with the `IsTerminal` check (M6-10).
- [x] A3. `cli/mod.rs` bare branch: on TTY and no `--json`, resolve root (`change::resolve_root`), call `tui::run`; otherwise the existing usage + `status::bare` output. `main.rs`: `mod tui;`.

### Step B: Model and views (pure, testable)

- [x] B1. `tui/app.rs`: `App`, `Mode { Loading, Picker, Plan, Applying, Result, Interrupted, Bundles }`, pending sets (adds, removals), query, focus pane, category filter, `handle_key(Key) -> Effect`; footer counts and catalog estimate from the plan/Catalog.
- [x] B2. `tui/actions.rs`: `enum Action` with `ALL`, `fn core_entry(self) -> &'static str` naming the core function, and the executor that calls `catalog::search::search`, `verify::verify`, `change::prepare`, `change::run(&RealFs, ..)`, `change::has_journal/resume/rollback`, `bundle::load_dir`, `bundle::crud::save`, `adapter::adapters`, `change::resolve_target` (and nothing else; no direct fs).
- [x] B3. `tui/picker.rs`: draw the three panes (22/48/30 percent), footer, markers, category tree with `3/12`; `tui/views.rs`: plan view, result view, Interrupted modal, bundle overlay, help (`?`), keys per spec 6 minus `e` (D-EKEY).

### Step C: Tests

- [x] C1. `tui/tests.rs`: state-machine tests, `ac_44_*`, parity (D: `Action::ALL` vs CLI source substring check), snapshots; `tests/usage.rs` and `tests/phase1_checks.rs` additions.
- [x] C2. Perf tests per M6-9; run them in release and record the measured values.
- [x] C3. Full M1 to M5 regression and both invariants (G3).

### Step D: Close

- [ ] D1 (OUTSTANDING, owner Taylor; closeout CONDITIONAL, backlog stub `process/features/v1-static-activation/backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md`). Taylor's manual real-terminal run (scenario: temp project, bare `unam`, type a query, space, enter, enter; also quit, a deliberate panic is NOT required) recorded in the report; if not yet done, the gate stays CONDITIONAL with a backlog stub (never Known-Gap PASS).
- [x] D2. Phase report with the deviations for M7 (termion, `e` dropped, no highlighting, static loading frame, M6-5), the lockfile delta, perf numbers; commit on `main`, code first then process; no push.

---

## Exit Gate

```bash
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check                                          # G1 Expected: exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings   # G2 Expected: exit 0 (no unwrap in non-test code)
cargo test --workspace --locked                                  # G3 Expected: all pass, including every test named in the Test Plan
cargo test -p unam --locked --release --bin unam perf_warm_start_picker   # G4a Expected: pass, median under 50 ms
cargo test -p unam --locked --release --bin unam perf_memory_picker       # G4b Expected: pass, peak RSS under 50 MB
```

| Gate | Check |
|---|---|
| G1 to G3 | The commands above |
| G4 | The two release perf commands (values recorded); `perf_cold_scan` only if `crates/unam-core/src/catalog/` changed (not expected) |
| G5 | Presence via `cargo test --workspace --locked -- --list`: (a) each of `(^|::)ac_44_`, `(^|::)tui_parity_`, `(^|::)tui_snapshot_` (at least 5), `(^|::)tui_state_`, `perf_warm_start_picker`, `perf_memory_picker`, `bare_unam_non_tty_`, `no_panic_abort_profile`, `no_network_dependencies`, `no_network_symbols`, `phase1_ac_coverage` is listed (a filter matching zero tests fails the gate); (b) `! grep -rEq '#\[ignore' crates`; (c) `git diff --diff-filter=MD --exit-code <base> -- crates/unam-cli/tests/snapshots` exits 0 (the 39 existing snapshots unchanged; new TUI snapshots live under `src/tui/snapshots/`); (d) `phase1_ac_coverage` still lists exactly AC-1 to 43 and 53 |
| G6 | Invariants and rules: both invariants asserted in `ac_44_*`; `git diff --exit-code <base> -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0; `! grep -rE 'async_stdin|crossterm' crates/*/src Cargo.toml crates/*/Cargo.toml`; `Cargo.lock` has no package named `crossterm`, `mio`, `socket2`, `signal-hook` |

- All checklist items checked; AC-44, parity, snapshots, non-TTY, both perf checks green; 643 existing tests and 39 snapshots unchanged.
- Regression: M1 to M5 criteria tests green.
- Phase report written to the report destination.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract. VERIFIED (WITH_GAPS) is allowed when only Taylor's manual run is outstanding, recorded as CONDITIONAL with a backlog stub.
- Commit execution changes on `main` before milestone M7; no push.

---

## Blockers That Would Justify BLOCKED Status

- The lockfile delta adds a package on the `no_network_dependencies` deny list (stop and report).
- A TUI action cannot be routed through an existing core entry without touching apply/journal formats (stop and ask).
- A fixture that would write into this repository or run a real agent CLI (hard safety constraint).

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH (2026-10-17): gate record verified; M5 report read; test context loaded; core reuse and gaps mapped; feasibility probe run (VERDICT VIABLE, termion)
- [x] 2. INNOVATE (2026-10-17): skipped, light lane; every design choice was resolved by Taylor (D-TERM, D-EKEY, D-TEST, D-LANE) and the feasibility probe; remaining small choices recorded as M6-1 to M6-11
- [x] 3. PLAN-SUPPLEMENT (2026-10-17): existing phase plan rewritten into an executable plan; Inner Loop Refresh Note written
- [x] 4. PVL (2026-10-17, inner-pvl: phase-7, 2 cycles, Gate: PASS): vc-validate-agent: at most 2 cycles; only FAIL-class gaps (wrong requirement mapping, steps that cannot compile in order) trigger a cycle; nits go to execute time; validate-contract per `.claude/skills/vc-validate-findings/references/example-validate-output.md`
- [x] 5. EXECUTE (2026-10-17): all checklist items done except D1 (Taylor's manual run, backlog stub); G1 to G6 green at EXECUTE; report `phase-07-m6-tui_REPORT_17-10-26.md`
- [x] 6. EVL (2026-10-17): independent vc-tester re-run, 2 iterations, both HALTED_SUCCESS (0425429, then 9658536 after the review-fix round bd97432); G1 to G6 green, 699 tests, warm start 45.0 to 46.8 ms of 50, RSS 9.1 MB; follow-up stubs m6-ac44-manual-terminal-run and m6-warm-start-margin; EVL HANDOFF SUMMARY WITH_GAPS
- [x] 7. UPDATE PROCESS (2026-10-17): phase report completed (EVL outcome, code-review summary, For Taylor decisions, commits), umbrella state and WAVES and registry updated, all-context.md (termion stack line, source layout) and tests/all-tests.md (699 tests, TUI test commands) updated, process commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Inner Loop Refresh Note: 2026-10-17 - changed sections: header (Status, Execution strategy), Overview, Scope, Acceptance Criteria (AC-44 Hybrid with exact method, parity, snapshots, non-TTY rows; perf measurement method made concrete), Entry Gate (hold lifted), Decisions (new: D-TERM, D-EKEY, D-TEST, D-LANE, M6-1 to M6-11), Blast Radius (adds workspace and crate `Cargo.toml`, `Cargo.lock`, `usage.rs`, `phase1_checks.rs`; `nucleo` corrected to `nucleo-matcher` by omission), Test Plan (new), Implementation Checklist (rewritten in compile order), Exit Gate (`--locked`, `<file>` placeholder replaced by `--bin unam`, G5 presence list, G6), Blockers, Phase Loop Progress, Touchpoints, Public Contracts, Verification Evidence, Resume and Execution Handoff. Removed: the PRODUCT PHASE 2 HOLD paragraph, the first-item hard-stop Entry Gate text, and the `e` key. Research found: `unam-cli` is bin-only so tests are in-crate; `Edit` carries one kind per apply so mixed pending needs a decision (M6-5); ratatui's crossterm backend forces `mio` (VERDICT), so the backend is termion; `nucleo-matcher` gives no match indices; bare `unam` today is usage plus status (kept for non-TTY); `no_network_symbols` bans the text `.spawn(` in `crates/*/src`, so the memory test uses `Command::output()`; `phase1_ac_coverage` is an explicit list that must not gain AC-44. No validate-contract existed to invalidate.

---

## Touchpoints

- Files listed in Blast Radius.
- Read only: spec 1.1, 1.4, 5, 6, 9, 10; PRD 7.10 (UX-2, UX-3), 12, 13.2 (AC-44), S10; M5 report; feasibility VERDICT; `process/context/tests/all-tests.md`.
- Core entry points reused: `catalog::search::search`, `Catalog::load`, `change::{resolve_root, resolve_target, prepare, run, has_journal, resume, rollback}`, `verify::verify`, `bundle::load_dir`, `bundle::crud::save`, `adapter::adapters`; CLI `output::{render_plan, render_result}`.

---

## Public Contracts

- Bare `unam`: picker on a TTY with no `--json`; usage plus status, exit 0, otherwise (spec 5, 6).
- Keys (spec 6 minus `e`): type to filter, arrows, `space`, `tab`, `enter`, `b`, `t`, `?`, `esc`; plan view `y` or `enter` applies, `n` or `esc` returns (AC-44 says "pressing enter twice", so the second `enter` must apply; PVL fix P2); result view `q`/`enter`; Interrupted modal `r`/`u`/`esc`. Markers `[x] [●] [○] [!]`, counts `3/12`.
- Dependencies: `ratatui` (termion backend) only; no `crossterm`, `mio`, `socket2`, `signal-hook`.
- No new state formats; no new core public API (M6-5 avoids one). Unchanged: exit codes, receipt, journal, six-state record.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` | Fully-Automated | spec 8 gates |
| `ac_44_*` query, space, enter, enter yields an active skill (synthetic keys, `TestBackend`, temp project) | Hybrid (automated assertions; real-terminal half by Taylor) | AC-44, S10 |
| Taylor's real-terminal run of the primary scenario | Hybrid (precondition: TTY, temp project) | PRD 12 product Phase 2 done condition |
| `tui_parity_*` every action maps to a core function the CLI calls; TUI writes no files | Fully-Automated | spec 10 M6 parity, spec 1.1 |
| `tui_snapshot_*` layout 22/48/30, markers, counts, plan, result, Interrupted | Fully-Automated | spec 6 |
| `tui_state_*` state machine and modal | Fully-Automated | spec 6 state machine |
| `bare_unam_non_tty_*` and `usage_bare` snapshot | Fully-Automated | spec 5 (non-interactive bare `unam`) |
| `no_panic_abort_profile` | Fully-Automated | D-TERM terminal-restore constraint |
| `no_network_dependencies`, `no_network_symbols` | Fully-Automated | spec 9 network none |
| `perf_warm_start_picker` median under 50 ms | Hybrid (release build, 500-skill fixture; assertion automated) | spec 9 warm start |
| `perf_memory_picker` peak RSS under 50 MB | Hybrid (release build, `/usr/bin/time`; assertion automated) | spec 9 memory |

No Known-Gap rows for developed behavior. Residual named: terminal-restore on panic and resize-on-next-key are exercised only in the manual run (Hybrid, Taylor).

---

## Test Infra Improvement Notes

- The bin-only crate forces in-crate tests; a future `[lib]` split would let integration tests import `tui::*` (not needed for v1).
- No pty harness (R8); the manual run covers the real-terminal half of AC-44.
- `cargo-insta` is not installed; snapshots use the no-cargo-insta procedure.

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md`
- Last completed step: 5 EXECUTE (2026-10-17)
- Validate-contract status: written 2026-10-17 (inner-pvl: phase-7, PASS, 2 cycles; honor G5 presence checks and EX1 to EX18)
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, `process/context/planning/all-planning.md`, feasibility VERDICT in this folder, M5 report
- Next step: EVL (independent vc-tester re-run of G1 to G6), then UPDATE PROCESS. Pass `PATH` and `INSTA_UPDATE` as in the Exit Gate block.

---

## Validate Contract

Status: PASS
Date: 17-10-26
date: 2026-10-17
generated-by: inner-pvl: phase-7

Parallel strategy: sequential
Rationale: score 2/7 (S4 phase program; S7 more than 5 files; no S1 because only one crate gains code, no S2 (no schema, API, auth, or state-format change), no S3, no S5, no S6 because no listed high-risk class applies: the apply path is reused unchanged). One compile chain (deps, `tui` modules, routing, tests), so no disjoint file group exists for a team and no independent direction exists for parallel subagents. EXECUTE: one sonnet execute agent (Taylor, 2026-10-09 and the plan header: all agents sonnet), then an independent sonnet vc-tester EVL run. Agent counts: sequential 1; parallel subagents 4 + 6 + 3 = 13 with nothing to split; agent team 3 x 2 = 6, no benefit; workflow 6 x 1 x 1 = 6, not warranted. Cost guard not triggered. Light lane (D-LANE): worked inline by the validate agent in 2 cycles (cycle 1 found 2 FAIL-class gaps and applied plan fixes P1 to P3; cycle 2 re-traced and found none), no fan-out, no helper agent, no `unam` run, no agent CLI, skill library untouched, Cargo files of the real repository untouched (all dependency work in a disposable scratch copy). Only FAIL-class gaps were counted. Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so none is written here. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Cycle log (rows in `results-m6.tsv`):
- Cycle 1, FAIL-class gap F1: Step A (terminal loop, `cli/mod.rs` routing) calls `App` and the views that Step B defines, but the plan said "Order is the compile order". A2 and A3 cannot compile before B1 to B3. Fixed in the plan (P1): order is A1, B1 to B3, A2, A3, C, and Step B starts with the `tui/mod.rs` module list and `mod tui;`.
- Cycle 1, FAIL-class gap F2: PRD AC-44 says "pressing enter twice", but Public Contracts gave the plan view only `y`/`n`, and the AC-44 test row wrote "Enter(y)". A literal reading would pass the test with `y` while the PRD wording fails. Fixed in the plan (P2): in the plan view `enter` or `y` applies, `n` or `esc` returns; the test sends Enter, Enter.
- Cycle 1, plan nits applied with the fixes: P3 adds M6-12 (key routing between filter typing and the `b`, `t`, `?` keys); the lockfile counts in D-TERM now read 141 to 180 (the plan said 137 to 176).
- Cycle 2: re-traced the compile order and the AC-44 key path against the edited plan. 0 FAIL-class gaps. Structural validator on the edited plan: 0 failures, 0 warnings (288 lines).

Requirement mapping check (spec 6, 9, 10 M6; PRD 7.10 UX-1 to UX-3, 13.2 AC-44, S10, D-decisions): AC-44 is the only M6 criterion (spec 10 M6) and maps to `ac_44_query_space_enter_enter_yields_active_skill` plus Taylor's manual run (Hybrid, never Known-Gap). The M6 parity requirement maps to `tui_parity_*`. Spec 6 layout, markers, plan and result views, modal, and state machine map to `tui_snapshot_*` and `tui_state_*`. Spec 9 maps to the two perf tests. Bare `unam` without a terminal (spec 5) is kept. AC-1 to 43 and 53 stay owned by their existing tests. No criterion is missed. Tensions with the protected documents, all already Taylor-decided or recorded for the report: spec 1.4 and all-context.md name `crossterm` (D-TERM, termion); spec 6 plan view lists `e` (D-EKEY, dropped); spec 6 "spinner" becomes a static frame (M6-8); spec 6 lists `b`, `t`, `?` and "type to filter" without a routing rule (M6-12). M6-5 (mixed edits), M6-6 (`b` overlay), and M6-7 (`r`/`u` modal keys) were checked for a real conflict and have none: M6-5 follows UX-1 (the CLI has no combined add-and-remove command, `change::Edit` carries one kind); M6-6 matches spec 6 "b bundle actions (use, save)" and calls `change::Edit::UseReplace`/`UseAdd` and `bundle::save`, which the CLI calls; M6-7 matches "offering resume or rollback" and the key press is the confirmation the CLI asks for. Taylor may still change them.

Reuse claims checked against the real code (all hold except where noted):

| Claim | Evidence read or run | Verdict |
|---|---|---|
| Lockfile stays clean with ratatui 0.30.2 `default-features = false, features = ["termion"]` | Scratch copy of the tracked workspace (`scratchpad/m6-pvl/ws`): `cargo check -p unam` unlocked updated the lock from 141 to 180 packages, 39 added, the same 39 names as the VERDICT, 0 removed; no `crossterm`, `mio`, `socket2`, `signal-hook`, `signal-hook-registry`, `tokio`, `http`, `h2`, `hyper` in `Cargo.lock`; only `libc` and `rustix` (already locked) among the checked names; afterwards `cargo test -p unam --locked --test phase1_checks` passed all four tests (`no_network_dependencies`, `no_network_symbols`, `phase1_ac_coverage`, `no_background_process`) with `phase1_checks.rs` unchanged | Holds (counts 141 to 180, not the plan's 137 to 176; plan text corrected) |
| Bin-only seam: in-crate `#[cfg(test)]` modules work | Prototype in the scratch copy: `tui/mod.rs` plus `tui/tests.rs` with `ratatui::backend::TestBackend`, `ratatui::termion::event::Key`, `insta::assert_snapshot!`; `cargo test -p unam --locked --bin unam tui::` ran them; `cargo test --workspace --locked -- --list` printed `tui::tests::<name>: test`, so the G5 regexes `(^|::)name` match; `output.rs` already has in-crate tests, so the seam is existing practice here; clippy `--all-targets -D warnings` clean on the prototype | Holds |
| Snapshot files and procedure for in-crate insta | The first run with `INSTA_UPDATE=always` wrote `crates/unam-cli/src/tui/snapshots/unam__tui__tests__proto.snap`; `INSTA_UPDATE=no` then passed; the tests/snapshots directory (G5 c) is untouched by in-crate snapshots | Holds |
| Memory test via `/usr/bin/time` through `Command::output()` | Prototype: re-run of the test binary as `/usr/bin/time -l <exe> tui::tests::perf_memory_picker_child --exact --nocapture` with `UNAM_PERF_CHILD` set; stderr carried `maximum resident set size` in BYTES on this macOS (3.6 MB debug, 3.2 MB release for a trivial child); Linux `-v` prints `Maximum resident set size (kbytes)` (kilobytes, not run here); the test source has no `.spawn(` text (`Command::output()` is not the banned text) and `no_network_symbols` and `no_background_process` stayed green; the child exits before the parent reads output, so nothing outlives the test | Holds with EX5 |
| Reused core entries exist, are public, and have the signatures the plan assumes | `change::{resolve_root, resolve_target, prepare, run, has_journal, resume, rollback}`, `change::Edit::{Add, Remove, UseReplace, UseAdd}`, `Decision::Proceed`, `ChangeRequest`, `Outcome::Interrupted`, `RecoverRequest`, `catalog::search::search(catalog, query, category, source)`, `verify::verify(&VerifyRequest)`, `bundle::{load_dir, save}`, `adapter::adapters`, `Dirs::from_process_env`, `Config::load_or_create`, `hash::tree_hash`; `output::{render_plan, render_result, ResultView, neutralize}` are `pub` in the bin crate | Holds |
| Parity needles: "a named core function that the CLI source also calls" | Read of `cli/*.rs`: the CLI contains `search::search(`, `verify::verify(`, `change::run(`, `change::resume(`, `change::rollback(`, `change::has_journal(`, `change::resolve_target`, `change::resolve_root`, `unam_core::bundle::load_dir`, `bundle::save(`, `Catalog::load(`. It does NOT contain `change::prepare` (reached only inside `change::run`), `adapter::adapters`, or `bundle::crud::save` (the CLI uses the re-export `bundle::save`) | Does not hold as written; resolved by EX3 (a two-column table: TUI call needle and CLI needle), not a gap |
| `Ctx` and the helper functions are usable from `tui` | `Ctx::load`, `followup_lines`, and `shared_consumers` are private to `cli`; every core function they call is public | Holds with EX2 (the TUI builds its own context from core; visibility-only edits are allowed) |
| Bare `unam` non-TTY path is preserved | `cli/mod.rs` bare branch prints `usage_text()` then `status::bare`; `usage_bare` runs `TestEnv::run(&[])` under `assert_cmd` (stdin null, stdout piped); the new routing needs stdin AND stdout terminals, so the snapshot and `bare_unam_prints_*` tests keep their path | Holds with EX10 |
| `phase1_ac_coverage` excludes AC-44 | The list is `(1..=43).chain([53])`; the test reads only `crates/*/tests/*.rs` (minus itself), so in-crate `src/tui/tests.rs` is never scanned; the unchanged file passed in the scratch run | Holds |
| `no_panic_abort_profile` can pass today | Root `Cargo.toml` has no `[profile]`; `dist-workspace.toml` has only `[workspace]` and `[dist]`; no `.cargo/` directory in the repository | Holds with EX16 |
| Hard stop and entry gate present | Plan Entry Gate, Blockers, and the umbrella `## Product Phase 1 Gate` (cleared by Taylor 2026-10-17) | Holds |
| Hook-safe gate text | Every command text in G1 to G6 below was fed to `.claude/hooks/scout-block.cjs` as a Bash tool input: all exit 0 | Holds |

Compile order traced (after P1): A1 (dependency lines, one unlocked `cargo check -p unam` to write the lock) compiles alone. B1 `tui/app.rs` (`App`, `Mode`, `handle_key`, uses only `ratatui::termion::event::Key` and core types) needs the module list in `tui/mod.rs` and `mod tui;` first. B2 `actions.rs` uses B1 types and core entries. B3 `picker.rs` and `views.rs` use B1 and B2 and `crate::output`. A2 `term.rs` and `run` use B1 to B3 (key loop calls `handle_key`, draws with B3). A3 routing in `cli/mod.rs` calls `tui::run`. C1 to C3 tests use everything. Forward references: none after P1. Dead-code warnings between steps are fine; clippy runs at the gate.

Gates (hook-safe; set `PATH` and `INSTA_UPDATE` first, as in the plan's Exit Gate block; base commit for diffs is `9a4e9df`, EXECUTE records the actual HEAD):
- G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets --locked -- -D warnings`; G3 `cargo test --workspace --locked` (this runs the perf tests in the debug profile; EX5).
- G4a `cargo test -p unam --locked --release --bin unam perf_warm_start_picker -- --nocapture`; G4b `cargo test -p unam --locked --release --bin unam perf_memory_picker -- --nocapture` (record the printed median and peak RSS). `perf_cold_scan` only if `git diff --name-only 9a4e9df -- crates/unam-core/src/catalog` prints anything (none expected).
- G5 presence plus snapshot and ignore checks (G3 alone stays green if a named test is never written). Run `cargo test --workspace --locked -- --list` and require: (a) each of `(^|::)ac_44_`, `(^|::)tui_parity_`, `(^|::)tui_state_`, `(^|::)tui_term_`, `bare_unam_non_tty_`, `perf_warm_start_picker`, `perf_memory_picker`, `no_panic_abort_profile`, `no_network_dependencies`, `no_network_symbols`, `phase1_ac_coverage` matches at least one name, and at least 5 names match `(^|::)tui_snapshot_`; (b) `! grep -rEq '#\[ignore' crates`; (c) `git diff --diff-filter=MD --exit-code 9a4e9df -- crates/unam-cli/tests/snapshots` exits 0 (the 39 existing snapshots unchanged; TUI snapshots live under `src/tui/snapshots/`); (d) `git diff 9a4e9df -- crates/unam-cli/tests/phase1_checks.rs` shows no change to the `wanted` list (AC-1 to 43 and 53); (e) every `.snap` under `src/tui/snapshots/` was read by the agent and holds no temp path or timestamp.
- G6 invariants and rules: `ac_44_*` ends with both invariants (source tree hash unchanged via `hash::tree_hash`, planted foreign file byte-identical); `git diff --exit-code 9a4e9df -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0; `! grep -rE 'async_stdin|crossterm' crates/*/src Cargo.toml crates/*/Cargo.toml` (this includes comments, doc strings, and test strings: EX6); `! grep -qE '^name = "(crossterm|mio|socket2|signal-hook)"' Cargo.lock`.

AC-to-test map check (plan Test Plan against spec 10 M6 and PRD 13.2 text):

| Criterion | PRD or spec text | Plan test | Verdict |
|---|---|---|---|
| AC-44 | from bare `unam`, type a query, press space on a result, press enter twice: an active skill | `ac_44_query_space_enter_enter_yields_active_skill` (keys: query chars, Space, Enter, Enter; asserts symlink, `verify` Active, both invariants, 3 keys after the query) plus Taylor's real-terminal run | Maps after P2 and EX9 |
| UX-3, S10 | incremental search, multi-select, visible pending set; four or fewer keystrokes | `ac_44_*`, `tui_state_*`, `tui_snapshot_markers_and_category_counts`, key count assertion | Maps |
| UX-1 / spec 10 M6 parity | every TUI action calls a core function also reachable from the CLI | `tui_parity_every_action_maps_to_a_core_function_the_cli_calls`, `tui_parity_tui_never_writes_files` | Maps with EX3 and EX4 |
| Spec 6 | layout 22/48/30, markers, counts, plan view, result view, modal, state machine | `tui_snapshot_*` (5), `tui_state_*` | Maps |
| Spec 5 | bare `unam` outside a terminal stays usage plus status | `bare_unam_non_tty_*`, `usage_bare` | Maps with EX10 |
| Spec 9 | warm start under 50 ms; memory under 50 MB | `perf_warm_start_picker`, `perf_memory_picker` (+ child) | Maps with EX5 |
| Spec 9 | network none, background none | `no_network_dependencies`, `no_network_symbols`, `no_background_process` | Proven now in the scratch copy |
| D-TERM | terminal restored on panic; no abort profile | `no_panic_abort_profile`, `tui_term_panic_hook_*` (EX7) | Maps with EX7 and EX16 |

Test gates (C3 table; legacy form is the Verification Evidence table of this plan):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 gates | fmt, clippy, workspace tests green | Fully-Automated | G1, G2, G3 | B |
| AC-44 (automated half) | query, space, enter, enter yields an active skill; both invariants hold | Hybrid | `ac_44_query_space_enter_enter_yields_active_skill` | B |
| AC-44 (real-terminal half) | the same scenario in a real terminal in a temp project; raw mode, alternate screen, key decoding, restore on quit | Hybrid | Taylor's manual run recorded in the report (D1) | D (named residual; if outstanding at EXECUTE the closeout is CONDITIONAL with a backlog stub) |
| AC-44 / UX-3 | second target via `t`; ancestor and generated rows render as information only | Fully-Automated | `ac_44_second_target_and_ancestor_rows_render` | B |
| spec 10 M6 parity | each TUI action maps to a core function the CLI also reaches; the TUI writes no files | Fully-Automated | `tui_parity_*` | B |
| spec 6 | layout, markers, counts, plan, result, modal | Fully-Automated | `tui_snapshot_*` (at least 5) | B |
| spec 6 state machine | Loading, Picker, Plan, Applying, Result, modal, esc and Ctrl-C, mixed-edit footer, Locked message, key routing (M6-12) | Fully-Automated | `tui_state_*` | B |
| D-TERM panic restore | the hook writes leave-alternate-screen and show-cursor before the panic text; exit code 101 under unwind | Fully-Automated (child re-exec, no pty) | `tui_term_panic_hook_*` (EX7) | B |
| D-TERM abort ban | no profile, dist file, or cargo config sets panic to abort | Fully-Automated | `no_panic_abort_profile` | B |
| spec 5 | bare `unam` without a terminal prints usage plus status, exit 0 | Fully-Automated | `bare_unam_non_tty_*`, `usage_bare` | B |
| spec 9 network and background none | lockfile deny list, source symbol scan, process sample | Fully-Automated | `no_network_dependencies`, `no_network_symbols`, `no_background_process` | A (run and green in the scratch copy with the new lock) |
| spec 9 warm start | median under 50 ms (release) on a 500-skill catalog | Hybrid (release build, local SSD; assertion automated; in-process, exec time excluded) | `perf_warm_start_picker` | B |
| spec 9 memory | peak RSS under 50 MB (release) | Hybrid (release build, `/usr/bin/time`; assertion automated) | `perf_memory_picker` | B |
| Phase 1 unchanged | AC-1 to 43 and 53 each have a named passing test; AC-44 not added | Fully-Automated | `phase1_ac_coverage` plus G3 | A |
| D-TERM known limits | resize applies on the next key; external SIGTERM or SIGINT leaves the terminal raw; termios restore on panic | Hybrid (manual) | Taylor's run (optional deliberate panic) | D (named residual, documented, not fixed) |

Failing stubs: each named test is written red-first by the execute agent from the Scenario text in the AC-to-Test Map; per-row stub blocks are omitted under the lighter process. Record whether the red-first run happened (the plan's TDD note asks for it).

Dimension findings:
- Infra fit: PASS - one compile chain on the frozen M3 to M5 apply path; every reuse claim holds except the parity needle names (EX3); the lockfile delta is verified in a scratch copy (141 to 180, none denied); the CLI-private `Ctx` is handled by EX2; Linux and `/usr/bin/time -v` are unrun (named residual).
- Test coverage: PASS - every M6 criterion and rule maps to a named test; G5 presence closes the vacuous-green path; the panic hook, which had no automated gate and was not in the manual scenario, now has `tui_term_panic_hook_*` (EX7) and a G5 entry; the debug-profile bounds for the perf tests (G3 runs them) are bound in EX5; no developed behavior rests on a Known-Gap alone; the manual half of AC-44 is a named Hybrid residual (D-TEST).
- Breaking changes: PASS - no state format, core API, exit code, or JSON change; bare `unam` outside a terminal is byte-identical (`usage_bare` unchanged); `cli/mod.rs` gains a branch only; new dependency tree is additive and documented as a deviation for M7 docs.
- Security surface: PASS - the TUI is a renderer over `unam-core`; applies go through the existing `change::run` (lock, journal, receipt, only receipted entries removed); untrusted skill text (name, description, category, preview) is shown through `output::neutralize` (EX15), with ratatui's own control-character filter as a second layer (verified in `ratatui-core` `buffer.rs`); no network, no thread, no `async_stdin`, no `unsafe` (workspace forbids it); the terminal-left-raw cases are documented limits, nothing beyond the threat model is demanded.
- Sections A (deps and routing), B (model and views), C (tests), D (close): PASS each after P1 to P3; 0 FAILs / 0 CONCERNs.

Execute-time notes (nits, binding on the execute agent, not gaps; list each applied or deviated in the phase report):
- EX1 Order: A1, then B1 to B3 (module list and `mod tui;` first), then A2, A3, then C (P1). Run `cargo check -p unam` after each of B1, B2, B3, A2.
- EX2 Context for the TUI: `Ctx` and `Ctx::load` are private to `cli`. Build the TUI context from core (`Dirs::from_process_env`, `Config::load_or_create`, `Catalog::load` with `LoadOptions::product(true)`, `change::resolve_root(project, cwd)`), or make `Ctx`, `Ctx::load`, and `catalog_warnings` `pub(crate)` (visibility only). `followup_lines` and `shared_consumers` in `cli/change.rs` are 10-line wrappers over `adapter.followups(root, dirs)` and `adapter_for(target).consumers()`; either make them `pub(crate)` (visibility-only edit to `cli/change.rs`, inside this blast radius) or repeat the calls in `actions.rs`. Record which.
- EX3 Parity table: `Action::ALL` plus `fn core_entry(self) -> (tui_needle, cli_needle)`. The test asserts `tui_needle` occurs in `tui/actions.rs` and `cli_needle` occurs in `cli/*.rs` source, using only needles that exist today: `search::search`, `verify::verify`, `change::run`, `change::resume`, `change::rollback`, `change::has_journal`, `change::resolve_target`, `change::resolve_root`, `bundle::load_dir`, `bundle::save`, `Catalog::load`. For the plan preview the TUI calls `change::prepare`, which the CLI reaches only through `change::run`: use `change::run` as the CLI needle; for `t` (target cycle) the TUI calls `adapter::adapters` and the CLI needle is `adapter_for` or `resolve_target`. Not vacuous: a unit test feeds the checker a bogus needle and expects an error; an exhaustive `match` in `core_entry` (no wildcard arm) forces every variant to be listed; `Action::ALL.len()` equals the variant count (assert against a hand-written count).
- EX4 `tui_parity_tui_never_writes_files`: scan `src/tui/*.rs` except `tests.rs` (the tests create fixtures); build the needles with `format!` or `concat!` so no file matches itself; flag `std::fs::` write calls, `fs::write`, `create_dir`, `remove_`, `rename`, `copy`, `File::create`, `OpenOptions`, `symlink`, and any `fsops::` token other than `RealFs`; a control case plants a bad string and expects a hit; assert that at least 5 files were scanned.
- EX5 Perf tests and the debug profile: G3 runs them in the debug build. Mirror `unam-core/tests/perf_cold_scan.rs`: `let release = !cfg!(debug_assertions)`; print the median and profile; assert 50 ms in release and a generous bound in debug (5 s for warm start; for memory a bound well above the debug value, for example 200 MB, with the spec 50 MB asserted in release). `perf_memory_picker`: use `std::env::current_exe()`, run `/usr/bin/time` with `-l` (macOS, bytes, line ends with `maximum resident set size`) or `-v` (Linux, line `Maximum resident set size (kbytes):`, multiply by 1024) through `Command::output()` (never the banned spawn text), pass the child as `tui::tests::perf_memory_picker_child --exact --nocapture` with `UNAM_PERF_CHILD=<fixture path>`; the child prints a marker line; the parent asserts exit success, the marker in stdout, and a parsed RSS above 1 MB (so a skipped or empty run cannot pass); if `/usr/bin/time` is missing print `self-skip` (named residual) and return. The child self-skips when the variable is unset (no `#[ignore`). Run the fixture in a temp dir with the priming load first (warm cache).
- EX6 Forbidden words: `async_stdin` and `crossterm` must not appear anywhere under `crates/*/src`, in `Cargo.toml`, or in `crates/*/Cargo.toml`, including comments, doc strings, and test strings (G6 greps them). Likewise the `no_network_symbols` needles (`std::` plus `net`, the TCP and UDP type names, and the spawn call text) must not appear in any `src` file, including `tui/tests.rs`. Write comments about these without the literal words.
- EX7 Panic hook test (closes the vacuous-green path): factor the restore bytes into `fn restore_bytes() -> Vec<u8>` (`ToMainScreen` then `cursor::Show`) and `fn install_panic_hook()`. `tui_term_panic_hook_restore_bytes` asserts the bytes contain `\x1b[?1049l` and `\x1b[?25h`. `tui_term_panic_hook_writes_restore_before_message` re-runs the test binary through `Command::output()` with an env var and a child test that installs the hook and panics; the parent asserts stderr holds the leave-alternate-screen sequence before the panic message and exit code 101. The child self-skips when the variable is unset. Raw-mode termios restore stays with Taylor's run.
- EX8 Key loop in `term.rs`: blocking `stdin().keys()` only; single thread; draw once before the first key and after each key; `Key::Ctrl('c')` is delivered as a key in raw mode (the VERDICT showed it), so handle it as quit; every `Key::Char(c)` (including multi-byte) appends to the query per M6-12.
- EX9 AC-44 test details: keys after the query are exactly Space, Enter, Enter (3, at most 4); the second Enter applies in the plan view (P2); also unit-test that `y` applies, `n` and `esc` return to the picker, and that Enter in the result view returns to the picker.
- EX10 Bare routing (M6-10): open the picker only when `std::io::IsTerminal` is true for BOTH stdin and stdout, `--json` is absent, and `change::resolve_root` succeeds; otherwise keep the exact current branch (usage text, then `status::bare`). Leave `usage_text` and `status::bare` untouched. `bare_unam_non_tty_prints_usage_and_status_exit_0` asserts exit 0, `Usage:` present, and no `\x1b[?1049h` in stdout; `bare_unam_prints_usage_without_config` must stay green (no config file created off a terminal).
- EX11 Interrupted modal: do not call `resume` or `rollback` with `target: None` when two targets have journals (`TargetAmbiguous`); loop `adapters()` with `has_journal(root, Some(id))` and pass `Some(id)` for the first (name the target in the modal). Show the `Locked` and other errors as a footer message, never a panic or `unwrap`.
- EX12 In-crate fixtures: `tests/common` is not importable from the bin crate. Build a temp HOME, config dir, cache dir, library, and project inside `tui/tests.rs` with `tempfile` and `Dirs::resolve(Env::from_map(..))` (pattern: `unam-core/tests/perf_cold_scan.rs`), a `.git` marker or `resolve_root(Some(path), cwd)` for the project, `hash::tree_hash` for the source invariant, and a planted foreign file under `.claude/skills/`. Assert zero divergence after apply with `verify`/`reconcile::report`. Never read the real HOME, never `unam` against this repository, no agent CLI.
- EX13 Snapshots: render at a fixed 100x30 `TestBackend`; use fixed temp-independent text (relative names, no temp paths, no timestamps); write the five-plus `.snap` files once with `INSTA_UPDATE=always`, read each, then gate with `INSTA_UPDATE=no`. Files land in `crates/unam-cli/src/tui/snapshots/unam__tui__tests__<name>.snap` (verified in the prototype).
- EX14 Lockfile (A1): one unlocked `cargo check -p unam` writes `Cargo.lock`; expect 141 to 180 packages (39 added, the VERDICT name list, nothing removed); then run the two network tests. Stop and report if any package on the deny list, or `crossterm`, `mio`, `socket2`, `signal-hook`, appears. Commit `Cargo.lock` with the code.
- EX15 Untrusted text: pass skill name, description, category, and preview text through `output::neutralize` before it reaches a `Span` or `Paragraph`; add one `tui_state_` or `tui_snapshot_` case with an escape byte in a description and assert the rendered text shows `\x1b` and no raw escape.
- EX16 `no_panic_abort_profile`: scan the text of `Cargo.toml`, `dist-workspace.toml`, and `.cargo/config.toml` (absent now; scan when present) for `panic` set to `"abort"` with optional whitespace (a small hand matcher, no new dependency) and for `-C panic=abort`; assert the files that exist were read (root `Cargo.toml` and `dist-workspace.toml` must be found) and unit-test the matcher against a positive and a negative sample. Put it in `tests/phase1_checks.rs` without touching `phase1_ac_coverage`.
- EX17 REPORT TO TAYLOR (standing rule: report tension, do not resolve silently): D-EKEY; M6-5 mixed-edit footer; M6-8 static loading frame; M6-12 key routing and `enter` applying in the plan view (spec 6 says `y`); termion in place of crossterm (spec 1.4 and all-context.md); parity needles reached through `change::run` and `adapter_for`; no match highlighting (M6-2); Linux, resize-on-next-key, and external SIGTERM untested or unfixed.
- EX18 Do not edit: the five protected documents, `docs/`, `spike/`, the skill library, existing snapshots, `phase1_ac_coverage`'s list. UPDATE PROCESS (not EXECUTE) updates `all-context.md` and the tests context for termion and the new test counts.

Open gaps: none FAIL-class. Named residuals (not proving): Taylor's manual real-terminal run of AC-44 (Hybrid; if not done at EXECUTE the closeout is CONDITIONAL with a backlog stub, never Known-Gap PASS); termios restore on panic and resize-on-next-key are exercised only by hand; external SIGTERM and SIGINT leave the terminal raw (documented, not fixed); Linux and `/usr/bin/time -v` unrun; no pty test (R8); real 1.89 toolchain compile unrun (local rustc 1.99, highest dependency MSRV 1.88 per the VERDICT); wide Unicode input, bracketed paste, and non-macOS terminals untested.
What This Coverage Does NOT Prove:
- G1 to G3: nothing about Linux, CI, or a real terminal; the in-crate tests drive `App::handle_key` with synthetic `Key` values and render to `TestBackend`, so raw mode, the alternate screen, key decoding from real bytes, and window-size handling are not exercised.
- AC-44 automated half: that a human can run the scenario in a real terminal in four keystrokes; that is Taylor's manual run (Hybrid residual).
- `tui_term_panic_hook_*`: the bytes the hook writes and their order before the panic text; not that the terminal's termios is restored (the `RawTerminal` drop guard is termion's, shown only by the VERDICT probe and Taylor's run).
- `tui_parity_*`: that each action calls the named core function and that the CLI source mentions the same function; not that CLI and TUI produce identical results for every input.
- Perf: one machine, release build, in-process timing (exec and dynamic-link time excluded) and `/usr/bin/time` peak RSS of a test process (macOS bytes, Linux kilobytes unrun); a 500-skill synthetic catalog, not Taylor's library.
- Network none and background none: a lockfile deny list, a source symbol scan, and a process sample after commands; the termion tree adds no network crate, but this is not a runtime trace.
- Snapshots: layout and text at 100x30 only; other sizes and colors are not asserted.
- Product Phase 2 done: daily use of the picker is Taylor's.
Net result: PASS (0 FAIL-class gaps after cycle 2, 0 CONCERNs; no developed behavior rests on a Known-Gap alone)
Gate: PASS
Accepted by: not required (no CONCERN remains); the named residuals above are recorded as known-gaps. EXECUTE is legal: honor G5 presence checks and EX1 to EX18. AC-44's manual Taylor run is a named Hybrid residual; if it is outstanding at EXECUTE it makes the phase closeout CONDITIONAL, not this gate.

---

## Deviations

Recorded by EXECUTE 2026-10-17. All stay inside the blast radius; none touch a hard-stop class (no schema, auth, public API, container, or secret change). Detail and the EX1 to EX18 ledger are in `phase-07-m6-tui_REPORT_17-10-26.md`.

| # | What deviated | Why | Impact |
|---|---|---|---|
| 1 | The product code was written before the tests, so the red-first runs were not literal. | The `App` state machine and the tests were developed in one compile chain; writing tests first against non-existent types gave no signal. | Red evidence was obtained by mutation instead: four deliberate breaks each turned the named test red, then were reverted (listed in the report). |
| 2 | The parity table (`Action::core_entry`) sits between `BEGIN parity table` and `END parity table` markers in `actions.rs`, and the test removes that block before it looks for the TUI needles. | The first version let every needle match itself in the table (vacuous green, caught by the mutation check). | `tui_parity_*` is now non-vacuous (a wrong needle fails). |
| 3 | The write scan (`tui_parity_tui_never_writes_files`) flags `std::fs`, `fs::write`, `fs::create_dir`, `fs::remove_`, `fs::rename`, `fs::copy`, `File::create`, `OpenOptions`, `hard_link`, `set_permissions`, `symlink(`, and any `fsops::` token except `RealFs`. | EX4 listed bare words (`remove_`, `rename`, `copy`) that would match ordinary identifiers; the `fs::` prefix keeps the scan precise. | Same intent, fewer false hits. |
| 4 | A footer message ("no skill sources yet: run unam source add <alias> <path>") shows when the config has no source. | Without it a first run shows an empty picker with no explanation. | Text only; no new action. |
| 5 | `esc` in the picker clears the query, the category filter, and both pending sets in one press (then quits on the next). | M6-5's message says "esc clears"; one reset keeps that message true. | Documented in help. |
| 6 | The result view follows spec 6: `q` quits, `enter` returns (the plan's contract line "q/enter" was ambiguous). | Spec 6 is explicit. | None. |
| 7 | The code-reviewer, simplifier, and scenario/security skill invocations were done as a self-review, not as spawned agents. | This EXECUTE agent has no agent-spawn tool. | The independent EVL run (vc-tester) still follows; STRIDE-style notes are in the report. |

---

## Next Step

EXECUTE is done (code). Run the independent EVL confirmation (vc-tester), then UPDATE PROCESS. Taylor's manual AC-44 terminal run stays a Hybrid residual (backlog stub).
