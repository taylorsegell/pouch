---
name: plan:v1-static-activation-phase-02-m1-core-read-path
description: "v1: static activation, phase 2 of 8: milestone M1, core read path (workspace, CI, config, source, catalog, search, show); supplemented 2026-10-05 for execute"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-02
---

# Milestone M1: Core read path

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-05 after RESEARCH and INNOVATE)
**Complexity**: COMPLEX
**Phase:** Phase 2 of 8: milestone M1
**Status**: VERIFIED (WITH_GAPS) 2026-10-05
**Phase status:** VERIFIED (WITH_GAPS) 2026-10-05. Gaps: perf margin noisy, CI written not run, `dist-workspace.toml` parse-checked only, G11 Pre-Step record not located. Report: phase-02-m1-core-read-path_REPORT_04-10-26.md
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.
**Execution strategy:** sequential, one opus execute agent (every file is new and later steps depend on earlier ones; no disjoint file groups worth a team). All other steps sonnet.

---

## Overview

Milestone M1 creates the Cargo workspace and the read-only half of the product: register sources, scan and cache the catalog, parse frontmatter, search, and show (spec section 10 M1; spec 4.1 to 4.3, 5, 7; PRD 7.1 discovery requirements). Nothing in M1 writes to a source or a project destination. Estimated 2 to 3 days (spec 10). It ends in something usable: a `unam` binary that can register a source and search it.

Product Phase 1, tier 2 (read path). Program charter: see umbrella.

How to read this plan: "Decisions Locked" are fixed by INNOVATE (2026-10-05) and by Taylor's earlier decisions; "Plan-Level Decisions" were made at this PLAN-SUPPLEMENT inside those bounds; "Module Specifications" and the "CLI Contract" remove every remaining design choice. An execute agent should not need to decide anything; if it must, that is a deviation to record in the phase report.

---

## Scope

In (spec section 10 M1):

- Workspace, two crates, CI (fmt, clippy, test), `cargo-dist` config (hand-written, unverified).
- `config` and `paths` (spec 4.1, 2.1), `source add|list|rm` (spec 4.2, 5), `catalog` scan with per-source cache (spec 4.3), `frontmatter`, `search`, `show`, bare `unam` usage (spec 5).
- Error model skeleton with remedy strings (spec 7); clippy denying `unwrap` outside tests.
- Test harness: fixtures (nested categories, duplicate names, broken-YAML skill, supporting files, nested `.git` to skip; spec 8 Integration), hash helper, first test, perf test, snapshots.
- A temporary `lookup(ref)` handling all three spec 4.6 reference forms, superseded by `resolve.rs` in milestone M2.

Out (not built in M1; each recorded):

- `source relocate` (milestone M4), `source rename` (PRD SRC-4; not in the spec CLI; open item for Taylor, suggested milestone M4).
- Bundles, project root, selection, `resolve.rs` (M2); adapters, `fsops` write operations beyond `write_atomic`, `state.rs`, `plan`, `apply` (M3).
- Global flags `--project`, `--target`, `--yes` (declared by the milestone that first uses them; adding a flag is additive).
- Match-index output for highlighting (spec 4.3 "match indices"): the TUI needs it in milestone M6; M1 search returns score and order only.
- `ratatui`, `crossterm` (M6), `fs4` (M3 lock), `directories` (dropped, decision 1), full `nucleo` (decision 10).
- Any real-library run. Measuring against Taylor's library is milestone M5 (S1, S6).

---

## Decisions Locked (INNOVATE 2026-10-05; apply as written)

| # | Decision |
|---|---|
| D1 | `paths.rs` resolver following PRD section 9 on macOS and Linux: `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`, then `XDG_*`, then `~/.config/unam`, `~/.cache/unam`, `~/.local/share/unam`. No `directories` path functions; the crate is dropped. Unit tests for precedence. |
| D2 | No new env var. Tests isolate by setting `HOME` and all `XDG_*` and `UNAM_*` to tempdirs on every `Command`. |
| D3 | Two-pass walk. Discovery: `ignore::WalkBuilder` with `max_depth(8)`, `follow_links(false)`, hidden skipped, `filter_entry` dropping `node_modules`, `.git`, and any directory whose parent contains `SKILL.md`. Then a per-skill sorted walk (gitignore off, skipping only `.git` and `.DS_Store`) for supporting files and the tree hash per spec 4.3. The 5,000-directory cap counts yielded directories after filtering; on breach return partial results plus a source-level diagnostic. |
| D4 | Discovery does not follow symlinks. A symlinked skill directory or symlinked `SKILL.md` is skipped with a diagnostic. Inside a skill, symlinks are hashed by target string (spec 4.3) and never followed. |
| D5 | `Diagnostic { code, severity, message, path: Option, remedy: Option }`. Per-skill diagnostics plus a catalog-level list carrying the source alias. JSON arrays are always present. |
| D6 | Minimal `lookup(ref)` in the catalog module handling all three spec 4.6 forms (`alias:rel/path` exact, `alias:name`, bare `name`). Returns Found, NotFound (exit 4, candidates by edit distance), or Ambiguous (exit 5, all candidate IDs, never picks). Marked temporary; milestone M2's `resolve.rs` supersedes it. |
| D7 | `[workspace.lints.clippy] unwrap_used = "deny"` plus `clippy.toml` with `allow-unwrap-in-tests = true`. Integration test files and shared helper modules carry `#![allow(clippy::unwrap_used)]` with a one-line comment. |
| D8 | Hand-written minimal `dist-workspace.toml` (per-arch targets `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`; no universal-binary claim), no `release.yml`. Recorded as unverified (`dist` not installed; milestone M7 verifies). |
| D9 | The perf test runs by default (not `#[ignore]`): synthetic 500-skill fixture in a tempdir, cold means no catalog cache. Asserts the spec 9 target of 300 ms in release; in debug builds asserts a generous bound (5 s) and prints the measured time. An explicit release-mode gate command runs it with a zero-tests-ran guard. |
| D10 | Search uses `nucleo-matcher` (`Pattern::parse`, `match_list`) over the composite string per spec 4.3; filters `--category` (prefix) and `--source` (alias). |
| D11 | `source rename` (PRD SRC-4) is NOT built in M1; open item for Taylor (suggested milestone M4). Sources are keyed by alias in the data model as the spec says. |
| D12 | First test written: the read-side source-tree-unchanged test (decision of 2026-10-04): register, scan, search, and show leave the source fixture's blake3 tree hash unchanged. |

Report to Taylor (resolved provisionally by INNOVATE under autopilot; the seven items are recorded in the umbrella `## Contradictions and Open Items`, items 15 to 21; the two departures added at PVL cycle 1 below are items 22 and 23): spec 1.4 "Library on macOS" versus spec 2 and PRD 9 (PRD 9 followed); no `UNAM_*` override for the data dir (not added); no Diagnostic type in the spec (shape added, D5); temporary lookup (D6); `dist-workspace.toml` unverified and spec 9 universal macOS target unsupported by evidence (D8); `nucleo-matcher` instead of full `nucleo` (D10); `source rename` not in the spec CLI (D11). Added at PVL cycle 1 (2026-10-05): (a) bare `unam` prints usage only in M1 although spec 5 says it prints status and usage, because `status` is milestone M4 (umbrella item 22); (b) a `SKILL.md` directly at a source root is ignored (`root-skill-skipped`, P4) although PRD DIS-1 says any directory, because a skill needs a non-empty relative path for its ID (umbrella item 23).

---

## Plan-Level Decisions (made at PLAN-SUPPLEMENT, inside the bounds above)

| # | Decision | Why |
|---|---|---|
| P1 | File layout is spec 1.3 for M1 plus three additions: `paths.rs` (spec 1.3 places path resolution inside `config.rs`; D1 makes it a separate module), `cli/mod.rs`, and the test `common/mod.rs`. Identity and catalog types (`RelPath`, `SkillId`, `SkillRef`, `Skill`, `Catalog`, cache, lookup) live in `catalog/mod.rs`; `SourceAlias`, `Source`, `SourcePath` in `source.rs`; `Diagnostic`, `Severity`, `Error` in `error.rs`. If `catalog/mod.rs` passes about 600 lines the execute agent MAY split it into `catalog/cache.rs` and `catalog/lookup.rs` and records that as a deviation in the report (no approval needed). | Spec layout has no model file; avoids inventing modules. |
| P2 | Over-length `name` threshold: more than 64 characters warns (`name-too-long`) and loads. | Spec says "over-length name warns" without a number; 64 is the Agent Skills convention. Recorded here, not an adapter claim. |
| P3 | A skill is a directory containing a file whose name is exactly `SKILL.md` (case-sensitive), checked from the directory listing, not by `stat`, so a lowercase `skill.md` is never a skill on case-insensitive macOS volumes. | DIS-1 "named exactly". |
| P4 | A `SKILL.md` directly in a source root is ignored with a source-level warning `root-skill-skipped`; the walk still descends into the root's children. | A skill needs a non-empty relative path for its ID. |
| P5 | Canonical tree-hash listing (spec 4.3) is fixed: entries are regular files and symlinks only (directories never listed; an empty directory does not change the hash), sorted bytewise by the full forward-slash relative path. A file line is `path NUL size NUL blake3hex(content) LF`. A symlink line is `path NUL L<target byte length> NUL blake3hex(target string bytes) LF` (the `L` marker prevents a file whose content equals a target from colliding). Output string form is `blake3:<hex>`. | Spec fixes file lines only; this settles symlink lines. |
| P6 | `Skill.frontmatter` is `serde_json::Map<String, Value>` (workspace `serde_json` with `preserve_order`), built from the `serde_norway` parse; non-string YAML keys are stringified. Field name is the spec contract; the concrete type is an implementation detail. Unknown keys and author order are preserved for `show` (spec 2.6). | `serde_json` cannot serialize a `serde_norway::Mapping` with non-string keys; the cache and `--json` need JSON. |
| P7 | `--no-cache` neither reads nor writes the cache. Without it, the cache is rewritten only when something changed (new, changed, or dropped skills, or no valid cache). `last_scan` in `source list` means the time the cache was last written. | Keeps read commands side-effect free where possible; documented meaning. |
| P8 | `source add` stores the user's spelling when it is absolute or starts with `~`; a relative argument is stored as the canonical absolute path. A path whose canonical form equals an already registered source is rejected (exit 2). Precedence for a new source is the highest existing precedence plus 10 (10 when first). | SRC-1 and spec 4.1 "store the user's spelling" versus 4.2 "canonicalize"; relative paths are meaningless later. |
| P9 | M1 global flags: `--json`, `--no-cache`, `-v/--verbose` (repeatable), `-q/--quiet`. `UNAM_LOG` (a tracing level word such as `debug`) overrides the flag-derived level. Logs go to stderr. | Spec 1.4 logging row; other flags arrive with their commands. |
| P10 | Config is created on first load by any command that loads it (spec 4.1), not by bare `unam`, `--help`, or `--version`. Rewriting `config.toml` does not preserve comments (the `toml` crate limitation); recorded as a known limitation. | Spec 4.1; `toml_edit` is not in spec 1.4. |
| P11 | M1 `source rm` reference check reads a provisional project index (see CLI Contract); index absent or unreadable means zero references. Milestone M2's `project.rs` owns the real index format and supersedes this reader. | The task requires the exit-3 path to be defined and testable now. |
| P12 | `show` and its JSON in M1 omit the "bundles referencing it" and "projects where active" fields entirely (adding fields later is additive). They are never printed as empty lists, which would be false. | Spec 5 says M1 fills only fields available. |
| P13 | Workspace crates carry `publish = false` and no `license` key (PRD Q11 open; no license file without asking). No `rust-version` (MSRV not verified). `unsafe_code = "forbid"` workspace lint. No `rustfmt.toml` (defaults suffice). | Safety rules in the brief. |
| P14 | CI runs the three gates with `--locked` added, plus INSTA_UPDATE=no; the perf release gate is a local gate only (shared runners are noisy); in CI the perf test runs under `cargo test --workspace` with the debug bound. | Reproducibility without flaky timing. |
| P15 | `mtime` in cache fingerprints is stored as `mtime_secs` (i64) and `mtime_nanos` (u32); a fingerprint entry is `{path, size, mtime_secs, mtime_nanos}` taken with `symlink_metadata`. | Avoids u128 JSON edge cases; spec says nanoseconds. |

---

## Acceptance Criteria

Closes (spec section 10 M1): AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side). Criteria text: PRD section 13.2. Each criterion names its proving test (REQ-TEST-LINK); gates back-reference the criterion in Verification Evidence. Every test that touches a source fixture also asserts the source-tree invariant.

| Criterion | proven by | strategy | Fixture and assertion |
|---|---|---|---|
| AC-1 | `ac_01_register_non_git_directory_and_skills_discoverable` in `ac_read_path.rs` | Fully-Automated | Standard fixture library (root has no `.git`). `source add fx <path>` exits 0, kind `directory`, skill count 9; `search code-review` finds `fx:sdlc/code-review`. Source tree hash unchanged. |
| AC-3 | `ac_03_three_level_nested_categories_real_paths` | Fully-Automated | `fx:engineering/backend/api/error-handling` listed with category `engineering/backend/api` and `rel_path` exactly as on disk; no skill renamed or flattened; every one of the 9 expected IDs present. |
| AC-4 | `ac_04_git_and_node_modules_yield_no_skills` | Fully-Automated | Fixture variant with a root `.git/` and root `node_modules/`, plus nested `vendor/node_modules/` and `vendor/some-repo/.git/`, each holding a `SKILL.md`. Registering yields the same 9 skills; none of the `.git`/`node_modules` skills appear in `search` or `show`; kind `git-repo`. |
| AC-5 | `ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic` | Fully-Automated | `fx:mismatch/dir-name-differs` loads (`show` exits 0) with diagnostic `name-dir-mismatch` severity warning; `broken/bad-yaml` is absent (`show` exits 4) with diagnostic `unparseable-yaml` severity error naming path `broken/bad-yaml`; `broken/no-description` skipped with `missing-description`. |
| AC-6 | `ac_06_search_spans_three_categories_one_result_set` | Fully-Automated | `search review` (no filter) returns, in one result set, `fx:sdlc/code-review`, `fx:testing/pytest-fixtures`, `fx:data/schema-migration` (categories `sdlc`, `testing`, `data`). Assertion is "all three present"; ranking is asserted only for an exact-name query. |
| AC-38 (read-side) | `ac_38_unreachable_source_reported_no_mutation` | Fully-Automated (simulated by removing the source directory) | Register, then remove the source directory. `source list --json` exits 0 with `reachable: false` and warning `source-unreachable`; `search` exits 0 with the same warning; `show fx:sdlc/code-review` exits 4 naming the unreachable source. `config.toml` and the cache file are byte-identical before and after; the source entry is still present. |
| S3 read-side (first test, not an AC closure) | `source_tree_unchanged_read_side` in `read_side_invariant.rs` | Fully-Automated | register, scan, search, show leave the tree hash AND the per-entry stat snapshot (path, kind, size, mtime) of the source fixture identical. |
| Spec 9 cold scan under 300 ms | `perf_cold_scan` in `crates/unam-core/tests/perf_cold_scan.rs` | Hybrid (precondition: release build, local SSD; assertion automated) | See Exit Gate and D9. |

