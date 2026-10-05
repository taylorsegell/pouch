---
phase: phase-03-m2-bundles-and-selection
date: 2026-10-05
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_PLAN_04-10-26.md
---

# Milestone M2 phase report: bundles and selection

TL;DR: M2 is VERIFIED (WITH_GAPS). Bundle manifests, the line editor, project root and `selection.yaml` IO, the project index, and resolution with provenance all exist, and gates G1 to G14 plus G12b (`--check-rust`) and E1 are green on the independent EVL run (vc-tester, sonnet). AC-7, AC-8, AC-9, AC-11 are closed; AC-10, AC-12, AC-35 are closed on the desired side only (end-to-end form in M3, status side in M4). Thirteen design departures need Taylor's confirmation (umbrella open items 24 to 36). The M3 entry gate is met.

## What Was Done

- New core modules (`crates/unam-core/src/`):
  - `bundle/{mod,crud,slug,word_table,edit}.rs`: Hermes-compatible loader, CRUD (`save`, `duplicate`, `delete`, `add_refs`, `remove_refs`, `edit_in_editor`, `render_manifest`), exact `slugify` port with the generated `\w` exclusion table, and the hand-written line editor for the `skills:` block.
  - `project/{mod,selection,index}.rs`: `detect_root`, `selection.yaml` load and save, the string-list project index (the M1 provisional reader moved here), reference scanners.
  - `resolve.rs`: `resolve_selection` returning `Resolution { resolved, problems }` with provenance, not fail-fast; thin layer over the M1 lookup.
  - Changes to M1 files: `lib.rs`, `error.rs` (new variants), `paths.rs` (`bundle_file`, `edit_dir`), `source.rs` (calls the moved index reader), `catalog/mod.rs` (`source_rank`, `diag_class` codes), `catalog/lookup.rs` (candidate ordering, `canonical_ref_text`).
- Tools: `crates/unam-core/tools/gen-word-table.py` (modes `--write-rust-ranges`, `--write`, `--check`, `--check-rust`) and `crates/unam-core/tools/rust-word-ranges.txt`. Neither is part of any cargo build.
- CLI (`crates/unam-cli/src/cli/`): new `bundle.rs` with `bundle save|ls|show|edit|duplicate|rm` and `bundle add|rm` (skills); `show` gains the `bundles` array and a `Bundles:` line.
- Tests: new `ac_bundles` (4), `ac_selection` (4), `bundle_cli` (28); `snapshots` grew from 10 to 19; `show_cli` from 7 to 8; static fixtures `bundles/{hermes-only,commented}.yaml` and `library2/**` (alias `hs`); `common/mod.rs` helpers (`dirs_for`, `build_second_library`, `copy_fixture_bundle`, `write_project_index`, `git_marker`, `cmd_in`, `run_in`, `split_lines`).
- Docs: `docs/adapters.md` created (sections "Bundle slugify parity with Hermes", "Bundle loader: differences from Hermes", "Maintenance"). Sweep line: Python 3.14.5, Unicode 16.0.0, rustc 1.99.0; 1495 code points excluded in 250 ranges; Python-only 0; 4672 newer-Unicode code points kept as word characters.
- Pre-Step record persisted (plan P20, closes the M1 Known Gap): `phase-03-m2-pre-step_REF_05-10-26.md` holds the three real unam directory listings (all absent) and G9 compares against it.
- Acceptance criteria:
  - Closed: AC-7, AC-8, AC-9, AC-11.
  - Closed on the desired side only: AC-10 (resolution half; end-to-end needs apply, M3), AC-12 (status provenance side, M4), AC-35 (status `selected` side, M4).
- First test record: `ac_11_removing_one_entry_leaves_every_other_line_byte_identical` FAILED red at step A7 (binary exited 2, "unrecognized subcommand 'bundle'") and passed at G4.
- Changed snapshots, exactly as predicted (G7b to G7d): `snapshots__show_found.snap` (only added line `"bundles": [],`) and `snapshots__usage_bare.snap` (only added `bundle` help lines). Nothing else among the pre-existing snapshots changed.
- No dependency change: `Cargo.lock` and the three `Cargo.toml` files are unchanged (G11). The five kickoff design documents are untouched (G13).

