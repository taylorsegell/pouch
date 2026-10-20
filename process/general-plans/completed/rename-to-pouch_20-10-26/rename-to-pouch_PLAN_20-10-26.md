---
name: plan:rename-to-pouch
description: "COMPLEX plan: rename the project from Unambiguous / unam to pouch (binary, packages, crates, paths, env vars, state dir, docs, snapshots) with no compatibility shim"
date: 20-10-26
---

# Rename to pouch (COMPLEX)

TL;DR: one mechanical, script-driven rename of every product-name use of `unam` / `unambiguous` to `pouch` across crates, tests, snapshots, docs, and current context, with the five protected docs getting a single added line. Gated by the full existing suite (including the full fault matrix), `dist` checks, and a new residual-name guard test. Three commits on `main`, no push. GitHub, release, tap, and folder steps are Taylor's, listed in the Handoff.

Status: PLANNED (validate-contract written 2026-10-20, generated-by outer-pvl; VALIDATE plan updates P1 to P14 applied, listed in the contract)
Date: 20-10-26
Complexity: COMPLEX (about 120 code files, 45 snapshots, one Cargo.lock regeneration, no runtime-logic change)

## Phase Completion Rules

- CODE DONE: steps 1 to 13 applied, not yet independently confirmed.
- VERIFIED: gates G1 to G14 green in a spawned vc-tester EVL run, protected-doc diff proven, three commits made on main.
- Known-gap residuals (aarch64 Linux never executed, agent skill discovery on Linux not spike-tested, AC-44 real-terminal TUI run) are named residuals; they never promote or block rename VERIFIED status. Linux and CI HAVE run (CI green on ubuntu-latest and macos-latest, runs 39179076283 and 39345121170; release workflow succeeded once for unam v0.1.0, run 39345610585), so "Linux/CI never run" is not a residual.
- Out-of-scope Handoff items stay open until Taylor does them; they do not block VERIFIED.

## Overview

Taylor decided on 2026-10-20 to rename the tool. v0.1.0 was released once under the old name (release run 39345610585); the Handoff deletes and re-cuts it as pouch. Nothing outside this repo and Taylor's machine uses the old name, so there is no compatibility shim, no old-name fallback, and no migration of existing `.unam/` state.

## Goals and Scope

In scope (locked decisions):

| Surface | Old | New |
|---|---|---|
| binary | `unam` | `pouch` |
| CLI package | `unambiguous` | `pouch` |
| core crate (dir, package, Rust ident) | `crates/unam-core`, `unam-core`, `unam_core` | `crates/pouch-core`, `pouch-core`, `pouch_core` |
| CLI crate dir | `crates/unam-cli` | `crates/pouch-cli` |
| config / cache / data | `~/.config/unam`, `~/.cache/unam`, `~/.local/share/unam` (and `$XDG_CONFIG_HOME/unam`) | `.../pouch` |
| env vars | `UNAM_*` (CONFIG_DIR, BUNDLES_DIR, CACHE_DIR, LOG, FULL_FAULT_MATRIX, REAL_LIBRARY, ENTERPRISE_SKILLS_DIRS, PERF_CHILD, TUI_PANIC_CHILD, TEST_LIBS, TEST_UNSET_VAR) | `POUCH_*` |
| per-project dir | `.unam/` | `.pouch/` |
| temp prefixes | `.unam-tmp-`, `.unam-tmp-rollback`, `.unam-doctor-` | `.pouch-tmp-`, `.pouch-tmp-rollback`, `.pouch-doctor-` |
| bundle metadata key | `metadata.unambiguous.*` | `metadata.pouch.*` |
| tagline (clap about, package description, usage snapshot) | "Activate the right agent skills per project, unambiguously." | "Only the skills this project needs." |
| README title | `# Unambiguous` | `# Pouch` |
| repository URL | `github.com/taylorsegell/unambiguous` | `github.com/taylorsegell/pouch` (Cargo.toml, `release_files.rs` REPOSITORY, README installer and release URLs, installer file `pouch-installer.sh`) |
| TUI snapshot files | `unam__tui__tests__*.snap` | `pouch__tui__tests__*.snap` |

Stays unchanged: the English word "unambiguous" where it is not the name (README "a bare name works when it is unambiguous"; protocol and agent-prompt prose such as `vc-plan-agent` "**Unambiguous**: No creative decisions", `orchestration.md:838`); `dist-workspace.toml` (Homebrew tap `taylorsegell/homebrew-tap`); `release.yml`; `process/features/**` and `process/general-plans/{completed,backlog}/**` history; all `target/` build output (ignored, rebuilt); `.claude/hooks/.logs/`.

Protected docs (`docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`): file names unchanged, text unchanged except ONE added line under the title: `Renamed: this project is now called pouch (binary \`pouch\`) as of 2026-10-20; this document keeps the original name as history.`

Out of scope for EXECUTE (Handoff only): GitHub repo rename, deleting the old v0.1.0 release and tag, pushing, re-tagging pouch v0.1.0, Homebrew tap, local folder rename.

## Standing Constraints (for every step)

- Never run `unam` or `pouch` against this repository. Tests use temp dirs only; every test that spawns the binary must set `POUCH_CONFIG_DIR`, `POUCH_CACHE_DIR`, `POUCH_BUNDLES_DIR` to temp dirs (a missed env var rename would silently read the real home; the guard test in step 12 covers this).
- No em dashes, no emojis in any file written or edited.
- Skill library `~/Developer/01-AI/skillzone/` is read-only.
- Bash commands must not contain path segments named `target`, `dist`, or `build` (hook blocks them): put such commands in a script file in the scratchpad and run that. `dist plan` and `dist generate --check` themselves are fine to invoke.
- Every wait is bounded: Bash `timeout` parameter, `perl -e 'alarm N; exec @ARGV'` on macOS (`timeout` and `gtimeout` do not exist here), no until-grep loops. Budgets measured in the VALIDATE rehearsal: G2, G3 and G9 recompile the renamed workspace, so give each Bash timeout 600000 (the full `cargo test --workspace --no-fail-fast` finished inside one such call, with `ac_briefs` about 80 s and `ac_recovery` about 54 s of that); G4 400000; `dist plan` and `dist generate --check` 120 s via perl alarm. `dist plan`, `dist generate --check` and `cargo check --profile dist` ran fine as plain Bash command text (the hook matches path segments, not the word as a command or profile name); `target`, `dist` or `build` as a path segment (for example `du -sh target`) is blocked, so keep any such command in a scratchpad script.
- cargo-insta is not installed: use `INSTA_UPDATE=always` once to write snapshots, then review `git diff`; gates run with `INSTA_UPDATE=no`.
- Commit on `main`; no push, no tag. Co-author trailer per session attribution.
- `export PATH="$HOME/.cargo/bin:$PATH"` before cargo or dist.

## Touchpoints

Runtime sites (from research; line numbers are starting points, the script catches the rest):