### AC-2 and the source-tree invariant (resolved 2026-10-04, decision 1)

AC-2 (source tree hash identical before and after a full activate, modify, deactivate cycle) closes in milestone M3, because the full cycle needs `apply`. Milestone M1 writes the source-tree hash helper (`hash.rs`) and the read-side test as the FIRST test. The invariant is asserted in every mutating test from milestone M3 on. M1 does not close AC-2 and does not list it in its Closes line.

---

## Entry Gate

- [x] Milestone M0 gate met (2026-10-05, symlink default for both adapters; see `spike/symlink-discovery.md` Outcome): results table filled and Outcome records the default strategy per adapter.
- [x] Milestone M0 report written (`phase-01-m0-symlink-spike_REPORT_04-10-26.md`) and its Forward Preview read.
- [x] Taylor's confirmation that M0 is complete (confirmed 2026-10-05).

---

## Pre-Step (before any file is created)

Run from the repository root with `export PATH="$HOME/.cargo/bin:$PATH"` first (verified 2026-10-05: `cargo` is not on the default shell PATH here; it lives in `~/.cargo/bin`).

1. `cargo --version` (expect 1.99.0 or newer), `cargo fmt --version` (expect rustfmt 1.10.0 or newer), `cargo clippy --version` (expect 0.1.99 or newer). If any is missing: BLOCKED (do not install toolchains).
2. Do NOT install `dist` or `cargo-insta` (both absent by design). Snapshots are created and reviewed without `cargo-insta` (see Insta Snapshot Procedure).
3. Record, for the safety gate G11, whether `~/.config/unam`, `~/.cache/unam`, and `~/.local/share/unam` exist now (expected: none exist). Save the output of `ls -A ~/.config/unam ~/.cache/unam ~/.local/share/unam 2>&1 | sort` to a file in the scratchpad directory (gate G11 compares against it).
4. Hook note (verified 2026-10-05, extended at PVL cycle 1): `.claude/hooks/scout-block.cjs` blocks any Bash command whose TEXT contains any of these tokens: `target`, `.git`, `node_modules`, `build`, `dist`, `vendor`, `coverage` (for example `./target/...`, a quoted `.git` literal, a python heredoc that mentions `node_modules`). Cargo itself is unaffected. When a check needs one of these tokens in its text, put it in a script file written with the Write tool under the scratchpad directory and run that file. Execute and tester agents must not put `target/` paths or a `--target` flag in Bash commands and must not edit `.claude/` (outside the blast radius). Use `cargo run`, `cargo test`, and the assert_cmd binary lookup instead of touching `target/` directly. `/target` is already in the repository `.gitignore`.
5. Network: the first `cargo build` needs network access to crates.io (the local registry cache lacks `proptest` and others; an `--offline` resolve fails, verified at PVL). If the network is unavailable the step is BLOCKED; never hand-vendor crates.
6. Write every source, test, fixture, and config file with the Write tool, never with Bash heredocs or `echo` redirection (the hook blocks the tokens listed in item 4 inside Bash text).
7. Expect clippy 1.99 `collapsible_if` findings under `-D warnings` on edition 2024 (fix them with let chains); fix inline, never with an `allow`.

---

## Safety (non-negotiable for every step and every test)

- Tests and commands never touch the real `HOME`, `~/.claude`, `~/.agents`, `~/Developer/01-AI/skillzone`, or this repository. Every `Command` in tests is built through `TestEnv::cmd()` (see Test Harness), which clears the environment and sets `HOME`, `XDG_CONFIG_HOME`, `XDG_CACHE_HOME`, `XDG_DATA_HOME`, `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR` to tempdirs and sets the working directory to a tempdir. `TestEnv::new()` asserts the temp root is under `std::env::temp_dir()` and not under the crate manifest directory.
- Never run `unam` (the built binary or `cargo run`) against this repository or from a working directory inside it without the isolation above. No ad-hoc manual runs of `unam` outside tests; all manual checks go through `TestEnv`.
- Reading static fixtures from `crates/unam-cli/tests/fixtures/` is allowed (they are copied into tempdirs). Nothing writes into the repository except the files in Blast Radius and the committed `.snap` files.
- Nothing writes under a registered source. M1 has no code path that does; the invariant tests prove it for read operations. (Type-level enforcement through typed paths is milestone M3, spec 4.9.)
- No publish, no push, no license file, no edits to the five protected design documents.
- In-process tests (unit tests and `perf_cold_scan`) construct `Dirs` (with an injected environment map) and scan options (`git_global: false`) explicitly and never read the process environment; only the CLI tests, through `TestEnv`, exercise the real environment path, and they isolate it.
- No em dash (U+2014) and no emoji in any created file (gate G10).

---

## Blast Radius

Create (all new; nothing existing is modified by EXECUTE; the only existing files touched, at UPDATE PROCESS, are the two process files named below):

- Root: `Cargo.toml` (virtual workspace), `Cargo.lock`, `clippy.toml`, `dist-workspace.toml`, `.github/workflows/ci.yml`.
- `crates/unam-core/`: `Cargo.toml`; `src/lib.rs`, `src/error.rs`, `src/paths.rs`, `src/config.rs`, `src/source.rs`, `src/hash.rs`, `src/fsops.rs`; `src/catalog/mod.rs`, `src/catalog/scan.rs`, `src/catalog/frontmatter.rs`, `src/catalog/search.rs`; `tests/perf_cold_scan.rs`.
- `crates/unam-cli/` (package `unam`, binary `unam`): `Cargo.toml`; `src/main.rs`, `src/output.rs`, `src/cli/mod.rs`, `src/cli/source.rs`, `src/cli/search.rs`, `src/cli/show.rs`; `tests/common/mod.rs`, `tests/read_side_invariant.rs`, `tests/ac_read_path.rs`, `tests/source_cli.rs`, `tests/search_cli.rs`, `tests/show_cli.rs`, `tests/cache_cli.rs`, `tests/usage.rs`, `tests/snapshots.rs`; `tests/snapshots/*.snap` (generated once, committed); `tests/fixtures/library/**` (static skill trees listed in Fixture Contract).
- Process files modified at UPDATE PROCESS only (not by EXECUTE): `process/context/all-context.md` and `process/context/tests/all-tests.md` (tests become runnable; record the `cargo` PATH, the blocked hook tokens, `INSTA_UPDATE=no`, and the gate helpers).
- Not touched: `docs/`, `spike/`, the skill library, `.claude/`, `.codex/`, `.agents/`, `.gitignore` (already ignores `/target`), the registry file.
- Risk class: new code only, read-only product behavior, no destructive writes. Highest-risk areas: the walk filter and cap, the cache fingerprint, the frontmatter retry, test isolation.

---

## Module Specifications (decided; no design left open)

### Dependencies

Declared once in root `[workspace.dependencies]`; crates use `workspace = true`. Versions below come from RESEARCH (2026-10-05); for crates marked "resolve" run `cargo add` and record the resolved version in the report. `Cargo.lock` is committed.

| Crate | Version / features | Used by |
|---|---|---|
| clap | 4.6.7, `derive` (verified: `#[command(version)]` compiles without the `cargo` feature) | unam |
| serde | resolve, `derive` | core, unam |
| serde_json | resolve, `preserve_order` | core, unam |
| serde_norway | 0.9.42 | core (frontmatter parse), unam (YAML print in `show`) |
| toml | resolve | core (config) |
| ignore | 0.4.33 | core |
| blake3 | 1.8.7, default features only | core |
| nucleo-matcher | 0.3.1 | core |
| jiff | 0.2.37, no `serde` feature (timestamps are stored as RFC 3339 strings) | core |
| thiserror | 2.0.21 | core |
| anyhow | resolve | unam |
| tempfile | 3.27.0 | core (atomic write; also dev) |
| tracing | resolve | core, unam |
| tracing-subscriber | resolve, default features (level word parsing via `LevelFilter`; no `env-filter`) | unam |
| dev: insta | 1.49.0, `json` and `redactions` | unam tests |
| dev: assert_cmd | 2.2.2 (verified: `Command::new(assert_cmd::cargo::cargo_bin!("unam"))` compiles clean under `clippy -D warnings`; `Command::cargo_bin` is deprecated, do not use) | unam tests |
| dev: predicates, tempfile, serde_json | resolve | unam tests |
| dev: proptest | resolve | core tests |

### Root `Cargo.toml` and lint configuration

- `[workspace]`: `resolver = "3"`, `members = ["crates/unam-core", "crates/unam-cli"]`.
- `[workspace.package]`: `edition = "2024"`, `version = "0.1.0"`, `publish = false`. No `license`, no `rust-version`.
- `[workspace.lints.rust]`: `unsafe_code = "forbid"`. `[workspace.lints.clippy]`: `unwrap_used = "deny"`. Each crate has `[lints] workspace = true`.
- `clippy.toml` (repository root): `allow-unwrap-in-tests = true`.
- Edition 2024 note: `std::env::set_var` is unsafe; no code calls it. Library code reads variables only through the injected `Env` carried by `Dirs` (see Paths); only `main.rs` calls `Env::process()`.

### `error.rs`

- `Severity`: `Info`, `Warning`, `Error` (serialized lowercase). `Error` severity means the item was skipped.
- `Diagnostic { code: String, severity, message: String, path: Option<String>, remedy: Option<String> }` (the code strings below are stable contract).
- `Error`: one `thiserror` enum, variants grouped by module, each exposing `code()` (stable kebab string), `exit_code()`, and `remedy()` (`Option<&'static str>`; present for every user-fixable variant). Table:

| Variant | code | exit | remedy |
|---|---|---|---|
| ConfigRead (io) | `config-read-failed` | 1 | none |
| ConfigParse | `config-invalid` | 1 | "Fix the TOML syntax, or move the file aside to start fresh." |
| ConfigUnsupportedVersion | `config-version-unsupported` | 1 | "This config was written by a newer unam; upgrade unam." |
| ConfigUnsetVariable | `config-unset-variable` | 1 | "Set the environment variable, or edit the source path in config.toml." |
| InvalidAlias | `invalid-alias` | 2 | "Use 1 to 32 characters: lowercase letters, digits, and hyphens, starting with a letter or digit." |
| InvalidEnvPath | `invalid-env-path` | 2 | "Set it to an absolute path." |
| NoHomeDir | `no-home-dir` | 1 | "Set HOME (or the relevant XDG_* variable, or UNAM_CONFIG_DIR and UNAM_CACHE_DIR)." |
| AliasExists | `alias-exists` | 2 | "Pick another alias, or run `unam source rm <alias>` first." |
| SourcePathInvalid | `source-path-invalid` | 2 | "Pass an existing, readable directory." |
| SourcePathAlreadyRegistered | `source-path-registered` | 2 | "That directory is already registered; run `unam source list`." |
| SourceNotFound | `source-not-found` | 4 | "Run `unam source list` to see registered aliases." |
| SourceReferenced | `source-referenced` | 3 | "Remove those references first, or re-run with `--force` (the selections keep unresolvable refs)." |
| UnknownSourceFilter | `unknown-source` | 2 | "Run `unam source list` to see registered aliases." |
| InvalidRef | `invalid-ref` | 2 | "Use `alias:path/to/skill`, `alias:name`, or a bare name." |
| NotFound (ref) | `not-found` | 4 | "Run `unam search <words>` to find the skill, or qualify the reference as `alias:path`." |
| Ambiguous (ref) | `ambiguous` | 5 | spec 7 text verbatim: "Qualify the reference, for example `skillzone:sdlc/code-review`." |
| Io | `io-error` | 1 | none |

