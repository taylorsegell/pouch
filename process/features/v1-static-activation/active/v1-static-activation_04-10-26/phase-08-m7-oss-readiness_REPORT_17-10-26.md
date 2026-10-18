---
phase: phase-08-m7-oss-readiness
date: 2026-10-17
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md
---

# Milestone M7 EXECUTE report: OSS readiness

TL;DR: M7 is implemented as planned. Gates G1 to G3 are green (710 tests, 0 ignored), `dist plan`, `dist generate --check` and the `dist` profile check all exit 0, and the 45 snapshot files are byte-identical to BASE. Open: Linux and CI (Docker daemon down), the first real release run, Taylor's manual terminal runs. One plan wording defect: the literal G6 `.spawn(` grep matches a pre-existing test line. Code commit `9897bfe`; the process commit follows this report. EVL then ran (two iterations plus a test-strengthening commit); final classification: VERIFIED (WITH_GAPS). See "EVL outcome and final classification" at the end.

## Context Envelope

| Field | Value |
|---|---|
| feature | v1-static-activation |
| phase | EXECUTE |
| session-goal | Implement milestone M7 (OSS readiness) exactly as the plan says; no push, tag, publish, or tap |
| branch | main |
| worktree | main |
| context-group | product, tests, planning |
| blast-radius-packages | root `Cargo.toml`, `crates/unam-cli`, `crates/unam-core` (manifest only), docs, release files |
| active-plan | `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md` |
| test-runner | cargo test |
| validate-contract | inline in the plan, Gate PASS, inner-pvl: phase-8, 2026-10-17 |

Tier-0 restatement (E-S0): executing the M7 plan above, steps A to F, honoring G1 to G6 and EX1 to EX17.

## What Was Done

| Step | Result |
|---|---|
| A0 | BASE `fadca4b`; `Cargo.lock` had 180 package names |
| A1, A2 | Root `Cargo.toml`: `license = "MIT OR Apache-2.0"`, `repository`, `readme`, `authors`, and `[profile.dist]` with `inherits = "release"` (no `panic` key). Both crates inherit these and add `keywords` (agent, skills, claude-code, cli, symlink) and `categories` (command-line-utilities, development-tools). CLI package renamed `unambiguous` (`[[bin]] name = "unam"` kept) with `[package.metadata.dist] dist = true`. `publish = false` untouched. |
| A3 | Unlocked `cargo check --workspace` once. Lock name list: 180 before, 180 after, one difference (`unam` to `unambiguous`). Denied crates (`crossterm`, `mio`, `socket2`, `signal-hook`): 0. The textual lock diff is 34 lines because two adjacent blocks swapped order (EX2). |
| A4 | The 6 `tui_snapshot_` tests pass under the old `unam__tui__tests__*.snap` names. No snapshot renamed. `git status --short` over both snapshot directories prints nothing; the blob hash of all 45 files (39 CLI, 6 TUI) equals BASE. |
| A5 | `phase1_checks.rs` line 60 now asserts `"unambiguous"`. `-p unam` became `-p unambiguous` in `phase1_exit.rs` (line 9), `process/context/all-context.md` (line 191), and `process/context/tests/all-tests.md` (lines 19, 21, 23, 27, 72, 77, 78). `all-product.md` held no match. |
| B1, B2 | `LICENSE-MIT` (standard text, "Copyright (c) 2026 Taylor Segell"). `LICENSE-APACHE` is a byte-identical copy of `clap-4.6.7/LICENSE-APACHE` (202 lines, `cmp` identical): 9 numbered sections, `END OF TERMS AND CONDITIONS`, the appendix. It contains no em dash and no non-ASCII character. |
| C1 | `cargo install cargo-dist --version 0.32.0 --locked` installed `~/.cargo/bin/dist` only (no `cargo-dist` executable); `dist --version` prints `cargo-dist 0.32.0`. Writes only under `~/.cargo`. |
| C2 | `dist-workspace.toml` rewritten per EX6: `cargo-dist-version = "0.32.0"`, `ci = "github"`, `installers = ["shell", "homebrew"]`, `tap = "taylorsegell/homebrew-tap"`, the four targets, `install-path = "CARGO_HOME"`, `[workspace] members = ["cargo:."]`. No `publish-jobs`. |
| C3 | `dist generate` first: wrote only `.github/workflows/release.yml`. `release.yml` triggers on `pull_request` (runs `dist plan`, `publishing: false`) and on a version-tag push; nothing runs until a tag is pushed. |
| C4 | `dist plan` exit 0, `dist generate --check` exit 0, `cargo check --profile dist --locked --offline -p unambiguous` exit 0. See "dist outputs". |
| D1 | `docs/json.md`: envelope section, then one section per command; 38 markers, one per JSON snapshot. |
| D2 | `docs/adapters.md`: both M2 sections and all evidence tables kept (they already matched `doctor --evidence`); intro, a Linux sentence in the M0 outcome, and the Maintenance section updated. |
| D3 | `README.md` rewritten (primary scenario first, install from source, non-interactive first activation, command table, behavior and honesty notes, platforms and evidence, known limits, documents, license). |
| D4 | `CHANGELOG.md`, Keep a Changelog, `## [0.1.0] - Unreleased`, milestones M0 to M7 in user terms. |
| E1 | Four test files, 11 tests (docs_json 3, docs_adapters 2, docs_honesty 3, release_files 3). Red-first record below. |
| E2 | G1 to G3 green (below). |
| F1 | S11 walkthrough: 9 seconds end to end (details below). |
| F2 | Docker daemon still down; Known-Gap stub written; gate stays CONDITIONAL. |
| F3 | Release handoff written (see "For Taylor"). |
| F4 | This report. |