## What Was Skipped/Deferred

- `bundle rename` (PRD BND-6), standalone bundle validation (BND-9, folded into `bundle show` and `bundle edit`), the full "every project referencing a bundle" report (BND-10; only `bundle rm` lists referencing projects), bundle composition and export (BND-11, BND-12): out of M2 scope, recorded in the plan.
- `--project` and `--target` global flags: deferred to M3 (open item 32).
- Commands `use`, `add`, `rm`, `deactivate`, `status`: M3 and M4.
- Pinning, `adopt`, remote sources: product Phase 2.
- Status side of AC-12 and AC-35: M4. Backlog NOTE written: `process/features/v1-static-activation/backlog/m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md`.
- AC-10 end to end (saved bundle applied to an empty project): M3 re-verifies.
- CI still not run (carried from M1); `dist` and `cargo-insta` still not installed.

## Test Gate Outcomes

EVL run by vc-tester (sonnet, 2026-10-05) against the validate-contract commands. Loop log: `results-m2-evl.tsv` (baseline row, then HALTED_SUCCESS at cycle 1, zero gaps).

| Gate | Result |
|---|---|
| G1 fmt, G2 clippy, G3 all tests | PASS (no ignored tests) |
| G4 seven per-AC commands (AC-7 to AC-12, AC-35 desired side) | PASS, 1 passed each |
| G5 M1 regression (`ac_read_path`, `read_side_invariant`, `show_cli`, `source_cli`, `search_cli`, `cache_cli`) | PASS |
| G6 perf, release | PASS, EVL median 140.8 ms (execute run 161.6 ms; limit 300 ms) |
| G7, G7b, G7c, G7d snapshot hygiene and changed set | PASS (19 snapshots, only `show_found` and `usage_bare` changed) |
| G8 text hygiene, G9 real paths untouched | PASS |
| G10 harness validators, G11 no dependency change | PASS |
| G12 `gen-word-table.py --check` | PASS (`py_only=0`) |
| G12b `gen-word-table.py --check-rust` (Hybrid) | PASS |
| G13, G13b, G13c docs, sweep line, no ignored tests | PASS |
| G14 `bundle_commands_leave_sources_untouched` | PASS |
| E1 (no SKIP) | PASS |

Suites at EVL (after M2): unam-core unit 193 (M1: 107); integration `ac_bundles` 4, `ac_selection` 4, `bundle_cli` 28, `snapshots` 19, `show_cli` 8; the other M1 suites unchanged and green (`ac_read_path`, `cache_cli`, `read_side_invariant`, `search_cli`, `source_cli`, `usage`, `perf_cold_scan`).

Plan validation (PVL): Gate PASS 2026-10-05 after 1 validate-fix cycle (5 concerns, all closed) plus a validator in-body fix of the proptest renderer (see Process Incidents). Loop log `results-m2.tsv`; iteration reports `phase-03-m2-bundles-and-selection-pvl-iteration-001` and `002`.

What green proves: every closed AC has a named passing automated test; no M2 command writes under a source tree (G14 and the per-test invariant); the line editor removes exactly one item line and every other line stays byte-identical (AC-11, including CRLF plus BOM, dash at indent 0, first and last item); Hermes-style bundles with bare names load unchanged (AC-7); an ambiguous bare name errors naming every candidate and activates nothing (AC-8); the committed `\w` table equals the generator output and has zero Python-only code points on the generating versions. What it does not prove: behavior on Linux or CI, the end-to-end apply of a saved bundle (M3), `status` output (M4), concurrent manifest edits, or `\w` parity on any Unicode version other than the recorded ones.

## Plan Deviations

Eight in-scope deviations recorded in the plan under "Deviations (EXECUTE 2026-10-05)"; none changes a public contract beyond the plan and none touches a hard-stop class:

1. Bundle CRUD lives in `bundle/crud.rs` (new, about 1130 lines with tests) instead of `bundle/mod.rs`; `mod.rs` re-exports, so the API path is as planned (P1 permits the split).
2. `yaml_scalar` is `pub(crate)` in `bundle/edit.rs`.
3. `EntriesOutcome` carries two extra fields (`removed`, `rewrite`); `edit_in_editor` takes `editor: Option<&str>`.
4. M1 test assertion changed: `show_cli.rs` `frontmatter_order_supporting_and_diagnostics` asserted `data.bundles` absent (M1 P12); P16 makes it always present, so it now asserts `data.bundles == []`. This is a visible edit to an M1-verified test, required by P16.
5. Advisory A3 adopted: the symlink refusal (P21) also applies when `save --force` or `duplicate --force` overwrites an existing, unloaded `<slug>.yaml`.
6. Unspecified details decided: a non-text `name` is `bundle-name-blank`; an unresolved `remove` cancels earlier problems of every kind except `BundleMissing` with the same canonical reference; `bundle ls` cuts the description at 80 characters; loader diagnostics are reported by `bundle ls` only; `bundle edit` reports post-commit problems as warnings.
7. Test harness extras: `run_in`, `split_lines`, a `#[cfg(test)]` `tests_support::dirs_in`, an extra Hybrid unit test `register_does_not_write_an_unreadable_index` (self-skips under root), and the `bundle_edit` snapshot test is `#[cfg(unix)]`.
8. Process only: the execute agent wrote gate output to `/tmp/claude-501/g.out`, outside the repository and outside the scratchpad. Noted; harmless (no repo or user data involved) and not reproduced in later steps.

## Design Departures for Taylor (umbrella open items 24 to 36)

All provisional under autopilot; each needs Taylor's confirmation. None edits a protected design document.

| Item | Departure | Ask |
|---|---|---|
| 24 | AC-12 and AC-35 name `status` (M4). M2 closes the desired side through core-API tests; the status side closes in M4 via two tracked test notes. | Confirm the split closure. |
| 25 | Spec 4.4 `slugify` is an ASCII rule; M2 ports Hermes `slugify_skill_name` exactly (accents and CJK kept, `foo.bar` gives `foobar`), with a generated `\w` table for exact parity. | Confirm, or amend spec 4.4. |
| 26 | `source rm` ignores bundle membership; `bundle rm` lists referencing projects but never blocks; BND-10 report deferred. | Confirm. |
| 27 | Provenance keeps all records (deduplicated, first-seen order); the effective winner is the last record (spec 2.3 versus PRD 5.3). Same-name skills are not flagged at resolution (M3 `plan::diff`, EDG-2). | Confirm the reading. |
| 28 | Bundle scope: rename deferred, validate folded into `show` and `edit`, overwrite flag named `--force`, `bundle rm <name>` deletes the manifest while `bundle rm <name> <ref>...` removes entries (no confirmation prompt, as in Hermes). | Confirm; decide whether rename and a separate validate are wanted, and whether the overloaded `rm` is acceptable. |
| 29 | Load order is `*.yaml` sorted then `*.yml` sorted (Hermes), not one merged alphabetical order (spec 2.2). | Confirm, or amend spec 2.2. |
| 30 | A blank or whitespace-only `name` (including explicit `name: ""`) is an error diagnostic and the bundle is skipped; Hermes falls back to the file stem for an explicit empty string. | Confirm. |
| 31 | M1 candidate ordering changed to (edit distance, source precedence, ID) and (precedence, ID); identical with one source. | Confirm. |
| 32 | `--project` and `--target` not built; M3 declares them with its first use. | Confirm. |
| 33 | `bundle save` takes the root from the working directory and the target from the selection's single target; several targets give exit 2 until M3. | Confirm the interim behavior. |
| 34 | PRD 5.1 and spec 2.2 describe a blank bundle name as raising `ValueError`; in Hermes that exists only in `save_bundle`, while the loader skips with a warning. M2 follows the loader. | Confirm; fix the PRD row when convenient. |
| 35 | `docs/adapters.md` created by M2; spec 10 M7 says it is generated from `doctor --evidence`; the M7 plan must preserve the two M2 sections. | Confirm. |
| 36 | Further Hermes loader differences (null entries dropped, non-scalar entries dropped with warning, booleans `true`/`false`, YAML 1.1 booleans not mirrored, non-UTF-8 manifest skipped with error, no mtime cache, hidden files loaded, `bundle save` writes only `name` and `skills`). Recorded in `docs/adapters.md`. | Confirm. |