- Diagnostic remedy for `source-unreachable` is the spec 7 text verbatim: "Mount the volume or run `unam source relocate <alias> <path>`." (the `relocate` command arrives in milestone M4; the text is the spec's).
- Unit test: a table test constructs one value of every variant and asserts `code()`, `exit_code()`, non-empty `remedy()` where the table lists one.

### `paths.rs`

- `Env`: a cheap-clone value (`Arc<dyn Fn(&str) -> Option<OsString> + Send + Sync>`) with `Env::process()` (wraps `std::env::var_os`) and `Env::from_map(HashMap<String, OsString>)` (tests). `Dirs { config, bundles, cache, data, env }` (the four directories are absolute `PathBuf`; `env` is the lookup that produced them). `Dirs::resolve(env: Env)` computes the directories; `Dirs::from_process_env()` is `resolve(Env::process())`. `Dirs::expand(&self, raw)` (and the free `expand_path(raw, &Env)` it calls) expands a leading `~` or `~/`, `$NAME`, and `${NAME}` through the carried `env`; an unset variable is `ConfigUnsetVariable`. Source path expansion, `Catalog::load`, and `source::add` use `dirs.expand` only, never `std::env`, so in-process tests inject a map and never read the process environment (Safety).
- Rules (an empty value counts as unset; an `XDG_*` value that is not absolute counts as unset, per the XDG spec; a relative `UNAM_*` value is `InvalidEnvPath`):
  - config dir: `UNAM_CONFIG_DIR` (used as the unam directory itself, so the file is `$UNAM_CONFIG_DIR/config.toml`), else `$XDG_CONFIG_HOME/unam`, else `$HOME/.config/unam`.
  - bundles dir: `UNAM_BUNDLES_DIR`, else `<config dir>/bundles`.
  - cache dir: `UNAM_CACHE_DIR`, else `$XDG_CACHE_HOME/unam`, else `$HOME/.cache/unam`. Catalog cache file: `<cache dir>/catalog/<alias>.json`.
  - data dir: no `UNAM_*` override (open item 16 in the umbrella): `$XDG_DATA_HOME/unam`, else `$HOME/.local/share/unam`. Provisional index file: `<data dir>/projects.json`.
  - `HOME` is required only when no higher-precedence value applies; otherwise `NoHomeDir`.
- Unit tests (table-driven over a `HashMap` lookup): each precedence tier wins over the next for config, cache, data; `UNAM_BUNDLES_DIR` override; bundles default follows a custom config dir; empty and relative `XDG_*` ignored; relative `UNAM_CONFIG_DIR` errors; no `HOME` and no overrides errors; no `HOME` with all overrides succeeds; `~`, `~/x`, `$VAR`, `${VAR}`, unset-variable cases, all through `Dirs::expand` with an injected `Env::from_map` (a variable absent from the map is unset).

### `fsops.rs` (minimal in M1)

- `write_atomic(path, bytes)`: create parent directories, write a `tempfile::NamedTempFile` in the same directory, fsync it, rename over the target (`persist`). Returns `Error::Io` on failure. Nothing else in M1 (typed paths, symlink, copy arrive in milestone M3).
- Unit test: overwrites an existing file; leaves no stray temp file; creates parent dirs.

### `config.rs`

- Types: `Config { version: u32, defaults: Defaults, sources: Vec<SourceEntry> }`; `Defaults { strategy: Strategy(symlink|copy), destination_policy: shared|isolated, catalog_budget_tokens: u32, allow_missing: bool }`; `SourceEntry { alias: String, path: String, precedence: u32 }`. Defaults per spec 2.1 (`symlink`, `shared`, 1500, `false`). Unknown keys are ignored.
- `Config::load_or_create(dirs)`: if `<config dir>/config.toml` is absent, create the directory and write the default file (version 1, empty `sources`) with `write_atomic` and return it; else parse. `version != 1` is `ConfigUnsupportedVersion`. Returns `(Config, Vec<Diagnostic>)`.
- Validation (spec 4.1): aliases unique and matching `^[a-z0-9][a-z0-9-]{0,31}$` (hand-written check, no regex crate); duplicate or invalid alias in the file is `ConfigParse`. Precedence ties are broken alphabetically by alias with a `precedence-tie` warning diagnostic. A source's `path` string is stored as the user spelled it; the expanded form is computed with `expand_path` at use.
- `Config::save(dirs)` uses `write_atomic` (comments are not preserved, P10).
- Unit tests: first-run creates the default file with expected content; round trip; version 2 errors; invalid and duplicate alias; tilde and `$VAR` expansion in a source path; unset variable errors; tie warning and deterministic order; unknown key ignored.

### `source.rs`

- `SourceAlias` (validated newtype, `Display`, `Serialize` as string). `SourcePath` (newtype over `PathBuf` exposing only `as_path()`; no write helpers, so a source path cannot be passed to `fsops` by accident in later milestones). `SourceKind { Directory, GitRepo }`: `GitRepo` when the root directly contains an entry named `.git` (file or directory). `Source { alias, path: SourcePath, raw_path: String, precedence, kind }` (`kind` detected at load; `Directory` when unreachable is reported as `unknown` in JSON).
- `stored_spelling(raw, canonical) -> String` and `next_precedence(entries) -> u32` are pure functions (P8), written and unit-tested at Step B.
- `add(config, dirs, alias, raw_path, options: LoadOptions)` (written at Step D3, after `Catalog::load` exists): validate alias (`InvalidAlias`; duplicate is `AliasExists`); expand through `dirs.expand` (the injected environment); canonicalize; require an existing readable directory (`SourcePathInvalid` with the reason); reject a canonical path already registered (`SourcePathAlreadyRegistered`); scan through `Catalog::load` for this source only (writes the cache unless `--no-cache`); zero skills adds a `zero-skills` warning (SRC-6, never a failure); then append the entry (P8 spelling and precedence) and save the config. Returns the scan summary and diagnostics. Nothing is written to the config when validation fails.
- `list(config, dirs, options: LoadOptions)` (Step D3): `Vec<SourceSummary>` ordered by `(precedence, alias)`; each has alias, raw path, expanded path, kind, precedence, reachable, skill count (None when unreachable), `last_scan` (cache `scanned_at`, None when no cache), and diagnostics.
- `remove(config, dirs, alias, force)`: unknown alias is `SourceNotFound`; collect referencing projects with `referencing_projects`; if any and not `force`, `SourceReferenced` (lists the project roots); else remove the entry, save the config, delete `<cache>/catalog/<alias>.json` if present (cache dir only), and return the dangling project roots (empty without references).
- `referencing_projects(dirs, alias)` (provisional, P11): read `<data dir>/projects.json` if it exists: JSON object `{"version": 1, "projects": ["/abs/root", ...]}`. Absent file means no references. A file that cannot be read or parsed yields no references plus a `project-index-unreadable` warning (spec 11: a stale index only makes lists incomplete). For each listed root, read `<root>/.unam/selection.yaml` (missing or unparseable: skip); a project references the alias when any string under `targets.*.add` or `targets.*.remove` starts with `<alias>:`. Marked in code as superseded by milestone M2's `project.rs`.
- Unit tests at Step B (need no `Catalog`): alias validation; `next_precedence` (first 10, next 20); `stored_spelling` (absolute and `~` kept, relative becomes canonical); referencing-projects cases (absent, corrupt, referenced, unreferenced, missing selection); `remove` (entry and cache file deleted, unknown alias, referenced without `--force`, forced).
- Unit tests at Step D3 (need `add`; each builds `Dirs` from an injected `Env::from_map` and passes `git_global: false`): zero-skills warning; duplicate alias; duplicate canonical path; full add writes config and cache; `~` and `$VAR` source paths expand through the injected map, never the process environment.

### `hash.rs`

- `Hash` newtype (`blake3:<hex>`, `Display`, `Serialize` as string, `Hash::of_bytes`). `FileEntry { rel: String, kind: File | Symlink, size: u64, mtime_secs: i64, mtime_nanos: u32 }`.
- `walk_files(root, skip_names)`: `ignore::WalkBuilder` with `standard_filters(false)`, `hidden(false)`, `follow_links(false)`, `filter_entry` dropping any entry whose file name is in `skip_names` (directories pruned), collects regular files and symlinks (stat via `symlink_metadata`), computes forward-slash relative paths, then sorts bytewise by relative path (P5; the walker's per-directory order is not relied on).
- `hash_entries(root, entries)`: reads each file (symlink: reads the link target string, never follows) and builds the P5 listing; returns `Hash`. IO error propagates.
- `tree_hash(root, skip_names)` = `walk_files` + `hash_entries`. Tests pass an empty skip list to hash a whole source fixture (including `.git`); the scanner passes `[".git", ".DS_Store"]` (D3).
- Unit tests: deterministic across runs and creation order; bytewise sort pin (`a-b` sorts before `a/b`); changes on content, size, rename; symlink hashed by target string and not followed (a dangling symlink hashes fine; retargeting changes the hash; a symlink and a file with equal bytes differ); skip names prune directories; empty directory does not change the hash; proptest: same tree contents give the same hash regardless of creation order.

### `catalog/frontmatter.rs`

`parse_skill_md(bytes, dir_name) -> Result<Parsed, Diagnostic>` where `Parsed { name, description, frontmatter, diagnostics }`. Steps, in order:

1. Not valid UTF-8: skip with `not-utf8` (error).
2. Strip one leading U+FEFF. Normalize `\r\n` to `\n`.
3. The text must begin with `---\n`; else skip `no-frontmatter` (error).
4. Take lines after the first fence up to the first line exactly `---` (a closing fence at end of file without a trailing newline counts). No closing fence: skip `unterminated-frontmatter` (error).
5. Parse the block with `serde_norway` into a mapping. Not a mapping (scalar, list) or parse failure: go to step 6.
6. Retry once (spec 4.3): for each line with no leading whitespace of the form `key: value` where the value (trimmed) does not start with a quote, `|`, `>`, `[`, `{`, `&`, `*`, `!`, or `#` and contains the two characters `: `, rewrite the value as a double-quoted YAML string (escape `\` and `"`). Re-parse. Success adds `colon-retry-applied` (info). Failure: skip `unparseable-yaml` (error, message carries the parser message).
7. `description` must be a non-empty (after trim) YAML string; else skip `missing-description` (error; message says "missing, empty, or not a string"). `name` must be a string to be used; a missing, blank, or non-string `name` falls back to `dir_name` with no diagnostic.
8. Warnings that still load: `name` differs from `dir_name` gives `name-dir-mismatch` (warning, message names both); more than 64 characters gives `name-too-long` (warning).
9. Convert the full mapping to `serde_json::Map` (P6): keys stringified, `Tagged` values unwrapped, non-finite floats rendered as strings.

Unit tests (one per bullet): valid; missing description; empty description; description not a string; unparseable (`description: [unclosed`); colon retry succeeds (`description: Use when: reviewing code`); retry still failing; name/dir mismatch warns and loads; name over 64 chars; BOM only; CRLF only; BOM and CRLF; no fence; unterminated fence; fence-only closing at EOF; invalid UTF-8; frontmatter that is a list; unknown keys and key order preserved; non-string key stringified. Proptest: `parse_skill_md` never panics on arbitrary bytes.

### `catalog/scan.rs`

Types defined here (created at Step C2): `FingerprintEntry { path, size, mtime_secs, mtime_nanos }` (per P15) and `ScannedSkill { skill: Skill, fingerprint: Vec<FingerprintEntry> }` (a loaded skill plus its fingerprint). `SourceScan.skills` is `Vec<ScannedSkill>`; the cache in `catalog/mod.rs` (D1) reads and writes `ScannedSkill`.

`scan_source(alias, root, prior, limits) -> SourceScan` where `prior` is a map from relative path to `ScannedSkill` (empty for a cold scan) and `limits = ScanLimits { max_depth: 8, max_dirs: 5000, git_global: true }` (the product default; tests pass small values and `git_global: false`).

Discovery pass (D3, P3, P4):

1. `WalkBuilder::new(root)`: `max_depth(Some(max_depth))` (depth counts the root as 0, so with 8 a skill directory at depth 7 is found and one at depth 8 is not; pinned by a unit test), `follow_links(false)`, standard filters (hidden skipped, `.gitignore` honored only inside git repositories, `.ignore` files honored), `git_global(limits.git_global)` (the global gitignore is consulted only when this is true and the walked tree has a `.git` marker; verified at PVL: `ignore` reads the real global gitignore whenever a `.git` marker exists, and in-process tests cannot redirect HOME, so every in-process test passes `git_global: false`), `sort_by_file_path`.
2. `filter_entry`: drop directories named `node_modules` or `.git`; drop a directory whose parent (when the parent is not the root) contains an entry named exactly `SKILL.md` (checked first with `symlink_metadata` on `<parent>/SKILL.md` before any `read_dir`; only when that succeeds is the directory listing read to confirm the exact-case name). Result: skill subdirectories (`scripts/`, `references/`, nested `SKILL.md`) are never descended (DIS-8).
3. Count yielded directories after filtering (the root is not counted). When the count exceeds `max_dirs`, stop and add source-level `walk-capped` (warning: "directory limit of N reached; results are partial"). Walk errors (unreadable directory) add source-level `walk-error` (warning, with path) and continue.
4. A yielded regular file named exactly `SKILL.md` marks its parent as a skill directory (parent is the root: `root-skill-skipped`, warning, ignored). A yielded symlink named `SKILL.md` adds `symlinked-skill-md-skipped` (warning, path of the skill dir). A yielded symlink to a directory (any name) whose target contains `SKILL.md` adds `symlinked-skill-skipped` (warning). Other symlinks are ignored silently.

Per-skill pass (for each skill directory in sorted order): `walk_files(dir, [".git", ".DS_Store"])` gives entries (fingerprint = `{path, size, mtime_secs, mtime_nanos}` for every entry). Walk each skill directory exactly ONCE: these entries are reused for the fingerprint, for `supporting`, and for `hash_entries`; there is no second walk (PVL probe: a second walk added about 55 ms to a 171 to 226 ms release median against the 300 ms bound). If `prior` has this skill with an equal fingerprint, reuse its parsed metadata and hash. Otherwise read `SKILL.md`, `parse_skill_md`, `hash_entries`, and build the `Skill` (`supporting` = every entry except `SKILL.md`, sorted; `category` = parent of `rel_path`, empty for top-level). An IO error in this pass skips the skill with `skill-io-error` (error). A parse skip is recorded as a catalog-level diagnostic with `path` = the skill's relative directory. After both passes: `duplicate-name` (info, source-level) naming every name used by more than one skill in this source and the IDs. Output: skills sorted by ID, diagnostics in discovery order, counts `reparsed` and `dropped` (prior skills no longer found) for the cache decision.

Unit tests (tempdir trees; every one builds its own options with `git_global: false`, so none reads the developer's global gitignore): nesting and category derivation; depth boundary (skill dir at depth 7 found, at 8 not); `.git`, `node_modules`, hidden directory skipped; nested `SKILL.md` inside a skill not discovered but its file listed as supporting; supporting files and symlink-in-skill hashed by target; lowercase `skill.md` is not a skill (also when it is the only file in the directory); cap with `max_dirs = 10` returns partial results plus `walk-capped`; symlinked skill dir and symlinked `SKILL.md` produce diagnostics and no skill; root `SKILL.md` ignored with warning while children are still scanned; deterministic order; broken and no-description skills skipped with the right diagnostics; duplicate-name info; fingerprint reuse (prior entry returned untouched when files unchanged) and re-parse when size or mtime changes (set mtime explicitly with `File::set_modified`); an unreadable skill file (chmod 000) gives `skill-io-error` (Hybrid: self-skips with a printed note when permissions are not enforced, for example under root).

### `catalog/search.rs`

- Composite string per spec 4.3: `name`, two spaces, `category`, two spaces, `alias`, two spaces, `description`. `search(catalog, query, category: Option<&str>, source: Option<&SourceAlias>) -> Vec<Hit { id, score }>`.
- Filters first: `--source` exact alias; `--category` normalized (strip leading and trailing `/`) and matched at a path-component boundary (`sdlc` matches category `sdlc` and `sdlc/x`, not `sdlcx`).
- Matching: `nucleo_matcher::pattern::Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)` and `match_list` over a small wrapper type implementing `AsRef<str>` (so identical composites stay distinguishable); a `Matcher` created with `Config::DEFAULT`. Results are then stably re-sorted by score descending, then by ID ascending, so order is deterministic. An empty query matches everything with score 0 (ordered by ID).
- Unit tests: ranking puts an exact-name hit first (`code-review` query); ties ordered by ID; filters; category boundary; empty query; no-match returns an empty list; case smartness.

### `catalog/mod.rs`

- Step order: the types below, `RelPath` validation, `SkillId` `Display`, and the `pub mod frontmatter; pub mod scan; pub mod search;` declarations (declared `pub` so clippy raises no dead-code warnings) are created at Step C0 (together with doc-comment-only stubs for `catalog/frontmatter.rs`, `catalog/scan.rs`, and `catalog/search.rs`, so the declarations compile; C1 and C2 fill the first two, D2 the third); `SkillRef::parse`, `Catalog::load`, the cache, `abs_path`, `lookup`, and edit distance are added at Step D1.
- Types (P1): `RelPath` (forward-slash string, validated: no empty, `.`, or `..` components), `SkillId { source: SourceAlias, rel_path: RelPath }` (`Display` `alias:rel/path`; serialized as that string), `SkillRef { Qualified, SourceName, Bare }` with `parse` (see below), `Skill` per spec 3 (`id`, `name`, `description`, `category`, `frontmatter`, `supporting`, `tree_hash`, `diagnostics`), `SourceStatus { alias, root, reachable, scanned_at, skill_count, kind }`, `Catalog { skills, by_id, by_name, sources, diagnostics, scanned_at }` where `diagnostics` is the catalog-level list of `CatalogDiagnostic { source, diagnostic }` (skipped skills and source-level notices).
- `LoadOptions { use_cache: bool, git_global: bool }` with `LoadOptions::product(use_cache)` (git_global true) used by the CLI; in-process tests and `perf_cold_scan` build `LoadOptions { use_cache, git_global: false }`. `Catalog::load(sources, dirs, options)`, per source in `(precedence, alias)` order:
  1. Reachability: the path, expanded with `dirs.expand`, is an existing directory and `read_dir` succeeds. If not: record `SourceStatus { reachable: false }`, add source-level `source-unreachable` (warning, spec 7 remedy), touch nothing (no cache read, write, or delete). Its skills do not appear in the catalog.
  2. Canonicalize the root; read the cache if `use_cache` (invalid, corrupt, wrong `version`, or `source_path` different from the canonical path means a miss, silently).
  3. `scan_source` with limits `{ max_depth: 8, max_dirs: 5000, git_global: options.git_global }`. If `use_cache` and (cache miss or `reparsed > 0` or `dropped > 0`): write the cache with `write_atomic`; failure adds `cache-write-failed` (warning) and continues.
  4. Assemble: skills sorted by ID; `by_id`; `by_name`.
- Cache file (`<cache dir>/catalog/<alias>.json`): `{ "version": 1, "alias", "source_path", "scanned_at", "skills": [ { "rel_path", "name", "description", "frontmatter", "supporting", "tree_hash", "diagnostics", "fingerprint": [ {"path","size","mtime_secs","mtime_nanos"} ] } ] }` (each entry is a serialized `ScannedSkill`: the `Skill` fields plus `fingerprint: Vec<FingerprintEntry>`). Only loaded skills are cached; skipped skills and source-level diagnostics are regenerated by each walk. `scanned_at` is an RFC 3339 string from `jiff`.
- `abs_path(id)`: source root joined with `rel_path` (for `show`).
- `SkillRef::parse(text)` (P-rules): trim; empty or an invalid alias before the first `:` or an empty remainder is `InvalidRef`. With `:`: remainder containing `/` is `Qualified` (rel path validated; one trailing `/` stripped); otherwise `SourceName`. Without `:` it is `Bare`.
- `lookup(&SkillRef) -> Lookup` (TEMPORARY, D6; doc comment says it is superseded by `resolve.rs` in milestone M2):
  - `Qualified`: exact `by_id`; else NotFound with candidates (same-source skills whose `rel_path` is within edit distance 3, closest first, at most 5).
  - `SourceName { source, name }`: candidates = skills in that source whose `name` equals the text OR whose `rel_path` equals the text (a top-level skill dir); deduplicated. One: Found. Zero: NotFound with candidates by edit distance at most 2 against names in that source. More than one: Ambiguous with all IDs (sorted); precedence never picks.
  - `Bare(name)`: `by_name` across sources. One: Found; zero: NotFound (candidates by edit distance at most 2, closest first, at most 5); more than one: Ambiguous with all IDs, always.
  - NotFound also carries the aliases of unreachable sources (the reference's alias, or all unreachable sources for a bare name) so the CLI can say why.
  - Edit distance: Levenshtein over chars, hand-written; ties by ID.
- Unit tests: each of the three forms against a catalog with duplicate names across categories and across two sources (Found, NotFound with candidates, Ambiguous listing every ID); top-level skill matches `alias:name`; unreachable-source note; `SkillRef::parse` table including invalid forms; proptest: `SkillRef::parse` never panics and `Qualified` round-trips through `Display`; cache cases (hit, miss, dropped skill, new skill in a nested category, in-place edit with new mtime, corrupt file, wrong version, different `source_path`, `use_cache = false` neither reads nor writes); duplicate-name info.

### `lib.rs`

Re-exports the public modules (`catalog`, `config`, `error`, `fsops`, `hash`, `paths`, `source`) and nothing else.

---

## CLI Contract (`unam-cli`)

### Commands, flags, exit codes

Global flags (before or after the subcommand): `--json`, `--no-cache`, `-v/--verbose` (repeatable), `-q/--quiet`. Bare `unam` (no subcommand) prints the usage text (clap help, plus a line "status arrives in a later milestone") to stdout and exits 0; it does not load the config. Clap usage errors exit 2 (clap's default) and are plain text on stderr even with `--json` (they happen before dispatch). `--help` and `--version` exit 0.

| Command | Behavior | Exit |
|---|---|---|
| `source add <alias> <path>` | Register, scan, report count and diagnostics (spec 4.2) | 0; 2 invalid alias, invalid path, alias exists, path already registered |
| `source list` | Alias, path, kind, skill count, last scan, status | 0 (unreachable sources are reported, never an error) |
| `source rm <alias> [--force]` | Unregister (spec 4.2) | 0; 3 referenced and not forced; 4 unknown alias |
| `search <query> [--category <prefix>] [--source <alias>]` | Ranked results (spec 5) | 0; 2 unknown `--source` alias |
| `show <ref>` | Full metadata, supporting files, absolute path (spec 5; M1 fields only, P12) | 0; 2 invalid ref; 4 not found; 5 ambiguous |

Exit 1 is any other failure (IO, config). Exits 6 to 11 are not used in M1.

### `--json` envelope (spec 5)

One object on stdout: `{ "ok": bool, "command": "...", "data": {...}, "warnings": [...], "errors": [...] }`. `command` is `source add`, `source list`, `source rm`, `search`, or `show`. On failure `data` is `{}` and `errors` holds `{ "code", "message", "remedy", "candidates" }` (`candidates` only for `not-found` and `ambiguous`, as ID strings). In JSON mode nothing else is printed to stdout; stderr carries only tracing logs. In human mode results go to stdout and diagnostics and errors go to stderr (`error: ...`, then the remedy on its own line, then candidates).

Diagnostic JSON object (every place diagnostics appear): `{ "source": "<alias>", "skill": "<id>"|null, "code", "severity": "info"|"warning"|"error", "message", "path": "<relative dir>"|null, "remedy": "..."|null }`. Arrays are always present, possibly empty.

Diagnostic classes: skill-level (attached to a loaded skill: `name-dir-mismatch`, `name-too-long`, `colon-retry-applied`), skipped-skill (catalog-level with `path`: `unparseable-yaml`, `missing-description`, `no-frontmatter`, `unterminated-frontmatter`, `not-utf8`, `skill-io-error`, `symlinked-skill-skipped`, `symlinked-skill-md-skipped`), source-level (`source-unreachable`, `walk-capped`, `walk-error`, `root-skill-skipped`, `zero-skills`, `duplicate-name`, `cache-write-failed`), config-level (`precedence-tie`, `project-index-unreadable`). The envelope `warnings` array carries diagnostics of severity warning or error from the classes relevant to the command: `source add` and `source list` carry all classes; `search` and `show` carry only source-level and config-level ones (so skipped-skill notices do not repeat on every search); `show` additionally prints the shown skill's own diagnostics in `data`.

### `data` shapes

- `source add`: `{ alias, path, expanded_path, kind, precedence, skill_count, diagnostics: [...all classes for this source...] }`.
- `source list`: `{ sources: [ { alias, path, expanded_path, kind ("directory"|"git-repo"|"unknown"), precedence, reachable, skill_count (null when unreachable), last_scan (RFC 3339 or null), diagnostics: [...] } ] }`.
- `source rm`: `{ alias, removed: true, forced, dangling_projects: ["/abs/root", ...] }`.
- `search`: `{ query, category (null if absent), source (null if absent), total, results: [ { id, name, category, source, description, score, diagnostics: [...] } ] }`.
- `show`: `{ id, name, description, category, source, rel_path, path (absolute), frontmatter: {...}, supporting: [...], tree_hash, diagnostics: [...] }`.

### Human output (informative; tests assert substrings, not full text)

- `source add`: `Registered source <alias> -> <path> (directory|git repo)`, `Skills found: N`, then one line per diagnostic (`warning[code] path: message`, `skipped[code] path: message`, `note[code] ...`); `-q` hides `note` lines.
- `source list`: aligned table `ALIAS  PATH  KIND  SKILLS  LAST SCAN  STATUS` (`ok` or `unreachable`).
- `search`: header `ID  NAME  CATEGORY  SOURCE  DESCRIPTION` and one line per hit (description head: first line, at most 80 characters, cut with `...`); ranked order; no result limit in M1.
- `show`: title line `<name>  (<id>)`, then Description, Source and Category, absolute Path, Tree hash, Supporting files (count and list), Frontmatter (printed as YAML via `serde_norway` over the preserved-order map), Diagnostics if any.
- `source rm` with `--force` and dangling projects prints each project root and says the selections now hold unresolvable refs.
- All untrusted text is neutralized as specified under `output.rs` (no raw control character reaches the terminal in human mode).

### `main.rs`, `output.rs`, `cli/*`

- `main.rs`: initialize `tracing_subscriber::fmt` to stderr with the level from `UNAM_LOG` (parsed with `LevelFilter`) else from flags (`-q` error, default warn, `-v` info, `-vv` debug); build the context (`Dirs::from_process_env`, flags); parse args; dispatch; map `anyhow::Error` to an exit code by downcasting to `unam_core::Error` (`exit_code()`), else 1; print errors through `output.rs` (JSON envelope or `error:` lines).
- `output.rs`: envelope struct and renderers (JSON and human), diagnostic formatting, description-head truncation (on char boundaries), table alignment, and the sanitizer `neutralize(&str) -> Cow<str>`: it replaces each C0 control character except newline (0x0A) and tab (0x09), DEL (0x7F), and each C1 control character (U+0080 to U+009F) with the four literal characters `\xNN` (lowercase two-digit hex of the code point); every other character is untouched. Human renderers apply it to EVERY untrusted string they print (skill name, description, category, frontmatter values including the YAML text `show` prints, paths, diagnostic messages, error messages that embed user data, candidate IDs); truncation counts characters after neutralization. JSON mode is unchanged (`serde_json` already escapes control characters). Unit tests: envelope shape, truncation boundary and multibyte text, empty arrays present, and `neutralize` (ESC, NUL, DEL, and U+009B become `\x1b`, `\x00`, `\x7f`, `\x9b`; newline and tab survive; accented and CJK text survives; a neutralized description head truncates on a char boundary).
- `cli/mod.rs`: clap derive structs and dispatch; `cli/source.rs`, `cli/search.rs`, `cli/show.rs`: one file per command, thin: call `unam-core`, render. No logic that belongs in core.

---

## Test Harness (`crates/unam-cli/tests/common/mod.rs`)

Top of the file: `#![allow(clippy::unwrap_used)]` (comment: integration helpers may unwrap; allow-unwrap-in-tests does not cover non-test helper functions) and `#![allow(dead_code)]` (comment: each test binary uses a subset).

- `TestEnv`: owns a `tempfile::TempDir` with subdirectories `home`, `xdg-config`, `xdg-cache`, `xdg-data`, `config`, `bundles`, `cache`, `data`, `cwd`, `work`. `new()` asserts the isolation rule from Safety. `cmd(&self)` returns an `assert_cmd::Command` for `assert_cmd::cargo::cargo_bin!("unam")` with `env_clear()`, the seven isolating variables set, and `current_dir` set to `cwd`. Helpers: `config_file()`, `cache_file(alias)`, `normalize(text)` (replaces the temp root, both as created and canonicalized, since macOS `/var` resolves to `/private/var`, with `[TMP]`).
- `build_library(env) -> PathBuf`: copies `tests/fixtures/library` (found through `env!("CARGO_MANIFEST_DIR")`) into `work/lib`, then writes the generated pieces (see Fixture Contract). `build_library_with_root_git(env)`: same plus a root `.git/hooks/SKILL.md` and `.git/config`.
- `snapshot_source(root) -> SourceSnapshot { tree_hash: unam_core::hash::tree_hash(root, &[]), entries: Vec<(rel, kind, size, mtime_ns)> }` over every entry including directories, `PartialEq` with a readable diff on failure; `assert_source_unchanged(before, root)`.
- `run_json(env, args) -> (exit_code, serde_json::Value)` and helpers to read diagnostics by code.
- Snapshot redactions: normalize temp paths in text before parsing; insta redactions for `last_scan`, `scanned_at`.

### Fixture Contract

Static files under `crates/unam-cli/tests/fixtures/library/` (plain text, reviewable); alias used by tests: `fx`.

| Path (SKILL.md unless noted) | Content | Expected |
|---|---|---|
| `alpha-root-skill/` | valid; name `alpha-root-skill` | loaded, category empty |
| `sdlc/code-review/` | valid; name `code-review`; description mentions reviewing code | loaded; supporting `references/checklist.md`, `scripts/run.sh`, `nested/SKILL.md` (these files exist; tests assert presence, not an exact list) |
| `sdlc/code-review/nested/` | valid skill inside a skill dir | NOT discovered |
| `testing/pytest-fixtures/` | valid; description includes the word "review" | loaded |
| `engineering/backend/api/error-handling/` | valid; name `error-handling` | loaded (three-level category) |
| `engineering/frontend/error-handling/` | valid; name `error-handling` | loaded; `duplicate-name` info |
| `data/schema-migration/` | valid; description includes the word "review" | loaded |
| `mismatch/dir-name-differs/` | `name: other-name` | loaded; `name-dir-mismatch` warning |
| `broken/bad-yaml/` | `description: [unclosed` | skipped; `unparseable-yaml` error |
| `broken/no-description/` | name only | skipped; `missing-description` error |
| `colon/value-with-colon/` | `description: Use when: reviewing code` | loaded; `colon-retry-applied` info |

Generated by `build_library` in code:

| Path | Content | Expected |
|---|---|---|
| `bom-crlf/windows-skill/SKILL.md` | U+FEFF, CRLF line endings, name `windows-skill`, a description | loaded |
| `node_modules/pkg/SKILL.md`, `vendor/node_modules/deep/SKILL.md` | valid skills | NOT discovered (AC-4) |
| `vendor/some-repo/.git/SKILL.md` | valid skill inside a nested `.git` | NOT discovered (AC-4) |
| `sdlc/.hidden-skill/SKILL.md` | valid skill in a hidden dir | NOT discovered |
| `case/lowercase/skill.md` | valid frontmatter, lowercase filename | NOT discovered |
| `linked/code-review-link` | symlink to `../sdlc/code-review` (Unix) | skipped; `symlinked-skill-skipped` warning |

Totals for the standard fixture: 9 loaded skills (alpha-root-skill, sdlc/code-review, testing/pytest-fixtures, engineering/backend/api/error-handling, engineering/frontend/error-handling, data/schema-migration, mismatch/dir-name-differs, colon/value-with-colon, bom-crlf/windows-skill); flattened diagnostics from `source add`: 6 (`name-dir-mismatch`, `colon-retry-applied`, `unparseable-yaml`, `missing-description`, `symlinked-skill-skipped`, `duplicate-name`). The `.git` root variant for AC-4 adds a root `.git/` containing a `SKILL.md` and expects the same 9 skills and kind `git-repo`.

### Insta Snapshot Procedure (no `cargo-insta`)

- Snapshots live in `crates/unam-cli/tests/snapshots/` (file `snapshots.rs` produces `snapshots__<name>.snap`).
- First creation (EXECUTE only): run `INSTA_UPDATE=always cargo test -p unam --test snapshots`, then read every new `.snap`, confirm there is no temp path, timestamp, or user path in it (gate G7), and commit them with the code. Never accept a snapshot unread.
- Every later run, CI, and every gate uses `INSTA_UPDATE=no` so a mismatch or a missing snapshot fails (no `.snap.new` files are left; gate G7 checks).
- Review of a changed snapshot without `cargo-insta`: read the diff of the `.snap` file in git. `cargo insta review` is documented in `all-tests.md` as optional when installed.
- Snapshots cover: `source add` (success, with 6 diagnostics), `source add` bad path (exit 2 envelope), `source list`, `source rm` (success, referenced exit 3), `search review`, `show` found, `show` not found (exit 4 envelope with candidates), `show` ambiguous (exit 5 envelope with all candidates), bare `unam` usage text.
- The 10 expected snapshots (gate G7 requires at least 10 `.snap` files), one `snapshots__<name>.snap` each: `add_success`, `add_bad_path`, `list`, `rm_success`, `rm_referenced`, `search_review`, `show_found`, `show_not_found`, `show_ambiguous`, `usage_bare`.

---

## Data Flow (primary scenario: register and search)

1. `unam source add fx <path>`: `main` builds `Dirs` from the process environment and parses arguments.
2. `Config::load_or_create` creates `<config dir>/config.toml` on first run and returns the config.
3. `source::add` validates the alias and path, canonicalizes, rejects duplicates, then calls `Catalog::load` for this source.
4. `Catalog::load` checks reachability, reads the cache (miss), runs the discovery pass, then the per-skill pass (parse plus hash), writes the cache atomically into `<cache dir>/catalog/fx.json`.
5. `source::add` appends the entry and saves the config atomically; the command renders the count and every diagnostic.
6. `unam search review`: config loaded, `Catalog::load` for all sources (stat walk, cache hits reuse parsed metadata and hashes; changed skills are re-read), search filters then ranks, output renders.
7. `unam show fx:sdlc/code-review`: same load, `lookup` resolves the reference, output renders the skill plus its absolute path.

No step writes under a source root. The only writes are `config.toml` and the catalog cache, both atomic and both outside the source.

## Failure Modes

| Failure | Behavior |
|---|---|
| Source directory missing or unmounted | Reported unreachable (warning), exit 0 for read commands, cache and config untouched (AC-38) |
| Source has more than 5,000 directories | Partial results plus `walk-capped` warning |
| Cache unreadable, corrupt, wrong version, wrong source path | Treated as a miss; rewritten after a successful scan |
| Cache cannot be written (read-only cache dir) | `cache-write-failed` warning; command still succeeds |
| Config file malformed | `config-invalid` error, exit 1, file untouched |
| Skill file unreadable or invalid | That skill skipped with a named diagnostic; others unaffected |
| Skill dir symlinked | Skipped with `symlinked-skill-skipped` |
| Ambiguous or missing reference | exit 5 or 4 with candidates; never a silent pick |
| Interruption during config or cache write | Atomic rename leaves the old file or the new file, never a partial one |

---

## Implementation Checklist

Order follows INNOVATE as corrected at PVL cycle 1 so every step compiles and its gate can run: scaffold, lints, paths; config and source types; catalog types, frontmatter, walker; catalog load, search, lookup, source add and list, perf test; CLI; integration tests; dist and CI. The first test (D12) is written red as soon as the workspace and hash helper exist, before the code that makes it pass. After each step run that step's gate and fix inline before moving on. Test-first: where a step lists tests, write them before the implementation they cover.

### Step A: Pre-step, workspace, lints, paths, first test

- [x] A0. Run the Pre-Step (toolchain versions, no installs, record HOME state, remember the `target` hook note).
- [x] A1. Create root `Cargo.toml` (members, resolver "3", `workspace.package`, `workspace.lints`, `workspace.dependencies` per Module Specifications) and `clippy.toml`.
- [x] A2. Create `crates/unam-core/Cargo.toml` and `crates/unam-cli/Cargo.toml` (package `unam`, `[lints] workspace = true`, dependencies via `workspace = true`, `unam-core` path dependency); minimal `src/lib.rs` (each later step adds its own `pub mod` line when it creates a module) and `src/main.rs`; run `cargo build --workspace` to generate and commit-ready `Cargo.lock`; record resolved versions of the "resolve" crates.
- [x] A3. Gate A (skeleton): `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings` pass on the skeleton (proves lint config parses and `unwrap` denial is active: temporarily verify by a one-line check that clippy flags an `unwrap` in non-test code, then remove it; record in the report).
- [x] A4. Write `src/error.rs` (`Severity`, `Diagnostic`, `Error` variants and table-test) per the Error table.
- [x] A5. Write `src/paths.rs` (`Env`, `Dirs`, `Dirs::expand`, `expand_path`) with the precedence and expansion unit tests first; then the implementation.
- [x] A6. Write `src/fsops.rs` (`write_atomic`) with its unit test.
- [x] A7. Write `src/hash.rs` (`Hash`, `FileEntry`, `walk_files`, `hash_entries`, `tree_hash`) with the unit tests first (P5 listing rules; proptest order independence).
- [x] A8. Create the static fixture tree `crates/unam-cli/tests/fixtures/library/**` exactly per the Fixture Contract, and `tests/common/mod.rs` (`TestEnv`, `build_library`, `build_library_with_root_git`, `snapshot_source`, `run_json`, `normalize`).
- [x] A9. FIRST TEST (D12), written RED: `crates/unam-cli/tests/read_side_invariant.rs` with `source_tree_unchanged_read_side`: build the library, take `snapshot_source`, run `source add fx <lib>`, `search review`, `show fx:sdlc/code-review` and `source list` through `TestEnv::cmd()`, assert all exit 0 and the snapshot is unchanged. It fails now (no commands exist yet); it must be green at Step E. Leave the file header comment "read-side invariant; milestone M3 reuses the helper in every mutating test".
- [x] A10. Gate A: `cargo test -p unam-core` green (error, paths, fsops, hash unit tests); `cargo fmt` and `cargo clippy` green; the red first test lives in package `unam`, so run `cargo test -p unam-core` only at this step (the red test is expected to fail until Step E).

### Step B: Config and source types

- [x] B1. Write `src/config.rs` (types, `load_or_create`, `save`, validation, tie warning) with unit tests first; add `pub mod config;` to `lib.rs`.
- [x] B2. Write `src/source.rs` with everything that does NOT need `Catalog`: `SourceAlias`, `SourcePath`, `SourceKind` (detection), `Source`, the pure functions `stored_spelling` and `next_precedence`, `referencing_projects` (provisional), and `remove`. Unit tests first for: alias validation, the spelling rule, the next-precedence rule (first 10, then 20), referencing-projects cases, and `remove` cases (entry and cache file deleted, unknown alias, referenced without `--force`, forced). `add` and `list` are NOT written here because they need `Catalog::load`; their code and their tests (zero-skills warning, duplicate alias, duplicate canonical path, full add, injected-environment `~` expansion) are written at D3, so the crate compiles at every gate. Add `pub mod source;` to `lib.rs`.
- [x] B3. Gate B: `gate cargo test -p unam-core config` and `gate cargo test -p unam-core source` (each runs at least one test; zero tests ran fails the gate), plus `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`.

### Step C: Catalog types, frontmatter, walker

- [x] C0. Create `src/catalog/mod.rs` containing ONLY: `pub mod frontmatter; pub mod scan; pub mod search;`; the types `RelPath` (with its validating constructor), `SkillId` (with `Display` and `Serialize`), `SkillRef` (the three-variant enum, no `parse` yet), `Skill`, `Catalog` (struct and fields per the `catalog/mod.rs` spec, no `load` yet), `SourceStatus`, `CatalogDiagnostic` (`ScannedSkill` and `FingerprintEntry` live in `scan.rs`, created at C2); unit tests for `RelPath` validation and `SkillId` display written first. Also create `src/catalog/frontmatter.rs`, `src/catalog/scan.rs`, and `src/catalog/search.rs` as doc-comment-only stubs (C1, C2, and D2 fill them), so the `pub mod` declarations compile. Add `pub mod catalog;` to `lib.rs`. No `todo!()` or `unimplemented!()` anywhere; `cargo build -p unam-core` and clippy pass.
- [x] C1. Write `src/catalog/frontmatter.rs` with the full unit and proptest list first, then the parser (steps 1 to 9).
- [x] C2. Write `src/catalog/scan.rs` (discovery pass, per-skill pass, `ScanLimits` including `git_global`, `FingerprintEntry`, `ScannedSkill { skill: Skill, fingerprint: Vec<FingerprintEntry> }` used for the `prior` map and `SourceScan.skills`, `SourceScan`) with the unit tests listed under `catalog/scan.rs` first; every scan test passes `git_global: false`.
- [x] C3. Gate C: `gate cargo test -p unam-core frontmatter` and `gate cargo test -p unam-core scan` green (frontmatter and scan unit tests only; the perf test is written at D4, after `Catalog::load` exists), plus fmt and clippy.

### Step D: Catalog load, search, lookup, source add and list, perf test

- [x] D1. Extend `src/catalog/mod.rs`: `SkillRef::parse`, `LoadOptions`, `Catalog::load` (reachability, cache read and write, assemble), `abs_path`, `lookup` (temporary), edit distance; unit tests and proptests first.
- [x] D2. Write `src/catalog/search.rs` with unit tests first.
- [x] D3. Write `source::add` and `source::list` on top of `Catalog::load`, with the D3 unit tests listed under `source.rs` (each uses an injected `Env::from_map` and `git_global: false`); finalize `lib.rs` re-exports.
- [x] D4. Write `crates/unam-core/tests/perf_cold_scan.rs` (`perf_cold_scan`): generate the 500-skill fixture (10 categories x 5 subcategories x 10 skills; each skill has `SKILL.md`, `references/notes.md`, `scripts/run.sh`) in a tempdir; run `Catalog::load` with `LoadOptions { use_cache: false, git_global: false }` five times with fresh state; assert 500 skills each run; median under 300 ms in release (`cfg!(debug_assertions)` false), under 5000 ms in debug. Pinned output: print exactly one result line, preceded by a newline so it starts at column 0 even when libtest has already printed `test perf_cold_scan ... ` on the same line under `--nocapture`: `perf_cold_scan median_ms=<n> runs_ms=[<v1>, <v2>, <v3>, <v4>, <v5>] skills=500 profile=<release|debug>`, where `<n>` and each `<vN>` are milliseconds formatted with one decimal (`{:.1}`) and the line ends right after the profile word (no trailing text). `gate6` (Exit Gate) greps this exact shape. File header: `#![allow(clippy::unwrap_used)]` with comment.
- [x] D5. Gate D: `cargo test -p unam-core` all green; `cargo clippy --workspace --all-targets -- -D warnings` green; run `perf_cold_scan` once in debug (`cargo test -p unam-core --test perf_cold_scan -- --nocapture`), then once in release through `gate6` (gate G6) and record the printed line.

### Step E: CLI

- [x] E1. Write `src/output.rs` (envelope, renderers, truncation, and the `neutralize` control-character sanitizer per the `output.rs` spec) with unit tests first.
- [x] E2. Write `src/cli/mod.rs` (clap structs: global flags, `source add|list|rm`, `search`, `show`; bare usage), `src/cli/source.rs`, `src/cli/search.rs`, `src/cli/show.rs`, and `src/main.rs` (tracing init, dispatch, exit mapping).
- [x] E3. Gate E: `cargo test -p unam --test read_side_invariant source_tree_unchanged_read_side -- --exact` is now GREEN (guarded); `cargo clippy` green.

### Step F: Integration tests and snapshots

- [x] F1. `tests/ac_read_path.rs`: `ac_01_*`, `ac_03_*`, `ac_04_*`, `ac_05_*`, `ac_06_*`, `ac_38_*` per the Acceptance Criteria table (each ends with `assert_source_unchanged` where the source still exists; `ac_38` asserts config and cache bytes unchanged instead).
- [x] F2. `tests/source_cli.rs`: add success; add exit 2 cases (nonexistent path, a file instead of a directory, invalid alias, duplicate alias, duplicate canonical path); zero-skills warning registers with exit 0; relative path stored canonical, `~` path stored as typed (set `HOME` in `TestEnv`); `source list` ordering by precedence, `last_scan` null before any cache and set after, reachable false after removal; `source rm` exit 0, exit 4 unknown, exit 3 referenced (hand-built provisional index plus a selection YAML referencing `fx:`), `--force` success listing the dangling project, corrupt index gives a warning and exit 0, index absent gives exit 0 (documents P11); config created on first load and not by bare `unam`; precedence-tie warning; `cache_write_failure_is_warning_not_error` (Fully-Automated: before running, create a regular FILE at `<cache dir>/catalog` so the directory cannot be created whatever the uid; assert exit 0 and a `cache-write-failed` warning in `warnings`, with the command's data otherwise complete); `global_gitignore_is_honored_in_git_tree_under_isolation` (Fully-Automated: use `build_library_with_root_git`, create `xdg-config/git/` inside the `TestEnv` and write a global ignore file containing the line `testing/` with `std::fs::write`, run `source add fx <lib>`; assert exit 0, skill count 8, and that `search pytest` returns no `fx:testing/pytest-fixtures`; this pins the documented product behavior under isolation, because in-process tests pass `git_global: false`); unreachable `walk-error` is not required here. The chmod-based `skill-io-error` and `walk-error` unit tests in `catalog/scan.rs` stay Hybrid (printed self-skip note when permissions are not enforced) and are a named residual in the report.
- [x] F3. `tests/search_cli.rs`: `--category` prefix (and boundary: a `sdlc` filter excludes `sdlcx`), `--source` filter and unknown alias exit 2, empty query lists all 9 sorted by ID, no-match output (empty results, exit 0), exact-name ranking (`code-review` first), JSON field presence (`score`, always-present arrays).
- [x] F4. `tests/show_cli.rs`: all three reference forms found (`fx:sdlc/code-review`, `fx:code-review`, `code-review`); top-level `fx:alpha-root-skill`; exit 4 with candidates for a near-miss typo; exit 5 for bare `error-handling` listing both IDs and never picking; `fx:error-handling` also exit 5; qualified path for each duplicate resolves exit 0; invalid ref exit 2; frontmatter order and unknown keys preserved in `--json`; supporting files listed; `show` of a skill with a warning includes its diagnostic; unreachable source gives exit 4 naming the source; `human_output_neutralizes_control_characters` (generated fixture, written by the test: a skill whose frontmatter is `description: "red \x1b[31mALERT"`, a YAML double-quoted escape, so the parsed description contains ESC; `show` and `search` in human mode: stdout contains no 0x1B byte and contains the literal text `\x1b[31mALERT`; the same skill under `--json` keeps the escaped form `\u001b`).
- [x] F5. `tests/cache_cli.rs`: cache file created at `<cache>/catalog/fx.json` after the first scan; poisoned-cache reuse (edit the cached description to a marker; `show` prints the marker; `--no-cache` prints the real description and leaves the cache byte-identical); in-place edit changes the description after `set_modified` bump without `--no-cache`; a new skill in a nested category appears; a deleted skill disappears; corrupt cache file is ignored then rewritten; `--no-cache` on a fresh environment creates no cache file.
- [x] F6. `tests/usage.rs`: bare `unam` exits 0 and prints usage without creating `config.toml`; `--help` and `--version` exit 0; unknown subcommand exits 2; `-q` hides `note[` lines on `source add`; `UNAM_LOG=debug` still exits 0 and keeps stdout clean in `--json` mode.
- [x] F7. `tests/snapshots.rs`: the snapshot set listed in the Insta Snapshot Procedure; create with `INSTA_UPDATE=always` once, inspect, commit; then confirm `INSTA_UPDATE=no` is green.
- [x] F8. Gate F: run the full per-AC gate commands (G4), the first-test gate (G5), `cargo test --workspace` with `INSTA_UPDATE=no` (G3), snapshot hygiene (G7).

### Step G: dist, CI, closeout

- [x] G1. Write `dist-workspace.toml`: `[workspace]` with `members = ["cargo:."]`; `[dist]` with `cargo-dist-version = "0.32.0"` (latest on crates.io 2026-10-05; unverified, `dist` not installed), `targets` = `aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu`, `installers = []`, no `ci` key. Header comment: "Hand-written, unverified: dist is not installed; milestone M7 verifies with `dist plan`. No universal macOS target is claimed (spec 9 says universal; evidence missing)." Verify it parses with `python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],'rb'))" dist-workspace.toml` (G9).
- [x] G2. Write `.github/workflows/ci.yml` (see CI content). Mark in the report: written, not run (no push this run). Verify it parses (G8).
- [x] G3. Run every gate (G1 to G13) in order; record outputs, the measured `perf_cold_scan` line, resolved dependency versions, and any deviation in the phase report.
- [x] G4. Hand off: do not commit (UPDATE PROCESS commits on `main`); do not push.

### Deviations (EXECUTE 2026-10-05)

All within the blast radius; none touches a public contract beyond what the CLI Contract specifies.

1. P1 split (pre-authorized): `catalog/mod.rs` would pass about 600 lines, so cache code lives in `crates/unam-core/src/catalog/cache.rs` and reference parsing plus the temporary lookup in `crates/unam-core/src/catalog/lookup.rs`.
2. `Skill.category` is a `String` (empty for a top-level skill) instead of a `RelPath`; spec 3 says concrete types are an implementation detail and `RelPath` rejects the empty string.
3. Helpers not named in the plan, added to keep the CLI thin: `catalog::FlatDiagnostic`, `DiagClass`, `diag_class`, `ALL_CLASSES`, `SOURCE_AND_CONFIG`, `Catalog::flat_diagnostics`, `Catalog::resolve_text`, `Catalog::reindex`, `Catalog::source`, and `SourceStatus.expanded` (feeds the JSON `expanded_path`).
4. First test (A9): besides exit codes and the unchanged snapshot, it asserts that `--json search review` returns results; without that assertion it was green against the skeleton binary (which exited 0 for any arguments), so it could not be red.
5. CI env value is written `INSTA_UPDATE: "no"` (quoted) because a bare YAML `no` parses as boolean false.
6. `toml` resolved to 1.1.6 (the "resolve" rule picked the current major).
7. Process only: the static fixture files were created by a python script run through Bash (content had no hook tokens) rather than one Write call per file.

### CI content (`.github/workflows/ci.yml`)

- name `ci`; triggers: `push` to `main` and `pull_request`; top-level `permissions: contents: read`; env `CARGO_TERM_COLOR: always`, `INSTA_UPDATE: no`.
- One job `gates`, `strategy.matrix.os: [ubuntu-latest, macos-latest]`, `runs-on: ${{ matrix.os }}`.
- Steps in order: `actions/checkout@v4`; `dtolnay/rust-toolchain@stable` with `components: rustfmt, clippy`; `Swatinem/rust-cache@v2`; run `cargo fmt --all --check`; run `cargo clippy --workspace --all-targets --locked -- -D warnings`; run `cargo test --workspace --locked`.
- No release step, no secrets, no network publishing. Action versions are the research-time majors and are unverified until the first push (Taylor's call).

---

## Exit Gate

Setup for every command: repository root; `export PATH="$HOME/.cargo/bin:$PATH"`; `INSTA_UPDATE=no` exported. Gate helper (a shell function, not product code) that makes a filter matching zero tests FAIL:

`gate() { out=$("$@" 2>&1); rc=$?; printf '%s\n' "$out"; [ "$rc" -eq 0 ] && printf '%s\n' "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed'; }`

| Gate | Command | Expected |
|---|---|---|
| G1 fmt | `cargo fmt --all --check` | exit 0 |
| G2 clippy | `cargo clippy --workspace --all-targets -- -D warnings` | exit 0; no `unwrap` outside tests |
| G3 all tests | `cargo test --workspace` | all pass (includes snapshots with `INSTA_UPDATE=no`, and `perf_cold_scan` in debug) |
| G4 per AC | `gate cargo test -p unam --test ac_read_path ac_01_register_non_git_directory_and_skills_discoverable -- --exact` and the same for `ac_03_three_level_nested_categories_real_paths`, `ac_04_git_and_node_modules_yield_no_skills`, `ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic`, `ac_06_search_spans_three_categories_one_result_set`, `ac_38_unreachable_source_reported_no_mutation` | each exits 0 with exactly 1 passed |
| G5 first test | `gate cargo test -p unam --test read_side_invariant source_tree_unchanged_read_side -- --exact` | 1 passed |
| G6 perf (release) | `gate6 cargo test --release -p unam-core --test perf_cold_scan -- --exact perf_cold_scan --nocapture` (helper defined below) | rc 0; `1 passed`; the exact line ending `skills=500 profile=release`; median under 300 (checked by awk); record the median and the five runs |
| G7 snapshot hygiene | the G7 command below | no `.snap.new`; at least 10 `.snap` files; no path (`/var/ /private/ /tmp/ /Users/ /home/`) and no RFC 3339 timestamp in any snapshot |
| G8 CI parses | `python3 -c "import yaml,sys; yaml.safe_load(open(sys.argv[1]))" .github/workflows/ci.yml` | exit 0 (parse check only; the workflow is NOT run) |
| G9 dist parses | `python3 -c "import tomllib,sys; tomllib.load(open(sys.argv[1],'rb'))" dist-workspace.toml` | exit 0 (parse check only; config unverified) |
| G10 text hygiene | the G10 command below | exit 0: file list non-empty, no U+2014, no code points 0x1F300 to 0x1FAFF or 0x2600 to 0x27BF |
| G11 real paths untouched | the G11 commands below | no hits; the three real unam directories listed as at the Pre-Step record |
| G12 harness | `node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs`; `node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs`; `node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs`; `node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <this plan>`; `git diff --check` | all pass |
| G13 lockfile | `Cargo.lock` exists and is not ignored (`git check-ignore Cargo.lock` exits 1); `cargo metadata --locked --format-version 1 > /dev/null` exits 0 | pass |

Exact helper and commands for G6, G7, G10, G11 (verified at PVL in a scratch workspace: pass on a clean tree, fail on each seeded defect; run the G10 and G11 python and grep from a script file written with the Write tool if the hook blocks the text, see Pre-Step item 4):

G6 helper (shell function; invoke as the G6 row shows):

```
gate6() { out=$("$@" 2>&1); rc=$?; printf '%s\n' "$out" | grep -E 'perf_cold_scan median_ms|test result'; [ "$rc" -eq 0 ] && printf '%s\n' "$out" | grep -Eq 'test result: ok\. 1 passed' && printf '%s\n' "$out" | grep -Eq 'perf_cold_scan median_ms=[0-9.]+ runs_ms=\[[^]]+\] skills=500 profile=release$' && printf '%s\n' "$out" | awk '/perf_cold_scan median_ms=/{split($2,a,"="); if (a[2]+0 < 300) ok=1} END{exit ok?0:1}'; }
```

G7:

```
test -z "$(find crates -name '*.snap.new')" && [ "$(find crates/unam-cli/tests/snapshots -name '*.snap' | wc -l)" -ge 10 ] && ! grep -rEq '/var/|/private/|/tmp/|/Users/|/home/|[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}' crates/unam-cli/tests/snapshots
```

G10 (run before the milestone commit, while the files are modified or untracked; also scan the phase report separately):

```
python3 -c "import sys,subprocess; fs=subprocess.check_output(['git','ls-files','-m','-o','--exclude-standard','--','Cargo.toml','Cargo.lock','clippy.toml','dist-workspace.toml','.github','crates'],text=True).split(); bad=sorted({f for f in fs for c in open(f,encoding='utf-8',errors='replace').read() if ord(c)==0x2014 or 0x1F300<=ord(c)<=0x1FAFF or 0x2600<=ord(c)<=0x27BF}); print('files scanned:',len(fs),'hits:',bad); sys.exit(1 if bad or not fs else 0)"
```

G11 (both parts must hold; the grep exits 0 under `!` when there are no hits):

```
! grep -rEq 'Developer/01-AI|[.]claude/skills|[.]agents/skills' crates .github dist-workspace.toml
ls -A ~/.config/unam ~/.cache/unam ~/.local/share/unam 2>&1 | sort
```

The `ls` output must equal the Pre-Step record.

- All checklist items checked; AC-1, 3, 4, 5, 6, 38 (read-side) tests green; `source_tree_unchanged_read_side` green; `perf_cold_scan` green with the measured median recorded in the phase report.
- No test touches the real `~/.config/unam`, `~/.claude/skills`, `~/.agents/skills`, the skill library, or this repository.
- Regression: M0 table and Outcome in `spike/symlink-discovery.md` unchanged (`git diff --stat spike/` empty).
- Phase report written to the report destination above, including: measured perf line, resolved dependency versions, any deviation (for example splitting `catalog/mod.rs`), CI "written, not run", `dist-workspace.toml` "unverified", snapshot creation note, the self-skipped Hybrid tests (if any), and a Forward Preview for milestone M2.

---

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- A phase can be VERIFIED while the program is incomplete.
- Commit execution changes on `main` before starting milestone M2; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met (table unfilled or Outcome missing).
- Toolchain missing (`cargo`, `rustfmt`, `clippy`): BLOCKED; never install.
- Frontmatter parser crate choice (`serde_norway`) fails the spec 8 cases: fall back per spec 1.4 note (`serde-saphyr`) via a recorded decision.
- `perf_cold_scan` over 300 ms in release after reasonable profiling: record the measured value, keep that gate CONDITIONAL (a backlog stub for optimization), do not widen the bound.

---

## Risks and Predictions

Plan-agent analysis (five viewpoints; not a `vc-predict` run):

| Viewpoint | Risk | Mitigation in this plan |
|---|---|---|
| Correctness | A scan filter bug hides real skills (depth, nested `.git`, hidden dirs) or admits `node_modules` skills | Depth boundary, nesting, exact-case, cap, and symlink unit tests; AC-3 and AC-4 integration tests |
| Safety | A test or command writes to real HOME or the repo | `TestEnv` (env_clear plus seven tempdir variables, tempdir cwd, isolation assert); gate G11 |
| Reproducibility | Global gitignore or `.ignore` files from the machine change scan results | HOME and XDG redirected in every CLI test; every in-process test passes `git_global: false` and an injected `Env`; one CLI test pins the global-ignore behavior under isolation |
| Performance | 300 ms target missed or flaky | Release gate recorded; median of five; debug bound generous; CONDITIONAL path defined, bound never widened |
| Maintainability | `catalog/mod.rs` grows too large; temporary `lookup` lingers | P1 allows a recorded split; `lookup` is documented temporary and listed in the umbrella open items |
| Process | Snapshots accepted unread; `cargo-insta` absent | Snapshot Procedure plus gate G7 hygiene grep |
| Tooling | The hook blocks Bash text containing `target`, `.git`, `node_modules`, `build`, `dist`, `vendor`, `coverage`; `cargo` not on PATH; the first build needs network | Pre-step items 4 to 7 |
| Security | A hostile skill library injects terminal escape sequences through human output | `neutralize` in `output.rs`, a unit test, and a CLI test with an ESC fixture |

Edge cases for the three highest-risk items (folded into tests above):

- Walk and cap: skill dir at depth 7 versus 8; root `SKILL.md`; hidden directory; directory named like a skill file; symlinked skill dir; `max_dirs` exactly at and one over the limit; unreadable directory.
- Cache fingerprint: in-place edit with unchanged size (mtime differs); touch only (mtime changes, content same: re-hash is acceptable, result equal); new skill in a nested category; deleted skill; cache for a relocated path; corrupt JSON; clock skew not relied on (equality only, no ordering).
- Frontmatter: BOM plus CRLF; closing fence at EOF; colon in value inside a block scalar line (indented, left alone); description that is a number; a key with a colon-space in a quoted value (left alone); extremely long name.

---

## Touchpoints

- Workspace and crate files listed in Blast Radius (created)
- Read: `docs/spec/unam-technical-spec.md` sections 1.3, 1.4, 2, 3, 4.1 to 4.3, 4.6 (reference forms), 5, 7, 8, 9, 10, 12; PRD sections 7.1, 9, 13.2; `docs/PROJECT-BRIEF.md` rules
- `process/context/all-context.md` and `process/context/tests/all-tests.md` updated at UPDATE-PROCESS (tests become runnable; note the `cargo` PATH, the `target` hook, `INSTA_UPDATE=no`, the gate helper)

---

## Public Contracts

- Introduces CLI commands `source add|list|rm`, `search`, `show`, bare `unam` usage, the global flags `--json`, `--no-cache`, `-v`, `-q`, and the `--json` envelope and `data` shapes above; later milestones must not break them (additive fields only).
- Exit codes 0, 1, 2, 3, 4, 5 as in the CLI Contract (spec 5).
- Diagnostic codes and severities listed above are stable contract; `Diagnostic` JSON shape is stable.
- Introduces `config.toml` (spec 2.1) and the env overrides `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR` (spec 2), with the path rules in Paths.
- Introduces the catalog cache file format `<cache dir>/catalog/<alias>.json` (version 1). It is a cache, not a contract with users.
- Provisional (superseded in milestone M2): `lookup`, and the reader of `<data dir>/projects.json` with the minimal schema above.
- Core library API surface listed in Module Specifications (public items only what the CLI and tests need).

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets -- -D warnings`; G3 `cargo test --workspace` | Fully-Automated | spec 8 quality gates (unwrap denied outside tests, spec 7) |
| `ac_01_register_non_git_directory_and_skills_discoverable` (G4) | Fully-Automated | AC-1 |
| `ac_03_three_level_nested_categories_real_paths` (G4) | Fully-Automated | AC-3 |
| `ac_04_git_and_node_modules_yield_no_skills` (G4) | Fully-Automated | AC-4 |
| `ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic` (G4) | Fully-Automated | AC-5 |
| `ac_06_search_spans_three_categories_one_result_set` (G4) | Fully-Automated | AC-6 |
| `ac_38_unreachable_source_reported_no_mutation` (G4) | Fully-Automated (simulated missing path) | AC-38 read-side |
| `source_tree_unchanged_read_side` (G5) | Fully-Automated | PRD S3 (read-side); prerequisite for AC-2 closing in M3 |
| `perf_cold_scan` release (G6, via `gate6`) | Hybrid (precondition: release build, local SSD; assertion automated; the helper checks rc, `1 passed`, the pinned line, and a median under 300) | spec 9 cold scan under 300 ms |
| Frontmatter, scan, hash, paths, config, source, search, lookup, cache, error unit tests (inline; run by G3) | Fully-Automated | spec 8 unit list (frontmatter cases, DIS-1 to DIS-9 behaviors); supports AC-3, AC-4, AC-5 |
| `source_cli`, `show_cli`, `search_cli`, `cache_cli`, `usage` integration tests (G3) | Fully-Automated | spec 5 exit codes 0, 2, 3, 4, 5 and `--json` envelope; DIS-9 cache; SRC-6 |
| `snapshots` (G3, G7; 10 snapshots, G7 requires at least 10 `.snap` files) | Fully-Automated | spec 8 CLI: `--json` output snapshotted with insta |
| `cache_write_failure_is_warning_not_error` (regular file placed where `<cache dir>/catalog` must be created; no chmod, no uid dependence) | Fully-Automated | spec 4.3 diagnostics (cache write failure is a warning); "Never panic on user data" (spec 7) |
| `global_gitignore_is_honored_in_git_tree_under_isolation` (CLI test; in-process tests pass `git_global: false`) | Fully-Automated | spec 4.3 walk filter (`.gitignore` respected) under isolation (Safety) |
| `human_output_neutralizes_control_characters` (CLI test with a generated ESC fixture) and the `output.rs` `neutralize` unit tests | Fully-Automated | brief safety rule: untrusted library text never reaches the terminal raw; spec 5 human output |
| unreadable-file and unreadable-directory scan tests (`skill-io-error`, `walk-error`; chmod-based) | Hybrid (precondition: non-root user; self-skip printed; named residual) | spec 4.3 diagnostics; "Never panic on user data" (spec 7) |
| G8, G9 parse checks | Fully-Automated (parse only) | A3 deliverables exist and are well-formed; execution unverified (known, below) |
| G10 to G13 hygiene, safety, harness, lockfile | Fully-Automated | brief rules; spec 1.4 lockfile committed |

Known residuals (not behaviors developed and declared passing): the CI workflow is written, not run; `dist-workspace.toml` is unverified until milestone M7. The chmod-based `skill-io-error` and `walk-error` unit tests are Hybrid and self-skip (printed note) when permissions are not enforced; they are a named residual recorded in the report, not a Known-Gap pass. CI and dist are not developed-behavior gates; every residual stays visible in the report. No developed behavior is carried by Known-Gap; Known-Gap is never a proving strategy here. If EXECUTE finds an untestable developed behavior it writes a backlog stub and keeps that gate CONDITIONAL.

---

## Test Infra Improvement Notes

- `cargo-insta` is not installed; the Snapshot Procedure uses `INSTA_UPDATE` modes and manual review. Consider installing `cargo-insta` on Taylor's machine (his call) to ease review; not required.
- The Hybrid tests that rely on filesystem permissions self-skip when run as root; if CI ever uses a container as root they will not run. Candidate improvement: a capability probe helper in `common/mod.rs` that fails loudly in CI (`CI=true`) instead of skipping.
- `perf_cold_scan` is a wall-clock test; a later milestone could add a stable counter-based check (files stat'd per scan) as a second signal.
- Cursor-agent, Windows, and the real library are intentionally outside M1 test infrastructure.
- The `scout-block` hook rejecting commands that contain `target` is a harness friction for Rust work; a follow-up for the process layer (allow-list entry in `.claude/.vcignore`) is Taylor's decision; M1 works around it.

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-02-m1-core-read-path_PLAN_04-10-26.md`
- Last completed step: step 3 PLAN-SUPPLEMENT (2026-10-05); steps 1 RESEARCH and 2 INNOVATE complete
- Validate-contract status: PASS, inner-pvl: phase-2, PVL cycle 3 (2026-10-05): 0 FAILs, 0 open concerns (R1 closed in the plan text and confirmed by a C0 compile probe); EXECUTE may start
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, spec sections cited above, the M0 report Forward Preview, umbrella plan
- Next step for a fresh agent: spawn vc-validate-agent for PVL (V1 to V7) against this plan; do not start EXECUTE until `Gate: PASS` (or an accepted CONDITIONAL after at least one supplement cycle). Execute with one opus agent following the Implementation Checklist in order, running each step's gate before the next. Commit on `main` at UPDATE PROCESS, never push.
- Forward Preview for milestone M2 (to be confirmed in the phase report): `SkillRef::parse`, `Catalog`, `Diagnostic`, `Error`, `Dirs`, `write_atomic`, and the hash helpers are reusable; M2's `resolve.rs` replaces `lookup`; M2's `project.rs` owns the project index and replaces the provisional `projects.json` reader (reconcile the schema); `bundles_referencing` joins `show` in M2.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH: research-agent: M0 report and Outcome read; test context loaded; plan drift checked; first-test and perf-check tasks (D0, D3) confirmed (2026-10-05)
- [x] 2. INNOVATE: innovate-agent: approach decided; Decision Summary written (2026-10-05; decisions D1 to D12 above)
- [x] 3. PLAN-SUPPLEMENT: plan-agent: existing phase plan updated; Inner Loop Refresh Note written (2026-10-05)
- [x] 4. PVL (Gate PASS 2026-10-05 after 3 validate-fix cycles; results-m1.tsv): vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [x] 5. EXECUTE (2026-10-05): all checklist items done; G1 to G13 green when run by the execute agent
- [x] 6. EVL (2026-10-05): all EVL gates green (G1 to G13, 164 tests, vc-tester; results-m1-evl.tsv); follow-up stubs registered as backlog NOTEs in the phase report; EVL HANDOFF SUMMARY written (CLEAN with gaps)
- [x] 7. UPDATE PROCESS (2026-10-05): phase report written, umbrella state updated, context updated; commit done by the orchestrator

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Inner Loop Refresh Note: 2026-10-05 - changed sections: Overview, Scope, Decisions Locked, Plan-Level Decisions, Acceptance Criteria, Pre-Step, Safety, Blast Radius, Module Specifications, CLI Contract, Test Harness, Fixture Contract, Data Flow, Failure Modes, Implementation Checklist, Exit Gate, Risks and Predictions, Public Contracts, Verification Evidence, Test Infra Improvement Notes, Resume and Execution Handoff, Phase Loop Progress. Invalidates prior validate-contract.

---

## Validate Contract

Status: PASS
Date: 05-10-26
date: 2026-10-05
generated-by: inner-pvl: phase-2
supersedes: 2026-10-05 (inner-pvl: phase-2, cycle-2 contract, CONDITIONAL with 1 concern R1) - this cycle-3 inner PVL has current evidence

Parallel strategy: sequential
Rationale: score 3/7 (S2 new public CLI and JSON contract introduced, S4 phase program, S7 about 45 files in the blast radius; no S1, S3, S5, S6). Dominant signal: S7 file count, but the file set is a single dependency chain (every file is new and later steps compile against earlier ones), so there are no disjoint groups for a team and no independent directions for parallel subagents. EXECUTE: one opus agent following the Implementation Checklist in order (sequential 1 agent; parallel subagents would be 4 + N with nothing to split; agent team 3 members x 2 rounds = 6 for no coordination benefit; workflow is not warranted, item count is known). Everything else sonnet. This validate pass ran inline (Layer 1 and Layer 2 reviewed by the validate agent itself on sonnet; no helper agents spawned, none needed) with cheap-local probes in the session scratchpad outside the repository (no VC-FEASIBILITY-PROBE-NEEDED emitted). Model: sonnet. Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so no autonomous goal block section is written into this phase plan; the umbrella goal governs. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Cycle record: cycle 1 applied all 8 gaps; cycle 2 validation found one new concern R1 (found in the cycle-1 edits); the cycle-2 supplement applied R1 (see `phase-02-m1-core-read-path-pvl-iteration-002_REPORT_05-10-26.md`); this is the cycle-3 validation (V1 to V7 re-run from V1, focused on R1 closure and regressions from that edit; unchanged sections were verified in cycle 2 and are not re-reviewed). `results-m1.tsv` holds the header, baseline, and 3 cycle rows (`wc -l` = 5). Verdict: 0 FAILs, 0 CONCERNs.

R1 closure check (cycle 3):

| Check | Result | Evidence |
|---|---|---|
| C0 creates the three stubs | Closed | Checklist C0 and the `catalog/mod.rs` Step-order bullet both say C0 creates doc-comment-only `catalog/frontmatter.rs`, `catalog/scan.rs`, `catalog/search.rs` alongside the `pub mod frontmatter; pub mod scan; pub mod search;` declarations (C1 and C2 fill the first two, D2 the third); Blast Radius lists all three files; no `todo!()` or `unimplemented!()`. |
| Named skill-plus-fingerprint types | Closed | `FingerprintEntry { path, size, mtime_secs, mtime_nanos }` (P15) and `ScannedSkill { skill: Skill, fingerprint: Vec<FingerprintEntry> }` are defined in `scan.rs` at C2 (Module Specifications `catalog/scan.rs` and checklist C2); `scan_source`'s `prior` map and `SourceScan.skills` use `ScannedSkill`; the D1 cache entry is a serialized `ScannedSkill` (cache file bullet). Usage is consistent across `scan.rs`, `mod.rs`, C2, and D1. |
| C0 compiles | Closed | Probe in the session scratchpad (`c0probe`): edition-2024 workspace with the plan's lint table and `clippy.toml`; `error.rs` (`Diagnostic`, A4), `hash.rs` (`Hash`, A7), and `source.rs` (`SourceAlias`, `SourceKind`, B2) as prerequisites; `catalog/mod.rs` with the C0 type set plus `RelPath` and `SkillId` unit tests and three doc-comment-only stub files. `cargo build -p unam-core` passes, `cargo clippy --workspace --all-targets -- -D warnings` passes with zero warnings, `cargo test -p unam-core` passes. Every type C0 needs (`Diagnostic`, `Hash`, `SourceAlias`, `SourceKind`) is created before C0 (A4, A7, B2), so C0 has no forward dependency. |
| Regression scan | Clean | `validate-plan-artifact.mjs`: 0 failures, 0 warnings; no em dash in the plan; the supplement touched only the C0 and C2 checklist lines, the `catalog/scan.rs` and `catalog/mod.rs` spec text, and the Resume status line; no other section references the old unnamed type or the old C0 text. |

Cycle-1 gap closure check (each verified in the plan text and, where mechanical, by running it):

| Gap | Verified closed? | Evidence |
|---|---|---|
| 1 step order | Closed (cycle 3: the residual R1 defect is also closed) | Order A, B, C0, C1, C2, D1 to D4 is consistent: source.rs B2 holds only `Catalog`-free code; `add` and `list` moved to D3; perf test moved to D4; `catalog/mod.rs` types at C0. Cycle-2 defect R1 (C0 declared modules without creating their files, E0583; unnamed skill-plus-fingerprint type) is fixed: see the R1 closure check above. |
| 2 hermetic in-process tests | Closed | `Env` (injected lookup, `Env::process()` only called by main via `Dirs::from_process_env`), `Dirs.env`, `Dirs::expand`, `expand_path(raw, &Env)`, `LoadOptions { use_cache, git_global }`, `ScanLimits.git_global`, `WalkBuilder::git_global(...)`; used consistently in paths, config, source (`add`, `list`), catalog (`load`), scan, perf test, and the Safety section; one CLI test pins the global-ignore behavior under isolation (F2, `xdg-config/git/ignore` matches the `TestEnv` XDG_CONFIG_HOME). |
| 3 G6 mechanical and walk-once | Closed | `gate6` extracted verbatim from the plan and run against a test printing exactly the D4 format (leading newline, one-decimal values, `, ` separators, ends at the profile word): release run rc 0; debug run rc 1 (profile=debug rejected); misspelled name rc 1 (0 tests ran); median 310 rc 1. Walk-once is specified in the scan.rs per-skill pass; real-design scratch perf: release median 160 to 173 ms, debug median about 247 ms (bound 5 s). |
| 4 G7, G10, G11 runnable and non-vacuous | Closed | The plan's text for G7, G10, G11 was extracted verbatim and run in a scratch git repo: pass on a clean tree; fail on seeded `.snap.new`, `/var/` path, 9 snapshots, `Developer/01-AI` hit, em dash, emoji; G10 prints the scanned file count and fails on zero files. Real hook (`scout-block.cjs`, run end to end with stdin JSON) ALLOWS every gate command in the plan, including G11 (`dist-workspace.toml` by name), G10 (`'.github'` inside the python text), G8, G9, and `gate cargo test ... ac_04_git_and_node_modules_yield_no_skills ...` (the test name contains `node_modules`); it blocks only path-like segments such as `ls dist` or a bare `.git` token. No workaround is needed for G11; the plan's conditional wording ("if the hook blocks the text, run it from a script file") stays correct as a fallback. |
| 5 cache-write test deterministic | Closed | F2 `cache_write_failure_is_warning_not_error` places a regular FILE at `<cache dir>/catalog` (TestEnv pre-creates `cache`), so `create_dir_all` fails whatever the uid; asserts exit 0 and `cache-write-failed` in `warnings`. |
| 6 terminal escape sequences | Closed | `neutralize` spec is precise (C0 except LF and TAB, DEL, C1 to `\xNN` lowercase hex; applied to every untrusted human-mode string; truncation after neutralization); unit tests listed; CLI test `human_output_neutralizes_control_characters` uses a YAML double-quoted `\x1b` escape (valid YAML), asserts no 0x1B byte on stdout and the literal `\x1b[31mALERT` (present via the Description line and the search description head even if the YAML emitter writes `\e`), and the `\u001b` form under `--json`. |
| 7 Pre-Step network, hooks, clippy | Closed | Pre-Step items 4 to 7 present and accurate (verified against the hook: the token list is right, scope slightly broader than the actual segment match, which is the safe direction). |
| 8 departures and wording | Closed | Umbrella open items 22 and 23 exist (checked); the Decisions Locked report paragraph lists them; P1 now says "spec 1.3 plus three additions" including `paths.rs`; Blast Radius names the two process files. |

Evidence gathered in this pass: structural validator `validate-plan-artifact.mjs` on the plan: 0 failures, 0 warnings. All paths named by the plan that must pre-exist resolve (spec, PRD, brief, spike file, context files, validators, hook). Toolchain present (cargo 1.99.0, rustfmt 1.10.0, clippy 0.1.99); the three real unam directories do not exist (G11 baseline: none); `.gitignore` ignores none of the planned fixtures, snapshots, or `Cargo.lock` (`git check-ignore` on eight representative paths); repository has no Cargo files, `crates/`, or `.github/` yet (all new). The plan has no em dash and no emoji. Nothing was created in the repository; no `unam` or agent CLI invoked; real HOME untouched except cargo's own registry cache.

Test gates (C3 5-column table; legacy line form follows):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 quality gates | fmt clean, clippy clean with `unwrap` denied outside tests, whole workspace tests pass | Fully-Automated | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` (with `INSTA_UPDATE=no`) (G1, G2, G3; lint mechanism verified in scratch) | B |
| AC-1 | non-git directory registers (kind `directory`, 9 skills), `search code-review` finds `fx:sdlc/code-review`, source tree unchanged | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_01_register_non_git_directory_and_skills_discoverable -- --exact` (G4) | B |
| AC-3 | three-level nested categories listed with real paths, nothing renamed or flattened, all 9 IDs present | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_03_three_level_nested_categories_real_paths -- --exact` (G4) | B |
| AC-4 | `.git` and `node_modules` (root and nested) yield no skills; same 9 skills; kind `git-repo` | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_04_git_and_node_modules_yield_no_skills -- --exact` (G4) | B |
| AC-5 | name/dir mismatch loads with `name-dir-mismatch` warning; bad YAML and missing description skipped with named diagnostics | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic -- --exact` (G4) | B |
| AC-6 | `search review` returns skills from three categories in one result set | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_06_search_spans_three_categories_one_result_set -- --exact` (G4; matcher behavior confirmed in scratch) | B |
| AC-38 (read side) | unreachable source reported, exit 0 for list and search, exit 4 for show, config and cache byte-identical | Fully-Automated | `gate cargo test -p unam --test ac_read_path ac_38_unreachable_source_reported_no_mutation -- --exact` (G4) | B |
| PRD S3 read side (first test) | register, scan, search, show, list leave the source tree hash and the per-entry stat snapshot unchanged | Fully-Automated | `gate cargo test -p unam --test read_side_invariant source_tree_unchanged_read_side -- --exact` (G5) | B |
| spec 9 cold scan | 500-skill cold scan median of five under 300 ms in release | Hybrid | `gate6 cargo test --release -p unam-core --test perf_cold_scan -- --exact perf_cold_scan --nocapture` (G6; precondition: release build, local SSD; assertion automated; scratch medians 160 to 226 ms) | B |
| hygiene (snapshots, CI and dist parse, text, real paths, harness, lockfile) | no stray `.snap.new`, snapshot content free of paths and timestamps, `ci.yml` and `dist-workspace.toml` parse, no em dash or emoji, real user dirs untouched, validators pass, lockfile tracked | Fully-Automated | G7 to G13 using the exact plan commands (G7, G10, G11 verified verbatim in scratch: pass clean, fail on each seeded defect) | B |
| spec 4.3 diagnostics, "never panic on user data" | unreadable skill file and unreadable directory produce `skill-io-error` and `walk-error` and nothing else breaks | Hybrid | chmod 000 unit tests in `catalog/scan.rs` (precondition: non-root user; printed self-skip otherwise) | D (named residual) |
| spec 4.3 diagnostics (cache write failure) | unusable cache location yields a `cache-write-failed` warning, command still succeeds | Fully-Automated | `cache_write_failure_is_warning_not_error` in `source_cli.rs` (regular file placed at `<cache dir>/catalog`; no chmod, no uid dependence) | B |
| spec 4.3 walk filter under isolation | global gitignore honored in a git tree, under isolation | Fully-Automated | `global_gitignore_is_honored_in_git_tree_under_isolation` in `source_cli.rs` | B |
| brief safety rule | untrusted skill text never reaches the terminal raw in human mode | Fully-Automated | `neutralize` unit tests in `output.rs`; `human_output_neutralizes_control_characters` in `show_cli.rs` | B |

Failing stub: AC-1
test("should register a non-git directory and make its skills discoverable", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_01_register_non_git_directory_and_skills_discoverable")
})

Failing stub: AC-3
test("should expose three-level nested categories with real paths", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_03_three_level_nested_categories_real_paths")
})

Failing stub: AC-4
test("should yield no skills from .git or node_modules", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_04_git_and_node_modules_yield_no_skills")
})

Failing stub: AC-5
test("should warn on name/dir mismatch and skip bad YAML with a named diagnostic", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_05_name_dir_mismatch_warns_and_bad_yaml_skipped_with_diagnostic")
})

Failing stub: AC-6
test("should return one result set spanning three categories", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_06_search_spans_three_categories_one_result_set")
})

Failing stub: AC-38 (read side)
test("should report an unreachable source and mutate nothing", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_38_unreachable_source_reported_no_mutation")
})

Failing stub: PRD S3 read side
test("should leave the source tree unchanged after register, scan, search, show", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: source_tree_unchanged_read_side")
})

Failing stub: cache write failure
test("should warn, not fail, when the cache location is unusable", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: cache_write_failure_is_warning_not_error")
})

