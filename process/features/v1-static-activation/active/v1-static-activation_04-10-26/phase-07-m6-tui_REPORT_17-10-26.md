---
phase: phase-07-m6-tui
date: 2026-10-17
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md
---

# Milestone M6 (TUI): EXECUTE report

**Bottom line:** the picker is built and green. Bare `unam` on a terminal opens a three-pane picker; the query, space, enter, enter path yields an active skill in the automated test (AC-44, Hybrid). G1 to G6 are green at EXECUTE (692 tests, 655 before). One residual stays open and makes the closeout CONDITIONAL: Taylor's manual real-terminal run of AC-44 (steps in "For Taylor" below, backlog stub filed). EVL (independent vc-tester re-run) is next.

Code commit: see "Commits". Base for diffs: `9a4e9df`.

## What Was Done

- **A1 dependency:** `ratatui = { version = "0.30.2", default-features = false, features = ["termion"] }` in workspace `Cargo.toml`; `ratatui.workspace = true` in `crates/unam-cli/Cargo.toml`. One unlocked `cargo check -p unam` updated `Cargo.lock` from 141 to 180 packages (39 added, 0 removed; names identical to the VERDICT list: allocator-api2 approx by_address castaway compact_str darling darling_core darling_macro deranged either foldhash ident_case indoc instability itertools kasuari libm line-clipping lru num-conv num_threads numtoa palette palette_derive palette_math powerfmt ratatui ratatui-core ratatui-termion ratatui-widgets rustversion static_assertions strum strum_macros termion time time-core unicode-truncate unicode-width). No `crossterm`, `mio`, `socket2`, `signal-hook`, `signal-hook-registry` in the lock. `no_network_dependencies` and `no_network_symbols` green with `phase1_checks.rs` unchanged apart from the new test.
- **New modules** `crates/unam-cli/src/tui/`:
  - `mod.rs`: `is_interactive`, `root_resolves`, `run`.
  - `term.rs`: raw mode and alternate screen through termion drop guards, blocking `stdin().keys()` loop, panic hook (`restore_bytes`, `install_panic_hook`).
  - `app.rs`: the state machine (`Mode` Loading, Picker, Plan, Applying, Result, Interrupted, Bundles, Naming, Help), `handle_key`, `run_pending`, pending add and removal sets, category tree with `3/12` counts, markers.
  - `actions.rs`: `Action` enum with the parity table, and `Session`, the only module that calls `unam-core`.
  - `picker.rs`, `views.rs`: drawing (22/48/30 panes, footer, preview, plan, result, modal, overlays, help).
  - `tests.rs`: 35 in-crate tests; `snapshots/`: 6 new `.snap` files.
- **Routing:** `cli/mod.rs` bare branch opens the picker only when stdin and stdout are terminals, `--json` is absent, and a project root resolves; otherwise the old usage plus status output, exit 0. `main.rs` gained `mod tui;`.
- **Other tests:** `tests/usage.rs::bare_unam_non_tty_prints_usage_and_status_exit_0`; `tests/phase1_checks.rs::no_panic_abort_profile` (AC list unchanged, 1 to 43 and 53).
- **Core:** untouched (`git diff 9a4e9df -- crates/unam-core` is empty). No catalog change, so `perf_cold_scan` was not rerun.

### Mid-implementation progress note (written at about 50 percent)

Status then: A1, B1, B2, B3 done, A2 and A3 written, tests not yet written (about 8 of 17 checklist units). No deviation yet. The rest followed the plan order: tests, snapshots, perf, closeout. Not a user pause (mid-phase pauses are banned).

## What Was Skipped or Deferred

- **D1, Taylor's manual real-terminal run of AC-44:** outstanding by design (D-TEST). Backlog stub: `process/features/v1-static-activation/backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md`. The gate is CONDITIONAL, never a Known-Gap pass.
- Linux and `/usr/bin/time -v` (parser unit-tested against a Linux sample, not run), real 1.89 toolchain compile, wide Unicode input, bracketed paste, other terminals: named residuals from the validate contract, unchanged.
- No registry annotation (`phase-blast-radius-registry.md` `status: DONE`): that is written on EVL green by UPDATE PROCESS.
- No context-doc edits (`process/context/` is UPDATE PROCESS territory): `all-context.md` and `tests/all-tests.md` still say crossterm and 655 tests.