- `crates/unam-core/src/paths.rs` (71-91, 179, 214, 317-412: config/cache/data dir names, env var names, `.unam`), `state.rs` (82-108, 172-176), `project/selection.rs` (99, 158-162, 181), `project/index.rs` (116), `fsops.rs` (229, 254: temp prefixes), `apply.rs` (638, 683, 1472, 1561), `doctor.rs` (143, 241, 332, 358), `error.rs` (62-517, 1000: remedy strings), `bundle/mod.rs` (26, 194-200, 424, 427: metadata key), `generated.rs` (7, 266), `plan.rs` (313, 316), `reconcile.rs` (135, 195), `verify.rs` (329, 336, 443), `adapter/claude_code.rs` (219, 223), `adapter/agents_neutral.rs` (76, 242, 260, 264), `config.rs`, `source.rs`, `change.rs`, `observe.rs`, `catalog/mod.rs`, `lib.rs`.
- `crates/unam-cli/src/`: `cli/mod.rs` (24-26: clap name, about), `main.rs` (36), `output.rs` (201, 213, 291, 548, 608, 839, 966), `cli/change.rs` (96: JSON `next` array "unam resume" / "unam rollback"), `cli/source.rs` (90), `cli/bundle.rs` (56), `cli/status.rs` (125), `tui/app.rs` (229).
- Manifests: root `Cargo.toml` (members, `[workspace.dependencies]` path dep, repository URL), `crates/*/Cargo.toml` (names, description, `[[bin]] name`), `Cargo.lock` (entries near 1476, 1494, 1509; regenerated, never hand-edited).
- Tests (about 30 files in `crates/unam-cli/tests/`, `crates/unam-core/tests/perf_cold_scan.rs`, in-crate `*/tests.rs`): pinning tests `release_files.rs` (13, 50-82, 186-187, 202-227), `phase1_checks.rs` (60), `docs_honesty.rs` (20-32, 73-78, 124, 143-146), `usage.rs` (11-61, 101, 131-136), `broken_pipe.rs` (31), `docs_json.rs` (214), `cargo_bin!` in only 2 files (`common/mod.rs`, `broken_pipe.rs`; VALIDATE count, not 11), `perf_cold_scan.rs` imports, `common/{mod,faults}.rs`.
- Snapshots: 39 integration `.snap` with `source: crates/unam-cli/tests/snapshots.rs` headers (about 20 bodies carry the name), 6 TUI `.snap` files under `crates/unam-cli/src/tui/snapshots/` (rename files).
- Docs and process: `README.md`, `CHANGELOG.md`, `.gitignore` (lines 8-13), `docs/adapters.md`, `docs/json.md`, 5 protected docs (one line each), `process/context/all-context.md`, `process/context/tests/all-tests.md`, `process/context/product/all-product.md`, `process/context/planning/all-planning.md`, `.claude/agents/vc-plan-agent.md` and `.codex/agents/vc-plan-agent.toml` (verified: only the English word "Unambiguous" at lines 545 and 521, so NO edit; recorded so EXECUTE does not touch them), `process/development-protocols/*.md` (English-word hits only, NO edit).
- Read only: `dist-workspace.toml`, `.github/workflows/{ci,release}.yml` (no name text; verify by grep), `docs/spec/*`, `docs/adr/*` (link targets by path).

## Public Contracts

Everything below changes visibly; there is no old-name fallback.

- CLI: command name `pouch`; `--help` text and tagline; every message that prints `unam ...` hints (remedy strings, `next` hints).
- `--json` envelope: `cli/change.rs` `next` array values become `pouch resume` / `pouch rollback`; any JSON string containing the old name changes. `docs/json.md` and the 38 JSON snapshots stay in sync (a test already fails on any missing marker; markers themselves are unaffected).
- Filesystem: `.pouch/` per-project state (selection, receipt, journal, history, lock), `~/.config/pouch/{config.toml,bundles/}`, `~/.cache/pouch`, `~/.local/share/pouch`. Existing `.unam/` and `~/.config/unam` are ignored, not migrated (accepted by Taylor).
- Env vars: all `UNAM_*` become `POUCH_*`; old names are not read.
- Bundle files: `metadata.pouch.*` replaces `metadata.unambiguous.*`; a bundle with the old key is treated as having no metadata (lenient parse already ignores unknown keys).
- Cargo: package `pouch`, crate `pouch-core`; release archives and installer derive as `pouch-*` and `pouch-installer.sh`.
- Generated briefs and journal/receipt text that mention the tool name change; receipts written by the old name are not read.

## Blast Radius

- Size: about 2,500 substring hits; about 120 code and test files; 45 snapshots; 2 crate directories moved; 1 lockfile regeneration; 5 protected docs (one line each); 4 context docs; README, CHANGELOG, `.gitignore`, `docs/adapters.md`, `docs/json.md`.
- Risk class: no logic change, but it touches schema-adjacent surfaces (state dir name, JSON output, bundle metadata key) and release naming. Treated as high-risk for the public-contract class only: manual-first evidence is the gate list below, not a new evidence pack.
- Layout-sensitive output: `pouch` is one character longer than `unam`; any column-aligned table, the 100x30 TUI `TestBackend` snapshots, and usage wrapping can shift. These are expected snapshot-body changes and must be reviewed line by line, not blanket-accepted.
- Build output `target/` holds stale `unam` artifacts; harmless, rebuilt on first build. Do not delete or edit.
- Out of repo: Claude memory dir `~/.claude/projects/-Users-databased-Developer-01-AI-unambiguous` is keyed by the folder path (Handoff item).

## Data Flow and Failure Modes

Flow: name string constants and env var names originate in `pouch-core` (`paths.rs`, `fsops.rs`, `bundle/mod.rs`, error remedies), flow to CLI rendering (`output.rs`, clap), to snapshots and docs. Tests set env vars to temp dirs, spawn the binary (name resolved by `cargo_bin!` from the bin target), and compare against snapshots.

| Failure mode | Effect | Mitigation |
|---|---|---|
| English word "unambiguous" rewritten | prose damage, `docs_honesty` mismatch | script treats `unambiguous` as name only via explicit per-file rules and a reviewed dry-run report; default is "report, do not rewrite" |
| `unam(?!biguous)` rewrite hits a non-name token | broken identifier | dry-run lists every distinct matched token with context; only reviewed tokens apply |
| Rust ident vs kebab confusion (`unam_core` vs `unam-core`) | compile error | ordered rules: `unam_core`->`pouch_core`, `unam-core`->`pouch-core`, `unam-cli`->`pouch-cli` first |
| Missed `UNAM_*` in a test | test reads real home or real library | env-var guard in step 12 plus `grep -rn 'UNAM_'` must be empty |
| Protected file renamed or text rewritten | violates standing rule | script excludes `docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/**`, `docs/adr/**`; a gate diffs each to prove only the one line was added |
| TUI snapshot file names stale | insta reports missing snapshot (fails under `INSTA_UPDATE=no`) | `git mv` all 6 to `pouch__tui__tests__*`; gate runs the TUI tests |
| Cargo.lock stale | `--locked` fails | regenerate once with `cargo update -w` offline-safe path (see step 7), then all later commands use `--locked` |
| Rename interrupted midway | mixed tree | each step ends in a checkpoint (`cargo check` or grep count); rollback below |

## Pre-Plan Test Discovery (vc-test-coverage-plan, TEST-SCENARIO-DISCOVERY)