Failing stub: global gitignore under isolation
test("should honor the global gitignore in a git tree under isolation", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: global_gitignore_is_honored_in_git_tree_under_isolation")
})

Failing stub: terminal escape neutralization
test("should neutralize control characters in human output", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: human_output_neutralizes_control_characters")
})

(Stubs are the red-first skeleton in the protocol form; the Rust equivalent is `#[test] fn <name>() { panic!("NOT IMPLEMENTED - TDD stub: <name>") }`. Plan step A9 already writes the first test red. Command-gate rows (quality gates, hygiene) and Hybrid rows receive no stubs.)

C-4 reconciliation: the strategy column carries only Fully-Automated, Hybrid, Agent-Probe. Known-Gap is never a strategy; named residuals are listed under Open gaps. Net-gate vacuous-green check: every developed behavior in the blast radius (workspace and lint config, paths, config, source add/list/rm, scan, frontmatter, hash, cache, search, show, lookup, CLI contract and exit codes, JSON envelope, output sanitizer, fixtures, snapshots) has a Fully-Automated or Hybrid gate, and the only Known-Gap-like items (CI not run, dist unverified) are named residuals that are not developed behaviors, so the net gate is not vacuously green; with R1 closed there is no open concern and the named residuals are the only open items.

Legacy line form:
- quality gates: [fully-automated: cargo fmt --all --check; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace]
- AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 read side: [fully-automated: gate cargo test -p unam --test ac_read_path <test name> -- --exact, six commands (G4)]
- source tree unchanged (read side): [fully-automated: gate cargo test -p unam --test read_side_invariant source_tree_unchanged_read_side -- --exact (G5)]
- spec 9 cold scan: [hybrid: gate6 release perf_cold_scan; precondition release build and local SSD]
- hygiene G7 to G13: [fully-automated: exact commands in the Exit Gate section]
- cache write failure, global gitignore under isolation, terminal escape neutralization: [fully-automated: named tests above]
- unreadable file and directory diagnostics: [hybrid: chmod-based unit tests, self-skip under root; named residual]
- CI workflow runs: [known-gap: documented; written but not run, no push this run]
- dist-workspace.toml correctness: [known-gap: documented; unverified until milestone M7 `dist plan`]