## Test Gate Outcomes

Run with `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no`, from the repo root, after the last code change.

| Gate | Command | Real exit | Result |
|---|---|---|---|
| G1 | `cargo fmt --all --check` | 0 | clean |
| G2 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | clean |
| G3 | `cargo test --workspace --locked` | 0 | 692 passed, 0 failed, 0 ignored (655 before: +35 `tui::tests`, +1 `bare_unam_non_tty_*`, +1 `no_panic_abort_profile`) |
| G4a | `cargo test -p unam --locked --release --bin unam perf_warm_start_picker -- --nocapture` | 0 | median 42.4 ms (runs 44.7, 42.1, 42.4, 42.6, 42.1); limit 50 ms |
| G4b | `cargo test -p unam --locked --release --bin unam perf_memory_picker -- --nocapture` | 0 | peak RSS 8.8 MB; limit 50 MB |
| G5a | `cargo test --workspace --locked -- --list`, name matches | 0 | ac_44_ 2, tui_parity_ 4, tui_snapshot_ 6 (needs at least 5), tui_state_ 16, tui_term_ 3, perf_warm_start_picker 1, perf_memory_picker 3 (parent, child, parser), bare_unam_non_tty_ 1, no_panic_abort_profile 1, no_network_dependencies 1, no_network_symbols 1, phase1_ac_coverage 1; 692 tests listed |
| G5b | `! grep -rEq '#\[ignore' crates` | 0 | none |
| G5c | `git diff --diff-filter=MD --exit-code 9a4e9df -- crates/unam-cli/tests/snapshots` | 0 | the 39 existing snapshots unchanged |
| G5d | `phase1_checks.rs` `wanted` list | n/a | not in the diff (grep for `wanted` in the diff finds nothing); `phase1_ac_coverage` passes |
| G5e | read each new `.snap` | n/a | all six read; no temp path, no timestamp (plus a run with a 30-character longer `TMPDIR` passed unchanged) |
| G6 | `! grep -rE 'async_stdin\|crossterm' crates/*/src Cargo.toml crates/*/Cargo.toml` | 0 | no hit |
| G6 | `! grep -qE '^name = "(crossterm\|mio\|socket2\|signal-hook)"' Cargo.lock` | 0 | none |
| G6 | `git diff --exit-code 9a4e9df -- <five protected docs>` | 0 | unchanged |
| extra | `git diff --check` | 0 | clean |

Perf notes: the warm-start number is dominated by `Catalog::load` with a warm cache (about 42 ms; the picker adds about 1 ms, one draw 0.5 ms). It is in process, so exec and dynamic-link time is excluded. One extra release run under other load gave a median of 49.9 ms: the margin is thin. Backlog stub `process/features/v1-static-activation/backlog/m6-warm-start-margin_NOTE_17-10-26.md`. Peak RSS is of the test process re-run under `/usr/bin/time -l` (bytes, macOS).

### Red-first record

The plan asked for the state-machine tests and the parity table red-first. They were **not** written first (see Plan Deviations 1). Red evidence by deliberate mutation, each reverted and re-run green:

| Mutation | Test that went red |
|---|---|
| Plan view: `enter` no longer applies | `ac_44_query_space_enter_enter_yields_active_skill` (`the second enter applies`, left: Plan) |
| Parity table: Verify needle changed to a non-existent function | `tui_parity_every_action_maps_to_a_core_function_the_cli_calls` (`tui/actions.rs does not contain ...`); the first attempt stayed green because every needle matched itself in the table, which exposed and fixed a vacuous check |
| `panic = "abort"` appended to the root `Cargo.toml` | `no_panic_abort_profile` |
| Bare routing always interactive | `bare_unam_non_tty_prints_usage_and_status_exit_0` (exit 1 instead of 0) |

## Plan Deviations

All within the blast radius; none in a hard-stop class (no schema, auth, public API, container, or secret change). Also written to the plan's `## Deviations`.