Loaded: `process/context/tests/all-tests.md` (router; its deeper chain for this repo is `docs/spec/unam-technical-spec.md` section 8 and PRD 13.2, which hold no extra runners beyond cargo). Existing blast-radius test files discovered above (about 30 integration files, in-crate TUI tests, `perf_cold_scan.rs`). Runner: `cargo test` (714 tests, 0 ignored, 45 snapshots; VALIDATE-confirmed count, not 712), `insta`, `proptest`; release tooling via `dist`. Test-runner field: `cargo test | dist plan`.

Existing coverage that already proves the rename safe: 714-test suite spawns the real binary against temp dirs; `docs_json` (marker parity), `docs_adapters` (evidence rows), `docs_honesty` (README caveats and forbidden claims), `release_files` (package metadata, dist config), `usage` (help snapshot), `broken_pipe`, `phase1_checks` (package list, AC coverage), `ac_recovery` / `ac_briefs` fault matrices.

Tier assignments (no Known-Gap for developed behavior):

| Area | Fully-Automated | Hybrid | Agent-Probe |
|---|---|---|---|
| crates (paths, state, messages, JSON) | suite + snapshots + new `rename_guard` | full fault matrix `POUCH_FULL_FAULT_MATRIX=1` (needs ~150 s) | read the snapshot diff for layout shifts |
| release naming | `release_files` pinned to `pouch` | `dist plan`, `dist generate --check` (needs `dist` on PATH, offline) | check archive and installer names in `dist plan` output |
| docs and protected docs | `docs_json`, `docs_adapters`, `docs_honesty`, protected-doc diff script | none | skim README for English-word survivors |
| process context | none (prose) | `vc-audit-context` validators | none |

Known gaps (named residuals, backlog stub required; see `## Known Gaps (Resolved via Backlog)`): aarch64 Linux never executed, agent skill discovery on Linux not spike-tested (the rename adds no new Linux risk because no logic changes). Linux and CI have run and are green (ubuntu-latest, macos-latest). The real-terminal TUI manual run (AC-44) is unchanged and remains Taylor's. Neither is used to declare rename behavior PASS: the rename behavior is proved by the automated and Hybrid rows above.

Failing stubs (red-first, written in step 3 before the rewrite so they fail against the old tree for the right reason):

```
Failing stub:
test("should find no old-name residue in crates", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: residual-name guard")
})
Failing stub:
test("should read only POUCH_ env vars for config, bundles, cache", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: env var rename")
})
```

## Implementation Checklist (ordered, atomic)

Phase A: baseline and tooling