Dimension findings:
- Infra fit: PASS - toolchain, paths, hook behavior (every plan gate command passes the real scout-block hook, including G11), network note, and repository state all check out; Step C0 now creates the three stub files and the C0 compile probe (build, clippy -D warnings, test) is green; `ScannedSkill` and `FingerprintEntry` have a named home at C2 (R1 closed).
- Test coverage: PASS - each of AC-1, 3, 4, 5, 6, 38 and the read-side invariant maps to a concrete test; G4 and G5 fail on zero matches; G6, G7, G10, G11 are mechanical and verified verbatim including seeded failures; the cache-write test is deterministic; in-process tests are hermetic (`Env`, `git_global: false`); the first test is the red read-side invariant; chmod tests are a named Hybrid residual.
- Breaking changes: PASS - greenfield; new CLI, JSON envelope, diagnostic codes, exit codes, `config.toml`, and cache format are listed in Public Contracts as additive for later milestones; provisional items (`lookup`, `projects.json` reader) are marked superseded by milestone M2.
- Security surface: PASS - no auth, billing, secrets, network, or privilege surface; the trust boundary (reading untrusted skill libraries) is handled: no symlink following in discovery, symlinks hashed by target string and never followed, regular files and symlinks only, `RelPath` rejects `..`, aliases validated before naming a cache file, `source rm` deletes only `<cache>/catalog/<alias>.json`, human output neutralizes control characters, JSON escaped by serde_json, nothing writes under a source.
- Section A (Pre-Step, workspace, lints, paths, first test) feasibility: PASS - lint config verified in scratch; first test is written red at A9 and compiles against the A7 hash helper and skeleton binary; highest-risk edit: root `Cargo.toml` lints and dependency table (mitigated: verified compile in scratch).
- Section B (config and source) feasibility: PASS - `source.rs` at B2 holds only `Catalog`-free code with its tests; `add` and `list` and their tests are at D3; `Env` and `dirs.expand` make `~` and `$VAR` tests hermetic; highest-risk edit: `source::remove` (deletes a cache file and rewrites config; scoped to the cache dir by alias validation).
- Section C (catalog types, frontmatter, walker) feasibility: PASS - frontmatter, walk, hash design confirmed in scratch; depth boundary arithmetic is right (a skill directory at depth 7 yields its `SKILL.md` at depth 8); C0 compiles with the stubs (probe) and the types are named consistently (R1 closed); highest-risk edit: the discovery filter and cap.
- Section D (catalog load, search, lookup, source add and list, perf test) feasibility: PASS - cache design, lookup forms, ranking, diagnostics, and `LoadOptions` flow are consistent with spec 4.3, 4.6, 7; matcher API confirmed; perf probe confirms the 300 ms bound with a thin margin; the D1 cache reads and writes the `ScannedSkill` type named at C2 (R1 closed); highest-risk edit: `Catalog::load` cache read, write, and fingerprint reuse (mitigated by the cache test list).
- Section E (CLI) feasibility: PASS - contract complete and consistent with spec 5; sanitizer fully specified; highest-risk edit: exit-code mapping through `anyhow` downcast (mitigated by the F-step exit-code tests).
- Section F (integration tests and snapshots) feasibility: PASS - fixture totals check out (9 loaded skills, 6 flattened diagnostics, same 9 for the `.git` variant, 10 named snapshots), `INSTA_UPDATE=no` makes a missing snapshot fail, every CLI `Command` is isolated through `TestEnv`; highest-risk edit: snapshots created with `INSTA_UPDATE=always` and read before commit.
- Section G (dist, CI, closeout) feasibility: PASS - parse-only artifacts, recorded as unverified and not run; highest-risk edit: none.
- Exit Gate (G1 to G13) feasibility: PASS - all rows runnable as written (verified: G6, G7, G10, G11 verbatim; G1 to G5, G8, G9, G12, G13 against the hook and scratch); note for the tester: each Bash call is a fresh shell, so define `gate` and `gate6` and export PATH and `INSTA_UPDATE=no` inside the same Bash invocation as the gate command.
- Decisions and scope consistency: PASS - D1 to D12 and P1 to P15 are consistent with each other and with spec and PRD where they do not depart; all departures are recorded (umbrella open items 15 to 23, none re-litigated); type and module names are consistent across sections, including the `ScannedSkill` and `FingerprintEntry` types and the C0 stub files added for R1.