1. Product code before tests; red evidence by mutation (above).
2. Parity table lives between `BEGIN parity table` and `END parity table` markers in `actions.rs`; the test removes that block before searching for the TUI needles (the first version matched itself).
3. The write scan uses precise needles (`std::fs`, `fs::write`, `fs::create_dir`, `fs::remove_`, `fs::rename`, `fs::copy`, `File::create`, `OpenOptions`, `hard_link`, `set_permissions`, `symlink(`, any `fsops::` token except `RealFs`) instead of the bare words `remove_`, `rename`, `copy` that EX4 listed (they would match ordinary identifiers).
4. A footer message appears when no source is registered ("no skill sources yet: run unam source add <alias> <path>"): text only.
5. `esc` in the picker clears the query, the category filter, and both pending sets in one press, then quits on the next press (M6-5's message says "esc clears").
6. Result view follows spec 6 (`q` quits, `enter` returns); the plan's contract line was ambiguous.
7. Review gates (code-reviewer, simplifier, scenario, security) were a self-review: this agent cannot spawn agents. STRIDE-style notes: tampering is limited to `change::run`/`resume`/`rollback` and `bundle::crud::save` (the parity tests prove no other write path exists in `tui/`); information disclosure: no network crate, no network symbol; spoofing and untrusted text: skill name, description, category, source alias, file names, messages, and result text go through `output::neutralize` or `head` (test `tui_state_untrusted_text_is_neutralized` asserts `\x1b` shows as text and no raw escape reaches the buffer); denial of service: a blocking apply, documented; terminal left raw on external SIGTERM or SIGINT, documented limit, not fixed.

## EX1 to EX18 ledger

| EX | Status | Note |
|---|---|---|
| EX1 order | Applied, with a small deviation | Order A1, B (module list and `mod tui;` first), A2, A3, C. `cargo check -p unam` ran after A1 and once after writing B1 to A3 together, not after each of B1, B2, B3, A2 individually. |
| EX2 context | Applied | `Session` in `actions.rs` is built from core (`Dirs::from_process_env`, `Config::load_or_create`, `Catalog::load` with `LoadOptions::product`, `change::resolve_root`). `Ctx`, `followup_lines`, `shared_consumers` were not made `pub(crate)`; the adapter calls are repeated in `Session::followups` and `Session::shared_consumers`. `cli/change.rs` untouched. |
| EX3 parity | Applied, plus Plan Deviation 2 | Two-column needle table; no wildcard arm; `ALL` length 13 equals a hand-written 13; bogus-needle control test; distinct-pair check. |
| EX4 no writes | Applied, plus Plan Deviation 3 | Scans `src/tui/*.rs` except `tests.rs`; needles built from parts; control case; at least 5 files asserted. |
| EX5 perf tests | Applied | `!cfg!(debug_assertions)` switch: 50 ms release and 5 s debug (warm start), 50 MB release and 200 MB debug (memory); `/usr/bin/time -l` or `-v` through `Command::output()`; child marker line, exit success, RSS above 1 MB asserted; `self-skip` print if `/usr/bin/time` is missing; child self-skips without the variable; priming load first. |
| EX6 forbidden words | Applied | G6 grep green; none of the `no_network_symbols` needles in any `src` file. |
| EX7 panic hook | Applied | `restore_bytes`, `install_panic_hook`, `tui_term_panic_hook_restore_bytes`, `..._writes_restore_before_message` (child re-exec, exit 101, bytes before message), child self-skips. The hook writes to stdout and stderr. |
| EX8 key loop | Applied | Blocking `stdin().keys()`, one thread, draw before the first key and after each key, `Key::Ctrl('c')` quits, every printable `Key::Char` appends to the query (search focus). |
| EX9 AC-44 details | Applied | Keys after the query are exactly Space, Enter, Enter; `y` applies, `n` and `esc` return, result `enter` returns, tested. |
| EX10 bare routing | Applied | Both stdin and stdout terminals, no `--json`, `change::resolve_root` succeeds (`tui::root_resolves`); `usage_text` and `status::bare` untouched; `bare_unam_prints_usage_without_config` stays green. |
| EX11 interrupted modal | Applied | `Session::journal_target` loops the adapters with `has_journal(root, Some(id))`; the target is named in the modal; resume and rollback get `Some(id)`; `Locked` and other errors become a footer message, no `unwrap`. |
| EX12 fixtures | Applied | `tempfile` plus `Dirs::resolve(Env::from_map(..))` in `tests.rs`; a `FailAt` fault wrapper over `RealFs` makes the interrupted journal; the real HOME is never read. |
| EX13 snapshots | Applied | 100 by 30 `TestBackend`, root and library paths replaced by `<project>` and `<library>` before render, six `.snap` files written once with `INSTA_UPDATE=always`, read, then gated with `INSTA_UPDATE=no`. |
| EX14 lockfile | Applied | 141 to 180 packages, 39 added (VERDICT names), none denied; network tests green; `Cargo.lock` committed with the code. |
| EX15 untrusted text | Applied | `tui_state_untrusted_text_is_neutralized`. |
| EX16 abort profile | Applied | `no_panic_abort_profile` reads root `Cargo.toml`, `dist-workspace.toml`, each crate manifest, and `.cargo/config*` when present; matcher unit-tested with positive and negative samples. |
| EX17 report to Taylor | Applied | See "Spec deviations and decisions for Taylor". |
| EX18 do not edit | Applied | Protected docs, `docs/`, `spike/`, the skill library, the 39 snapshots, `phase1_ac_coverage`'s list: untouched (G5c, G5d, G6). |

## Spec deviations and decisions for the M7 docs (and for Taylor)

These record tension between the picker and the protected spec and PRD. They were not resolved in the protected files.

1. **Backend: termion, not crossterm** (spec 1.4 and all-context.md name `ratatui` plus `crossterm`). Reason: the ratatui crossterm backend always turns on crossterm's `events`, which pulls `mio`, which `no_network_dependencies` denies (the VERDICT proved this with a negative control). Known limits: resize shows on the next key press; external SIGTERM or SIGINT leaves the terminal raw. Panic restores through a hook plus drop guards; `panic = "abort"` is banned by `no_panic_abort_profile`.
2. **No `e` key** (spec 6 plan view lists `e` to toggle strategy): dropped, v1 is symlink-only (D-EKEY).
3. **Static loading frame** (spec 6 "cold scan runs with a spinner"): the picker is single-threaded, so `Loading` draws a "Loading catalog..." frame and then calls the synchronous `Catalog::load` (M6-8).
4. **No match highlighting** (spec 6 does not require it but the search API gives scores only): `nucleo-matcher` search returns `Hit { id, score }`, no indices (M6-2).
5. **Lock only during apply** (spec 701 remedy "close the open picker" is moot): the project lock is held only inside `change::run`, never while the picker is open; a `Locked` error shows in the footer (M6-4).
6. **Mixed edits** (M6-5): `change::Edit` carries one kind per apply, so a pending set that mixes additions and removals shows "apply additions and removals one at a time; esc clears" and opens no plan. A combined edit would need a new core entry; deferred.
7. **Keys** (M6-12): the search box has focus at start; every printable character except space is typed into the query, so `b`, `t`, `?` act only when the category or results pane has focus (`tab` cycles). In the plan view `enter` or `y` applies (AC-44 says "enter twice"; spec 6 says `y`). `esc` clears the filter and the pending sets, then quits. `ctrl-c` quits from every mode.
8. **Parity needles** (EX3): `change::prepare` is reached by the CLI only inside `change::run`, and the target cycle (`t`) is reached through `adapter_for`; the parity table names those CLI needles.
9. **Bundles** (M6-6): `b` overlay lists bundles; enter uses one (replace), `a` adds, `s` prompts a name and saves the pending-selected skills; no bundle editing.
10. **Interrupted modal** (M6-7): `r` resume, `u` rollback, `esc` quits; the key press is the confirmation.
11. **Catalog estimate in the footer**: the footer shows pending counts, active count, and the catalog size; the token estimate appears in the plan view (from the plan), not in the picker footer.
12. **Result view**: the six-state list after an apply is `output::render_records` after a fresh `verify`.

Mostly untested by design: Linux, other terminals, wide Unicode input, bracketed paste.

## Test Infra Gaps Found

- Bin-only crate, so TUI tests are in-crate (a future `[lib]` split would allow integration tests). Known from the plan.
- No pty harness (R8): raw mode, key decoding from real bytes, and resize are covered only by Taylor's manual run.
- `cargo-insta` is not installed: snapshots written with `INSTA_UPDATE=always`, then read.
- Warm-start margin is thin (about 42 of 50 ms); stub filed.
- Classification of the AC-44 real-terminal half: `test-breakage` does not apply; it is a named Hybrid residual (D-TEST).

## Closeout Packet

- **Selected plan:** `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-07-m6-tui_PLAN_04-10-26.md`
- **Classification:** Keep in active/testing until EVL (independent vc-tester run) is green; after that `Ready for UPDATE PROCESS archival` with the CONDITIONAL residual recorded.
- **Finished:** A1 to C3, D2; code committed.
- **Verified here (EXECUTE agent):** G1 to G6 (table above). **Not yet independently verified:** everything; EVL is next and counts only when a spawned vc-tester does it.
- **Unverified by design:** Taylor's manual run (D1); Linux; resize; external signals.
- **Remaining:** EVL; UPDATE PROCESS (umbrella state, `all-context.md` stack line: ratatui termion, tests context count 692, registry `status: DONE`, validators); Taylor's manual run.
- **Follow-up stubs created:** `process/features/v1-static-activation/backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md`, `process/features/v1-static-activation/backlog/m6-warm-start-margin_NOTE_17-10-26.md`.
- **CONTEXT_PARTIAL:** none.
- **Single best next valid state:** EVL (spawn vc-tester with the validate-contract gates G1 to G6), then UPDATE PROCESS.

## Forward Preview

### Test Infra Found

- In-crate TUI tests with `TestBackend` plus a `FailAt` fault wrapper (`tui/tests.rs`) for interrupted journals; `perf_memory_picker` re-runs the test binary under `/usr/bin/time`. M7 can reuse the same pattern for any docs-generation test that needs a child process (use `Command::output()`, never the banned spawn text).
- Full workspace test count is now 692; the fault-matrix suites are unchanged (run `UNAM_FULL_FAULT_MATRIX=1` only if apply paths change; they did not).

### Blast Radius Changes

- New crate dependencies: ratatui 0.30.2 (termion feature) and its 38 transitive packages. M7's `cargo-dist` and docs must describe termion, not crossterm.
- `docs/adapters.md` and the README (M7) need the spec deviations above (termion, no `e`, static loading frame, no highlighting, lock only during apply, mixed-edit rule, key routing).
- `dist-workspace.toml` is scanned by `no_panic_abort_profile`: M7's release profile must not set `panic = "abort"`.

### Commands to Stay Green

```bash
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p unam --locked --release --bin unam perf_warm_start_picker -- --nocapture
cargo test -p unam --locked --release --bin unam perf_memory_picker -- --nocapture
```

In-crate TUI tests: `cargo test -p unam --bin unam tui::`.

### Dependency Changes

- `Cargo.lock`: 141 to 180 packages. M7 must not re-resolve the lock in a way that adds `mio`, `socket2`, or `signal-hook`.
- PRD Q11 (crates.io name and license) is still open and is an M7 entry condition.

## For Taylor: manual AC-44 run in a real terminal

Run this in your own terminal. It uses a temp directory, a tiny temp library, and overridden config paths, so it never touches this repository, your real config, or `~/Developer/01-AI/skillzone`. First produce the binary (this does not run `unam`): in the repo, `cargo build --release -p unam`. The binary is then `/Users/databased/Developer/01-AI/unambiguous/target/release/unam`.

```bash
# 1. A scratch world (never this repository)
T=$(mktemp -d)
export UNAM_CONFIG_DIR=$T/config UNAM_CACHE_DIR=$T/cache XDG_DATA_HOME=$T/data
U=/Users/databased/Developer/01-AI/unambiguous/target/release/unam

# 2. A tiny library with two skills, and a project with a .git marker
mkdir -p $T/lib/sdlc/code-review $T/lib/testing/pytest-fixtures $T/proj/.git
printf -- '---\nname: code-review\ndescription: Review a diff.\n---\n\n# code-review\n' > $T/lib/sdlc/code-review/SKILL.md
printf -- '---\nname: pytest-fixtures\ndescription: Pytest fixtures.\n---\n\n# pytest-fixtures\n' > $T/lib/testing/pytest-fixtures/SKILL.md

# 3. Register the library (scratch config only) and go to the project
cd $T/proj && $U source add lib $T/lib

# 4. The scenario: bare `unam` opens the picker
$U
```

In the picker:

1. The three panes appear (categories left, search and results center, preview right); the cursor is in the search box.
2. Type `code-review`. One result shows.
3. Press `space`: the row shows `[x]` and the footer says `1 to add`.
4. Press `enter`: the plan view opens (title "Plan", a `+ sdlc/code-review` line).
5. Press `enter` again: the result view opens and lists `active  lib:sdlc/code-review`. That is the primary scenario in four keystrokes after the query (space, enter, enter).
6. Press `enter` to return to the picker (row shows the filled-circle marker), then `esc` (clears the query), then `esc` again to quit.

Check after quitting:

- The shell works normally: you can see what you type, line editing works, and the cursor is visible (the terminal was restored).
- `ls -l $T/proj/.claude/skills/` shows `code-review` as a symlink into `$T/lib/sdlc/code-review`.
- Optional: run `$U` again, resize the window, then press any key: the layout redraws for the new size on that key press (known limit: not before).
- Optional: in the picker press `tab` twice, then `t` (target switches to `agents-neutral`; footer shows it), `b` (bundle overlay opens), `esc`, `?` (help), any key.

Cleanup: `cd $T/proj && $U deactivate --yes`, then `rm -rf $T` and `unset UNAM_CONFIG_DIR UNAM_CACHE_DIR XDG_DATA_HOME`.

Record in the report (or tell me): whether the picker opened, whether the four-key path worked, and whether the terminal came back clean.

## Commits

- Code: `37292c2` feat: M6 interactive picker (ratatui termion backend), on `main`, not pushed. Includes `Cargo.toml`, `Cargo.lock`, the `tui/` modules and snapshots, the routing change, and the two test additions.
- Process: a `process:` commit on `main` follows (plan ticks and Deviations, this report, umbrella state, the VERDICT, `results-m6.tsv`, two backlog notes); its hash is in the EXECUTE handoff.

## Review fix round

Scope: the six code-review findings only. Code commit: see "Commits" below. Base for diffs: `0425429`.

| # | Severity | Fix | Test (red first) |
|---|---|---|---|
| 1 | Medium | Catalog and config warnings now show in the picker footer as `N warning(s): warning[code] where: message` (first warning plus a count). Text comes from the CLI's own `diag_line`; the set comes from a new `output::catalog_warning_set`, which `cli::catalog_warnings` now also calls (no duplicated logic). The summary is a standing note: it survives key presses and shows whenever no transient message is set. Skill-level diagnostics stay out, same as the CLI. | `tui_state_unreachable_source_warning_shows_in_footer` (red: no "1 warning"); `tui_state_no_warning_text_without_diagnostics` (guard, green from the start) |
| 2 | Low | `Session::apply` takes the confirmed plan text. Inside `decide` the plan `change::run` re-prepared under the lock is rendered with `render_plan` and compared; a mismatch returns `Decision::Decline` and flags it. `do_apply` then reopens the plan view with the fresh plan and the footer says "the plan changed; review it again". No core change was needed. | `tui_state_plan_changed_between_preview_and_apply_is_not_applied` (red: skill linked anyway) |
| 3 | Low | `views::draw` clamps plan and result scroll to the wrapped height. The wrapped row count comes from drawing the text into a scratch `Buffer` with the real `Paragraph` and `Wrap`, so it matches what is shown. The source-line clamp in `scroll_plan` is gone (draw is the bound). | `tui_state_plan_scroll_reaches_the_tail_of_wrapped_text`, `tui_state_result_scroll_cannot_pass_the_end` (red: tail never visible; scroll ran away) |
| 4 | Low | `Session::initial_target` returns `(TargetId, Option<String>)`. On `resolve_target` error it keeps the first-adapter fallback and sets a standing footer note: `ambiguous target: using claude-code, press t to switch (<core message>)`. `t` clears it. | `tui_state_ambiguous_initial_target_names_the_ambiguity` (red: no note) |
| 5 | Low | `tui_state_ctrl_c_quits_from_every_mode` now covers Picker, Help, Bundles, Naming, Plan, Result, Loading, Interrupted. | Green from the start (the quit check runs before mode dispatch); this is added coverage, not a behavior change. |
| 6 | Parity | `strip_comments` (drops everything from `//` on each line, so `//`, `///`, `//!`) is applied to the TUI source (parity table block removed first) and to every CLI source before the substring search. Block comments are not handled; a `//` inside a string literal also cuts the line, which only makes the check stricter. | `tui_parity_comment_only_needle_fails` (red with a pass-through stub: raw check passes a comment-only needle; also covers the CLI side and code-before-trailing-comment). The write scan (`tui_parity_tui_never_writes_files`) is unchanged. |

Red run (before the fixes, `cargo test -p unam --bin unam tui_`): 6 failed, 30 passed. Failed: `tui_parity_comment_only_needle_fails`, `tui_state_unreachable_source_warning_shows_in_footer`, `tui_state_plan_changed_between_preview_and_apply_is_not_applied`, `tui_state_ambiguous_initial_target_names_the_ambiguity`, `tui_state_result_scroll_cannot_pass_the_end`, `tui_state_plan_scroll_reaches_the_tail_of_wrapped_text`. After the fixes: `cargo test -p unam --bin unam` 54 passed, 0 failed.

### Gates

| Gate | Result |
|---|---|
| G1 `cargo fmt --all --check` | exit 0 |
| G2 `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| G3 `cargo test --workspace --locked` | 699 passed, 0 failed, 0 ignored (692 before this round) |
| `perf_warm_start_picker` (release), median of three runs | 47.4 ms (runs' medians 47.8, 47.4, 46.1); limit 50 ms. Same machine, same hour, HEAD `0425429` without my changes: 46.0 ms (47.1, 46.0, 45.4). The change is about 1 ms, inside run-to-run noise; the 42.4 ms in G4a above was a quieter machine. Headroom is thin (about 2.6 ms); a loaded machine can cross 50 ms. |
| `perf_memory_picker` (release) | peak RSS 9.1 MB, ok |
| 39 CLI snapshots | `git diff --diff-filter=MD --exit-code 9a4e9df -- crates/unam-cli/tests/snapshots` exit 0 |
| 6 TUI snapshots | unchanged (no file modified; no footer warning appears in any of them) |
| G6 greps | no `crossterm`, `async_stdin`, `.spawn(` in crates/*/src or Cargo files; no denied package in `Cargo.lock` (`Cargo.lock` untouched) |
| Protected docs vs `8d688e1` | no diff |
| `git diff --check` | exit 0 |
| `#[ignore` | none |

### Not fixed (Low, per instructions)

- Pending work (`run`) is cleared without a prompt in some paths.
- Typeahead replay (keys typed during `Applying` are replayed after).
- Panic-hook staircase output (raw mode still on when the hook prints).

### Deviations

- D-R1 (finding 4, as instructed): the fallback to the first adapter is kept; the CLI errors in the same case. The picker now says so in the footer.
- D-R2 (finding 1): the warning summary uses the CLI's class set (source and config), so skill-level diagnostics (malformed `SKILL.md`) still do not appear in the picker. `unam search` and `show` behave the same way.
- Note: finding 2's comparison is rendered-text equality, so any change in the lines the user sees triggers a re-review, and a change that does not alter the text is not detected (it cannot matter to what the user confirmed).

### Forward Preview

- Test Infra Found: none new.
- Blast Radius Changes: `crates/unam-cli/src/output.rs` (new `catalog_warning_set`), `cli/mod.rs` (delegates to it), `tui/{actions,app,mod,picker,views,tests}.rs`.
- Commands to Stay Green: G1 to G3, release `perf_warm_start_picker` and `perf_memory_picker`, the snapshot diff.
- Dependency Changes: none.

CONTEXT_PARTIAL: none. Follow-up stubs created: none. The manual AC-44 run by Taylor remains the open residual.