1. Confirm clean start: `git status --short` shows only the pre-existing `D START-HERE.md` (leave it as is, do not stage it); `git log -1` is 483e729 or later; no `.unam/` exists in the repo.
2. Baseline green on the OLD names, recorded in the phase report: `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `INSTA_UPDATE=no cargo test --workspace --locked` (expect 714 passed, 0 ignored); `UNAM_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs` then `--test ac_recovery` (about 150 s; Bash timeout 400000); `dist plan` and `dist generate --check`. If any is red, stop: the baseline must be green before the rename (record and surface).
3. TDD (red first): add `crates/unam-cli/tests/rename_guard.rs` (old path, becomes `pouch-cli` after step 5) with two tests: (a) `no_old_name_in_crates` walks `crates/**` (`.rs`, `.toml`, `.snap`, fixtures; skip any path with a `target` component) and fails on regex `(?i)unam(?!biguous)` and on `unambiguous` outside an allowlist constant (empty at first; each English-word survivor is added with a one-line reason); (b) `env_vars_are_pouch_only` fails on any `UNAM_` text under `crates/**` and requires the exact set of `POUCH_` names from the Scope table to appear in `paths.rs` or the test files that set them. Both fail now; record the red output. Use only std (no new dependency; `regex` is not in the lockfile for tests, so implement with manual scanning). The guard file contains the forbidden needles, so it MUST NOT flag itself: build the needles at runtime (for example `["un", "am"].concat()` and `["UN", "AM_"].concat()`) and also skip its own file name; the rewrite script excludes this file by name (VALIDATE finding P3).

Phase B: structure moves

4. Write the rewrite script `rename_to_pouch.py` in the scratchpad (not in the repo) with: file selection = `git ls-files` minus exclusions; exclusions = `START-HERE.md` (deleted in the working tree but still listed by `git ls-files`: skip any listed path that does not exist, never recreate it), `spike/**` (historical M0 evidence record, history stays history), `crates/*/tests/rename_guard.rs`, `process/features/**`, `process/general-plans/completed/**`, `process/general-plans/backlog/**`, the five protected docs, `.claude/**`, `.codex/**`, `.agents/**`, `process/development-protocols/**`, `Cargo.lock`, `dist-workspace.toml`, `.github/**`, `CLAUDE.md`, `AGENTS.md`, this plan folder, binary files; ordered rules (first match wins per span): `UNAM_`->`POUCH_`; `unam_core`->`pouch_core`; `unam-core`->`pouch-core`; `unam-cli`->`pouch-cli`; `unam__tui`->`pouch__tui`; `UNAM(?!BIGUOUS)`->`POUCH`; `Unam(?!biguous)`->`Pouch`; `unam(?!biguous)`->`pouch` (covers `.unam`, `.unam-tmp-`, `unam_dir`, `let unam`, test fn names, `bare_unam_non_tty`); tagline literal replaced exactly; name uses of `unambiguous` (package `-p unambiguous`, `name = "unambiguous"`, `pkg("unambiguous"`, `metadata.unambiguous`, `unambiguous:` YAML key, `get("unambiguous")`, repository URL path, "cargo install unambiguous works", `# Unambiguous`) via explicit literal rules only. Modes: `--report` (prints every distinct matched token with file:line context and the file count, writes nothing) and `--apply`. It must be idempotent. VALIDATE rehearsal traps (a scratch copy was rewritten with the rules exactly as first drafted; fixed here): (1) the unguarded `Unam` rule turned README line 1 `# Unambiguous` into `# Pouchbiguous`, so ALL three case rules carry the `(?!biguous)` lookahead and the literal `# Unambiguous` -> `# Pouch` rule runs before any generic rule; (2) the generic `unam` rule turned the protected-file link `docs/spec/unam-technical-spec.md` into the dead link `docs/spec/pouch-technical-spec.md` in `README.md:108`, and the same token sits in `process/context/all-context.md` (45, 145), `product/all-product.md` (38, 58, 87, 112) and `tests/all-tests.md` (55, 57): the script MUST mask the tokens `unam-technical-spec.md` and `PRD-unambiguous.md` (and any `docs/spec/`, `docs/adr/` path) before applying rules and restore them after; (3) `spike/symlink-discovery.md` was rewritten (one prose line) because it was not excluded.
5. `git mv crates/unam-core crates/pouch-core`; `git mv crates/unam-cli crates/pouch-cli`; `git mv` each of the 6 `crates/pouch-cli/src/tui/snapshots/unam__tui__tests__*.snap` to `pouch__tui__tests__*.snap`. Checkpoint: `git status` shows renames only.
6. Run the script with `--report`; a human (EXECUTE agent, then EVL reviewer) reads the full distinct-token list. Any token not in the intended set is either added as an explicit exclusion or flagged. Record the reviewed report in the phase report.
7. Run `--apply`. Then hand-fix the traps the rules deliberately did not cover, each checked by reading the diff: (a) `bundle/mod.rs` local `let unam = get("metadata")...get("unambiguous")` becomes a `pouch` binding reading the `pouch` key, plus the two test strings at 424 and 427 and the doc comment at 26; (b) `generated.rs:7` doc comment key; (c) `crates/pouch-cli/Cargo.toml`: `name = "pouch"`, `description = "Only the skills this project needs."`, `[[bin]] name = "pouch"`; `crates/pouch-core/Cargo.toml` description text "Core library for pouch: sources, catalog, search."; root `Cargo.toml` members, path dependency key `pouch-core`, repository `https://github.com/taylorsegell/pouch`; (d) `cli/mod.rs:24-26` clap `name`, `about` = tagline; (e) every `use unam_core::` and `unam_core::` path now `pouch_core`, and `Cargo.toml` dependency key `pouch-core.workspace = true` (checked by `cargo check`); (f) `release_files.rs` REPOSITORY constant and all `pkg("pouch", ...)` fixtures, the `["pouch", "pouch-core"]` loop, "package pouch missing" message; (g) `phase1_checks.rs:60` package pair; (h) `docs_honesty.rs` FORBIDDEN entry becomes "cargo install pouch works" and the primary-scenario check looks for a bare `pouch` README line; (i) any English-word `unambiguous` the rules left alone stays and is added to the guard allowlist with a reason.
8. Refresh the lockfile once: `cargo metadata --format-version 1 --offline > /dev/null` (writes the new entries for renamed workspace members without network; VALIDATE rehearsal: works offline, no dependency version changes); if the lock is not updated, `cargo generate-lockfile --offline` is NOT acceptable (it re-resolves versions); use `cargo update -w --offline`. Verify `git diff Cargo.lock`: rehearsal result is 37 insertions and 37 deletions, because cargo keeps the file alphabetical, so the `pouch` and `pouch-core` blocks MOVE up to just before `powerfmt` (near line 865) and the old `unam-core` and `unambiguous` blocks (near 1476 and 1494) are deleted; the line numbers the plan first named do not apply. Exact check (script file): the removed lines with `unam-core`->`pouch-core`, `unambiguous`->`pouch` normalised, sorted, must equal the added lines sorted; any dependency version or checksum change is a stop. From here on every cargo command uses `--locked`.
9. Snapshots: delete nothing. Run `INSTA_UPDATE=always cargo test --workspace --locked` once (wrapped with Bash timeout 600000), then `INSTA_UPDATE=no cargo test --workspace --locked` must pass. Review `git diff -- '*.snap'` fully: allowed changes are (i) `source:` header paths `crates/pouch-cli/tests/snapshots.rs`, (ii) the name in about 20 bodies, (iii) the tagline in `snapshots__usage_bare.snap`, (iv) width or alignment shifts caused by the 1-character longer name, each looked at. Any other body change is a bug, not a snapshot update: revert and investigate. Rehearsal expectation (so the review has a yardstick): 45 snapshot files change (45 `source:` headers); body changes are `.unam` -> `.pouch` in paths and ignore-advice lines, `unam resume` / `unam rollback` hints, and `(not managed by pouch)`; the ONLY layout shift is `pouch__tui__tests__tui_snapshot_result_view.snap` (the "Add these lines to .gitignore (pouch never edits it)" rows re-pad by one column). Run the pre-review `INSTA_UPDATE=no` pass with `--no-fail-fast`, because plain `cargo test --workspace` stops at the first failing test binary and hides the rest.

Phase C: docs and process

10. Docs, by hand or by the script's included list: `README.md` (title `# Pouch`, all `unam` commands to `pouch`, env var names, `pouch-installer.sh`, release and installer URLs under `github.com/taylorsegell/pouch`, protected-doc links by path unchanged, keep "a bare name works when it is unambiguous"), `CHANGELOG.md` (there is no `Unreleased` section: the file has `## [0.1.0] - 2026-10-19`. Hand-edit under `### Changed`: replace the existing line "The command-line package is named `unambiguous`; the binary is still `unam`, and the library crate is still `unam-core`." (it would read falsely after a blanket rewrite) with "Renamed from Unambiguous / `unam` to pouch / `pouch` (package `pouch`, library crate `pouch-core`); there is no migration from `.unam/` or `~/.config/unam`." Historical milestone bullets may take the generic `unam` -> `pouch` rewrite), `.gitignore` lines 8-13 (`.pouch/...`, comment text), `docs/adapters.md`, `docs/json.md` (`next` hints, any command lines).
11. Protected docs: insert exactly one line directly after the title line (with one blank line separation) in each of the five files: `Renamed: this project is now called pouch (binary \`pouch\`) as of 2026-10-20; this document keeps the original name as history.` Use a small script that asserts line 1 of each file starts with `# ` (match the FIRST line only: `PRD-unambiguous.md` has 3 and `unam-technical-spec.md` has 5 lines starting `# `, so a count-based "exactly one title" assertion would fail or pick the wrong line) and writes nothing else. Gate script (see Verification) proves `git diff --numstat` shows `2 0` per file (one added line plus one blank) or `1 0` if a blank already follows; the diff body contains no removed lines.
12. Process context (current docs only): `process/context/all-context.md` (name in title and prose, status line gets "renamed to pouch on 2026-10-20", repository tree, crate paths, `-p pouch`, env var names, `.pouch/`, license block, "Never run `pouch` against this repository", keep historical statements like the protected-doc file names as paths), `tests/all-tests.md`, `product/all-product.md`, `planning/all-planning.md`; update `description:` front matter and "Last updated". While editing these four docs, also correct the facts VALIDATE found stale: tests are 714 not 712; CI HAS run and is green on ubuntu-latest and macos-latest (runs 39179076283, 39345121170) so "CI written, not yet run" and "never run" (`all-tests.md` description and Known Gaps, `all-context.md` lines 22, 103, 111, 169, 171) become "CI green on ubuntu and macos; release workflow ran once for v0.1.0 (run 39345610585)"; keep the open items aarch64 Linux never executed, agent skill discovery on Linux not spike-tested, AC-44, Homebrew tap. Do NOT edit `process/features/**` (so the backlog note `m7-linux-ci-verification_NOTE_17-10-26.md` stays as history; its staleness is recorded in the new residual stub), completed or backlog plans, protocol files, or agent files (English-word hits only). Do not change the umbrella plan's Current Execution State; the rename is noted in `all-context.md` status only (routing does not require umbrella changes).
13. Complete the guard tests from step 3: add a TUI snapshot-name consistency test in `crates/pouch-cli/src/tui/tests.rs` (or the existing module) that asserts every file in `src/tui/snapshots/` starts with `pouch__tui__tests__` and that no file there starts with `unam`. Both step-3 tests must now pass; record the green output.

Phase D: verification and commit

14. Run the full gate list below. Fix forward inside this plan's scope only; a failure that needs logic changes is a stop.
15. Commit 1 (structure and code): `refactor: rename crates, binary, paths, env vars and state dir from unam to pouch` containing `crates/**`, `Cargo.toml`, `Cargo.lock`, snapshots. Commit 2 (docs): `docs: rename to pouch in README, CHANGELOG, adapters, json, gitignore; add rename notes to protected docs`. Commit 3 (process): `process: update context for the pouch rename; add rename plan`. Each commit message ends with the session attribution trailer. Leave `START-HERE.md` deletion unstaged: stage ONLY by explicit pathspec (`git add -- <paths>`), never `git add -A`, `git add .` or `git commit -a`; before each commit run `git diff --cached --name-status` and require no `START-HERE.md` line; after commit 3 `git status --short` must still show ` D START-HERE.md`. Do not push or tag.

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| G1 `cargo fmt --all --check` | Fully-Automated | AC-R1: formatting unchanged after mass rewrite |
| G2 `cargo clippy --workspace --all-targets --locked -- -D warnings` | Fully-Automated | AC-R2: renamed crates compile cleanly, no unwrap regressions |
| G3 `INSTA_UPDATE=no cargo test --workspace --locked` (expect 714 + 3 new tests = 717, 0 ignored, 45 snapshots) | Fully-Automated | AC-R3: behavior unchanged apart from the name; AC-R4: all JSON and TUI output matches reviewed snapshots |
| G4 `POUCH_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs` then `--test ac_recovery` (about 150 s) | Hybrid | AC-R5: apply, resume, rollback state-dir and temp-prefix renames keep recovery correct under every injected failure |
| G5 `rename_guard` tests: no `unam` / `UNAM_` / name-use `unambiguous` in `crates/**` | Fully-Automated | AC-R6: no residual old name in code, tests, snapshots; env vars POUCH-only |
| G6 residual-name grep outside crates, run from a scratchpad SCRIPT file (the table cell cannot hold a raw pipe: use `git grep -InE 'UNAM_|unam(-core|_core|-cli)?\b|\.unam'` with plain `|`; a literal `\|` under `-E` matches a pipe character and passes vacuously). Allowlist: protected docs, `process/features/**`, completed and backlog plans, this plan, `process/development-protocols/**`, `.claude/**`, `.codex/**`, `.agents/**`, `spike/**`, `START-HERE.md`, the token `unam-technical-spec.md` (protected-file link), and lines containing "renamed from" or "formerly" in `CHANGELOG.md` and `process/context/**`. POSITIVE CONTROL: the same script run against base commit 483e729 (`git grep ... 483e729 -- .`) must report hits; the working tree minus allowlist must report none | Fully-Automated | AC-R7: docs and context carry only the new name |
| G7 protected-doc diff script: for each of the 5 files `git diff -U0` has exactly one added content line (the Renamed line), zero removed lines; `git diff --name-status` shows no rename or delete of them | Fully-Automated | AC-R8: protected docs untouched beyond the one line; links by path still resolve |
| G8 `dist plan` shows `announcing v0.1.0`, archives named `pouch-*`, installer `pouch-installer.sh`, formula `pouch`; `dist generate --check` passes (run from a scratchpad script to avoid the path-word hook) | Hybrid (needs `dist` 0.32.0 on PATH) | AC-R9: release metadata derives pouch names with no workflow edits |
| G9 `cargo check --profile dist --locked --offline -p pouch` (via script file) | Hybrid | AC-R10: release profile still builds under the new package name |
| G10 `git diff --check` and the Cargo.lock normalised-diff check from step 8 (removed lines with names normalised, sorted, equal added lines sorted; no `version =` or `checksum =` line differs outside the two workspace blocks); `git diff HEAD~3 --stat -- Cargo.lock` is informational (expect 37 and 37) | Fully-Automated | AC-R11: no whitespace damage, no dependency drift |
| G11 `docs_json`, `docs_adapters`, `docs_honesty`, `release_files`, `usage`, `phase1_checks` inside G3, called out individually in the report | Fully-Automated | AC-R12: documented contracts match the renamed surface |
| G12 Agent-probe: read full `git diff -- '*.snap'` and the TUI snapshots for alignment shifts; confirm tagline and README skim for surviving English-word uses | Agent-Probe | AC-R13: width-sensitive output reviewed by judgment |
| G13 `! grep -rEq '#\[ignore' crates` | Fully-Automated | AC-R14: no ignored tests introduced |
| G15 real-home isolation (script file): before step 2 record `stat` mtimes of `~/.cache/unam` and `~/.local/share/unam/projects.json` (real state from 2026-10-19 use; `~/.config/unam` does not exist); after G3 and G4 the mtimes are unchanged AND `~/.config/pouch`, `~/.cache/pouch`, `~/.local/share/pouch` do not exist (all absent on 2026-10-20) | Fully-Automated | AC-R16: no test reads or writes the real home under either name |
| G16 commit hygiene: before each commit `git diff --cached --name-status` has no `START-HERE.md` line and each commit lists only its intended paths; after commit 3 `git status --short` shows ` D START-HERE.md`; `git log --oneline origin/main..HEAD` shows exactly 4 commits (483e729 plus the three) | Fully-Automated | AC-R17: the unrelated deletion is never committed; nothing pushed |
| G14 harness validators for touched areas: `node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs` (context docs changed), `validate-plan-artifact.mjs` on this plan | Fully-Automated | AC-R15: context routing and plan artifact still valid |

SPEC-link note: no separate SPEC file exists (mechanical rename under Taylor's locked decisions). The AC-R rows above are this plan's acceptance criteria; each names its proving gate and strategy (REQ-TEST-LINK): AC-R1 proven by G1, strategy Fully-Automated; AC-R2 by G2, Fully-Automated; AC-R3 and AC-R4 by G3, Fully-Automated; AC-R5 by G4, Hybrid; AC-R6 by G5, Fully-Automated; AC-R7 by G6, Fully-Automated; AC-R8 by G7, Fully-Automated; AC-R9 by G8, Hybrid; AC-R10 by G9, Hybrid; AC-R11 by G10, Fully-Automated; AC-R12 by G11, Fully-Automated; AC-R13 by G12, Agent-Probe; AC-R14 by G13, Fully-Automated; AC-R15 by G14, Fully-Automated; AC-R16 by G15, Fully-Automated; AC-R17 by G16, Fully-Automated. Known-Gap residuals (aarch64 Linux never executed, agent skill discovery on Linux not spike-tested, AC-44 manual terminal run) are recorded as backlog; they are not used to pass any AC-R row.

## Risk Predictions (vc-predict, 5-persona summary)

- Maintainer: mass rewrite over-reaches into English prose. Mitigation: report-first script, explicit literals for `unambiguous`, guard allowlist with reasons.
- Tester: snapshot churn hides a real regression. Mitigation: classify every snapshot hunk (headers, name, tagline, width) and revert others.
- Release engineer: `dist` names or lock drift. Mitigation: G8, G9, G10 plus lockfile numstat check.
- Skeptic: env var missed so tests touch real `~/.config`. Mitigation: G5(b) and the standing temp-dir rule; EVL tester also runs `git status` on `$HOME/.config/pouch` absence check (read-only `ls`).
- Operator: stale `.unam/` state silently orphaned. Accepted by Taylor (no shim); CHANGELOG states it.

Highest-risk items edge cases (vc-scenario): (1) snapshot regeneration: an `INSTA_UPDATE=always` run also rewrites unrelated snapshots if tests are nondeterministic; mitigation: diff must be limited to allowed hunks, rerun twice and require identical results. (2) Cargo.lock refresh: `cargo update -w` can bump nothing but must be proven by numstat; if offline resolution fails, stop rather than touch registry. (3) TUI snapshot file names: insta derives the prefix from the bin target name, so if the `[[bin]]` rename is missed the prefix stays `unam__` and tests fail; G3 plus the step-13 name test catch it.

## Rollback

All work is local and uncommitted until step 15. Before step 4 record `git rev-parse HEAD` (expect 483e729 or later).

- Before commit: do not use a blanket `git restore` (it would drop the unrelated `START-HERE.md` deletion). Reverse the three `git mv` groups (`crates/pouch-core` to `crates/unam-core`, `crates/pouch-cli` to `crates/unam-cli`, the 6 TUI snapshots), then `git checkout --` each tracked path the script touched (the script's `--apply` output lists them), and delete `rename_guard.rs`.
- After commit: `git revert` the three commits newest first (nothing is pushed, so no history rewrite).
- The scratchpad script is idempotent and not needed for rollback.

## Test Infra Improvement Notes

- New permanent guard `rename_guard.rs` (residual old-name scan plus POUCH-only env check) stays in the suite as a regression guard.
- TUI snapshot-name consistency test added in step 13.
- Observed risk: tests rely on discipline to set `*_CONFIG_DIR` env vars; a shared harness helper that fails if a spawned binary lacks them would remove the class of error. Backlog candidate, not done here.

## Resume and Execution Handoff

1. Selected plan file: `/Users/databased/Developer/01-AI/unambiguous/process/general-plans/active/rename-to-pouch_20-10-26/rename-to-pouch_PLAN_20-10-26.md`
2. Last completed phase or step: PLAN written; no code touched.
3. Validate-contract status: written 2026-10-20 (Gate: PASS, generated-by outer-pvl); see `## Validate Contract` and the `## Autonomous Goal Block`.
4. Supporting context loaded: `process/context/all-context.md`, `tests/all-tests.md`, `planning/all-planning.md`, `product/all-product.md` (routing only), research findings from vc-research-agent, `process/development-protocols/orchestration.md`, `implementation-standards.md`.
5. Next step for a fresh agent: VALIDATE is done; after "ENTER EXECUTE MODE", a spawned vc-execute-agent starts at step 1 (baseline) and must not skip step 2 or step 3. EVL: a spawned vc-tester re-runs G1 to G14. Model: sonnet for every spawn including EXECUTE (Taylor's standing note), never fable.

### Handoff: steps that are NOT part of this plan's EXECUTE (each needs Taylor's explicit go-ahead at the time)

1. Rename GitHub repo to `taylorsegell/pouch` (origin URL then updates: `git remote set-url origin https://github.com/taylorsegell/pouch.git`).
2. Delete the old v0.1.0 release and tag (local and remote).
3. Push `main` (local `main` is 1 commit ahead of origin, 483e729, plus the three rename commits: 4 unpushed).
4. Re-tag and release `pouch` v0.1.0 (first real release run; confirm installer name `pouch-installer.sh` on the release page).
5. Homebrew tap `taylorsegell/homebrew-tap`: formula name becomes `pouch` after the first release run.
6a. Old real-home state is orphaned, not migrated (accepted): `~/.cache/unam` and `~/.local/share/unam/projects.json` exist from 2026-10-19 use (`~/.config/unam` does not). Nothing reads them after the rename; Taylor may delete them and re-run `pouch source add` for his library.
6. Local folder rename (Taylor does it last): first copy the Claude memory dir `~/.claude/projects/-Users-databased-Developer-01-AI-unambiguous` to the new path-derived name; then the folder rename; then check `process/context` links and `git status`.

## Known Gaps (Resolved via Backlog)

Named residuals, pre-classified `known-gap: documented as NEW PLAN REQUIRED` (excluded from CONCERN and FAIL counts; none is used to pass any AC-R row):

- aarch64 Linux never executed: only x86_64 Linux ran in CI. Justification: the rename changes no logic and no Linux-specific code. Backlog stub: created at UPDATE PROCESS as `process/general-plans/backlog/rename-residual-linux-and-terminal_NOTE_20-10-26.md` (also records that `process/features/v1-static-activation/backlog/m7-linux-ci-verification_NOTE_17-10-26.md` is stale on CI and Linux).
- Agent skill discovery on Linux not spike-tested (spike M0 ran on macOS). Same stub.
- AC-44 real-terminal TUI run: Taylor's manual run, unchanged by the rename (existing note `process/features/v1-static-activation/backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md`).

## Validate Contract

Status: PASS
Date: 20-10-26
date: 2026-10-20
generated-by: outer-pvl

Parallel strategy: sequential
Rationale: signal score 3/7 (S2 public-contract surface, S6 public-contract high-risk class, S7 5+ files; S1 no, only 2 crates; S3, S4, S5 no). The threshold table says parallel subagents, but the fit rule overrides it: one rewrite script, one shared cargo build state and one Cargo.lock mean no independent slices and no benefit from fan-out. EXECUTE is one agent (1 agent; Taylor's standing note: sonnet) followed by one vc-tester EVL spawn (sonnet). Parallel subagents would be 4 + N sections + 3 (about 8 to 15 agents) for no gain; an agent team has no coordination need.

Method: besides the Layer 1 and Layer 2 reading, this VALIDATE ran a rehearsal in a scratch copy outside the repo (tracked files copied, plan rules applied as first drafted, then `cargo metadata --offline`, `dist plan`, `dist generate --check`, `cargo check --profile dist`, full `cargo test --workspace --no-fail-fast` with `INSTA_UPDATE=no`). Result: the approach works; 713 of 714 tests passed untouched, the one failure is the expected width shift in `tui_snapshot_result_view`; the plan text had 14 defects, all fixed by the plan updates below. No sub-agents were spawned (the V2 dimension and section checks were done directly, with evidence).

Test gates (C3 5-column table; the legacy line form follows):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| AC-R1 | formatting unchanged after mass rewrite | Fully-Automated | `cargo fmt --all --check` (G1) | A |
| AC-R2 | renamed crates compile cleanly, no unwrap regressions | Fully-Automated | `cargo clippy --workspace --all-targets --locked -- -D warnings` (G2) | A |
| AC-R3, AC-R4 | behavior unchanged apart from the name; JSON and TUI output match reviewed snapshots | Fully-Automated | `INSTA_UPDATE=no cargo test --workspace --locked`, expect 717 passed (714 + 3 new), 0 ignored, 45 snapshots (G3) | A |
| AC-R5 | apply, resume, rollback keep recovery correct under every injected failure with the new state dir and temp prefixes | Hybrid | `POUCH_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs` then `--test ac_recovery` (G4; about 150 s; precondition: idle machine, Bash timeout 400000) | A |
| AC-R6 | no residual old name in code, tests, snapshots; env vars POUCH-only | Fully-Automated | new `rename_guard` tests `no_old_name_in_crates`, `env_vars_are_pouch_only` (G5) | B |
| AC-R7 | docs and current context carry only the new name | Fully-Automated | G6 script with allowlist and positive control against base commit 483e729 | B |
| AC-R8 | protected docs gain exactly one line; no rename, delete or link break | Fully-Automated | G7 protected-doc diff script (first-line title match) | A |
| AC-R9 | release metadata derives pouch names with no workflow edits | Hybrid | `dist plan` (announcing v0.1.0, `pouch-*` archives, `pouch-installer.sh`, `pouch.rb`) and `dist generate --check` (G8; precondition: cargo-dist 0.32.0 at `~/.cargo/bin`, offline). Rehearsed green. | A |
| AC-R10 | release profile builds under the new package name | Hybrid | `cargo check --profile dist --locked --offline -p pouch` via script file (G9). Rehearsed green. | A |
| AC-R11 | no whitespace damage, no dependency drift | Fully-Automated | `git diff --check` plus the normalised Cargo.lock diff check (G10) | B |
| AC-R12 | documented contracts match the renamed surface | Fully-Automated | `docs_json`, `docs_adapters`, `docs_honesty`, `release_files`, `usage`, `phase1_checks` named individually inside G3 (G11) | A |
| AC-R13 | width-sensitive output reviewed by judgment | Agent-Probe | read `git diff -- '*.snap'` against the rehearsal yardstick in step 9; README skim for English-word survivors (G12) | A |
| AC-R14 | no ignored tests introduced | Fully-Automated | `! grep -rEq '#\[ignore' crates` (G13) | A |
| AC-R15 | context routing and plan artifact still valid | Fully-Automated | `validate-context-discovery.mjs`, `validate-plan-artifact.mjs` on this plan (G14) | A |
| AC-R16 | no test reads or writes the real home under either name | Fully-Automated | G15 real-home isolation script (mtimes of old dirs unchanged; `~/.config/pouch`, `~/.cache/pouch`, `~/.local/share/pouch` absent) | B |
| AC-R17 | the unrelated `START-HERE.md` deletion is never committed; nothing pushed | Fully-Automated | G16 commit-hygiene script (`git diff --cached --name-status`, `git log --oneline origin/main..HEAD` equals 4) | B |

gap-resolution legend:
- A: proven now (gate passes in this cycle)
- B: fixed in this plan (gate added or tightened by this plan's checklist or VALIDATE updates)
- C: deferred to a named later phase/plan
- D: backlog test-building stub (named residual; keep-active; continue)

C-4 reconciliation: the `strategy:` column carries only Fully-Automated, Hybrid, Agent-Probe. Known-Gap is never a strategy; it is a named residual row carried via gap-resolution D:

| residual (not a proving strategy) | why it stays open | gap-resolution |
|---|---|---|
| aarch64 Linux never executed | only x86_64 Linux ran in CI; rename changes no logic | D |
| agent skill discovery on Linux not spike-tested | spike M0 ran on macOS only | D |
| AC-44 real-terminal TUI run | Taylor's manual run, unchanged by the rename | D (existing note) |

Net-gate vacuous-green check: every developed behavior in the blast radius (paths, state dir, temp prefixes, env vars, JSON `next` hints, bundle metadata key, clap name and tagline, release names, docs) has a Fully-Automated or Hybrid gate above. Known-Gap rows are residuals about platforms and a manual run, not behaviors passing silently. Not vacuously green.

Legacy line form (retained so existing validate-contract consumers still parse):
- crates (paths, state, messages, JSON): [Fully-automated: `INSTA_UPDATE=no cargo test --workspace --locked` (717 expected) + `rename_guard`] | [hybrid: `POUCH_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs` then `--test ac_recovery`; precondition idle machine] | [agent-probe: read the snapshot diff for layout shifts] | [known-gap: aarch64 Linux, Linux skill discovery, AC-44 documented]
- release naming: [Fully-automated: `release_files` pinned to `pouch`] | [hybrid: `dist plan`, `dist generate --check`, `cargo check --profile dist --locked --offline -p pouch`] | [agent-probe: archive and installer names in `dist plan` output] | [known-gap: none]
- docs and protected docs: [Fully-automated: `docs_json`, `docs_adapters`, `docs_honesty`, G6, G7] | [hybrid: none] | [agent-probe: README skim] | [known-gap: none]
- process context: [Fully-automated: `validate-context-discovery.mjs`, `validate-plan-artifact.mjs`] | [hybrid: `vc-audit-context`] | [agent-probe: none] | [known-gap: none]

Failing stubs (Fully-Automated rows that are NEW test code; red-first in step 3, copied from the plan and extended; the other Fully-Automated rows are command gates, not test code, so they carry a positive control or a pre-rename baseline run instead of a stub):

```
Failing stub:
test("should find no old-name residue in crates", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: residual-name guard")
})
Failing stub:
test("should read only POUCH_ env vars for config, bundles, cache", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: env var rename")
})
Failing stub:
test("should name every TUI snapshot file with the pouch__tui__tests__ prefix", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: TUI snapshot-name consistency")
})
```

Dimension findings:
- Infra fit: PASS (was CONCERN on stale CI facts and gate hygiene; resolved by P2, P9, P13) - dist, lockfile and release profile verified by rehearsal: `dist plan` announces v0.1.0 with `pouch-*` archives, `pouch-installer.sh`, `pouch.rb`; `dist generate --check` exit 0; `cargo metadata --offline` refreshes the lock with no version change; `cargo check --profile dist --locked --offline -p pouch` finished. `.github/**` and `dist-workspace.toml` carry no old-name text.
- Test coverage: PASS (was CONCERN; resolved by P1, P3, P6, P10) - test count corrected to 714 (rehearsal sum 713 passed + 1 expected snapshot failure); guard tests must not flag themselves; G6 could pass vacuously (a literal `\|` under `grep -E`) and now has a positive control; G15 and G16 added; insta metadata-only `source:` headers are not compared, so the script (not insta) must rewrite them, and it does.
- Breaking changes: PASS - intended and accepted: state dir `.pouch/`, env vars `POUCH_*`, `--json` `next` hints, bundle key `metadata.pouch`, tagline, release names; no shim; Taylor accepted orphaned old state (real `~/.cache/unam` and `~/.local/share/unam/projects.json` exist from 2026-10-19 use; recorded in Handoff 6a).
- Security surface: PASS - no auth, network or secret change; the threat model (single user, local) needs no extra hardening; the `.gitignore` state-dir lines move with the rename so project state stays ignored; the env-var set was checked against the code: the 11 `UNAM_*` names in `crates/**` equal the plan's table exactly and no non-prefixed variable carries the name.
- Phase A feasibility (steps 1-3): PASS (was CONCERN) - baseline facts corrected (714); guard self-scan and runtime-built needles required; START-HERE.md deletion noted. Highest-risk edit: `rename_guard.rs` flagging itself or being rewritten by the script; mitigated by P3.
- Phase B feasibility (steps 4-9): PASS (was CONCERN) - rehearsal found three script traps (`# Pouchbiguous` from the unguarded `Unam` rule; dead link `pouch-technical-spec.md` in README:108 and four context docs; `spike/symlink-discovery.md` rewritten), plus `git ls-files` listing the deleted START-HERE.md; all fixed by P3 and P4. Lockfile diff is 37/37 with blocks moved alphabetically, not "three entries near 1476" (P5). Only `tui_snapshot_result_view` shifts layout (P6). Highest-risk edit: the mass rewrite; mitigated by the report-first mode, the mask list and the yardstick.
- Phase C feasibility (steps 10-13): PASS (was CONCERN) - CHANGELOG has no Unreleased section and a line 22 that would read falsely (P7); protected-doc title match must use line 1 only (PRD has 3 and the spec 5 `# ` lines) (P8); context docs carry stale CI and count facts (P9).
- Phase D feasibility (steps 14-15): PASS (was CONCERN) - commit pathspec hygiene made explicit (P10); G16 added.

Plan updates applied (VALIDATE V6; each verified against the repo or the scratch rehearsal):
- P1: test count 712 -> 714 everywhere; expected after the new tests is 717 (714 + guard (a) + guard (b) + TUI name test).
- P2: residuals restated: Linux CI has run and is green (ubuntu-latest, macos-latest; runs 39179076283, 39345121170), release workflow succeeded once (run 39345610585); open residuals are aarch64 Linux, Linux skill discovery, AC-44. Overview corrected (v0.1.0 was already released once under the old name).
- P3: `rename_guard.rs` builds its needles at runtime, skips itself; script excludes it.
- P4: script rules: `(?!biguous)` on all three case rules; literal `# Unambiguous` first; mask `unam-technical-spec.md` and `PRD-unambiguous.md` and the `docs/spec/`, `docs/adr/` paths; exclude `START-HERE.md` (skip paths that do not exist), `spike/**`.
- P5: `cargo metadata --format-version 1 --offline`; lock diff expectation corrected; exact normalised-diff check.
- P6: snapshot review yardstick and `--no-fail-fast` for the pre-review pass.
- P7: CHANGELOG instructions corrected (no Unreleased section; replace the line 22 text).
- P8: protected-doc title match is line 1 only.
- P9: context-doc stale-fact refresh (tests 714, CI green, release workflow ran) added to step 12.
- P10: G6 rewritten (script file, plain `|`, allowlist, positive control); G10 rewritten; G15 and G16 added; commit pathspec rule.
- P11: `cargo_bin!` file count corrected (2 files, not 11).
- P12: Handoff: unpushed count (4), orphaned real-home state item 6a, validate-contract status.
- P13: bounded-wait budgets and the hook-safe command list.
- P14: `## Known Gaps (Resolved via Backlog)` section added.

Execute-agent instructions:
- E1: Record `git rev-parse HEAD` and the mtimes of `~/.cache/unam` and `~/.local/share/unam/projects.json` before step 2 (G15 baseline).
- E2: Run step 6 `--report` and read the full distinct-token list before `--apply`; the three rehearsal traps (P4) must show as masked or excluded, not as rewrites.
- E3: Never run `pouch` or `unam` against this repository; never `git add -A`, `git add .` or `git commit -a`.
- E4: Put every command containing a `target`, `dist` or `build` path segment in a scratchpad script; `dist plan`, `dist generate --check` and `cargo check --profile dist` as plain commands are fine. Every wait uses the Bash timeout parameter or `perl -e 'alarm N; exec @ARGV'`; no until-grep loops; macOS has no `timeout`.
- E5: A failure that needs a logic change is a stop, not a fix.
- E6 (UPDATE PROCESS, not EXECUTE): create the backlog stub `process/general-plans/backlog/rename-residual-linux-and-terminal_NOTE_20-10-26.md` named in the Known Gaps section; do not edit the feature backlog note.

Open gaps:
- aarch64 Linux never executed: known-gap: documented as NEW PLAN REQUIRED - see backlog stub in E6 (to be created at UPDATE PROCESS)
- Agent skill discovery on Linux not spike-tested: known-gap: documented as NEW PLAN REQUIRED - see backlog stub in E6
- AC-44 real-terminal TUI run: known-gap: documented as NEW PLAN REQUIRED - see backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md (existing)

What this coverage does NOT prove (What This Coverage Does NOT Prove):
- G3 and the snapshots do not prove the GitHub-side result: the renamed repo, deleted v0.1.0 release and tag, re-tagged pouch v0.1.0, installer URLs resolving, Homebrew tap (all Handoff, Taylor's).
- G8 and G9 prove the names `dist` derives and that the release profile checks; they do not prove a release build, a published installer, or that `release.yml` succeeds under the new repo name.
- G4 proves recovery under injected failures on this machine (macOS); it does not prove Linux or aarch64 behavior.
- G5 and G6 prove no old-name text remains in the scoped trees; they do not prove prose quality, nor that English-word `unambiguous` survivors outside `crates` were all intended (G12 is judgment).
- G15 proves the tests did not touch the real home during the gate runs; it does not prove no future test omits the config env vars (backlog candidate in Test Infra Improvement Notes).
- Nothing proves migration: old `.unam/` state and `~/.config/unam` are ignored by design.
- AC-44 (real terminal picker) is not exercised.

Gate: PASS
Accepted by: not required (Gate is PASS: no FAILs and no unresolved CONCERNs; every CONCERN found was fixed in the plan by P1 to P14). The three named residuals are known-gaps carried through the documented backlog route; Taylor's explicit "ENTER EXECUTE MODE" remains the execution consent.


## Autonomous Goal Block

Written by VALIDATE (BRANCH A: this is a single general plan; the v1-static-activation umbrella and its Stable Program Goal are unrelated and unchanged).

```
SESSION GOAL: Rename Unambiguous / unam to pouch (binary, packages, crates, paths, env vars, state dir, docs, snapshots), no compatibility shim, no logic change
Charter + umbrella plan: N/A - single plan: /Users/databased/Developer/01-AI/unambiguous/process/general-plans/active/rename-to-pouch_20-10-26/rename-to-pouch_PLAN_20-10-26.md
Autonomy: auto-proceed on reversible steps; blocked items go to backlog; subagent delegation stays mandatory (vc-execute-agent edits, vc-tester runs the EVL gates; the orchestrator never edits source or runs gates in its own shell); every spawn uses sonnet (Taylor's standing note, including EXECUTE); see feedback_autonomous_phase_execution.md
Hard stop conditions / safety constraints:
- Never run unam or pouch against this repository; tests use temp dirs and POUCH_CONFIG_DIR, POUCH_CACHE_DIR, POUCH_BUNDLES_DIR only
- Do not push, tag, publish, rename the GitHub repo, touch the old v0.1.0 release, the Homebrew tap, or the local folder (Handoff, Taylor's go-ahead each)
- Never stage or commit the START-HERE.md deletion: explicit pathspecs only, never git add -A, git add . or git commit -a
- Five protected docs get exactly one added line each and nothing else; the skill library skillzone is read-only
- No em dashes or emojis in any file
- Stop (do not improvise) if the baseline is red, Cargo.lock shows any version or checksum change, a snapshot body changes outside the allowed hunks, or a fix needs a logic change
- Every wait bounded: Bash timeout parameter or perl alarm; macOS has no timeout; no until-grep loops
Next phase: EXECUTE: /Users/databased/Developer/01-AI/unambiguous/process/general-plans/active/rename-to-pouch_20-10-26/rename-to-pouch_PLAN_20-10-26.md (start at step 1; steps 2 and 3 are not skippable)
Validate contract: inline in plan, Gate: PASS (generated-by outer-pvl, 2026-10-20)
Execute start: INSTA_UPDATE=no cargo test --workspace --locked (expect 714 baseline, 717 after) | POUCH_FULL_FAULT_MATRIX=1 cargo test --workspace --locked --test ac_briefs then --test ac_recovery | dist plan and dist generate --check | probe: read git diff -- '*.snap' against the step 9 yardstick | high-risk pack: no (public-contract class, gates G1 to G16 are the evidence)
```