Items 15 to 23 (M1) are still open and unchanged.

## Known Gaps

- AC-12 and AC-35 status side: M4 (backlog NOTE written). AC-10 end-to-end form: M3.
- Chmod Hybrid tests (including the new `register_does_not_write_an_unreadable_index`) self-skip when run as root; they prove nothing under root.
- Concurrent manifest edits are last-writer-wins until the M3 lock; there is no cross-process lock on bundle files.
- A corrupt `projects.json` is rebuilt from the current root alone (`project-index-unreadable` warning), so the other registered projects are forgotten until they next register.
- Slugify parity is tied to the generating versions: Python 3.14.5 with Unicode 16.0.0 and rustc 1.99.0 with Unicode 17.0.0. G12 exit 3 flags drift between the running Python and the committed header; a newer Rust Unicode needs `--write-rust-ranges` and `--write`.
- Linux behavior is unverified (CI never run): the `show_found` tree hash and `add_bad_path` OS text snapshots from M1 still apply, and the new `bundle_edit` snapshot depends on `/bin/sh`.
- The `\w` table uses a committed Rust-side data file, so `--check` proves consistency with the committed Rust data, not with the live compiler; only `--check-rust` (needs `rustc`) re-derives it.

## Process Incidents

1. Edit tool disabled for the session (second milestone running). Plan edits used exact single-match python scripts from the scratchpad or full-file Write. The existing feedback memory already covers this.
2. The validator edited a plan body to fix a defect: at PVL cycle 2 vc-validate-agent fixed the proptest renderer text in the plan itself (an in-pass plan edit, recorded as P1 in the validate contract and re-probed) instead of routing back to plan-agent. It was recorded and accepted in the contract; note the boundary: validators normally emit findings and plan-agent applies them.
3. The first PVL pass was CONDITIONAL with 5 concerns plus E1 to E11; one supplement cycle closed them. One concern, the mechanical gate `grep -c 'Gate: PASS'` on the plan file, matches resume and prose lines as well as the verdict line; the anchored form `grep -c '^Gate: PASS'` is the reliable check (recorded as a feedback memory this session).
4. The execute agent wrote gate output to `/tmp/claude-501/g.out` (outside the repo and outside the scratchpad). No repo impact; noted.
5. The scout-block hook friction on `target`, `.git`, `build` text recurred; gate commands avoided the tokens as planned (no `--target` flag exists in M2).

## Test Infra Gaps Found

- No Linux or CI verification yet (carried). Backlog NOTE exists from M1; the first push settles it.
- Scripted `EDITOR` tests need unix `/bin/sh`; a portable editor stub is a candidate if Windows is ever supported.
- `common/mod.rs` keeps growing; split into `common/{env,fixtures,snapshot}.rs` past about 500 lines (candidate for a later milestone).
- A property-test generator for whole bundle files exists only for the line editor; it could later fuzz `resolve_selection`.
- G12 `--check` only needs Python, so a scheduled CI job can watch Unicode drift (exit 3).
- Backlog NOTE written for the one concrete follow-up this session: `m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md`. The other items are recorded here and in the plan's Test Infra Improvement Notes; they stay observational until a milestone needs them.

## SPEC Achievement

No locked `*_SPEC_*.md` exists (phase-program inner loop; the umbrella plan governs). Scored against the plan acceptance criteria:

| Criterion | Score | Proven by |
|---|---|---|
| AC-7 Hermes-only bundle with bare names loads unchanged | met | `ac_07_hermes_only_bundle_with_bare_names_loads_unchanged` (Fully-Automated) |
| AC-8 ambiguous bare name errors naming both IDs; nothing activates | met | `ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates` |
| AC-9 bundle spans two categories and two sources | met | `ac_09_bundle_spans_two_categories_and_two_sources` |
| AC-10 saved bundle in empty project yields original selection | met (desired side); end-to-end form unmet until M3 | `ac_10_saved_bundle_in_empty_project_yields_original_selection` |
| AC-11 removing one entry leaves every other line byte-identical | met | `ac_11_removing_one_entry_leaves_every_other_line_byte_identical` |
| AC-12 bundle plus add plus remove resolves with provenance | met (desired side); status side unmet until M4 | `ac_12_bundle_plus_add_plus_remove_resolves_with_provenance` |
| AC-35 hand-edited selection is desired, not applied | met (desired side); status side unmet until M4 | `ac_35_hand_edited_selection_is_desired_not_applied` |

## SPEC Gaps

Unmet (full-criterion) items, each with a backlog NOTE or a tracked milestone:

- AC-12 status side and AC-35 status side: backlog NOTE `m2-ac12-ac35-status-side-for-m4_NOTE_05-10-26.md` (M4 tests).
- AC-10 end-to-end form: carried in the M3 plan (re-verification at M3); M3 Forward Preview below.

These are planned split closures (D1), not vacuous green: each desired-side criterion has a passing automated test, and the remainder is assigned to a named later milestone.

## Closeout Packet

1. Selected plan path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_PLAN_04-10-26.md`
2. Closeout classification: Ready for UPDATE PROCESS archival, with gaps (EVL classification CLEAN). The task folder stays in `active/` because it holds the whole program; the M2 plan file stays in place, marked VERIFIED (WITH_GAPS). Archival of the program folder waits for program completion.
3. What was finished: see What Was Done.
4. Verified: G1 to G14, G12b, E1 (EVL, vc-tester). Unverified: Linux and CI behavior, concurrent edits, the end-to-end apply of a saved bundle, `status` output.
4b. Validate-contract: present in the plan, Gate PASS 2026-10-05 after 1 supplement cycle plus one in-pass fix.
5. Cleanup done: plan ticked, umbrella state rewritten, registry DONE, context docs updated, backlog NOTE written. Still needed: execution commit and process commit by the orchestrator (execution first), Taylor's decisions on items 24 to 36.
6. Next valid state: milestone M3 (phase 4 of 8), loop step 1 RESEARCH.
7. Commit checkpoint: execution commit recommended before the process commit (M2 has source code); the orchestrator commits on `main`, no push. UPDATE PROCESS did not commit.
8. Regression status: M1 surfaces (AC-1, 3, 4, 5, 6, 38; first test; `perf_cold_scan`) re-run at G5 and G6: PASS; only `show_found` and `usage_bare` snapshots changed as predicted; M0 spike file unchanged.
9. SPEC achievement: see above (no SPEC; seven plan criteria met on the desired side).

Drift score: 5 signals (files touched >= 1 and >= 10 gives 2; 3+ memory-worthy observations; feature-folder structural change via the new report and backlog NOTE; validate-contract deviation, namely the 8 recorded deviations including the M1 `show_cli` assertion change). No `.claude/`, agent, or protocol file was edited. HIGH by count, driven by file volume. Strongly recommend UPDATE PROCESS -- harness/protocol files touched.

## Forward Preview

### Test Infra Found

- Suites and helpers listed under Test Gate Outcomes; harness in `crates/unam-cli/tests/common/mod.rs`; 19 snapshots under `crates/unam-cli/tests/snapshots/`.
- Gates G12 (`python3 crates/unam-core/tools/gen-word-table.py --check`), G12b (`--check-rust`, needs `rustc`), G13b (sweep-numbers line in `docs/adapters.md`), G13c (no `#[ignore` under `crates`), G14 (`bundle_commands_leave_sources_untouched`).
- Scripted `EDITOR` pattern: a `#!/bin/sh` file written inside the `TestEnv` tempdir, passed through the `EDITOR` variable of the injected env (unix only, `#[cfg(unix)]`); no test starts a real editor.
- In-process tests build `Dirs` from an injected map (`common::dirs_for`) and `detect_root` tests pass a `ceiling`; every `bundle save` CLI test plants `git_marker` first.