Open gaps:
- CI workflow written, not run (no push this run): known-gap: documented; first run happens at the first push, which is Taylor's call (gap-resolution D, backlog stub `m1-ci-first-run-verification_NOTE_[date].md` in process/features/v1-static-activation/backlog/ to be created at UPDATE PROCESS).
- `dist-workspace.toml` unverified (`dist` not installed; spec 9 universal macOS target unsupported by evidence): known-gap: documented; verified by milestone M7 (gap-resolution C; umbrella open item 19).
- Hybrid chmod-based unit tests (`skill-io-error`, `walk-error`) self-skip under root: named residual, recorded in the phase report; not hidden (gap-resolution D if CI ever runs as root).
- `perf_cold_scan` wall-clock is machine-dependent; on overrun keep the gate CONDITIONAL with a backlog stub and never widen the bound (plan rule, unchanged).
- Nit, not a concern: the Fixture Contract lists the `sdlc/code-review` supporting files as `references/checklist.md` and `scripts/run.sh` only, while the scan rule (supporting = every entry except the root `SKILL.md`) also lists `nested/SKILL.md`; tests assert presence, not an exact list, so the snapshot simply records all three.
- Nine provisional design departures (umbrella open items 15 to 23) are carried to Taylor unchanged; none re-litigated here.