### Red-first record (EX1)

Test files were written before the rename, `docs/json.md`, the licenses, the README, `dist-workspace.toml` and `release.yml` existed. First run (`cargo test -p unam --no-fail-fast` over the four test binaries, before the rename):

| Test | First run |
|---|---|
| `docs_json_every_snapshot_is_documented` | FAILED (red) |
| `docs_json_every_marker_has_a_snapshot` | FAILED (red) |
| `docs_honesty_readme_carries_required_caveats` | FAILED (red) |
| `docs_honesty_no_forbidden_claims` | FAILED (red) |
| `release_files_license_and_metadata` | FAILED (red) |
| `release_files_dist_config` | FAILED (red) |
| `docs_adapters_cover_every_doctor_evidence_claim` | passed (EX1: cannot go red; the tables existed since M4 and M5; its value is the row-level match and the control) |
| The four `*_check_detects_*` controls | passed (pure helpers on synthetic input; green by construction) |

Output saved in the scratchpad (`red_run.txt`). After D1 to D4, C2 and C3 all 11 were green.

## What Was Skipped or Deferred

| Item | Why | Where recorded |
|---|---|---|
| Docker offline Linux test run (F2, D-DOCKER) | Daemon not running (`docker image ls`: cannot connect to `/var/run/docker.sock`). Not started, no container touched (EX14). | Backlog stub `process/features/v1-static-activation/backlog/m7-linux-ci-verification_NOTE_17-10-26.md` |
| macOS universal target | cargo-dist 0.32.0 rejects it (M7-4, EX12) | "Universal outcome" below; changelog known limits |
| Interactive half of S11 (bare `unam` picker) | Needs a real terminal (no pty harness) | Taylor's M6 run, stub `m6-ac44-manual-terminal-run_NOTE_17-10-26.md`; steps below |
| Reviewer read of the umbrella and context status lines | UPDATE PROCESS territory (EX17) | CONTEXT_PARTIAL list below |

## Test Gate Outcomes

Run with `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no`. All run by this agent (EXECUTE); the independent EVL re-run is the orchestrator's next step.