### Blast Radius Changes

- Created versus the registry entry: `bundle/crud.rs` (split from `mod.rs`), the generator and data file under `crates/unam-core/tools/`, `phase-03-m2-pre-step_REF_05-10-26.md`. Registry M2 entry marked DONE.
- Modified M1 test file beyond the plan: `crates/unam-cli/tests/show_cli.rs` (assertion change, deviation 4).
- `fsops.rs`, `hash.rs`, `config.rs`, `catalog/{scan,cache,frontmatter,search}.rs`, `output.rs`, `main.rs` untouched.
- M3 owns `adapter/{mod,claude_code}.rs`, `fsops.rs` (typed paths), `plan.rs`, `apply.rs`, `state.rs`, CLI `add|rm|use|deactivate`, `output.rs` plan rendering, and extends `project/`, `resolve.rs`.

### Commands to Stay Green

```
export PATH="$HOME/.cargo/bin:$PATH"; export INSTA_UPDATE=no
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 crates/unam-core/tools/gen-word-table.py --check
node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs
node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs
node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs
node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <plan>
git diff --check
```

Also re-run `perf_cold_scan` (release, median of five) when M3 touches catalog code; the 300 ms limit stays.

### Dependency Changes

- None in M2 (`Cargo.lock` and all `Cargo.toml` files unchanged, G11). M3 adds `fs4` (planned) for the lock; that is a new crate and the first dependency change since M1.

### M3 Entry Notes (what M3 consumes and must finish)

- Scope per the 2026-10-04 decisions: adapter trait and `claude_code`; `fsops` typed paths; `plan::diff`; apply with journal, receipt, history, and lock; journal-based `resume` and `rollback`; `add|rm|use|deactivate`.
- Consumes the M2 `Resolution { resolved, problems }` and `DesiredEntry` (with `winner()`); `Provenance` serialization (`Bundle(slug)`, `Adhoc`) is provisional and may be fixed by M3. `Pin` is `Follow` only.
- `save_selection` already registers the root in the project index on every write; M3's `use` and `add` need the gitignore lines for `.unam/` state files.
- `--project` and `--target` flags arrive in M3 (open item 32); `detect_root(cwd, ceiling)` backs `--project`; once `--target` exists, `bundle save` should accept it (open item 33).
- `plan::diff` must add the EDG-2 same-name collision check that resolution deliberately does not do (open item 27).
- AC-10 end-to-end: apply a saved bundle to an empty project and compare to the original selection. AC-2 (source tree unchanged through a full cycle) closes in M3.
- Required M3 checks: S2 (`s2_one_command_add`), S3, S4 (invariants in every mutating test), S9 for claude-code (`adapter_claims_have_evidence`); interruption test proves consistency by comparing destination and receipt to the expected state (M4 adds the reconcile zero-divergence assertion).
- The concurrent-manifest last-writer-wins gap closes only for receipts and journals under the M3 lock; bundle manifests stay unlocked unless M3 decides otherwise.
- First adopting project (brief) is still unnamed and is due at the end of M3.
- Safety (unchanged): never run `unam` against this repository; never invoke real agent CLIs; tests use `TestEnv` only; Edit tool may again be disabled, so give agents the python single-match fallback up front.

### M4 Tracked Test Notes

Carried in the backlog NOTE: (a) extend `ac_12_*` so `unam status` prints per-entry provenance (bundle slug or ad hoc); (b) extend `ac_35_*` so `unam status` on a hand-edited selection with no receipt prints `selected`, never `active`.