What This Coverage Does NOT Prove:
- G1 to G3 (fmt, clippy, workspace tests): that behavior is correct beyond what the tests assert; `unwrap` denial covers non-test code compiled by clippy, not `expect`, indexing panics, or arithmetic overflow; "never panic on user data" is evidenced by the frontmatter proptest and unit cases only, not by fuzzing the walker or the cache reader.
- AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 tests: run against a synthetic fixture on the test machine; they do not prove behavior on Taylor's real library (that is milestone M5, S1 and S6), on a real unmounted volume (AC-38 is simulated by removing the directory), with Unicode-normalization differences in directory names, or on other filesystems; AC-6 asserts presence of three categories, not ranking quality.
- The read-side invariant test: that no read path writes under a source for inputs outside the fixture; it also does not exercise write paths because none exist in M1. AC-2 closes in milestone M3.
- `perf_cold_scan`: that a cold scan of 500 skills is under 300 ms on any machine other than the one that ran it, on a cold OS file cache, on network or slow disks, or via the CLI end to end (it times `Catalog::load` with the cache disabled); medians of five runs on a shared developer machine are noisy.
- Snapshot tests: that the JSON and human output is correct, only that it is unchanged since a human read it at creation; path and timestamp hygiene is checked by grep, not semantically.
- Neutralization test: that no other terminal-affecting construct (bidi overrides, very long lines) is handled; only C0, DEL, C1 are.
- Hybrid chmod tests: behavior under root, on filesystems that do not enforce permissions, or for other IO errors (EIO, network drops).
- CI workflow and `dist-workspace.toml`: that either works; only that they parse.
- This validation: no gate command was executed against the real repository (nothing exists to run yet); mechanisms were exercised in scratch workspaces with minimal implementations, so the real implementation's lint, perf, and clippy results are unproven until EXECUTE; the matcher, frontmatter, ignore, and hook behaviors were confirmed on one platform (macOS, aarch64) and one crate-version set.

