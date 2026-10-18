---
name: plan:v1-static-activation-phase-08-m7-oss-readiness
description: "v1: static activation, phase 8 of 8: milestone M7, OSS readiness (README, docs, CHANGELOG, MIT OR Apache-2.0, crate rename to unambiguous, cargo-dist release workflow, no publish); light-lane plan supplemented 2026-10-17"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-08
---

# Milestone M7: OSS readiness

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-17; light lane per `v1-static-activation_WAVES_09-10-26.md`: plan about 400 lines, PVL at most 2 cycles, only FAIL-class gaps count)
**Complexity**: COMPLEX
**Phase:** Phase 8 of 8: milestone M7
**Status**: CODE DONE 2026-10-17 (EXECUTE complete; next loop step EVL)
**Phase status:** CODE DONE; product Phase 1 gate cleared 2026-10-17; PRD Q11 decided by Taylor 2026-10-17
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.
**Execution strategy:** sequential, one sonnet execute agent (Taylor: all agents sonnet; this overrides the umbrella's "opus EXECUTE" line). INNOVATE skipped: Taylor's 2026-10-17 decisions (D-NAME to D-BREW below) resolved every design choice. Reuse the M6 test pattern (`Command::output()`, never the spawn text).

---

## Overview

M7 makes the tool releasable and understandable by a stranger (spec 10 M7; PRD 12 product Phase 2 "done when": a stranger can install and use the tool from the README; S11: install to first activation under 10 minutes). It adds no product behavior. It renames the CLI package to `unambiguous` (binary stays `unam`), adds licenses and metadata, writes the README, `docs/json.md`, CHANGELOG and a refreshed `docs/adapters.md`, and wires a cargo-dist release workflow that is verified locally with `dist plan` and never run. Every outward action (publish, push, tag, tap repo) is a hard stop owned by Taylor. Estimated 1 to 2 days.

---

## Scope

In:
- Package rename `unam` -> `unambiguous` with `[[bin]] name = "unam"`; `unam-core` keeps its name.
- Metadata on both crates: `license`, `repository`, `readme`, `keywords`, `categories`, `authors`.
- `LICENSE-MIT`, `LICENSE-APACHE`.
- `README.md` (primary scenario first), `docs/json.md`, `CHANGELOG.md`, `docs/adapters.md` evidence refresh.
- cargo-dist config in `dist-workspace.toml`, generated `.github/workflows/release.yml`, Homebrew installer config (no tap repo).
- Four new test files (below). The 6 TUI snapshot files are NOT renamed (PVL cycle 1, F1: insta derives their prefix from the bin target name `unam`, which stays; renaming them breaks all six tests).
- Live-doc fallout of the rename (`-p unam` -> `-p unambiguous`).

Out (hard stops, Taylor only): publishing to crates.io, pushing, tagging, creating a tap repo, any registry write. `publish = false` stays. Out of scope: product Phase 3, new features, edits to the five protected design documents, Windows.

Stale spec text, reported and never edited: spec 1.4 names `crossterm`, `directories`, full `nucleo`; spec 1.3 and 9 name package and binary `unam` and two crates by those names; PRD Q11 text is now decided; spec 9 and umbrella item 19 say "macOS universal" (resolved by D-TARGETS). List these in the phase report under "Stale spec text".

---

## Acceptance Criteria

No AC IDs in spec 10 for M7. The criteria are PRD S9 (13.1: no adapter claim without an evidence URL), S11 (13.1: install to first activation under 10 minutes from the README), spec 648 (`--json` schemas documented in `docs/json.md` and covered by snapshots), spec 9 and 10 M7 (release workflow and targets), and the Taylor decisions D-NAME to D-BREW. Each names its proving scenario; the gate names the criterion back (G ids below).

| Criterion | proven by | strategy |
|---|---|---|
| S9: every adapter claim has an evidence URL or spike result, and `docs/adapters.md` carries them all | `docs_adapters_cover_every_doctor_evidence_claim` (G3, G5) runs `doctor --evidence` in a temp project and asserts each `claim_key` and `source` appears in `docs/adapters.md`; plus reviewer read of the Cursor and Windsurf wording | Hybrid |
| Spec 648: `docs/json.md` matches the real `--json` snapshots | `docs_json_every_snapshot_is_documented` and `docs_json_every_marker_has_a_snapshot` (G3, G5) | Fully-Automated |
| S11: install to first activation under 10 minutes | README walkthrough in a clean temp HOME, timed (D3 step) | Hybrid (the interactive half is Taylor's M6 manual run, stub exists) |
| README honesty caveats (PRD G12, SES-1, SES-2; no Cursor editor or Windsurf claim; Linux not verified) | `docs_honesty_readme_carries_required_caveats` (G3, G5) | Fully-Automated |
| Licenses, metadata, rename, no-publish | `release_files_*` tests (G3, G5) | Fully-Automated |
| Release workflow valid and targets listed (spec 9, 10 M7) | `dist plan` and `dist generate --check` (G4) | Hybrid (precondition: `dist` installed in EXECUTE step C1) |
| Linux works | Docker run (D6) if it can run offline; otherwise recorded | Hybrid; Known-Gap residual (named, backlog stub, gate CONDITIONAL, never a PASS on its own) |
| No regression: 699 tests, 39 CLI snapshots and 6 TUI snapshots unchanged (names and bytes) | G3, G5 | Fully-Automated |

---

## Entry Gate

- Product Phase 1 gate: CLEARED by Taylor 2026-10-17 (umbrella `## Product Phase 1 Gate`). M0 and M6 gates met; M6 VERIFIED (WITH_GAPS).
- PRD Q11 decided by Taylor 2026-10-17 (D-NAME, D-LICENSE).
- Hard stop at any point: a step needing push, tag, publish, tap repo, or a write outside the blast radius.

---

## Decisions

Taylor's decisions (2026-10-17) are binding; do not reopen them. Small choices are marked M7-n.

| Id | Decision |
|---|---|
| D-NAME | Package `unambiguous` (CLI crate), `[[bin]] name = "unam"`, `unam-core` unchanged. `cargo_bin!("unam")` in tests is unaffected. |
| D-LICENSE | `license = "MIT OR Apache-2.0"`; `LICENSE-MIT` with "Copyright (c) 2026 Taylor Segell"; `LICENSE-APACHE` is the unmodified standard Apache-2.0 text. M7-1: copy it from a crate already in `~/.cargo/registry/src` that ships a `LICENSE-APACHE`, then confirm it is the standard text: title line `Apache License`, `Version 2.0, January 2004`, 9 numbered sections (1 Definitions to 9 Accepting Warranty), `END OF TERMS AND CONDITIONS`, and the appendix. Use `clap-4.6.7/LICENSE-APACHE` (202 lines, has the appendix; verified at PVL); the 176-line copies in serde and anyhow omit the appendix. No network. |
| D-META | In `[workspace.package]`: `license`, `repository = "https://github.com/taylorsegell/unambiguous"` (the origin URL without `.git`), `readme = "README.md"`, `authors = ["Taylor Segell"]`; per crate: `keywords` (at most 5) and `categories = ["command-line-utilities", "development-tools"]`; both crates inherit. Keywords: `agent`, `skills`, `claude-code`, `cli`, `symlink` (M7-2). |
| D-PUBLISH | `publish = false` stays. Publishing, pushing, tagging need Taylor's explicit go-ahead. To make cargo-dist build the binaries anyway, set `[package.metadata.dist] dist = true` on `unambiguous` (M7-3); if `dist plan` rejects that key, use the equivalent the installed version documents and record it. |
| D-DISTINSTALL | EXECUTE step C1 installs the tool: `cargo install cargo-dist --version 0.32.0 --locked` (network; changes `~/.cargo` only; the binary may be named `dist` or `cargo-dist`; record which). `dist plan` never publishes. |
| D-DISTFILES | The only repo files cargo-dist may write are `dist-workspace.toml` (hand-edited by the agent) and `.github/workflows/release.yml` (written by `dist generate`). Do not run interactive `dist init`. One REQUIRED extra (F4): a `[profile.dist]` block in the root `Cargo.toml` with `inherits = "release"` and no `panic` key. `dist plan` and `dist generate` pass without it, but the release workflow builds with `cargo build --profile dist` and cargo fails with "profile `dist` is not defined" (verified at PVL); without it the first real release run would fail. Record it in the report as a planned deviation from D-DISTFILES. Review `git diff` after every dist command; revert any other change. |
| D-TARGETS | `targets = ["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"]`. M7-4 macOS universal rule: try adding `universal2-apple-darwin`; if `dist plan` accepts it and lists a universal artifact, keep it and drop nothing; otherwise leave the two Apple targets and record the gap (closes umbrella item 19). Verified at PVL with dist 0.32.0: both `universal2-apple-darwin` and `universal-apple-darwin` fail `dist plan` with "Unrecognized architecture", so expect the gap; try once, then remove the entry before C4 (a rejected target makes `dist plan` exit 255). |
| D-BREW | `installers = ["shell", "homebrew"]` with `tap = "taylorsegell/homebrew-tap"` (M7-5: name guessed from the origin owner; Taylor confirms). No `publish-jobs` entry, no tap repo, so the workflow only attaches a formula file to a release when a tag is pushed later. |
| D-HONEST | README and docs state, using the exact marker phrases in the Test Plan: Linux is not yet verified (CI never ran; the Docker run, if any, is best effort); evidence for Cursor is for the `cursor-agent` CLI only, the Cursor editor and Windsurf are not tested; G12 and SES-1/SES-2 caveats. |
| D-DEVIATIONS | Docs list these M5 and M6 deviations in the README "Known limits" section (short bullets): termion instead of crossterm (resize shows on the next key press; external SIGTERM leaves the terminal raw); no `e` key (symlink-only); static "Loading catalog..." frame; no match highlighting; the project lock is held only during apply; D-R1 (an ambiguous initial target falls back to the first adapter in the picker and says so in the footer; the CLI errors); D-R2 (the footer warning summary covers source and config warnings only, not malformed `SKILL.md`); the ancestor walk for `agents-neutral` stops at the nearest `.git`; a brief is written by default (opt out per the generated-brief rules in `docs/adapters.md`). |
| D-DOCKER | Optional best-effort Linux signal (step F2): a `--network none` container that already exists locally. Not required for the gate. |
| D-LANE | Light lane. No validate-contract is written by this PLAN-SUPPLEMENT; PVL writes it. |

---

## Blast Radius

Written:
- Root `Cargo.toml` (metadata, required `[profile.dist]` with `inherits = "release"`), `crates/unam-cli/Cargo.toml`, `crates/unam-core/Cargo.toml`, `Cargo.lock` (entry rename only).
- `LICENSE-MIT`, `LICENSE-APACHE`, `README.md`, `CHANGELOG.md`, `docs/json.md`, `docs/adapters.md` (evidence sections only), `dist-workspace.toml`, `.github/workflows/release.yml`.
- `crates/unam-cli/tests/docs_json.rs`, `docs_adapters.rs`, `docs_honesty.rs`, `release_files.rs` (new).
- Nothing is renamed under `crates/unam-cli/src/tui/snapshots/` (F1): the 6 `unam__tui__tests__*.snap` files keep their names and bytes.
- `crates/unam-cli/tests/phase1_checks.rs` line 60 (F2): `no_network_dependencies` asserts the lock holds a package named `unam`; change that string to `unambiguous` (the assertion fails after the rename otherwise; rustfmt must still pass).
- `-p unam` -> `-p unambiguous` in the `crates/unam-cli/tests/phase1_exit.rs` doc comment (line 9) and live docs `process/context/tests/all-tests.md`, `process/context/product/all-product.md`, `process/context/all-context.md`, and this plan. Completed reports and earlier plans are history and are not rewritten; the umbrella and context status lines are UPDATE PROCESS territory.
- Backlog stubs under `process/features/v1-static-activation/backlog/`.

Not touched: `crates/unam-core/src` and `crates/unam-cli/src` code (no product change), the 39 CLI snapshots, the 6 TUI snapshots, the skill library, `spike/`, toolkit directories, the five protected documents. No remote, registry, or tap is modified.

Risk class: outward-facing release mechanics plus a package rename. High-risk classes: none of auth, billing, schema, public API, or trust boundary changes. The public name and license are hard to reverse once published (publishing stays out).

---

## Test Plan

TEST-SCENARIO-DISCOVERY: `process/context/tests/all-tests.md` read (699 tests after M6; cargo; `insta` without `cargo-insta`; `INSTA_UPDATE=no` in gates; `.github/workflows/ci.yml` never ran). Existing coverage in the blast radius: `doctor_cli.rs::doctor_evidence_lists_every_adapter_claim_with_its_source` (harness `tests/common` `Fx`, temp HOME and config dirs), 39 `snapshots__*.snap`, `tests/phase1_checks.rs` (`no_network_*`, `no_panic_abort_profile`). New tests reuse `tests/common`. Tests that run a child use `Command::output()`; the text `.spawn(` and `#[ignore` never appear. No `unam` run touches this repository.

Marker phrases (D-HONEST), asserted literally: README contains `Linux: not yet verified`, `Cursor editor and Windsurf are not tested`, `no telemetry`, `no network`; the README and `docs/adapters.md` contain no line matching `Linux is verified`, `Windsurf is supported`, or `Cursor editor is supported`.

### Criterion-to-Test Map

| Criterion | Test | File | Notes |
|---|---|---|---|
| Spec 648 | `docs_json_every_snapshot_is_documented` | `tests/docs_json.rs` | reads `tests/snapshots/snapshots__*.snap`; each body that parses as JSON must have a marker `<!-- snap: NAME -->` in `docs/json.md`, and every top-level key of its `data` object (and of the envelope) appears in backticks in that marker's section |
| Spec 648 | `docs_json_every_marker_has_a_snapshot` | `tests/docs_json.rs` | each marker NAME maps to an existing snapshot file; at least 15 markers (control: the count is asserted, so an empty doc cannot pass) |
| Spec 648 | `docs_json_check_detects_missing_key` | `tests/docs_json.rs` | pure control on the helper with a synthetic doc and snapshot |
| S9 | `docs_adapters_cover_every_doctor_evidence_claim` | `tests/docs_adapters.rs` | `Fx` temp project; `doctor --evidence --json` via `Command::output()`; ROW-level match (F5): within the section `## Adapter evidence: the <adapter> claims` there is a line that contains the backticked `claim_key` and the `source` for that evidence row (a bare substring match is vacuous: `destination` occurs throughout the file); the two M2 headings still present; the M0-outcome section is checked for the `cursor-agent` CLI wording |
| S9 | `docs_adapters_check_detects_missing_row` | `tests/docs_adapters.rs` | control (F5): the helper `(doc_text, adapter, claim_key, source) -> bool` is fed the real doc (true) and a copy with that row removed, and a copy with the row under the wrong adapter section (both false); also asserts at least 20 evidence rows were checked |
| Honesty | `docs_honesty_readme_carries_required_caveats` | `tests/docs_honesty.rs` | marker phrases; the first fenced example after the first `##` heading is the primary scenario (contains `unam` bare and the word `space`) |
| Honesty | `docs_honesty_no_forbidden_claims` | `tests/docs_honesty.rs` | forbidden lines absent in README, `docs/adapters.md`, `docs/json.md`, `CHANGELOG.md`; asserts all four files were read and are non-empty |
| Honesty | `docs_honesty_check_detects_violations` | `tests/docs_honesty.rs` | control (F5): the marker and forbidden-line helpers run on synthetic text: a text with every marker passes; one missing a marker fails; a text with `Linux is verified` on a line fails (each of the three forbidden phrases is exercised) |
| Release | `release_files_license_and_metadata` | `tests/release_files.rs` | `LICENSE-MIT` holds `Copyright (c) 2026 Taylor Segell`; `LICENSE-APACHE` holds `Apache License` and `Version 2.0`; `cargo metadata --no-deps --offline` via `Command::output()` shows both packages with `license = MIT OR Apache-2.0`, the repository URL, `publish` false, package `unambiguous` with bin target `unam` |
| Release | `release_files_check_detects_bad_input` | `tests/release_files.rs` | control (F5): the pure helpers run on synthetic metadata JSON, `dist-workspace.toml` text, and `Cargo.toml` text: a wrong license, a missing target, a `publish-jobs` line, a missing `[profile.dist]`, and a `panic = "abort"` line each make the matching check fail; the good samples pass |
| Release | `release_files_dist_config` | `tests/release_files.rs` | `dist-workspace.toml` lists the 4 required targets, `homebrew` installer, no `publish-jobs`; `release.yml` exists; root `Cargo.toml` holds a `[profile.dist]` table with `inherits = "release"` (F4); neither `Cargo.toml` nor `dist-workspace.toml` sets `panic` to abort (the existing `no_panic_abort_profile` also stays green) |
| Rename | `tui_snapshot_*` (6, existing) | `src/tui/tests.rs` | pass unchanged with `INSTA_UPDATE=no` after the package rename (verified in a scratch copy at PVL); the files are not renamed or modified (G5 hash compare) |

Hybrid and Agent-Probe scenarios (no automation substitute):
1. Reviewer read of `docs/adapters.md` Cursor and Windsurf wording and of the README for unverified claims (Agent-Probe, in the report).
2. S11 walkthrough (D3 step): a clean `HOME`, config, cache and data dirs under a `mktemp -d` root, `CARGO_HOME` and `RUSTUP_HOME` pointed at the real ones so cargo still works; install with `cargo install --path crates/unam-cli --locked --offline --root $T/inst`; then follow the README commands to first activation with a two-skill temp library and a temp project containing `.git`; time the whole path; no real agent CLI is run. The bare-`unam` picker half cannot run without a TTY and is Taylor's M6 manual run (stub `m6-ac44-manual-terminal-run_NOTE_17-10-26.md`); record this.
3. Docker (D6): see step F2; Known-Gap if it cannot run offline.

Known-Gap rows (named residuals, never a PASS): Linux and CI never ran (stub `m7-linux-ci-verification_NOTE_17-10-26.md`, written at step F2 if the Docker run does not complete); `rust-version = "1.89"` never compiled on an older toolchain (same stub); actual release workflow execution (needs a tag push, a hard stop).

TDD: write `docs_json_*` helper control test and `release_files_*` red-first (they fail until the docs and files exist); record the first red run in the report.

---

## Implementation Checklist

Dependency order. Run `cargo check --workspace --locked` after A, and the cargo gates after E.

### Step A: Rename, metadata, snapshots (one compile chain)

- [x] A0. Record `BASE=$(git rev-parse HEAD)` and the sorted package-name list of `Cargo.lock`.
- [x] A1. Root `Cargo.toml`: add `license`, `repository`, `readme`, `authors` to `[workspace.package]`; append `[profile.dist]` with `inherits = "release"` (F4). `crates/unam-core/Cargo.toml` and `crates/unam-cli/Cargo.toml`: inherit them (`license.workspace = true`, and so on), add `keywords`, `categories`.
- [x] A2. `crates/unam-cli/Cargo.toml`: `name = "unambiguous"` (keep `[[bin]] name = "unam"`, `path = "src/main.rs"`); add `[package.metadata.dist] dist = true` (M7-3).
- [x] A3. `cargo check --workspace` (unlocked, once, so `Cargo.lock` renames the entry); then verify the lock diff: the sorted package-name list differs from A0 only by `unam` -> `unambiguous`; no new or removed package; no denied package (`crossterm`, `mio`, `socket2`, `signal-hook`). Any other change: stop and report.
- [x] A4. Do NOT rename the TUI snapshots (F1). Run `INSTA_UPDATE=no cargo test -p unambiguous --locked --bin unam tui_snapshot_`: all 6 pass under the existing `unam__tui__tests__*.snap` names (insta's prefix is the bin target name `unam`; verified at PVL, and verified that renaming the files makes all 6 fail with a "new snapshot" diff). Never run `INSTA_UPDATE=always` here. Prove no drift: `git status --short crates/unam-cli/src/tui/snapshots crates/unam-cli/tests/snapshots` prints nothing, and the blob hash of each of the 6 TUI and 39 CLI snapshot files equals `git rev-parse BASE:<path>`.
- [x] A5. (F2) In `crates/unam-cli/tests/phase1_checks.rs` change the package-name assertion in `no_network_dependencies` from `"unam"` to `"unambiguous"` (keep `"unam-core"`). Replace `-p unam` with `-p unambiguous` in the `crates/unam-cli/tests/phase1_exit.rs` doc comment and in the live docs listed in Blast Radius. Find them with `git grep -nE -- '-p unam( |$|`)'` before and after (the pattern must not match `-p unam-core`; `\b` would); at PVL the matches were `phase1_exit.rs:9`, `process/context/all-context.md:191`, `process/context/tests/all-tests.md` lines 19, 21, 23, 27, 72, 77, 78, plus history that stays (`docs/spec/unam-technical-spec.md` 840 to 844 is protected, completed reports and earlier plans, `harness/verification.json`, the umbrella and the autopilot goal file). Only the files in the Blast Radius change.

### Step B: Licenses

- [x] B1. Write `LICENSE-MIT` (standard MIT text, "Copyright (c) 2026 Taylor Segell").
- [x] B2. Create `LICENSE-APACHE` per M7-1. Do not edit the text.

### Step C: Release mechanics (local only)

- [x] C1. Install cargo-dist per D-DISTINSTALL; record the version and binary name.
- [x] C2. Rewrite `dist-workspace.toml` (remove the "hand-written, unverified" comment): `cargo-dist-version = "0.32.0"`, `ci = "github"`, D-TARGETS, D-BREW, `install-path = "CARGO_HOME"`. Do not set `publish-jobs`.
- [x] C3. (F3) Run `dist generate` FIRST to write `.github/workflows/release.yml`. Reason, verified at PVL in a scratch copy: with `ci = "github"` set, `dist plan` and `dist generate --check` both exit 255 ("release.yml has out of date contents and needs to be regenerated") until the file exists, so a plan-before-generate order cannot pass. Review `git diff` and `git status` against D-DISTFILES (verified: only `release.yml` is written); revert anything else. Read `release.yml`: it triggers on `pull_request` (plan only) and on a version-tag push; confirm nothing runs now. Do not push, tag, or create a tap.
- [x] C4. Run `dist plan` (exit 0) and `dist generate --check` (exit 0) and save both outputs. Assert the plan lists `[bin] unam` under each of the four targets and a Homebrew formula artifact (`unambiguous.rb`); apply the M7-4 universal rule; if zero artifacts are planned, apply D-PUBLISH's fallback and re-run. Run G4c: `cargo check --profile dist --locked --offline -p unambiguous` (exit 0; F4). The `WARN` lines about a Homebrew homepage, a tap without `publish-jobs`, and missing `rustup` for cross targets are advisory; record them.

### Step D: Documentation

- [x] D1. `docs/json.md`: one section per documented `--json` snapshot, each opening with `<!-- snap: NAME -->` and a heading `` `unam <command> --json` ``; first the shared envelope (spec 5: `ok`, `data`, error shape, exit codes), then per section a table of every top-level key in `data` with type and meaning, read from the snapshot and from `crates/unam-cli/src/output.rs`. Only keys that exist.
- [x] D2. `docs/adapters.md`: keep "Bundle slugify parity with Hermes" and "Bundle loader: differences from Hermes" unchanged. Refresh the evidence sections (M0 outcome, claude-code, agents-neutral) from `doctor --evidence` run in a temp project via the same harness the test uses (print once, never against this repo). State that Cursor evidence is for the `cursor-agent` CLI only. Update the Maintenance section: the evidence tables are checked by `docs_adapters_*`.
- [x] D3. `README.md` (replace the stale content): title; one-paragraph pitch; Journey A first (bare `unam`, type, `space` two skills, `enter` to confirm, `enter` to accept); install (from source `cargo install --path crates/unam-cli --locked`; release installers and `cargo install unambiguous` only as "after the first release / publication", not as available); a non-interactive first activation (`source add`, `add`, `status`); command table; "Known limits" with D-DEVIATIONS; honesty caveats with the marker phrases; documents table (keep the existing one, drop "Design complete. No code yet."); license section; no emojis, no em dashes.
- [x] D4. `CHANGELOG.md`: Keep a Changelog format, `## [0.1.0] - Unreleased`, grouped by milestones M0 to M7 in user terms; no tag.

### Step E: Tests

- [x] E1. Write `tests/docs_json.rs`, `tests/docs_adapters.rs`, `tests/docs_honesty.rs`, `tests/release_files.rs` per the Test Plan (red-first for the docs and release tests, before D1 to D4 and C2 exist; record the red run, then green).
- [x] E2. Run G1 to G3 and fix.

### Step F: Walkthrough, Linux signal, handoff

- [x] F1. S11 walkthrough per Test Plan item 2; record minutes and any README step that failed.
- [x] F2. Linux signal (D-DOCKER): at PVL the Docker daemon was not running (`docker image ls` fails to connect to `/var/run/docker.sock`); in that case write the Known-Gap stub, do not start Docker Desktop or any daemon, and keep the gate CONDITIONAL. (The local registry cache does hold the Linux-only crates: `cargo fetch --locked --offline --target x86_64-unknown-linux-gnu` exits 0, so a later offline container run is feasible.) Otherwise `docker image ls` shows a local `rust` image or not. If one exists: copy tracked files (`git ls-files -z | rsync -a --from0 --files-from=- ./ $T/src/`) so the repo `target/` is untouched; mount `$T/src` read-write and `~/.cargo/registry` read-only; run `cargo test --workspace --locked --offline` with `--network none` and `CARGO_TARGET_DIR` inside the container; record the pass count or failure. Otherwise write the Known-Gap stub `process/features/v1-static-activation/backlog/m7-linux-ci-verification_NOTE_17-10-26.md` and keep the gate CONDITIONAL. Never `docker exec` any existing container.
- [x] F3. Write the release handoff list for Taylor into the phase report (order: confirm tap name and licenses, push `main`, watch ci.yml, tag `v0.1.0`, release workflow, create tap repo and attach formula, then optional crates.io publish after flipping `publish`). Taylor executes it, not an agent.
- [x] F4. Phase report (flat in the program folder) including: tests added and total, `dist plan` output, universal outcome, stale spec text, D-DISTFILES diff review, S11 timing, Docker result.

---

## Exit Gate

```bash
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check                                          # G1 Expected: exit 0
cargo clippy --workspace --all-targets --locked -- -D warnings   # G2 Expected: exit 0
cargo test --workspace --locked                                  # G3 Expected: all pass (699 before; at least 710 after), 0 ignored
dist plan                                                        # G4a Expected: exit 0, lists `unam` for the D-TARGETS targets
dist generate --check                                            # G4b Expected: exit 0 (release.yml current)
cargo check --profile dist --locked --offline -p unambiguous     # G4c Expected: exit 0 (the profile the release workflow builds with exists)
```

| Gate | Check |
|---|---|
| G1 to G3 | The commands above. Never run `unam` against this repository. |
| G4 | `dist plan`, `dist generate --check`, and the G4c profile check; output saved; macOS universal outcome recorded |
| G5 | Presence via `cargo test --workspace --locked -- --list`: each of `docs_json_every_snapshot_is_documented`, `docs_json_every_marker_has_a_snapshot`, `docs_adapters_cover_every_doctor_evidence_claim`, `docs_adapters_check_detects_missing_row`, `docs_json_check_detects_missing_key`, `docs_honesty_readme_carries_required_caveats`, `docs_honesty_no_forbidden_claims`, `docs_honesty_check_detects_violations`, `release_files_license_and_metadata`, `release_files_dist_config`, `release_files_check_detects_bad_input`, `no_panic_abort_profile`, `no_network_dependencies`, `no_network_symbols`, `phase1_ac_coverage`, and six `tui_snapshot_` names; total at least 710 (11 new tests: docs_json 3, docs_adapters 2, docs_honesty 3, release_files 3). Snapshot proof: the 39 `snapshots__*.snap` and the 6 `unam__tui__tests__*.snap` blobs equal BASE and no snapshot file is added, removed, or renamed (`git status --short` over both snapshot directories is empty) |
| G6 | Invariants: `git diff --exit-code BASE -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec/PRD-unambiguous.md docs/spec/unam-technical-spec.md docs/adr/ADR-0001-dynamic-skill-routing.md` exits 0; `! grep -rE '\.spawn\(|#\[ignore' crates`; `Cargo.lock` package-name list equals BASE except `unam` -> `unambiguous`; no `panic = "abort"` in any `Cargo.toml`, `dist-workspace.toml`, `.cargo/config.toml`; `git tag --list` and `git remote -v` unchanged; no `git push` was issued; `git diff --check` clean; `git status` shows no file outside the Blast Radius |

- All checklist items checked; S9, spec 648, honesty and release tests green; 699 existing tests unchanged.
- S11 walkthrough time recorded (target under 10 minutes, or the shortfall recorded).
- Linux: Docker result or the Known-Gap stub; gate stays CONDITIONAL on Linux and CI until Taylor pushes and CI runs.
- Phase report written to the report destination.

---

## Phase Completion Rules

- CODE DONE means artifacts exist with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract. VERIFIED (WITH_GAPS) is allowed when only Linux and CI, the first real release run, and Taylor's M6 manual run are outstanding, recorded as CONDITIONAL with backlog stubs.
- Program completion: when M7 is VERIFIED, the program is complete as scoped; later work (product Phase 3, v2 items, publication) moves to a new feature folder or backlog.
- Commit execution changes on `main`; no push, no tag.

---

## Blockers That Would Justify BLOCKED Status

- `Cargo.lock` changes beyond the single entry rename, or adds a denied package (stop and report).
- cargo-dist cannot be installed, or `dist plan` plans zero artifacts after the D-PUBLISH fallback (stop; report what the tool printed).
- `dist` or any step writes outside D-DISTFILES in a way that cannot be reverted cleanly.
- Any step needing publish, push, tag, tap repo, or crates.io action: stop and hand to Taylor.
- A fixture that would run `unam` against this repository or a real agent CLI (hard safety constraint).

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH (2026-10-17): gate record and Q11 verified; M5 and M6 reports read; test context loaded; repo state mapped (metadata, dist config, remote, snapshots)
- [x] 2. INNOVATE (2026-10-17): skipped, light lane; every design choice was resolved by Taylor's 2026-10-17 decisions (D-NAME to D-BREW); small choices recorded as M7-1 to M7-5
- [x] 3. PLAN-SUPPLEMENT (2026-10-17): existing phase plan rewritten into an executable plan; Inner Loop Refresh Note written
- [x] 4. PVL (2026-10-17, inner-pvl: phase-8, 2 cycles, Gate PASS, contract written; results in results-m7.tsv): vc-validate-agent: at most 2 cycles; only FAIL-class gaps (wrong requirement mapping, steps that cannot run in order) trigger a cycle; nits go to execute time; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [x] 5. EXECUTE (2026-10-17, code commit 9897bfe): all checklist items done; G1 to G3 green (710 tests), G4a to G4c exit 0; gaps documented in the phase report (Linux/CI stub, universal macOS, S11 interactive half)
- [ ] 6. EVL: independent vc-tester re-run of G1 to G6; follow-up stubs registered; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, context docs updated (Q11 decided, crate name, 699+ tests, `-p unambiguous`), commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Inner Loop Refresh Note: 2026-10-17 - changed sections: header (Status, Execution strategy, removal of the Product Phase 2 hold), Overview, Scope, Acceptance Criteria (criterion-to-test with strategy tags), Entry Gate (Q11 decided), Decisions (new), Blast Radius, Test Plan (new, replaces Step D notes), Implementation Checklist (rewritten A to F), Exit Gate (G1 to G6, `--locked`, G5 presence), Blockers, Phase Loop Progress, Verification Evidence, Resume. Reason: Taylor's 2026-10-17 decisions (Q11, metadata, rename, cargo-dist install, targets, Homebrew) and M7 RESEARCH. Removed stale text: the undecided-Q11 wording, the Phase 2 hold, the claim that no Known-Gap rows exist, and the old cargo-run form of the evidence gate (it would run the tool with this repository as the project; the check is now a temp-project test). Corrections to earlier plan facts: a git remote exists (origin, GitHub), so every push or tag is a hard stop; `dist-workspace.toml` had no Linux aarch64, no universal target, no `ci = "github"`, no installers. INNOVATE skipped. Spec text found stale is only reported (Scope).

---

## Touchpoints

- Files listed in Blast Radius.
- Read only: spec 5, 9, 10 M7, 11; PRD 6 (Journey A), 12, 13.1, 15 (Q11), 16; PRD 96 (G12), 423-424 (SES-1, SES-2); M5 and M6 reports (deviation lists); `crates/unam-cli/tests/common`, `doctor_cli.rs`; `process/context/tests/all-tests.md`.

---

## Public Contracts

- Package `unambiguous`, binary `unam`, license `MIT OR Apache-2.0`, repository URL: public and hard to reverse once published (publishing is out of this plan).
- README install and first-use instructions, `docs/json.md`, `docs/adapters.md`: public documentation of contracts set in M1 to M6; no behavior changes.
- Release workflow contract: tag push triggers builds for the four (or five) targets and attaches installers; nothing runs until Taylor pushes.
- Unchanged: CLI behavior, exit codes, JSON schemas, receipt, journal, snapshots.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` | Fully-Automated | spec 8 gates; no regression of the 699 tests |
| `docs_json_*` (snapshot to doc coverage, with control) | Fully-Automated | spec 648 |
| `docs_adapters_cover_every_doctor_evidence_claim` plus reviewer read of Cursor and Windsurf wording | Hybrid (automated assertion; reviewer judgment on wording) | S9 |
| `docs_honesty_*` | Fully-Automated | PRD G12, SES-1, SES-2; D-HONEST |
| `release_files_*` (licenses, metadata, rename, dist config, no `panic` abort) | Fully-Automated | D-NAME, D-LICENSE, D-META, D-PUBLISH; spec 9 |
| `dist plan`, `dist generate --check`, `cargo check --profile dist` (G4c) | Hybrid (precondition: `dist` installed at C1; assertions on saved output) | spec 10 M7 release workflow; spec 9 targets; the workflow's build profile exists |
| README walkthrough in a clean temp HOME, timed | Hybrid | S11; PRD 12 product Phase 2 done condition |
| Docker offline `cargo test` on Linux | Hybrid (best effort; precondition: local `rust` image) | Linux signal only; Known-Gap residual otherwise |
| 39 CLI snapshots and 6 TUI snapshots unchanged (names and bytes) | Fully-Automated (G5 hash compare) | rename fallout contained |
| M1 to M6 regression suite (inside `cargo test --workspace --locked`) | Fully-Automated | AC-1, AC-3 to AC-44, AC-53 still green |

Known-Gap residuals (never a PASS): Linux and CI never ran unless the Docker run works; older-toolchain compile of `rust-version = "1.89"`; the first real release run; the interactive half of S11 (Taylor's M6 manual run). Each has a backlog stub or an existing stub.

---

## Test Infra Improvement Notes

- A pty harness is still absent (M6 R8), so the bare-`unam` half of S11 stays manual.
- `cargo-insta` is not installed; no snapshot is renamed or regenerated in M7 (the TUI prefix follows the bin target name `unam`), so the check is a blob-hash compare against BASE.
- If the Docker run works, a future CI job replaces it; CI itself has never run.

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-08-m7-oss-readiness_PLAN_04-10-26.md`
- Last completed step: 4 PVL (2026-10-17; 2 cycles, 6 FAIL-class gaps F1 to F6 fixed in the plan, Gate PASS)
- Validate-contract status: written (2026-10-17, generated-by inner-pvl: phase-8, Gate PASS)
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, `process/context/planning/all-planning.md`, `phase-07-m6-tui_REPORT_17-10-26.md` (deviation list), `phase-06-m5-agents-neutral-and-briefs_REPORT_04-10-26.md`
- Next step: EXECUTE (step 5) on sonnet with the exact plan path, honoring the Validate Contract's EX1 to EX17; pass `PATH` and `INSTA_UPDATE=no` as in the Exit Gate block; EXECUTE step C1 is the only network use and writes only under `~/.cargo`.

---

## Validate Contract

Status: PASS
Date: 17-10-26
date: 2026-10-17
generated-by: inner-pvl: phase-8

Parallel strategy: sequential
Rationale: score 2/7 (S4 phase program; S7 more than 5 files; no S1 because only one crate gains code and that is test code, no S2 (no schema, API, auth, or state-format change), no S3, no S5, no S6 because no listed high-risk class applies: release mechanics are local and nothing is published, pushed, or tagged). One compile chain (rename, metadata, lock, tests) plus documents that must agree with the test fixtures, so no disjoint file group exists for a team and no independent direction exists for parallel subagents. EXECUTE: one sonnet execute agent (Taylor: all agents sonnet), then an independent sonnet vc-tester EVL run. Agent counts: sequential 1; parallel subagents 4 + 8 + 3 = 15 with nothing to split; agent team 3 x 2 = 6, no benefit; workflow 6 x 1 x 1 = 6, not warranted. Cost guard not triggered. Light lane (D-LANE): worked inline by the validate agent in 2 cycles (cycle 1 found 6 FAIL-class gaps F1 to F6 and applied plan fixes; cycle 2 re-traced and found none), no fan-out, no helper agent, no `unam` run, no agent CLI, skill library untouched, Cargo files and sources of the real repository untouched (every cargo and dist run happened in disposable scratch copies under `scratchpad/m7-pvl`). Only FAIL-class gaps were counted. Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so none is written here. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Cycle log (rows in `results-m7.tsv`):
- Cycle 1, FAIL-class gap F1: A4 and the Blast Radius renamed the 6 TUI snapshots to `unambiguous__tui__tests__*.snap`. insta takes the prefix from the module path root, which for this bin-only crate is the bin target name `unam` (kept by `[[bin]] name = "unam"`). Scratch proof: after only the package rename all 6 `tui_snapshot_` tests pass under the old names; after `git mv` to the new names all 6 fail with a "new snapshot" diff. Fixed (P1): no snapshot is renamed; A4, Blast Radius, G5, G6 proof and the criterion map now require names and bytes unchanged.
- Cycle 1, FAIL-class gap F2: `crates/unam-cli/tests/phase1_checks.rs` line 60 asserts the lock holds a package named `unam`. After the rename `no_network_dependencies` fails (scratch run: 1 failed, 4 passed in that file). The plan listed only `phase1_exit.rs`. Fixed (P2): Blast Radius and A5 name the file and the edit; A5's grep pattern no longer uses `\b` (it would match `-p unam-core`).
- Cycle 1, FAIL-class gap F3: C3 ran `dist plan` before C4 ran `dist generate`. With `ci = "github"`, `dist plan` and `dist generate --check` both exit 255 ("release.yml has out of date contents and needs to be regenerated") until the file exists. Fixed (P3): C3 is now `dist generate`, C4 is `dist plan` plus `dist generate --check` plus G4c.
- Cycle 1, FAIL-class gap F4: `[profile.dist]` was "optional, if dist generate demands it". `dist plan` and `dist generate` never demand it, but the generated workflow builds with `cargo build --profile dist`, and cargo fails with "profile `dist` is not defined" (scratch proof). The first real release run would have failed. Fixed (P4): `[profile.dist]` with `inherits = "release"` is required (A1), `release_files_dist_config` asserts it, and G4c (`cargo check --profile dist --locked --offline -p unambiguous`) proves it builds.
- Cycle 1, FAIL-class gap F5: the new tests had controls only for `docs_json`. `docs_adapters` matched bare substrings, so `destination` (a claim key that appears throughout the file) made the claim-key half vacuous, and `docs_honesty` and `release_files` had no control. Fixed (P5): row-level match inside the adapter's own section, and three new control tests (`docs_adapters_check_detects_missing_row`, `docs_honesty_check_detects_violations`, `release_files_check_detects_bad_input`); G5 and the total (at least 710) updated.
- Cycle 1, FAIL-class gap F6: M7-1 told EXECUTE to confirm the Apache text has "11 numbered sections". The standard text has 9; serde and anyhow copies (176 lines) also omit the appendix. A literal check cannot pass. Fixed (P6): 9 sections, and `clap-4.6.7/LICENSE-APACHE` (202 lines, with appendix) named as the source.
- Cycle 2: re-traced A to F in order against the fixed plan and the scratch results; 0 FAIL-class gaps; Gate PASS.

Evidence read or run at PVL (scratch copy `scratchpad/m7-pvl/ws2`, tracked files only, `git ls-files` + rsync):

| Claim | Evidence read or run | Verdict |
|---|---|---|
| Rename touches only the lock entry name, no package added or removed | `cargo check --workspace` unlocked after renaming the package: sorted `name = ` list is 180 before and 180 after, `diff` shows only `unam` -> `unambiguous`; `crossterm`, `mio`, `socket2`, `signal-hook` count 0. The textual lock diff is 34 lines because `unam-core` now sorts before `unambiguous` (two adjacent blocks swap); this is expected (EX2) | Holds |
| `cargo_bin!("unam")` unaffected | `[[bin]] name = "unam"` unchanged; `cargo test --workspace --locked` in the scratch copy: 26 suites, 699 passed, 0 failed, 0 ignored (with the F2 edit applied) | Holds |
| TUI snapshots get a new prefix | They do not (F1 above). Both directions proven | Plan wrong, fixed |
| `no_network_dependencies` and `no_panic_abort_profile` pass with the new metadata and `[profile.dist]` | Scratch run with license, repository, readme, authors, keywords, categories, `[package.metadata.dist]`, `[profile.dist] inherits = "release"`, the new `dist-workspace.toml`, and the F2 edit: all 5 tests in `phase1_checks.rs` pass; without the F2 edit `no_network_dependencies` fails at line 60 | Holds after P2 |
| fmt and clippy stay clean | `cargo fmt --all --check` exit 0 and `cargo clippy --workspace --all-targets --locked -- -D warnings` exit 0 in the scratch copy with the edits | Holds |
| `cargo metadata` shape for the release test | Both packages show `license = MIT OR Apache-2.0` and the repository URL; `publish` is `[]` (the JSON form of `publish = false`; `null` means unrestricted); bin target `unam` under package `unambiguous` | Holds with EX9 |
| cargo-dist 0.32.0 exists and installs as planned | crates.io: `cargo-dist` 0.32.0, rust_version 1.74, not yanked, published 2026-05-22; `cargo install cargo-dist --version 0.32.0 --locked --root <scratch>` exit 0 in 51 s with a scratch `CARGO_HOME`; it installs the single executable `dist` (no `cargo-dist` binary). The prebuilt 0.32.0 tarball (sha256 matched) ran the plan checks | Holds with EX5 |
| `[package.metadata.dist] dist = true` overrides workspace `publish = false` | Without it `dist plan` exits 255 "This workspace doesn't have anything for dist to Release!" and names `publish = false` as the cause; with it the plan lists `unambiguous 0.1.0` | Holds |
| macOS universal target | `universal2-apple-darwin` and `universal-apple-darwin` both fail `dist plan` with "Unrecognized architecture"; expect the gap (closes umbrella item 19) | Gap, plan rule handles it |
| `dist generate` writes only `release.yml` | In the scratch copy `git status` after `dist generate` shows only the new `.github/workflows/release.yml` beyond my own edits; `Cargo.toml` and `dist-workspace.toml` unchanged by dist | Holds |
| `dist plan` output | After generate: exit 0, `announcing v0.1.0`, `unambiguous 0.1.0` with `source.tar.gz`, `unambiguous-installer.sh`, `unambiguous.rb`, `sha256.sum`, and four archives (aarch64 and x86_64 apple-darwin, aarch64 and x86_64 unknown-linux-gnu), each with `[bin] unam` and `[misc] LICENSE-APACHE, LICENSE-MIT, README.md`. The LICENSE files must exist first (Step B precedes C) | Holds |
| Homebrew installer config without a tap repo or `publish-jobs` | Valid: plan and generate succeed; advisory WARN lines only ("Homebrew publish job is enabled but no homepage", "A Homebrew tap was specified but the Homebrew publish job is disabled", missing `rustup` for cross targets). `release.yml` contains no homebrew job | Holds (EX11) |
| `[profile.dist]` | Not needed by plan or generate; needed by `cargo build --profile dist` (source `build/cargo.rs` in the 0.32.0 tag passes `--profile dist`; `cargo check --profile dist` without the table prints "profile `dist` is not defined"; with `inherits = "release"` it passes in 7 s) | Required (F4) |
| `release.yml` triggers | `on: pull_request` (runs `dist plan` only, `publishing` false) and `push: tags: '**[0-9]+.[0-9]+.[0-9]+*'`; nothing runs until a tag is pushed | Holds |
| README install line | `cargo install --path crates/unam-cli --locked --offline --root <scratch>` exit 0: `Installed package unambiguous v0.1.0 (executable unam)`; the installed binary was not run | Holds |
| Apache text source | Registry copies: serde, anyhow 176 lines (no appendix), clap 4.6.7 202 lines (appendix); 9 numbered sections in the text | Fixed (F6) |
| Protected documents stay untouched | The plan writes none of them; G6 `git diff --exit-code BASE --` over the five files; `docs/spec/unam-technical-spec.md` lines 840 to 844 still say `cargo add -p unam` (stale, reported only) | Holds |
| Linux Docker step | `docker image ls` fails: no daemon at `/var/run/docker.sock`. `cargo fetch --locked --offline --target x86_64-unknown-linux-gnu` exits 0, so a later offline run is feasible once a daemon and a local `rust` image exist | Degrades cleanly to the Known-Gap stub (F2 step edited) |
| Hook-safe gate text | The command text of G1 to G4c, the C1 install, `cargo install --path ... --offline`, `cargo test -p unambiguous ...` fed to `.claude/hooks/scout-block.cjs`: all exit 0. Commands that carry a path segment named `target` or `dist` (for example `CARGO_TARGET_DIR=.../target`) are rejected by the hook (EX4) | Holds with EX4 |

Gates (hook-safe; set `PATH` and `INSTA_UPDATE` first, as in the plan's Exit Gate block; base commit for diffs is `fadca4b`, EXECUTE records the actual HEAD at A0):
- G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets --locked -- -D warnings`; G3 `cargo test --workspace --locked` (all pass, 0 ignored, at least 710).
- G4a `dist plan`; G4b `dist generate --check`; G4c `cargo check --profile dist --locked --offline -p unambiguous`; all exit 0 after C3 wrote `release.yml`.
- G5 presence and snapshot proof: `cargo test --workspace --locked -- --list` must show each name in the plan's G5 row (now including the three control tests and `docs_json_check_detects_missing_key`), six `tui_snapshot_` names, total at least 710; the 39 + 6 snapshot blobs equal BASE.
- G6 invariants as written in the plan (protected documents, no `.spawn(` or `#[ignore`, lock names, no `panic` abort, tags and remotes unchanged, no push, `git diff --check`, status inside the Blast Radius).

AC-to-test map check (plan Test Plan against the cited requirement text):

| Criterion | Plan test | Verdict |
|---|---|---|
| S9 (no adapter claim without an evidence URL or spike result) | `docs_adapters_cover_every_doctor_evidence_claim` (row-level, 29 rows today: 13 claude-code + 16 agents-neutral) + `docs_adapters_check_detects_missing_row` + reviewer read of Cursor and Windsurf wording | Maps after P5 |
| Spec 648 (`--json` schemas in `docs/json.md`, covered by snapshots) | `docs_json_every_snapshot_is_documented` (38 JSON snapshot bodies; 1 text snapshot `usage_bare` is skipped), `docs_json_every_marker_has_a_snapshot`, `docs_json_check_detects_missing_key` | Maps with EX7 |
| S11 (install to first activation under 10 minutes) | README walkthrough in a clean temp HOME (plan step F1); bare-`unam` picker half is Taylor's M6 run | Maps; Hybrid residual |
| PRD G12, SES-1, SES-2, D-HONEST | `docs_honesty_readme_carries_required_caveats`, `docs_honesty_no_forbidden_claims`, `docs_honesty_check_detects_violations` | Maps after P5 |
| D-NAME, D-LICENSE, D-META, D-PUBLISH | `release_files_license_and_metadata`, `release_files_dist_config`, `release_files_check_detects_bad_input` | Maps after P4 and P5 |
| Spec 9 and 10 M7 (release workflow, targets) | G4a to G4c | Maps; universal gap recorded |
| No regression | G3, G5 | Maps after P1 and P2 |

Test gates (C3 table; legacy form is the Verification Evidence table of this plan):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 gates | fmt, clippy, workspace tests green (699 existing + 11 new) | Fully-Automated | G1, G2, G3 | A (699 proven green in the scratch copy with the rename, metadata, and F2 edit; the new tests are B) |
| D-NAME / no regression | package `unambiguous`, bin `unam`; `cargo_bin!("unam")`, 6 TUI and 39 CLI snapshots unchanged; `no_network_dependencies` passes with the new name | Fully-Automated | G3, `no_network_dependencies`, `tui_snapshot_*` (6), G5 blob compare | B (A5 edit to `phase1_checks.rs`) |
| lock and profile checks | lockfile deny list; no `panic` abort in `Cargo.toml`, `dist-workspace.toml`, `.cargo/config.toml` | Fully-Automated | `no_network_dependencies`, `no_network_symbols`, `no_panic_abort_profile` | A (green in the scratch copy with the new metadata and `[profile.dist]`) |
| spec 648 | every JSON snapshot has a documented section and every top-level key (data and envelope) is documented | Fully-Automated | `docs_json_every_snapshot_is_documented`, `docs_json_every_marker_has_a_snapshot` | B |
| spec 648 control | the check fails on a missing key and the doc is not empty (at least 15 markers) | Fully-Automated | `docs_json_check_detects_missing_key` | B |
| S9 | every doctor evidence claim has its own row with its source in the right adapter section; M2 headings kept | Hybrid | `docs_adapters_cover_every_doctor_evidence_claim` (automated; reviewer judges the Cursor and Windsurf wording) | B |
| S9 control | the row check fails on a deleted row and on a row under the wrong adapter | Fully-Automated | `docs_adapters_check_detects_missing_row` | B |
| S9 wording | README and `docs/adapters.md` make no unverified claim about the Cursor editor, Windsurf, or Linux | Agent-Probe | reviewer read recorded in the phase report (scenario: read both files, list every sentence naming Cursor, Windsurf, or Linux, judge each against the evidence tables) | B |
| honesty | README carries the marker phrases and a primary scenario first; no file claims Linux verified, Windsurf supported, or the Cursor editor supported | Fully-Automated | `docs_honesty_readme_carries_required_caveats`, `docs_honesty_no_forbidden_claims` | B |
| honesty control | the helpers fail on a missing marker and on each forbidden phrase | Fully-Automated | `docs_honesty_check_detects_violations` | B |
| licenses, metadata, no publish | MIT text with the copyright line; Apache text; both packages carry the license and repository; `publish` empty list; package and bin names | Fully-Automated | `release_files_license_and_metadata` | B |
| release config | four targets, `homebrew` installer, no `publish-jobs`, `release.yml` present, `[profile.dist]` inherits release, no abort | Fully-Automated | `release_files_dist_config` | B |
| release control | the helpers fail on a wrong license, missing target, `publish-jobs`, missing profile, abort line | Fully-Automated | `release_files_check_detects_bad_input` | B |
| spec 9 and 10 M7 release workflow | `dist plan` lists `[bin] unam` for four targets plus `unambiguous.rb`; `release.yml` current; the `dist` profile exists | Hybrid | G4a, G4b, G4c (precondition: `dist` 0.32.0 installed at C1) | B |
| macOS universal | `universal2-apple-darwin` is rejected by dist 0.32.0 | Hybrid | the M7-4 try recorded from the saved `dist plan` output | D (named residual: gap recorded, closes umbrella item 19) |
| S11 | install from source and reach first activation from the README commands in under 10 minutes in a clean temp HOME | Hybrid | plan step F1 walkthrough with timing, scripted from the README commands | B |
| S11 interactive half | bare `unam` picker in a real terminal | Hybrid | Taylor's M6 manual run (`m6-ac44-manual-terminal-run_NOTE_17-10-26.md`) | D (named residual) |
| Linux works | offline `cargo test` on Linux in a `--network none` container | Hybrid | F2 Docker run | D (daemon not running at PVL, so the stub `m7-linux-ci-verification_NOTE_17-10-26.md` is the expected outcome; closeout CONDITIONAL, never a PASS on its own) |

Failing stubs: each named test is written red-first by the execute agent from the Scenario text in the Test Plan; per-row stub blocks are omitted under the lighter process (same as M6). Record whether the red-first run happened.

Dimension findings:
- Infra fit: PASS - the rename, metadata, and dist wiring were exercised end to end in a scratch copy (check, tests, fmt, clippy, install-from-path, `dist generate`, `dist plan`, profile check); the lock delta is one entry name; the one cross-file coupling the plan missed (`phase1_checks.rs`) and the snapshot-prefix mistake are fixed; the Docker daemon is down so Linux stays a named residual.
- Test coverage: PASS - every M7 criterion maps to a named test or a recorded Hybrid/Agent-Probe scenario; every new test file now has a control and a non-vacuity assertion (row-level match, marker count, rows-checked count, files-read count); no developed behavior rests on a Known-Gap alone; Linux, the first real release run, and the interactive S11 half are named residuals.
- Breaking changes: PASS - no CLI behavior, exit code, JSON schema, or state format change; the package name and license become public only if Taylor publishes (out of scope); `publish = false` stays and `dist = true` keeps the workflow able to build.
- Security surface: PASS - no network use at run time; the only network use is the EXECUTE C1 install into `~/.cargo`; no push, tag, publish, or tap; `release.yml` runs nothing until a tag is pushed and its `contents: write` permission is the generator default (noted for Taylor's handoff); the threat model asks for nothing more.
- Sections A (rename and lock), B (licenses), C (release mechanics), D (documents), E (tests), F (walkthrough, Linux, handoff): PASS each after P1 to P6; 0 FAILs / 0 CONCERNs.

Execute-time notes (nits, binding on the execute agent, not gaps; list each applied or deviated in the phase report):
- EX1 TDD order: write the four test files (E1) before C2 and D1 to D4 even though the checklist lists them later; run them red and record it. `docs_adapters_*` cannot go red because `docs/adapters.md` already carries both evidence tables since M4 and M5; its value is the row-level match and the control. Never write the `spawn` call text or the ignore attribute text in any file under `crates`, comments and strings included.
- EX2 Lock check: judge `Cargo.lock` by the sorted package-name list (180 entries before and after, one name changed), not by the textual diff, which also shows the two adjacent blocks swapped because `unam-core` sorts before `unambiguous`. Stop if any other name differs or a denied crate appears.
- EX3 `phase1_checks.rs` edit: replace only `"unam"` with `"unambiguous"` in the `no_network_dependencies` assertion; run `cargo fmt --all` afterwards (the line stays within rustfmt's width at the verified edit). Do not touch `phase1_ac_coverage`.
- EX4 Hook and timeouts: the scout-block hook rejects a command that contains a path segment named `target` or `dist` (for example `export CARGO_TARGET_DIR=.../target`, `cat dist/x`); `dist plan`, `dist generate --check`, `cargo check --profile dist ...`, and `cargo install cargo-dist ...` pass. If a blocked path is unavoidable, put the exports in a script file under the scratchpad and run the script. The workspace suite takes about 2 minutes (`ac_activation` alone 70 s): use a 10 minute timeout or run it in the background.
- EX5 C1: `cargo install cargo-dist --version 0.32.0 --locked` took 51 s with network, installs only a `dist` executable (no `cargo-dist`), needs rustc 1.74 or newer (local 1.99). Do not run `dist init`, `dist build`, `dist host`, `dist migrate`, or `dist selfupdate`.
- EX6 `dist-workspace.toml` content verified at PVL (keep `[workspace] members = ["cargo:."]`): `cargo-dist-version = "0.32.0"`, `ci = "github"`, the four targets, `installers = ["shell", "homebrew"]`, `tap = "taylorsegell/homebrew-tap"`, `install-path = "CARGO_HOME"`; nothing else. Do not set `publish-jobs`.
- EX7 `docs/json.md`: 38 of the 39 CLI snapshots are JSON (`usage_bare` is text and is skipped). Every JSON snapshot has the envelope keys `ok`, `command`, `data`, `warnings`, `errors`; five error snapshots (`add_bad_path`, `bundle_show_not_found`, `rm_referenced`, `show_ambiguous`, `show_not_found`) have an empty `data`. Define a marker's section as the text from its nearest preceding `##` heading to the next `##` heading, so several markers may share one section when the data shape is shared (the apply-family commands share one `data` shape); check the envelope keys once in the shared Envelope section. Strip the snapshot header (the two `---` lines) before parsing the body. Keep at least 15 markers (expect 38).
- EX8 `docs_adapters`: the evidence rows carry `adapter`, `claim_key`, `status`, `source`, `claim`; match the row by adapter section, backticked claim key, and source on one line (29 rows today). Build the temp project with `Fx` as `doctor_cli.rs` does; run the binary through `Command::output()`; the project must not be this repository; set nothing outside the `Fx` temp dirs.
- EX9 `release_files`: `cargo metadata` prints `"publish": []` for `publish = false`; assert the empty list. Run it as `Command::new(env!("CARGO"))`-style with `--no-deps --offline --locked --format-version 1` through `Command::output()`; read `CARGO_MANIFEST_DIR`'s parent's parent as the repo root as `phase1_checks.rs` does. Also assert the 2 packages, package `unambiguous` with bin target `unam`, package `unam-core`, the repository URL, and that `[package.metadata.dist]` has `dist = true`.
- EX10 LICENSE-APACHE: copy `~/.cargo/registry/src/*/clap-4.6.7/LICENSE-APACHE` unmodified; confirm 9 numbered sections, `END OF TERMS AND CONDITIONS`, and the appendix. `LICENSE-MIT` is the standard MIT text with `Copyright (c) 2026 Taylor Segell`.
- EX11 Advisory dist warnings (record, do not fail): Homebrew homepage missing (optional fix: add `homepage` equal to the repository URL to `[workspace.package]` and inherit it in both crates; only if the metadata test is updated too), tap without `publish-jobs` (unavoidable under D-BREW), no `rustup` for cross targets (`rustup` is absent from PATH at PVL; the plan never builds).
- EX12 Universal: write `universal2-apple-darwin` into targets, run `dist plan`, expect exit 255 "Unrecognized architecture", remove the entry, re-run. Record the gap text in the report (closes umbrella item 19). Do not try other spellings beyond what PVL already tried.
- EX13 README: install from source with `cargo install --path crates/unam-cli --locked` (verified offline form works); keep "release installers" and `cargo install unambiguous` as future only. S11 walkthrough: never run `unam` with this repository as the project and never invoke an agent CLI; install into the temp root and use only the temp library and temp project.
- EX14 F2 Docker: the daemon was down at PVL; if it is still down write the stub and stop. Do not start Docker Desktop, do not `docker exec` any container. Any `CARGO_TARGET_DIR` path inside a script must come from a script file (EX4).
- EX15 Stale spec text to report, never edit: the plan's Scope list, plus `docs/spec/unam-technical-spec.md` lines 840 to 844 (`cargo add -p unam ...`).
- EX16 REPORT TO TAYLOR (standing rule: report tension, do not resolve silently): the plan's snapshot rename was wrong and was dropped (F1); `phase1_checks.rs` had to change (F2); `[profile.dist]` was added to the root `Cargo.toml` (F4); the macOS universal target is unsupported in dist 0.32.0; the tap name is a guess (M7-5); `release.yml` carries `contents: write` (generator default) and runs on `pull_request` as plan-only; Docker, CI, and the 1.89 toolchain remain unverified.
- EX17 Phase boundary: commit on `main`; no push, no tag, no publish, no tap. UPDATE PROCESS (not EXECUTE) updates `all-context.md` status lines, the umbrella, and the tests context counts; EXECUTE changes only `-p unam` strings in the live docs listed in the Blast Radius.

Open gaps: none FAIL-class. Named residuals (not proving): Linux and CI never ran (Docker daemon down; stub `m7-linux-ci-verification_NOTE_17-10-26.md` is written at F2); `rust-version = "1.89"` never compiled on an older toolchain; the first real release run (needs a tag push, a hard stop) and the Homebrew formula's behavior after a tap exists; macOS universal binary unsupported by dist 0.32.0; Taylor's M6 manual terminal run and the interactive half of S11; the 10-minute S11 target is checked only by a scripted walkthrough.
What This Coverage Does NOT Prove:
- G1 to G3: nothing about Linux, CI, or a real terminal; the suite runs on one macOS machine with the local toolchain (rustc 1.99).
- `docs_json_*`: that each documented key has the right type and meaning (the test checks that keys and snapshots are covered, not the prose); that the documented shapes cover every possible runtime variant beyond the 38 snapshots.
- `docs_adapters_*`: that the documented claims are true of the vendors' current behavior; only that each doctor evidence row (adapter, claim key, source) has its own row in the right section.
- `docs_honesty_*`: only literal marker phrases and three forbidden phrases; wording that implies a forbidden claim in other words passes (the reviewer read covers it).
- `release_files_*`: file contents and metadata, not that a release build, archive, or installer works.
- `dist plan`, `dist generate --check`, G4c: the plan, the workflow text, and that the build profile exists; not that GitHub accepts the workflow, that cross builds link, that the shell installer or the Homebrew formula installs, or that the tap name exists.
- S11 walkthrough: a scripted run on this machine from a locally built binary with `--offline`; not a stranger on a fresh machine with network and a released installer.
- Rename fallout: the package name and in-repo references; not any external consumer of the old package name (there is none published).
Net result: PASS (0 FAIL-class gaps after cycle 2, 0 CONCERNs; no developed behavior rests on a Known-Gap alone; six cycle-1 gaps fixed in the plan)
Gate: PASS
Accepted by: not required (no CONCERN remains); the named residuals above are recorded as known-gaps. EXECUTE is legal: honor G5 presence checks and EX1 to EX17. Linux and CI, Taylor's M6 manual run, and the first real release run are Hybrid or Known-Gap residuals that make the phase closeout CONDITIONAL (VERIFIED WITH_GAPS), not this gate.

---

## Next Step

ENTER EXECUTE MODE with this plan path (validate-contract Gate PASS; honor G5 and EX1 to EX17).

---

## Deviations

EXECUTE 2026-10-17. Full list with EX1 to EX17 in `phase-08-m7-oss-readiness_REPORT_17-10-26.md`. All stay inside the Blast Radius.

1. G6 literal grep `! grep -rE '\.spawn\(|#\[ignore' crates` cannot pass: `crates/unam-cli/tests/phase1_checks.rs` line 223 holds a deliberate `.spawn()` in the existing `no_background_process` control, present at BASE. No M7 file contains either pattern. Judged as: no new hit.
2. The `-p unam` strings in the live docs listed in the Blast Radius were replaced as planned; `all-product.md` held none. One sentence in `process/context/tests/all-tests.md` line 23 still says "the CLI package is `unam`"; it is context-doc prose and is left for UPDATE PROCESS (EX17).
3. This plan file's own `-p unam` mentions are descriptions of the change and were left as they are.