| Gate | Command | Exit | Result |
|---|---|---|---|
| G1 | `cargo fmt --all --check` | 0 | clean |
| G2 | `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 | clean |
| G3 | `cargo test --workspace --locked` | 0 | 30 suites, 710 passed, 0 failed, 0 ignored (699 + 11 new) |
| G4a | `dist plan` | 0 | `announcing v0.1.0`; `[bin] unam` under all four targets; `unambiguous.rb`, `unambiguous-installer.sh`, `source.tar.gz`, `sha256.sum` |
| G4b | `dist generate --check` | 0 | `release.yml` current |
| G4c | `cargo check --profile dist --locked --offline -p unambiguous` | 0 | `Finished dist profile` |
| G5 | `cargo test --workspace --locked -- --list` | 0 | all 15 named tests present (1 each), 6 `tui_snapshot_` names, 710 tests listed; 45 snapshot blobs equal BASE, `git status --short` over both snapshot directories empty |
| G6 | see below | mixed | all invariants hold except the literal `.spawn(` grep (deviation 1) |

G6 detail:

| Check | Result |
|---|---|
| `git diff --exit-code BASE --` over the five protected documents | exit 0 |
| `.spawn(` and `#[ignore` grep over `crates` | one hit: `crates/unam-cli/tests/phase1_checks.rs:223` `.spawn()`, present at BASE (the `no_background_process` control). No M7 file has either pattern. See Plan Deviations 1. |
| `Cargo.lock` package names | 180 before and after; only `unam` to `unambiguous` |
| `panic = "abort"` in any `Cargo.toml` or `dist-workspace.toml` | none (`.cargo/config.toml` does not exist); `no_panic_abort_profile` green |
| `git tag --list`, `git remote -v` | zero tags; remote unchanged (origin, GitHub) |
| `git push` | never issued |
| `git diff --check` | exit 0 |
| `git status` | every changed file is inside the Blast Radius (plus the umbrella state edit and the report you asked for) |

### dist outputs

`dist plan` (saved as `plan.txt`), after `dist generate`:

```
announcing v0.1.0
  unambiguous 0.1.0
    source.tar.gz
    unambiguous-installer.sh
    unambiguous.rb
    sha256.sum
    unambiguous-aarch64-apple-darwin.tar.xz         [bin] unam   [misc] LICENSE-APACHE, LICENSE-MIT, README.md
    unambiguous-aarch64-unknown-linux-gnu.tar.xz    [bin] unam   [misc] LICENSE-APACHE, LICENSE-MIT, README.md
    unambiguous-x86_64-apple-darwin.tar.xz          [bin] unam   [misc] LICENSE-APACHE, LICENSE-MIT, README.md
    unambiguous-x86_64-unknown-linux-gnu.tar.xz     [bin] unam   [misc] LICENSE-APACHE, LICENSE-MIT, README.md
```

Advisory WARN lines (EX11, recorded, not failures): Homebrew homepage not specified (adding `homepage` to `[workspace.package]` and both crates would clear it; not done, it needs the metadata test updated too); a Homebrew tap was specified but the publish job is disabled (unavoidable under D-BREW); no `rustup` on PATH for the cross targets (the plan never builds).

### Universal outcome (M7-4, EX12)

Wrote `universal2-apple-darwin` into `targets`, ran `dist plan`: exit 255, `x Unrecognized architecture: universal2`. Entry removed, file restored byte-for-byte, C4 re-run green. Two separate macOS archives are built instead. This closes umbrella open item 19 as a recorded gap.

### D-DISTFILES diff review

After `dist generate`: `git status` showed one new file beyond my own edits, `.github/workflows/release.yml`. `Cargo.toml` and `Cargo.lock` diffs equal my own edits (dist changed neither). `dist plan`, `dist generate --check` and the profile check changed no tracked file. Nothing needed reverting. The `[profile.dist]` block is the planned deviation from D-DISTFILES (F4).

### S11 walkthrough (F1)

Clean `mktemp -d` root; `HOME`, config, cache and bundles dirs inside it; `env -i` for every `unam` call; real `CARGO_HOME`/`RUSTUP_HOME`; two-skill temp library; temp project with an empty `.git` directory; no agent CLI run; never in this repository.

| README command | Exit | Note |
|---|---|---|
| `cargo install --path crates/unam-cli --locked --offline --root <temp>` | 0 | 8 s; `Installed package unambiguous v0.1.0 (executable unam)` |
| `unam source add lib <lib>` | 0 | `Skills found: 2` |
| `unam search review` | 0 | one row |
| `unam show lib:sdlc/code-review` | 0 | |
| `unam add lib:sdlc/code-review --yes` | 0 | symlink under `.claude/skills`; refresh note and gitignore lines printed |
| `unam status` | 0 | `active` |
| `unam doctor` | 0 | all `ok` or `info` |
| `unam deactivate --yes` | 0 | removal caveat printed; created directories removed |
| `unam add ... --target agents-neutral --yes` | 0 | symlink under `.agents/skills`; refresh "not documented" note |

Total 9 seconds, well under the 10-minute target. This measures a scripted run on this machine with a warm cargo cache and no network, not a stranger on a fresh machine; the cold-cache install time was not measured. The library tree was unchanged afterwards (two files, as created).

One observation, not a product change in M7: piping a command's output into `head` made `unam` panic with "failed printing to stdout: Broken pipe" (exit 101). It only happens when the reader closes early. Recorded under open residuals.

## Plan Deviations

1. **G6 literal `.spawn(` grep cannot pass.** `crates/unam-cli/tests/phase1_checks.rs:223` has a `.spawn()` in the existing `no_background_process` control, present at BASE (the same file builds the needle with `format!(".{}(", "spawn")` for the `src` scan, which does not cover `tests`). M7 added none. Within-blast-radius reading: the invariant "no new `.spawn(` or `#[ignore`" holds. Suggest the EVL run use `git diff BASE -- crates | grep` on added lines instead of a whole-tree grep.
2. **Context-doc prose left stale.** `process/context/tests/all-tests.md` line 23 still says "the CLI package is `unam`" after its `-p` strings were fixed. EX17 limits EXECUTE to the `-p unam` strings; UPDATE PROCESS should reword it.
3. **This plan's own `-p unam` mentions** (they describe the change) were left as they are.
4. No other deviation. Hard-stop class: none touched (no auth, schema, public API, container, secret, or external-integration change). The only network use was the C1 install into `~/.cargo`.

### EX1 to EX17

| Id | Status | Note |
|---|---|---|
| EX1 | Applied | Tests written before C2 and D1 to D4; red run recorded; `docs_adapters_*` could not go red (stated). The literal text `.spawn(` and the ignore attribute appear in no new file. |
| EX2 | Applied | Lock judged by the sorted name list (180/180, one name changed). |
| EX3 | Applied | Only `"unam"` to `"unambiguous"` in `no_network_dependencies`; `cargo fmt --all` run; `phase1_ac_coverage` untouched. |
| EX4 | Applied | `dist`/`target` commands were run from script files in the scratchpad; the hook blocked one inline command and it was re-run from a script. |
| EX5 | Applied | Installed 0.32.0, only `dist`; no `dist init`, `build`, `host`, `migrate` or `selfupdate`. |
| EX6 | Applied | `dist-workspace.toml` exactly as listed (plus a comment header). |
| EX7 | Applied | 38 markers, shared sections for the apply family, envelope keys checked once in `## Envelope`, header stripped. |
| EX8 | Applied | Row-level match by adapter section, backticked claim key and source on one line; 29 rows; `Fx` temp project; `Command::output()` path via `Fx`. |
| EX9 | Applied | `"publish": []` asserted; 2 packages, bin target `unam`, repository URL, `dist = true`, run through `Command::output()`. |
| EX10 | Applied | Byte-identical clap copy, 9 sections, appendix; MIT standard text. |
| EX11 | Applied | Warnings recorded; the optional `homepage` fix not taken. |
| EX12 | Applied | Tried once, 255 "Unrecognized architecture", removed, re-run. |
| EX13 | Applied | README uses the from-source install; installers and `cargo install unambiguous` only as future; walkthrough isolated. |
| EX14 | Applied | Daemon down: stub written, nothing started, no `docker exec`. |
| EX15 | Applied | Stale spec text listed below, spec not edited. |
| EX16 | Applied | Tension items are in "For Taylor". |
| EX17 | Applied | Committed on `main`, no push, tag, publish or tap; only `-p unam` strings changed in live docs. |

## Stale spec text (reported, never edited)

| Where | What it says | Reality after M1 to M7 |
|---|---|---|
| spec 1.4 | `crossterm`, `directories`, full `nucleo` | `ratatui` with the `termion` backend (crossterm pulls `mio`, denied); paths resolved in `paths.rs`; `nucleo-matcher` |
| spec 1.3 and 9 | package and binary `unam`, two crates by those names | package `unambiguous`, binary `unam`, library `unam-core` |
| spec 9 and umbrella item 19 | "macOS universal" release target | two macOS archives; cargo-dist 0.32.0 rejects the universal spellings |
| spec lines 840 to 844 | `cargo add -p unam ...` | `-p unambiguous` |
| spec 6 | plan view `e` key; `y` to apply | no `e` key (symlink-only v1); `enter` or `y` applies |
| PRD Q11 | crate name and license undecided | decided 2026-10-17: `unambiguous`, `MIT OR Apache-2.0` |
| `docs/PROJECT-BRIEF.md` open-question table | Q11 listed as open | decided |

## Test Infra Gaps Found

- No pty harness: the bare-`unam` picker half of S11 and AC-44 stay manual.
- Linux and CI have never run; Docker daemon down; stub written.
- `rust-version = "1.89"` has only been compiled by rustc 1.99.
- `dist plan` cannot prove that GitHub accepts the workflow, that cross builds link, or that the installer or formula installs.
- `docs_json_*` check key coverage, not the prose; `docs_honesty_*` check literal phrases only (the reviewer read below covers other wording).
- The whole-tree G6 `.spawn(` grep is not satisfiable (deviation 1).

### Reviewer read (Agent-Probe, S9 wording)

Read every line naming Cursor, Windsurf, or Linux in `README.md` and `docs/adapters.md`. README: line 77 says Linux is "not yet verified"; line 79 limits Cursor evidence to the `cursor-agent` CLI and says the Cursor editor and Windsurf are not tested. `docs/adapters.md`: the M0 outcome and both notes say Windsurf and the Cursor editor were never tested and that the Cursor spike results are for the CLI only; the rows marked `verified` for Cursor and Windsurf/Devin cite vendor documentation and say so in their text (`the editor is untested`, `Windsurf was not tested`). The one `assumption` row is labeled as such. No sentence claims Linux verified, Windsurf supported, or the Cursor editor supported. Judgment: honest as written. No em dash and no emoji in any file written.

## Closeout Packet

- Selected plan: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md`
- Classification: **Keep in active/testing** until the orchestrator's EVL confirmation run (vc-tester) passes; then `Ready for UPDATE PROCESS archival` with WITH_GAPS (Linux and CI, the first release run, Taylor's manual runs).
- Finished: steps A to F, 11 new tests, all docs and release files, local release verification.
- Verified by this agent: G1 to G3, G4a to G4c, G5, G6 (with deviation 1), the S11 scripted walkthrough.
- Not verified: Linux, CI, the 1.89 toolchain, the first real release run, the Homebrew formula, the interactive picker in a real terminal, a stranger's cold install time.
- Remaining cleanup: UPDATE PROCESS reconciles `all-context.md` status lines (M7 done, crate name, 710 tests, Q11 decided), `tests/all-tests.md` counts and line 23 prose, the umbrella program status; archive the plan and the report.
- Follow-up stub paths created: `process/features/v1-static-activation/backlog/m7-linux-ci-verification_NOTE_17-10-26.md`.
- Single best next state: EVL (spawn vc-tester to re-run G1 to G6, using a diff-scoped check for the `.spawn(` invariant).

CONTEXT_PARTIAL: `process/context/all-context.md` status and structure lines, `process/context/tests/all-tests.md` counts and line 23, and the umbrella `## Contradictions and Open Items` still describe the pre-M7 state (UPDATE PROCESS territory).

## Forward Preview

### Test Infra Found

`tests/docs_json.rs`, `docs_adapters.rs`, `docs_honesty.rs`, `release_files.rs` (11 tests); all use `tests/common` (`Fx`) or pure helpers; none uses the banned patterns. `release_files_license_and_metadata` runs `cargo metadata --no-deps --offline --locked` through `Command::output()`.

### Blast Radius Changes

Package name `unambiguous` (binary `unam`, snapshot prefix still `unam__`). Any new doc that lists `--json` output needs a marker section in `docs/json.md`; any change to an adapter's `evidence()` needs a matching row in `docs/adapters.md`; any new snapshot needs a `docs/json.md` marker or `docs_json_every_snapshot_is_documented` fails.

### Commands to Stay Green

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked          # 710 tests, 0 ignored
dist plan; dist generate --check         # with ~/.cargo/bin on PATH
cargo check --profile dist --locked --offline -p unambiguous
```

### Dependency Changes

None in the lockfile except the package rename (180 names before and after). Tool installed outside the repo: cargo-dist 0.32.0 in `~/.cargo/bin`.

## For Taylor

### Decisions and tension to confirm (EX16)

- Tap name `taylorsegell/homebrew-tap` is a guess from the repository owner.
- The plan's snapshot rename was wrong and was dropped; `phase1_checks.rs` had to change; `[profile.dist]` was added to the root `Cargo.toml`; the macOS universal target is unsupported by dist 0.32.0.
- `release.yml` carries `contents: write` (generator default) and also runs on every `pull_request`, but only as `dist plan` (`publishing` false).
- Before publishing to crates.io (not done, out of scope): `unam-core = { path = "crates/unam-core" }` in the workspace dependencies has no `version`, which `cargo publish` requires for a path dependency. Add `version = "0.1.0"` there and publish `unam-core` first. I did not test a publish dry run. (Done in EVL fix cycle 1: the version is now there; `cargo publish` and `cargo package` were still not run.)

### Release handoff, in order (you run these, when you are ready)

1. Confirm the tap name, the license choice, the copyright line, the repository URL and the keywords. Optionally add `homepage` to `[workspace.package]` and both crates (clears the Homebrew warning; update `release_files_license_and_metadata` if you assert it).
2. Do the two manual terminal runs below (AC-44 and S11 interactive half) before you push.
3. `git push origin main`, then watch `.github/workflows/ci.yml` (`gh run watch`). The `ubuntu` job is the first Linux signal; expect to fix small things. Optionally `cargo +1.89 check --workspace --locked`.
4. When CI is green: `git tag v0.1.0` and `git push origin v0.1.0`. The release workflow builds four archives, the shell installer and `unambiguous.rb`, and attaches them to a GitHub release.
5. Create the repository `taylorsegell/homebrew-tap`, copy `unambiguous.rb` from the release into it (under `Formula/`). Only then consider `publish-jobs = ["homebrew"]` plus a tap token in the repository secrets.
6. Optional: crates.io. Remove `publish = false` (workspace), the `unam-core` version is already in place (EVL fix cycle 1), `cargo publish --dry-run -p unam-core`, publish `unam-core`, then `unambiguous`. Update the README install section from "after the first release" to the real commands at that point.
7. Update the README, which says Linux is "not yet verified", only after a green Linux run.

### S11 manual walkthrough (interactive half, in your own terminal)

Uses temp directories only; never your real config, this repository, or the skill library. Install first with your real `HOME`, then switch.

```
T=$(mktemp -d); mkdir -p $T/{home,config,cache,proj/.git,inst,lib/sdlc/code-review,lib/testing/pytest-fixtures}
printf -- '---\nname: code-review\ndescription: Review a diff.\n---\n\n# code-review\n' > $T/lib/sdlc/code-review/SKILL.md
printf -- '---\nname: pytest-fixtures\ndescription: Pytest fixtures.\n---\n\n# pytest-fixtures\n' > $T/lib/testing/pytest-fixtures/SKILL.md
cd /Users/databased/Developer/01-AI/unambiguous && cargo install --path crates/unam-cli --locked --root $T/inst
export HOME=$T/home UNAM_CONFIG_DIR=$T/config UNAM_CACHE_DIR=$T/cache UNAM_ENTERPRISE_SKILLS_DIRS=$T/none
$T/inst/bin/unam source add lib $T/lib
cd $T/proj && $T/inst/bin/unam
```

In the picker: type `review`, press `space` on a skill, type `pytest`, `space` on that one, `enter` (plan view), `enter` (apply), `enter` (back to the picker), `esc`, `esc`. Start a stopwatch at the `cargo install` line and stop when the result view lists both skills `active`; the target is under 10 minutes. Record: did the picker open, did the four-key path work, did the terminal come back clean. Then `rm -rf $T`. The longer M6 checklist is in the M6 report under "For Taylor".

### Open residuals

- Linux and CI never run (stub `m7-linux-ci-verification_NOTE_17-10-26.md`); `rust-version = "1.89"` never compiled on 1.89.
- First real release run; Homebrew formula and tap; universal macOS binary.
- Taylor's M6 manual AC-44 run (stub `m6-ac44-manual-terminal-run_NOTE_17-10-26.md`) and the interactive half of S11.
- Warm-start margin of about 3 to 5 ms (stub `m6-warm-start-margin_NOTE_17-10-26.md`).
- `eprintln!` to a closed stderr could still panic (low risk; stdout is fixed).
- `LICENSE-APACHE` ends with a trailing blank line (verbatim stock copy; left as is).
- macOS ships as two archives (aarch64 and x86_64); cargo-dist 0.32.0 rejects a universal target (umbrella item 19, closed as a recorded gap).
- Daily use, the D4 real activation, S5, the decision-sheet rows, and the untracked-lock question stay with you.

## EVL fix cycle 1

Two findings fixed; nothing else touched.

1. Broken-pipe panic. Every stdout write in `crates/unam-cli/src/cli/*.rs` now goes through `out!` / `outln!` (defined in `main.rs`), which write to a locked stdout and return io errors instead of panicking. `ErrorKind::BrokenPipe` ends the process with exit 0; any other write error prints `error: cannot write to stdout` and exits 1. No `SIGPIPE` trick, no new dependency. The TUI writes to the terminal and was not touched. Regression test: `crates/unam-cli/tests/broken_pipe.rs` (400-skill fixture, read 50 bytes, close the read end, assert exit 0 and no "panicked"). Red run before the fix: `json_output_to_closed_pipe_exits_zero` failed with "failed printing to stdout: Broken pipe (os error 32)" (std `println!` panic); the human-output case did not overflow the pipe on the old code and passed both before and after. Green after the fix.
2. `unam-core` workspace dependency now carries `version = "0.1.0"` next to its `path`. `Cargo.lock` is unchanged. `cargo publish` and `cargo package` were not run.

Gates: fmt, clippy `-D warnings`, `cargo test --workspace --locked` (712 passed, 0 failed), `dist plan`, `dist generate --check`, `cargo check --profile dist --locked --offline -p unambiguous`, snapshots unchanged against fadca4b, protected docs unchanged against 8d688e1, no `panic = "abort"`, no added `.spawn(` in src, no `#[ignore`, `git diff --check`: all clean. The broken-pipe item is removed from the open residuals.

## EVL outcome and final classification

Final classification: **Ready for UPDATE PROCESS archival of the M7 plan content, program state VERIFIED (WITH_GAPS)**. The plan stays in the active program folder because the program as a whole still has Known-Gap residuals (see the umbrella `## Current Execution State` and the archival recommendation in the closeout).

| Iteration | HEAD | Tester result | Notes |
|---|---|---|---|
| 1 | 5d5505a | G1 to G6 green; one defect found | `unam --json search ... | head` panicked on a closed stdout (exit 101) |
| fix cycle 1 | 15764ef | code fix | every stdout write goes through `out!`/`outln!`; a closed pipe ends quietly with exit 0; `unam-core` workspace dependency gained `version = "0.1.0"` |
| 2 | bae2268 | HALTED_SUCCESS; 712 tests, 0 ignored | G1 to G6 green; original defect gone (3 of 3); 45 snapshots identical to fadca4b; three concerns recorded |
| test strengthening | 473161c | quick fix | `human_output_to_closed_pipe_exits_zero` produced 33 KB, under the 64 KB pipe buffer, so it passed even against the pre-fix source. It now overflows the buffer; red against the pre-fix source in a scratch copy, green now (`broken_pipe` 2 of 2) |

Log: `results-m7-evl.tsv` (baseline plus iterations 1 and 2); iteration reports `phase-08-m7-oss-readiness-evl-iteration-001_REPORT_17-10-26.md` and `-002_REPORT_17-10-26.md`.

M7 commits: 9897bfe (code), 5d5505a (process, EXECUTE report), 15764ef (broken-pipe fix and `unam-core` version), bae2268 (process, EVL fix cycle 1), 473161c (test), plus the UPDATE PROCESS commit that carries this section.

### Known gaps carried out of EVL

- Linux and CI never run (Docker daemon down); backlog `m7-linux-ci-verification_NOTE_17-10-26.md`.
- First real release run, Homebrew formula and tap unexercised (needs Taylor to push and tag).
- No universal macOS archive (cargo-dist 0.32.0).
- `rust-version = "1.89"` never compiled on an older toolchain.
- Taylor's manual runs: M6 AC-44 terminal run and the interactive half of S11.
- `eprintln!` to a closed stderr could panic (low risk); `LICENSE-APACHE` trailing blank line.
- M6 warm-start margin thin (about 47 of 50 ms); backlog `m6-warm-start-margin_NOTE_17-10-26.md`.

### SPEC Achievement

M7 is a phase-program inner-loop phase: the umbrella governs, there is no per-phase SPEC. Plan acceptance criteria met by passing automated gates: licenses and metadata, `dist plan` and `dist generate --check`, docs/json and docs/adapters coverage tests, README honesty phrases, snapshot stability, no `panic = "abort"`. Not met by an automated or E2E gate (Known-Gap, each with a backlog stub): Linux/CI behavior, first real release, interactive picker half of S11 (AC-44 shares it). No gap is treated as met.

### Context reconciliation (UPDATE PROCESS)

The CONTEXT_PARTIAL items from EXECUTE and the EVL handoff are closed in this commit: `all-context.md` (status, package name, license, release tooling, remote, Q11, scan metadata), `tests/all-tests.md` (712 tests, package `unambiguous`, new test files, `dist` commands and the hook caveat), `product/all-product.md` (package name note).

### For Taylor

Unchanged: the "For Taylor" block above holds the release handoff steps 1 to 7, the S11 manual steps, and every residual. Step 6 no longer needs the `unam-core` version edit (done in fix cycle 1). The broken-pipe residual is closed.