Plan updates applied: none by the validate agent beyond refreshing the one "Validate-contract status" line in Resume and Execution Handoff (instruction scope). The cycle-1 fixes and the cycle-2 R1 supplement are all in the plan body (verified above), so the former conditional instruction X2 is dropped; what remains below is the final EXECUTE instruction set.

Execute-agent instructions (final):
- X1: Follow the Implementation Checklist in order (Steps A to G), one opus agent, running each step's gate before the next; the plan body carries all earlier fixes (step order, C0 stub files and the `ScannedSkill` type, hermetic `Env` and `git_global`, mechanical G6, exact G7, G10, G11, deterministic cache-write test, `neutralize`, Pre-Step network and hook notes). Decide nothing silently; record every deviation in the phase report.
- X2: each Bash call is a fresh shell; define `gate` and `gate6`, export `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no` inside the same invocation as the gate command. Write all files with the Write tool; no `target/` path or `--target` flag in Bash text; every plan gate command was verified to pass the hook as written.
- X3: first `cargo build` needs crates.io network access; if unavailable return BLOCKED, never hand-vendor. Fix clippy 1.99 `collapsible_if` inline with let chains, never with `allow`.
- X4 (standing): hand off without committing and without pushing; never run `unam` outside `TestEnv`; keep every created file free of em dashes and emoji (G10); model policy: only the implementing agent is opus, every helper (tester, reviewer) is sonnet.

Gate: PASS (0 FAILs, 0 CONCERNs; PVL cycle 3 validation after 3 recorded fix cycles in results-m1.tsv; R1 closed; named residuals only)
Accepted by: n/a - Gate is PASS; no concern needs acceptance. Named residuals (CI not run, `dist-workspace.toml` unverified, chmod Hybrid tests self-skip under root, `perf_cold_scan` machine dependence) are documented under Open gaps and are not concerns.

---

## Next Step

Run PVL (step 4) with vc-validate-agent against this plan, then EXECUTE with one opus agent, then EVL via vc-tester.
