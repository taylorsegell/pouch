---
name: context:all-tests
description: "Test runners, commands, and verification gates for unam (cargo suite exists since milestone M1, grown in M2 and M3; CI written, not yet run)"
keywords: tests, testing, cargo test, clippy, fmt, verification, acceptance criteria, ac_nn, insta, proptest, assert_cmd, fixtures, ci, gates, bundle, editor, slugify, word table
related: [context:all-product]
date: 04-10-26
---

# Unambiguous - All Tests

Last updated: 2026-10-09

Attach this file first when the task involves testing, verification, or test debugging.

---

## Current State

**M3 update (EVL 2026-10-09, 512 tests, all green):** new integration suites `ac_activation` (17 tests; AC-2, 10, 13 to 19, 21 to 25, 29, 30, 40, plus S2), `ac_recovery` (13; AC-31, 32, 53 and the recovery and trust-boundary rows) and `activation_cli` (36; command behavior and refusals); `snapshots` grew to 31. Run one with `cargo test -p unam --test ac_activation ac_NN`. The full AC-31 fault matrix takes about 62 s; the wave plan moves it to a strided sample by default with `UNAM_FULL_FAULT_MATRIX=1` for the full run (check the current default before relying on timing).

**FsOps fault injection pattern (M3):** core code does all writes through the public `FsOps` trait (`unam-core/src/fsops.rs`, `RealFs` in production). Tests use the fault-injecting implementation in `crates/unam-cli/tests/common/faults.rs` (`FailMode::FailBefore(n)` and similar) that fails the n-th operation, then call `resume` or `rollback` and compare destination and receipt to the expected state (AC-31, AC-32). Use a separate `StepClock` start per scenario so ids differ.

**Earlier baseline (M1 164 tests; M2 added bundle, selection, and resolve coverage; all green at EVL on 2026-10-05):** after M2, unam-core unit 193, integration files `ac_read_path` (6), `ac_bundles` (4), `ac_selection` (4), `bundle_cli` (28), `cache_cli` (6), `read_side_invariant` (1), `search_cli` (6), `show_cli` (8), `snapshots` (19), `source_cli` (10), `usage` (5), and `perf_cold_scan` (1, in `unam-core`). The cargo commands below are runnable. Integration tests use the `TestEnv` harness in `crates/unam-cli/tests/common/mod.rs`; every mutating or reading test asserts the source-tree invariant.

## Scope

- test runner selection and commands
- the invariants every mutating test must assert
- how acceptance criteria map to tests

## Read This When

- planning the test gates of any milestone
- running verification after implementation
- debugging a failing test

## Quick Routing

| If you need... | Read next |
|---|---|
| the full testing strategy | `docs/spec/unam-technical-spec.md` section 8 |
| what each acceptance criterion requires | `docs/spec/PRD-unambiguous.md` section 13.2 |
| which milestone closes which criterion | `docs/spec/unam-technical-spec.md` section 10 |

## Quick Decision Guide

- **Unit tests** (`unam-core`, inline `#[cfg(test)]`): frontmatter, resolve, slugify, `plan::diff` property tests (`proptest`), adapter evidence checks.
- **Integration tests** (`crates/unam-cli/tests/`, run against the built binary with `assert_cmd`): one test per PRD acceptance criterion, named `ac_NN_*`. The workspace root is virtual, so root-level `tests/` would not compile.
- **Snapshot tests** (`insta`): every command's `--json` output.
- **Interruption tests:** a test `FsOps` implementation fails at step N (AC-31, AC-32).
- **Spike (milestone M0):** manual procedure, not a test (`spike/symlink-discovery.md`).

## Commands

Runnable (from the repo root; first export `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no`):

