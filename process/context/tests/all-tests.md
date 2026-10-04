---
name: context:all-tests
description: "Test runners, commands, and verification gates for unam (cargo; planned until milestone M1)"
keywords: tests, testing, cargo test, clippy, fmt, verification, acceptance criteria, ac_nn, insta, proptest, assert_cmd, fixtures, ci, gates
related: [context:all-product]
date: 04-10-26
---

# Unambiguous - All Tests

Last updated: 2026-10-04

Attach this file first when the task involves testing, verification, or test debugging.

---

## Current State

**No test suite exists yet.** There is no Cargo workspace until the first task of milestone M1. Until then the only runnable gates are the harness validators listed below. Every cargo command in this file is planned (spec section 8) and becomes runnable in M1.

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

Planned (runnable from milestone M1):

| Scope | Command | Notes |
|---|---|---|
| format | `cargo fmt --all --check` | CI gate |
| lint | `cargo clippy --workspace --all-targets -- -D warnings` | `unwrap` denied outside tests via clippy config |
| all tests | `cargo test --workspace` | unit + integration |
| one criterion | `cargo test -p unam --test <file> ac_NN` | integration tests live in `unam-cli` (package `unam`) |
| update snapshots | `cargo insta review` | requires `cargo-insta` |

CI target: macOS and Linux runners, all three gates.

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

## Debugging Quick Reference

- Tests must never touch the real `~/.config/unam`, `~/.claude/skills`, `~/.agents/skills`, the skill library, or this repository. Point `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`, and any user-scope scan directory at `tempfile` directories.
- Never run `unam` against this repository: `.agents/skills` here is a symlink to the toolkit-owned `.claude/skills`.

## Known Gaps

- AC-2 closes in milestone M3 (spec 10, decided 2026-10-04). The first test, in M1, is the read-side source-tree-unchanged test (register, scan, search leave the source tree hash unchanged); the invariant is asserted in every mutating test from M3 on.
- Interruption tests (AC-31, AC-32) prove consistency in M3 by comparing destination and receipt to the expected state; M4 adds the `reconcile` zero-divergence assertion (spec 8).
- No CI exists until milestone M1.