| Scope | Command | Notes |
|---|---|---|
| format | `cargo fmt --all --check` | CI gate |
| lint | `cargo clippy --workspace --all-targets --locked -- -D warnings` | `unwrap` denied outside tests via clippy config |
| all tests | `cargo test --workspace --locked` | unit + integration |
| one criterion | `cargo test -p unam --test <file> ac_NN` | integration tests live in `unam-cli` (package `unam`) |
| update snapshots | no `cargo-insta` installed | use the no-`cargo-insta` Insta procedure in the M1 plan (`INSTA_UPDATE=no` in gates) |
| cold-scan perf | `cargo test --release -p unam-core --test perf_cold_scan` | Hybrid; gate is the median of five runs, limit 300 ms (M2 EVL median 140.8 ms) |
| slugify table check (G12) | `python3 crates/unam-core/tools/gen-word-table.py --check` | needs Python only; exit 0 ok, 3 Unicode drift between running Python and the committed header, 1 mismatch |
| slugify table vs compiler (G12b) | `python3 crates/unam-core/tools/gen-word-table.py --check-rust` | Hybrid; needs `rustc`; exit 4 without it |
| no ignored tests (G13c) | `! grep -rEq '#\[ignore' crates` | no `#[ignore` text anywhere under `crates` |
| bundle source invariant (G14) | `cargo test -p unam --test bundle_cli bundle_commands_leave_sources_untouched -- --exact` | no bundle command writes under a source tree |

CI target: macOS and Linux runners, all three gates. `.github/workflows/ci.yml` is written but has not been run (no push yet).

Runnable now (harness validators, from the repo root):

```bash
node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs
node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs
node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs
node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <plan.md>
git diff --check
```

## Invariants Asserted in Every Mutating Test

1. The `blake3` tree hash of the source fixture is identical before and after (AC-2, S3).
2. The foreign fixture file in the destination is byte-identical after (AC-23, S4).

## Gate Helper Pattern

Milestone gates are run through two shell functions defined inside each Bash call (each call is a fresh shell): `gate`, which fails when a test filter matches zero tests, and `gate6`, which runs the release perf test five times and takes the median. Exact definitions: the M1 plan Exit Gate (M2 reuses them unchanged) (`process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md`). Do not put path-like `target/`, `.git`, `node_modules`, `build`, `dist`, `vendor`, or `coverage` segments in Bash command text (the scout-block hook rejects them).

## Scripted EDITOR Pattern (bundle edit tests)

`bundle edit` reads `EDITOR` through the injected `Dirs.env`, never the process environment. Tests write a `#!/bin/sh` script inside the `TestEnv` tempdir, make it executable, and pass its path through the `EDITOR` variable of the env map. The script receives the temp copy path as its last argument. The command string is split on whitespace (no shell), so the script path must not contain spaces. Unix only (`#[cfg(unix)]`); no test starts a real editor. In-process tests build `Dirs` from an injected map with `common::dirs_for(env)`; `detect_root` tests pass a `ceiling`, and every `bundle save` CLI test plants `git_marker(dir)` first so root detection never walks above the tempdir.

## Debugging Quick Reference

- Tests must never touch the real `~/.config/unam`, `~/.claude/skills`, `~/.agents/skills`, the skill library, or this repository. Point `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`, and any user-scope scan directory at `tempfile` directories.
- Never run `unam` against this repository: `.agents/skills` here is a symlink to the toolkit-owned `.claude/skills`.

## Known Gaps

- AC-2 closes in milestone M3 (spec 10, decided 2026-10-04). The first test, in M1, is the read-side source-tree-unchanged test (register, scan, search leave the source tree hash unchanged); the invariant is asserted in every mutating test from M3 on.
- Interruption tests (AC-31, AC-32) closed in M3 by comparing destination and receipt to the expected state; M4 adds the `reconcile` zero-divergence assertion (spec 8). Gaps: no pty test for the interactive prompt, no kernel-level torn write, Linux never run, D4 real activation pending (backlog NOTE `m3-d4-first-real-activation_NOTE_05-10-26.md`), and the `status` halves of AC-13, 15, 21 (and 12, 35) belong to M4.
- CI is written, not run. First push will show Linux behavior; the `show_found` (tree hash) and `add_bad_path` (OS error text) snapshots may differ on Linux.
- `perf_cold_scan` is machine dependent: EVL median 183.9 ms but two of five runs were near or over 300 ms (297.9, 365.2). The gate is the median; watch it under CI load.
- Chmod-based Hybrid tests self-skip when run as root (M2 adds `register_does_not_write_an_unreadable_index`).
- AC-12 and AC-35 are closed on the desired side only (M2); their `status` side is an M4 test note (backlog `process/features/v1-static-activation/backlog/m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md`). AC-10 end to end needs apply (M3).
- Slugify `\w` parity is tied to Python 3.14.5 (Unicode 16.0.0) and rustc 1.99.0 (Unicode 17.0.0); G12 exit 3 flags drift.
- `dist` and `cargo-insta` are not installed; `dist-workspace.toml` is parse-checked only (milestone M7 verifies with `dist plan`).
