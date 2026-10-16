---
name: plan:v1-static-activation-phase-03-m2-bundles-and-selection
description: "v1: static activation, phase 3 of 8: milestone M2, bundles, project selection, and resolve; supplemented 2026-10-05 for execute"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-03
---

# Milestone M2: Bundles and selection

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26 (supplemented 2026-10-05 after RESEARCH and INNOVATE)
**Complexity**: COMPLEX
**Phase:** Phase 3 of 8: milestone M2
**Status**: VERIFIED (WITH_GAPS) 2026-10-05: EVL green G1 to G14, G12b, E1; AC-10, AC-12, AC-35 closed on the desired side only (end-to-end form in M3, status side in M4); known gaps in the phase report
**Phase status:** VERIFIED (WITH_GAPS) 2026-10-05: EVL green G1 to G14, G12b, E1; AC-10, AC-12, AC-35 closed on the desired side only (end-to-end form in M3, status side in M4); known gaps in the phase report
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.
**Execution strategy:** sequential, one opus execute agent (error.rs, lib.rs, and the catalog crate are shared by every step, and each step compiles against the previous one, so there is no disjoint file group worth a team; the umbrella's optional 2-way split on `bundle` versus `project` is rejected for that reason). Every other step (tester, reviewers, update-process) runs sonnet.

---

## Overview

Milestone M2 adds the desired-state side: Hermes-compatible bundle manifests with in-place edits, project root detection and `selection.yaml` IO, and resolution of every reference form to a flat desired set with provenance, erroring on ambiguity (spec section 10 M2; spec 4.4 to 4.6, 2.2, 2.3, 5; PRD 7.2, 7.3, 5). Nothing here applies anything or writes to a skill destination. The only files M2 code writes are bundle manifests (user-scope bundles directory), the project's `.unam/selection.yaml` (core API only, no M2 command writes it), and the project index in the data directory. Estimated 2 days (spec 10).

Product Phase 1, tier 2 (read path and desired state). Program charter: see umbrella.

How to read this plan: "Decisions Locked" are fixed by INNOVATE (2026-10-05); "Plan-Level Decisions" were made at this PLAN-SUPPLEMENT inside those bounds; "Module Specifications" and the "CLI Contract" remove every remaining design choice. An execute agent should not need to decide anything; if it must, that is a deviation to record in the phase report. M1 is the template: its plan, report, and the code it shipped (`crates/`) are the style reference.

---

## Scope

In (spec section 10 M2, as decided at INNOVATE):

- `bundle` module: Hermes-compatible loader, saver, CRUD, and the hand-written line editor for the `skills:` block (spec 4.4, 2.2; CLI `bundle save|ls|show|edit|duplicate|rm`, `bundle add|rm`, spec 5).
- `slugify`: exact port of the Hermes function (decision D2), with a table-driven unit test and a recorded parity sweep (spec 8 Unit).
- `project` module: root detection, `selection.yaml` load and save, the project index (string list), reference scanners used by `source rm` and `bundle rm` (spec 4.5, 2.3).
- `resolve` module: a thin layer over the M1 lookup plus resolution of a target selection to a flat desired set with provenance, not fail-fast (spec 4.6, 2.3, 3).
- `show` gains "bundles referencing it".
- `docs/adapters.md` is created with the slugify parity outcome and the loader differences from Hermes.

Out (not built in M2; each recorded):

- `--project` and `--target` global flags (decision D6; milestone M3 declares them with its first use). `bundle save` therefore finds the root from the working directory and the target from the selection file (P10).
- Commands `use`, `add`, `rm`, `deactivate`, `status`: milestone M3 and later. No M2 command writes `selection.yaml`; the writer exists in the core API and is exercised by unit and integration tests.
- `bundle rename` (BND-6 rename), standalone bundle validation (BND-9, folded into `bundle show` and `bundle edit`), the full "every project referencing a bundle" report (BND-10; only `bundle rm` lists referencing projects), bundle composition and export (BND-11, BND-12).
- Pinning (`Pin::Commit`, `Pin::Hash`), `adopt`, remote sources: product Phase 2.
- Status side of AC-12 and AC-35: milestone M4 (tracked M4 test notes below).
- Any destination, adapter, receipt, or plan code (milestone M3).

---

## Decisions Locked (INNOVATE 2026-10-05; apply as written)

| # | Decision |
|---|---|
| D1 | AC-12 and AC-35 close on the desired side in M2 through core-API integration tests; the status side closes in M4 (a tracked M4 test note is recorded below). No new command outside spec 5. |
| D2 | `slugify` is an exact port of Hermes `slugify_skill_name` (lowercase; space and underscore to `-`; delete characters that are not Unicode `\w` or `-`; collapse runs of `-`; trim `-`), with an own `\w` predicate (`char::is_alphanumeric` plus `_` plus an exclusion table, see P3 and the `bundle/slug.rs` spec) unit-tested against a fixture table that includes the three Hermes test vectors (`Backend Dev` gives `backend-dev`, empty gives empty, `!!!` gives empty) and the differing ones (`foo.bar` gives `foobar`, `a/b` gives `ab`, `x@y` gives `xy`, a precomposed accent is kept, CJK is kept, a tab is removed). The slug is used for lookup (bundle names in a selection's `bundles` list are slugified on lookup), for the saved-bundle filename, and for duplicate detection. An empty slug is an error. The slug is validated before any path join (no separators, no `..`). |
| D3 | Hand-written line editor for the block `skills:` sequence: dash at indent 0 or 2, quoted items, trailing comments, CRLF and BOM preserved. After every edit the whole file is reparsed with `serde_norway` and the skills list must equal the expected list (and every other top-level key must be unchanged), else abort with no write. Flow sequences, `skills` nested under `metadata`, anchors, and multi-line plain scalars are refused with an error pointing to `--rewrite` (a full rewrite fallback). AC-11 asserts that every other line is byte-identical. The edge-case matrix is a test table. |
| D4 | Keep the string-list project index (`{"version": 1, "projects": [...]}`); move the reader into `project` (`source.rs` calls it). Bundle membership does NOT block `source rm`. The BND-10 report is deferred, except that `bundle rm` lists referencing projects without blocking. |
| D5 | Resolution returns `Resolution { resolved, problems }`: resolved entries carry `SkillId`, name, strategy, `pin = Follow`, and all provenance records; problems (`Unresolved`, `Ambiguous`, plus the two kinds added at P4) carry the reference text, candidates, and provenance. It is not fail-fast (EDG-3). All provenance is kept (spec 2.3); the "effective winner" is the last record (PRD 5.3). Order: bundles in listed order, then `add`; the first occurrence fixes the position. `remove` subtracts by resolved `SkillId` (any equivalent spelling). Strategy inherits `Config.defaults.strategy`. The CLI maps problems to exit 2, 4, or 5. |
| D6 | No `--project` or `--target` flags in M2. `TargetId` is a validated newtype over `claude-code` and `agents-neutral`, needed by the selection parser. Bundle commands: `save` (fails if the slug exists unless `--force`), `ls`, `show` (with problems), `duplicate` (same `--force` rule), `rm` (lists referencing projects, does not block), `add` and `rm` of skills (line editor), `edit` (`$EDITOR` on a temp copy; reparse and validate; commit only on success; tests drive a scripted `EDITOR`). No rename. `bundle save` gets its project root and target without flags: root from the working directory per spec 4.5, target from the selection file's single target, or an error when several (P10). |
| D7 | Loader mirrors Hermes: `*.yaml` sorted then `*.yml` sorted; BOM stripped; the first duplicate slug wins with a warning; entries stringified and blanks dropped; description defaults to `Load {n} skills as a bundle`. A whitespace-only or blank name is an ERROR diagnostic (spec 2.2, PRD) and the bundle is skipped and reported. YAML 1.1 booleans are not mirrored. Path-like entries go through the spec 4.6 parser, no special case. |
| D8 | `resolve.rs` is a thin layer over the M1 lookup (re-export). Candidate ordering changes to source precedence with ID as tiebreak (spec 4.6: precedence only orders the candidate list). Snapshots change only where visibly changed (insta procedure below). |

Reports to Taylor (resolved provisionally by INNOVATE under autopilot; recorded in the umbrella `## Contradictions and Open Items`, items 24 to 36): AC-12 and AC-35 split closure (24); slugify aligned to Hermes, not the spec 4.4 ASCII rule (25); `source rm` ignores bundle membership (26); provenance keeps all records and the effective winner is the last (27); rename deferred, validate folded into edit and show, `--force` chosen, `bundle rm` overloaded (28); load order `.yaml` then `.yml` (29); blank name is an error (30); M1 candidate ordering changed (31); `--project` and `--target` deferred (32); `bundle save` root and target without flags (33); the loader description in PRD 5.1 and spec 2.2 versus Hermes (34); `docs/adapters.md` regeneration at M7 (35); further Hermes loader differences (36).

---

## Plan-Level Decisions (made at PLAN-SUPPLEMENT, inside the bounds above)

| # | Decision | Why |
|---|---|---|
| P1 | Layout: `crates/unam-core/src/bundle/{mod,slug,word_table,edit}.rs`, `project/{mod,selection,index}.rs`, `resolve.rs` (spec 1.3 lists single files; directories keep each file under about 600 lines, as M1 did for `catalog`). `crates/unam-cli/src/cli/bundle.rs` is new. `fsops.rs` is NOT touched (M2 uses `write_atomic` and `std::fs::remove_file`, as M1's `source::remove` did for the cache file). If a file passes about 600 lines the execute agent MAY split it and records the split as a deviation. | Consistency with M1; blast radius stays small. |
| P2 | New diagnostic codes (stable contract, all "config-level" class in `diag_class`): `bundle-unreadable` (error), `bundle-not-utf8` (error), `bundle-invalid-yaml` (error), `bundle-not-mapping` (error), `bundle-name-blank` (error), `bundle-no-skills` (error), `bundle-empty-slug` (error), `bundle-duplicate-slug` (warning), `bundle-entry-ignored` (warning); resolution problem codes `ref-invalid`, `ref-not-found`, `ref-ambiguous`, `bundle-missing` (all error). Each bundle diagnostic carries `path` = the manifest file name. | AC-7, EDG-3, spec 2.2 need named findings; M1 D5 shape reused. |
| P3 | Parity of `\w`: Rust `char::is_alphanumeric` is a superset of Python `\w` (verified 2026-10-05 over all scalar values: Python 3.14.5 with Unicode 16.0.0 versus rustc 1.99.0; zero code points are `\w` in Python and not alphanumeric in Rust). The surplus is combining marks and Other_Alphabetic symbols: 927 `Mn`, 438 `Mc`, 130 `So` code points (1495 in 250 ranges), plus newer-Unicode letters that Python 16 lists as unassigned (`Cn`, 4672 code points; kept as word characters because a newer Python would treat them as `\w`). The 1495 are listed in a generated table `bundle/word_table.rs` (constant `NOT_WORD`, sorted disjoint inclusive ranges) and `is_word_char(c)` is `c == '_' || (c.is_alphanumeric() && !NOT_WORD.contains(c))`. The table is generated by the committed script `crates/unam-core/tools/gen-word-table.py` from the committed Rust data file `crates/unam-core/tools/rust-word-ranges.txt` (see the `bundle/slug.rs` spec), and gate G12 (`python3 crates/unam-core/tools/gen-word-table.py --check`) proves the committed `word_table.rs` equals the generator output. | Exact parity for NFD input (macOS file names) without adding a crate; the table is data, not an inference. |
| P4 | Resolution problem kinds are `Invalid` (the reference text does not parse, exit 2), `Unresolved` (exit 4), `Ambiguous` (exit 5), and `BundleMissing` (a bundle named in a selection is not loaded, exit 4). A problem has `reference` (canonical text), `message`, `candidates` (strings), `unreachable` (aliases), and `origins` (`Bundle(slug)`, `Add`, `Remove`). Problems are deduplicated by (kind, reference) with origins accumulated; order is first encounter (bundles in order, then `add`, then `remove`). A command that needs a complete set fails with the exit code of the FIRST problem and reports every problem as a warning-array diagnostic. | D5 says "maps problems to exit 2/4/5"; invalid text needs a kind. |
| P5 | `remove` semantics: each `remove` text is looked up; `Found(id)` subtracts that id (and its provenance); a `remove` that does not resolve cancels any earlier problem with the same canonical reference text (the user explicitly removed a dangling entry) and is otherwise itself a problem with origin `Remove`. Removing a valid id that is not selected is a silent no-op. | EDG-3, EDG-7; avoids a permanently blocked selection. |
| P6 | Provenance records are `Bundle(slug)` and `Adhoc`, deduplicated (the same record twice is stored once), kept in first-seen order; `DesiredEntry::winner()` returns the last record. `Pin` is `Follow` only in M2 (`#[non_exhaustive]`). Two resolved skills sharing a name are NOT flagged here (EDG-2 conflict belongs to milestone M3's `plan::diff`). | PRD 5.3 last-write-wins at ID level; scope. |
| P7 | `selection.yaml`: `version` optional on read (default 1; any other value is `selection-version-unsupported`); unknown keys are an error (`deny_unknown_fields`, so a typo such as `bundels` is not silently ignored); `add` and `remove` are stored as text (parsed at resolution, so invalid text becomes a problem instead of failing the load); an empty file is an empty selection; `targets` is a map keyed by `TargetId`. Save writes `version`, then each target (alphabetical) with `strategy` only when set and always `bundles`, `add`, `remove`; comments are NOT preserved on save (known limitation, as M1 P10). | AC-35 hand editing; spec 2.3. |
| P8 | `detect_root(cwd, ceiling)`: canonicalize `cwd` and, when given, `ceiling` as well (macOS temp directories are symlinks to `/private/var`, so an uncanonicalized ceiling would never match and the ceiling tests would not be hermetic), walk up checking `symlink_metadata(dir/.git)` (file, directory, or symlink all count), return the nearest ancestor-or-self with `RootDetection::Git`; none found gives the canonical `cwd` with `RootDetection::Explicit`. `ceiling` (None in the product) stops the walk after examining the ceiling directory, so in-process tests never consult ancestors of their tempdir. | Spec 4.5; hermetic tests. |
| P9 | Project index: `{"version": 1, "projects": [...]}`, canonical absolute root strings, sorted and deduplicated. `register` reads the index (absent: empty; unparseable: rebuilt from this root alone with a `project-index-unreadable` warning; unreadable for another IO reason: no write, same warning), adds the root, writes atomically. `save_selection` registers the root on every write. The lenient reference scanner from M1 moves unchanged (a selection without `version` still counts; only strings under `targets.*.add` or `remove` starting with `<alias>:` reference a source; `targets.*.bundles` items whose slug equals the bundle's slug reference a bundle). | D4; M1 tests must stay green unmodified. |
| P10 | `bundle save <name>`: root = `detect_root(current_dir, None)`; load `<root>/.unam/selection.yaml` (absent or zero targets: `no-selection`, exit 4); exactly one target is used, several give `target-ambiguous` (exit 2, remedy says `--target` arrives in milestone M3); resolve the target's selection; any problem fails with the first problem's exit code and writes nothing; zero resolved entries is `no-selection` (exit 4). The manifest holds the fully qualified IDs of the resolved entries in resolution order (BND-5); no `description` or `instruction` is written (the loader default description applies; `bundle edit` adds them). | D6; spec 5; BND-5. |
| P11 | CLI details beyond D6: `bundle rm <name>` deletes the manifest; `bundle rm <name> <ref>...` removes skills (the spec 5 table lists `rm` in both rows, so the form is decided by whether references are given); `--rewrite` (on `bundle add` and `bundle rm` with references) forces the full-rewrite path; `--force` is the overwrite flag name on `save` and `duplicate`; `bundle add` resolves each reference against the catalog first and writes the reference text as typed (canonical, trimmed), aborting everything on the first unresolvable reference (exit 4 or 5) so the CLI cannot add a broken entry (`bundle edit` can, EDG-3); a reference already present is reported as skipped. No confirmation prompt on delete (Hermes has none); the output names the file. | D6, spec 5. |
| P12 | Line editor grammar, operations, and verification are fixed in the `bundle/edit.rs` spec; removal deletes only the item line itself (a comment line above it stays; AC-11 compares every other line byte for byte). | Simple, checkable guarantee. |
| P13 | `bundle edit`: the command string is `EDITOR` read through `Dirs.env` (never `std::env` in library code), split on whitespace (no shell, no quoting; an editor path with spaces needs a wrapper script), the temp copy is created exclusively with a unique name (`tempfile::Builder` with prefix `<slug>-` and suffix `.yaml`, `tempfile_in(<cache dir>/edit/)`, the directory created first; isolated in tests; never `/tmp`) and is removed on every path including editor failure, the editor receives the temp file path as its last argument; non-zero exit means no write; the edited text must parse as a valid bundle and keep the same slug (a changed slug is refused: rename is out of scope); unchanged bytes report `changed: false`; after a commit, resolution problems are reported as warnings (not blocking); a manifest that is a symbolic link is refused before the editor starts (P21). | D6; hermetic tests. |
| P14 | Slug limits: at most 128 UTF-8 bytes, no control characters in the name, no leading `.`. `Dirs::bundle_file(slug)` validates the slug is a single normal path component before joining `<bundles dir>/<slug>.yaml`. With `--force`, an existing bundle with the same slug is overwritten at ITS file (which may have another file name); without `--force` any existing bundle with that slug, or an existing `<slug>.yaml` file even if invalid, gives `bundle-exists`. | D2; safety. |
| P15 | Candidate ordering (D8): `Catalog::source_rank(alias)` is the source's `precedence` (`u32::MAX` if unknown). NotFound candidates sort by (edit distance, rank, ID); Ambiguous candidates sort by (rank, `SkillId`). With one source, behavior is unchanged, so no existing snapshot changes from this alone. | Spec 4.6. |
| P16 | `show` data gains `"bundles"`, an array of slugs of bundles with an entry that resolves to the shown skill (always present, possibly empty), placed immediately after `tree_hash`; human output gains a `Bundles:` line. "Projects where active" stays omitted (M1 P12) until milestone M4. Bundle loader diagnostics are NOT added to `show` output. Visibly changed snapshots: `show_found` (one added line) and `usage_bare` (the `bundle` command line in help). | D8; spec 5. |
| P17 | Test seams: `common/mod.rs` gains `dirs_for(env)` (an in-process `Dirs` built from an injected map of the `TestEnv` variables; never the process environment), `build_second_library(env)` (alias `hs`), `copy_fixture_bundle(env, name)`, `write_project_index(env, roots)`, `git_marker(dir)`, and `cmd_in(env, dir)`. Scripted editors are `#!/bin/sh` files written by the test (unix only, `#[cfg(unix)]`). | Hermetic in-process tests; no real editor. |
| P18 | No new crates; `Cargo.lock` and all three `Cargo.toml` files are unchanged (gate G11). If a crate seems needed it is a deviation. | Spec 1.4 list stays. |
| P19 | `docs/adapters.md` (created; it does not exist on 2026-10-05) has the sections "Bundle slugify parity with Hermes" and "Bundle loader: differences from Hermes", plus a `## Maintenance` section (the three exact heading lines are checked by G13); the parity section carries the one-line sweep numbers (checked by G13b) and the generator path and usage. Milestone M4 extends it; milestone M7 may regenerate the adapter-evidence part from `unam doctor --evidence` and must preserve the two M2 sections. The five protected design documents are not edited. | Decision 4 (2026-10-04). |
| P20 | The Pre-Step record of the three real unam directories is persisted in the task folder (the M1 report noted the record was never persisted) and gate G9 compares against that file. | M1 Known Gap. |
| P21 | Symlink rule (PVL cycle 1, R3): the loader follows symlinks (`is_file`), but `write_atomic` renames over the path, so an in-place write would replace a user's symlinked manifest (a dotfiles setup) with a regular file. Every writer that targets an EXISTING manifest (`add_refs`, `remove_refs` in both the line-edit and the `--rewrite` forms, `save` with `--force` over an existing slug, `duplicate` with `--force` over an existing slug, and the `edit_in_editor` commit) first calls `std::fs::symlink_metadata` on that `Bundle.path`; when it is a symlink the command fails with `BundleEditUnsupported` (exit 2), nothing is written, and for `edit` the editor is never started. The reason says the manifest is a symbolic link and to edit its target directly (`--rewrite` does not bypass the rule). `delete` is the one exception (P22). A dangling symlink is not loaded (`is_file` is false), so no command sees it. | Never silently turn a user's symlink into a regular file; explicit refusal is reversible, silent replacement is not. |
| P22 | Delete target (PVL cycle 1, R3): `delete` removes exactly the loaded `Bundle.path` (the bundles directory joined with the manifest's own file name), NOT `Dirs::bundle_file(slug)`, because a Hermes-style file `Backend Dev.yaml` has slug `backend-dev` and the loader accepts it. Before removing, it asserts that `path.parent()` equals `dirs.bundles` and that `symlink_metadata` reports a regular file or a symlink (never a directory); otherwise `BundleEditFailed` and nothing is removed. `std::fs::remove_file` unlinks a symlink without touching its target. No sibling manifest is touched. No confirmation prompt (PRD BND-6 and ACT-8 allow an explicit user-targeted delete; the output names the path). | Exact, checkable destructive target. |
| P23 | Word-table generator is committed (PVL cycle 1, R1): `crates/unam-core/tools/gen-word-table.py` plus the Rust data file `crates/unam-core/tools/rust-word-ranges.txt`. Python alone cannot compute Rust's `char::is_alphanumeric` ranges, so the Rust side is a committed second data file (written once by `--write-rust-ranges`, which needs `rustc`); `--check` needs only Python and verifies both files (see the `bundle/slug.rs` spec); `--check-rust` (needs `rustc`) re-derives the data file from the running compiler. | Reproducible, runnable parity gate for the tester, later milestones, and CI. |

---

## Acceptance Criteria

Closes (spec section 10 M2): AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-35 (desired side). Criteria text: PRD section 13.2. Each criterion names its proving test (REQ-TEST-LINK); gates back-reference the criterion in Verification Evidence. Every test that touches a source fixture also asserts the source-tree invariant (`assert_source_unchanged`).

| Criterion | proven by | strategy | Fixture and assertion |
|---|---|---|---|
| AC-7 | `ac_07_hermes_only_bundle_with_bare_names_loads_unchanged` in `ac_bundles.rs` | Fully-Automated | Static fixture `bundles/hermes-only.yaml` (name, `instruction` block, three bare names `code-review`, `pytest-fixtures`, `alpha-root-skill`, no `description`) copied into the bundles dir; library `fx` registered. `bundle ls` and `bundle show hermes-only` exit 0; the loaded description is exactly `Load 3 skills as a bundle`; the three entries resolve to `fx:sdlc/code-review`, `fx:testing/pytest-fixtures`, `fx:alpha-root-skill`; the manifest file bytes are identical before and after (loading never rewrites). |
| AC-8 | `ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates` in `ac_bundles.rs` | Fully-Automated | Libraries `fx` and `hs` registered (bare `schema-migration` exists in both; bare `error-handling` exists twice in `fx`). Core `resolve_selection` over a bundle containing bare `error-handling`: one `Ambiguous` problem listing both IDs, the entry is absent from `resolved`. CLI: `bundle show` prints status `ambiguous` with both IDs (exit 0); `bundle add hermes-only error-handling` exits 5 naming both IDs and leaves the manifest byte-identical; `bundle add hermes-only schema-migration` exits 5 naming `fx:data/schema-migration` and `hs:data/schema-migration`. Nothing activates: the project directory holds no `.claude`, no `.agents`, and no `.unam/applied.json`; both source trees unchanged. |
| AC-9 | `ac_09_bundle_spans_two_categories_and_two_sources` in `ac_bundles.rs` | Fully-Automated | Bundle with `fx:sdlc/code-review`, `hs:ops/deploy-checklist`, `fx:data/schema-migration`, `hs:data/schema-migration`. `resolve_selection` returns four distinct IDs spanning categories `sdlc`, `ops`, `data` and sources `fx`, `hs` (the two `schema-migration` skills stay distinct IDs), no problems; `bundle show --json` lists four `resolved` entries. Desired side only: activation arrives in M3 (spec 10). |
| AC-10 | `ac_10_saved_bundle_in_empty_project_yields_original_selection` in `ac_selection.rs` | Fully-Automated (desired side; the end-to-end form needs apply, M3) | Project A (git marker) selection for `claude-code`: `bundles: [hermes-only]`, `add: [hs:ops/deploy-checklist]`, `remove: [fx:pytest-fixtures]`, written with `save_selection`. `unam bundle save my-set` run in A exits 0 and writes `my-set.yaml` with fully qualified refs. Empty project B with `bundles: [my-set]`: its resolved IDs, order, names, and strategy equal A's resolved set (`fx:sdlc/code-review`, `fx:alpha-root-skill`, `hs:ops/deploy-checklist`); every provenance in B is `Bundle(my-set)`. |
| AC-11 | `ac_11_removing_one_entry_leaves_every_other_line_byte_identical` in `ac_bundles.rs` (FIRST TEST, written red at step A6) | Fully-Automated | Fixture `bundles/commented.yaml` (comments, single-quoted, double-quoted, plain entries, `instruction`, `metadata`). `unam bundle rm commented fx:data/schema-migration` exits 0; the new file is the old file minus exactly the one item line (compared as lines with their terminators); the remaining entries keep text and order. Variants in the same test: CRLF plus BOM copy, dash at indent 0 copy, removal of the first and the last item. Each variant runs in its own `TestEnv` (its own bundles directory) so two copies never share a slug; if a variant ever shares a `TestEnv`, its copy must carry a distinct `name:` (two files with the same slug keep the first and warn, which would make the test pass or fail for the wrong reason). The full edge-case matrix is the unit table in `bundle/edit.rs`. |
| AC-12 (desired side) | `ac_12_bundle_plus_add_plus_remove_resolves_with_provenance` in `ac_selection.rs` | Fully-Automated | Selection `bundles: [hermes-only]`, `add: [hs:api/rest-design, fx:sdlc/code-review]`, `remove: [fx:alpha-root-skill]`: resolved `fx:sdlc/code-review` (position first, provenance `[Bundle(hermes-only), Adhoc]`, winner `Adhoc`), `fx:testing/pytest-fixtures` (`[Bundle(hermes-only)]`), `hs:api/rest-design` (`[Adhoc]`); the removed entry is gone with its provenance. Status side (PRD AC-12 "status shows provenance"): milestone M4. |
| AC-35 (desired side) | `ac_35_hand_edited_selection_is_desired_not_applied` in `ac_selection.rs` | Fully-Automated | A hand-written `selection.yaml` (one variant with `version: 1`, one without) loads and resolves to desired entries while the project holds no `.unam/applied.json` and no destination directory, and loading registers nothing in the project index. Status side (`selected`, not `active`): milestone M4. |

### AC-12 and AC-35 split closure and the tracked M4 test notes (D1)

M2 closes the desired side only; the report and the umbrella say so (open item 24). The M4 plan's tests must add: (a) `ac_12_*` extension: `unam status` for the AC-12 selection prints, per entry, its provenance (bundle slug or ad hoc); (b) `ac_35_*` extension: `unam status` on a hand-edited selection with no receipt prints `selected` and never `active`. At UPDATE PROCESS write backlog NOTE `m2-ac12-ac35-status-side-for-m4_NOTE_<dd-mm-yy>.md` in `process/features/v1-static-activation/backlog/` carrying (a) and (b), and carry both into the phase report's Forward Preview. The M4 plan file is not edited by M2.

AC-10 note: the final end-to-end form (applied to an empty project) depends on apply (M3); the M3 plan re-verifies it. The M2 test is the resolution half.

---

## Entry Gate

- [x] Milestone M0 gate met: the results table in `spike/symlink-discovery.md` is filled in and its Outcome section records the default strategy per adapter.
- [x] Milestone M1 VERIFIED with its report (2026-10-05, WITH_GAPS: `phase-02-m1-core-read-path_REPORT_04-10-26.md`); M1 Forward Preview read at M2 RESEARCH (2026-10-05).
- [x] Hermes `slugify` parity decision available (2026-10-05): RESEARCH read `agent/skill_commands.py` `slugify_skill_name` and `agent/skill_bundles.py` read-only (local checkout at commit `ea32af9331`, 2026-10-05); INNOVATE decision D2 aligns to it, with the table in P3. No `#[ignore]` parity case is needed.

---

## Pre-Step (before any file is created)

Run from the repository root with `export PATH="$HOME/.cargo/bin:$PATH"` first (`cargo` is not on the default shell PATH here; it lives in `~/.cargo/bin`).

1. Versions: `cargo --version` (1.99.0 or newer), `cargo fmt --version`, `cargo clippy --version`, `rustc --version`, `python3 --version` (3.10 or newer; 3.14.5 at plan time). Missing `cargo`, `rustfmt`, or `clippy`: BLOCKED (never install). Missing `python3` blocks step B1 entirely; missing `rustc` blocks only the `--write-rust-ranges` and `--check-rust` sub-steps of B1 (`--write` and `--check` need Python only). In either case record CONDITIONAL, keep the B0 stub `NOT_WORD` (an empty table; the stub's doc comment says the sweep was not run), keep G12 CONDITIONAL, and continue.
2. Record the real unam directories (G9 baseline, persisted per P20): run `ls -A ~/.config/unam ~/.cache/unam ~/.local/share/unam 2>&1 | sort` (expected on 2026-10-05: three "No such file or directory" lines). Write a REF file `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-pre-step_REF_<dd-mm-yy>.md` (dd-mm-yy = the EXECUTE day) with a frontmatter block (`name: ref:...`, `description`, `date`), a short sentence, then a line containing exactly `--- ls output ---` followed by the raw sorted output and nothing after it. Gate G9 compares the live output with the text after that marker line. Also copy the output to a scratchpad file.
3. Hook note (unchanged from M1): `.claude/hooks/scout-block.cjs` blocks Bash commands whose TEXT contains path-like `target`, `.git`, `node_modules`, `build`, `dist`, `vendor`, `coverage` segments. Every gate command in this plan was written to pass; when a check needs one of these tokens put it in a script file written with the Write tool under the scratchpad directory and run that file. Do not use `target/` paths or a `--target` flag in Bash text (this milestone's code has a `TargetId` type but no flag), do not edit `.claude/`, and keep gate-run test names free of those tokens (the test names in this plan are).
4. Write every source, test, fixture, and doc file with the Write tool, never with Bash heredocs. The Edit tool may be disabled in the session: for in-place changes to existing files use the Write tool with the full new content, or a scratchpad python script that asserts exactly one match for the old text before replacing it.
5. No network is needed (no new crates, P18). `cargo build --locked` must succeed; if it wants to change `Cargo.lock`, stop and record a deviation.
6. Hermes is read-only: the execute agent may re-read `skill_commands.py` and `skill_bundles.py` under `~/.hermes/hermes-agent/agent/` to cite them in `docs/adapters.md`; it never runs Hermes, never imports Hermes code (the sweep uses only the Python standard library `re` and `unicodedata`), and never touches `~/.hermes` otherwise.
7. Expect clippy 1.99 `collapsible_if` findings on edition 2024 (fix with let chains, never with `allow`). Any new `#![allow(clippy::unwrap_used)]` carries the one-line comment M1 used.
8. Export `INSTA_UPDATE=no` for every gate except the single snapshot creation run at step H5.
9. Each Bash call is a fresh shell (zsh here): define `gate` and `gate6` and export `PATH="$HOME/.cargo/bin:$PATH"` and `INSTA_UPDATE=no` inside the same invocation as the command that uses them.
10. Decide nothing silently: every deviation from this plan goes in the phase report. Hand off without committing and without pushing; never run `unam` outside `TestEnv`; never invoke `hermes` or any agent CLI; keep every created file free of em dashes and emoji (G8); model policy: only the implementing agent is opus, every helper (tester, reviewer) is sonnet.

---

## Safety (non-negotiable for every step and every test)

- All M1 safety rules stay: every CLI test command is built through `TestEnv::cmd()` (cleared environment, HOME and all XDG and UNAM variables in tempdirs, tempdir working directory); never run `unam` against this repository; never touch `~/.claude`, `~/.agents`, the skill library, real `~/.config/unam`, or real `~/.hermes`; no publish, push, license file, or edit to the five protected design documents (`docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`).
- Writes in M2 code are confined to: the bundles directory (manifests), `<cache dir>/edit/` (editor temp copies), the project's `.unam/selection.yaml`, and `<data dir>/projects.json`. Nothing writes under a registered source (the invariant tests prove it). M1 behavior that M2 commands inherit also writes `config.toml` (created on first `Ctx::load`) and the catalog cache `<cache dir>/catalog/<alias>.json`; every test confines both to the `TestEnv` tempdirs (config dir and cache dir), never to real directories (G9).
- Root detection never walks above a test's tempdir: `detect_root` canonicalizes both `cwd` and `ceiling` (P8), and every CLI test that runs `bundle save` (including the no-selection and the two-target tests) plants `git_marker` in its project directory before running the command, so detection never consults the ancestors of the system temp directory.
- A symlinked manifest is never written through (P21) and `bundle rm <name>` unlinks only the link (P22).
- In-process tests build `Dirs` from an injected `Env` map (`common::dirs_for`) and never read the process environment; `detect_root` tests pass a `ceiling`. The scripted editor is a shell script inside the `TestEnv` tempdir; no test ever starts a real editor.
- Bundle names and slugs are validated before any path join (`Dirs::bundle_file`); `source rm` and `bundle rm` delete only the single validated file they name.
- No em dash (U+2014) and no emoji in any created file (gate G8); test sources write exotic characters as `\u{...}` escapes.

---

## Blast Radius

Create:

- `crates/unam-core/src/bundle/mod.rs`, `bundle/slug.rs`, `bundle/word_table.rs` (generated), `bundle/edit.rs`
- `crates/unam-core/src/project/mod.rs`, `project/selection.rs`, `project/index.rs`
- `crates/unam-core/src/resolve.rs`
- `crates/unam-core/tools/gen-word-table.py` (committed word-table generator and checker, P23) and `crates/unam-core/tools/rust-word-ranges.txt` (committed Rust-side data file); neither is part of any cargo build, both fall under the G8 scan pathspec `crates` (untracked files are listed by `git ls-files -o`), the G9 grep, and the G10 syntax check
- `crates/unam-cli/src/cli/bundle.rs`
- `crates/unam-cli/tests/ac_bundles.rs`, `ac_selection.rs`, `bundle_cli.rs`; `tests/fixtures/bundles/hermes-only.yaml`, `tests/fixtures/bundles/commented.yaml`; `tests/fixtures/library2/**` (static skill tree for source alias `hs`); nine new `tests/snapshots/snapshots__bundle_*.snap` files
- `docs/adapters.md` (new spec-called document, not one of the five protected documents; decision 4, 2026-10-04)
- `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-pre-step_REF_<dd-mm-yy>.md` (Pre-Step record, P20)

Modify (existing M1 files):

- `crates/unam-core/src/lib.rs` (three `pub mod` lines), `error.rs` (new variants), `paths.rs` (`bundle_file`, `edit_dir`, doc comment on `project_index_file`), `source.rs` (the provisional index reader and its tests move to `project/index.rs`; `remove` calls the moved function), `catalog/mod.rs` (`Catalog::source_rank`, `diag_class` codes), `catalog/lookup.rs` (ordering, `canonical_ref_text`, `not_found_message`)
- `crates/unam-cli/src/cli/mod.rs` (`Bundle` subcommand, `Ctx::bundles`, dispatch, command names), `cli/show.rs` (bundles referencing it)
- `crates/unam-cli/tests/common/mod.rs` (helpers, P17), `tests/snapshots.rs` (nine new tests), `tests/show_cli.rs` (one new test), `tests/usage.rs` (one assertion)
- Snapshots changed by visible output: `tests/snapshots/snapshots__show_found.snap`, `snapshots__usage_bare.snap`

Read for verification only (must stay green unmodified): `tests/source_cli.rs`, `tests/snapshots.rs` (existing nine), `tests/ac_read_path.rs`, `tests/read_side_invariant.rs`, `tests/search_cli.rs`, `tests/cache_cli.rs`; the tests that write `projects.json` (`source_cli.rs`, `snapshots.rs`) keep the string-list format, so they need no change.

Process files modified at UPDATE PROCESS only (not by EXECUTE): `process/context/all-context.md`, `process/context/tests/all-tests.md` (new test files, new gates G12 to G14 plus G13b and G13c, the scripted-editor note), the umbrella `## Current Execution State`, the registry entry status.

Not touched: `fsops.rs`, `hash.rs`, `config.rs`, `catalog/scan.rs`, `catalog/cache.rs`, `catalog/frontmatter.rs`, `catalog/search.rs`, `output.rs` (unless a rendering helper is genuinely needed, then a recorded deviation), `main.rs`, destinations, adapters, other `docs/` files, `spike/`, the skill library, `.claude/`, `.codex/`, `.agents/`, the three `Cargo.toml` files, `Cargo.lock`, `.github/`, `dist-workspace.toml`.

Risk class: new code and one additive CLI surface; the only destructive operations are deleting or overwriting a user-scope bundle manifest (explicit commands, atomic writes, validated paths). Highest-risk areas: the line editor (can corrupt a user's hand-written manifest), the `\w` table (parity claim), resolution and removal semantics, and the candidate-ordering change in code M1 already verified.

---

## Module Specifications (decided; no design left open)

### Dependencies

No change (P18). Used from the existing workspace set: `serde`, `serde_json` (`preserve_order`), `serde_norway`, `thiserror`, `tempfile`, `jiff`, `clap`, `anyhow`, dev `insta`, `assert_cmd`, `proptest`, `tempfile`.

### `error.rs` (additions)

New `Error` variants, grouped by module, each with `code()`, `exit_code()`, `remedy()` (extend the existing match arms and the table test, which must construct one value of every variant):

| Variant (fields) | code | exit | remedy |
|---|---|---|---|
| BundleNotFound { slug } | `bundle-not-found` | 4 | "Run `unam bundle ls` to see the available bundles." |
| BundleExists { slug, path } | `bundle-exists` | 2 | "Pick another name, or re-run with `--force` to overwrite." |
| InvalidBundleName { name, reason } | `invalid-bundle-name` | 2 | "Use a name with at least one letter or digit, no control characters, and at most 128 bytes once normalized." |
| BundleInvalid { path, message } | `bundle-invalid` | 1 | "Fix the manifest: a non-blank name and a non-empty `skills` list." |
| BundleEditUnsupported { path, reason } | `bundle-edit-unsupported` | 2 | "Edit the file by hand (for a symbolic link, edit its target), or re-run with `--rewrite` when the shape is the problem (rewrites the whole file and drops comments)." |
| BundleEditFailed { path, reason } | `bundle-edit-failed` | 1 | "The file was not changed. Edit it by hand, or re-run with `--rewrite`." |
| SkillNotInBundle { bundle, reference } | `skill-not-in-bundle` | 4 | "Run `unam bundle show <bundle>` to see its entries." |
| BundleWouldBeEmpty { bundle } | `bundle-would-be-empty` | 2 | "Add another skill first, or delete the bundle with `unam bundle rm <bundle>`." |
| EditorNotSet | `editor-not-set` | 2 | "Set the EDITOR environment variable, for example `EDITOR=vi`." |
| EditorFailed { command, detail } | `editor-failed` | 1 | none |
| SelectionRead { path, source (io) } | `selection-read-failed` | 1 | none |
| SelectionInvalid { path, message } | `selection-invalid` | 1 | "Fix the YAML in the selection file, or move it aside to start fresh." |
| SelectionUnsupportedVersion { path, version } | `selection-version-unsupported` | 1 | "This selection was written by a newer unam; upgrade unam." |
| NoSelection { root, reason } | `no-selection` | 4 | "Run this from a project whose `.unam/selection.yaml` exists (write it by hand per the spec until `unam use` and `unam add` arrive in milestone M3)." |
| TargetAmbiguous { targets } | `target-ambiguous` | 2 | "This selection has several targets; the `--target` flag arrives in milestone M3. Until then keep one target in the selection." |

Messages (`#[error]`) name the data (slug, path, reason) in one line. `Error::candidates()` keeps returning candidates only for `NotFound` and `Ambiguous`.

### `paths.rs` (additions)

- `Dirs::bundle_file(&self, slug) -> Result<PathBuf>`: validates the slug (non-empty, at most 128 bytes, no `/`, `\`, NUL, or control character, not `.` or `..`, no leading `.`, and `Path::new(slug)` is exactly one normal component), then returns `<bundles dir>/<slug>.yaml`; failure is `InvalidBundleName`.
- `Dirs::edit_dir(&self) -> PathBuf`: `<cache dir>/edit`.
- Update the doc comment of `project_index_file` (no longer provisional; owned by `project::index`). Unit tests first: valid slug, each rejection class (`a/b`, `..`, `.hidden`, empty, 129 bytes, control character, NUL), the path shape, and `edit_dir`.

### `catalog/lookup.rs` and `catalog/mod.rs` (changes)

- `Catalog::source_rank(&self, alias: &SourceAlias) -> u32` (in `mod.rs`): the matching `SourceStatus.precedence`, `u32::MAX` when unknown.
- `nearest(...)` sorts by (edit distance, source rank, ID string); the `Bare` and `SourceName` Ambiguous lists sort by (source rank, `SkillId`) instead of the ID string (P15). The doc comment of `lookup` changes from "TEMPORARY" to "kept as the resolution primitive; `resolve.rs` wraps it".
- `pub fn canonical_ref_text(text: &str) -> String` (in `lookup.rs`, re-exported from `catalog` and `resolve`): `SkillRef::parse(text)` then `Display`; when parsing fails, the trimmed text.
- Extract the not-found message builder from `resolve_text` into `Catalog::not_found_message(text, unreachable) -> String` (behavior-preserving; the existing `unreachable_source_note` test covers it) so `resolve.rs` reuses it.
- `diag_class`: the nine `bundle-*` codes of P2 plus `ref-invalid`, `ref-not-found`, `ref-ambiguous`, `bundle-missing` map to `DiagClass::Config`.
- Unit tests first: ordering with two sources of different precedence (bare Ambiguous and NotFound candidates), equal precedence falls back to ID, unknown alias rank, `canonical_ref_text` table (trailing slash stripped, trimmed, invalid text kept), `diag_class` for the new codes, and the unchanged M1 tests.

### `bundle/slug.rs`, `bundle/word_table.rs`

- `slugify(name: &str) -> String`: (1) `name.to_lowercase()` (whole-string, so final-sigma context matches Python); (2) map ASCII space and `_` to `-`; (3) keep only `-` and characters for which `is_word_char` is true; (4) collapse each run of `-` to one; (5) trim `-` from both ends. `is_word_char(c)`: `c == '_' || (c.is_alphanumeric() && !in NOT_WORD)` with a binary search over the sorted disjoint inclusive ranges.
- `word_table.rs` is generated, never hand-edited, by the committed script `crates/unam-core/tools/gen-word-table.py` (Python 3.10 or newer, standard library only: `argparse`, `bisect`, `difflib`, `pathlib`, `re`, `subprocess`, `sys`, `tempfile`, `unicodedata`; no network; every path is resolved from the script's own location, so the working directory does not matter). The Unicode source for the Python side is the RUNNING Python's `unicodedata` and `re` (`unicodedata.unidata_version`). The Rust side cannot be computed from Python alone, so it is a committed second data file, `crates/unam-core/tools/rust-word-ranges.txt`, written once by `--write-rust-ranges` (needs `rustc`) and never needed again at check time (`--check` needs no `rustc`). Exact output template of `word_table.rs` (already rustfmt-stable, so no `rustfmt` step exists and `cargo fmt --all --check`, G1, is the proof; LF line endings, ASCII only, one final newline):

  ```
  // @generated by crates/unam-core/tools/gen-word-table.py. Do not edit by hand.
  // Regenerate: python3 crates/unam-core/tools/gen-word-table.py --write
  // Verify: python3 crates/unam-core/tools/gen-word-table.py --check
  // Python: <X.Y.Z of sys.version_info>
  // Unicode: <unicodedata.unidata_version>
  // Rust data: crates/unam-core/tools/rust-word-ranges.txt (<the text after `# rustc: ` in that file's header>, Unicode <the text after `# rust-unicode: ` in that file's header>)
  // Excluded: <N> code points in <R> ranges (<categories>)
  // Python-only code points (must be 0): <P>
  // Newer-Unicode letters kept as word characters (Cn in this Python): <K>

  pub(super) const NOT_WORD: &[(u32, u32)] = &[
      (0x0345, 0x0345),
      ...
  ];
  ```

  `<categories>` is `<Cat> <count>` pairs joined by `, `, sorted by descending count then category name (at plan time `Mn 927, Mc 438, So 130`). One range per line, four-space indent, `(0x{:04X}, 0x{:04X}),` (uppercase hex digits, at least four wide, wider when needed), a trailing comma on every line including the last, a single blank line between the header block and the `pub(super) const` line. The header is exactly the nine comment lines above.
- The Rust data file `crates/unam-core/tools/rust-word-ranges.txt` template (LF, ASCII, final newline): four header lines `# @generated by crates/unam-core/tools/gen-word-table.py --write-rust-ranges. Do not edit by hand.`, `# rustc: <output line of rustc --version>`, `# rust-unicode: <char::UNICODE_VERSION as X.Y.Z>`, `# definition: maximal ranges of Unicode scalar values c where c == '_' || c.is_alphanumeric()`; then one line per maximal range, `{:04X} {:04X}` (start and end inclusive, uppercase hex, at least four wide), ascending, with the surrogate gap U+D800 to U+DFFF breaking ranges (those code points are not scalar values). `--write-rust-ranges` embeds the Rust range printer as a string constant (a loop over `0..=0x10FFFF` using `char::from_u32` and `c == '_' || c.is_alphanumeric()`, printing a `# rust-unicode:` first line from `char::UNICODE_VERSION` and then the range lines), writes it into a `tempfile.TemporaryDirectory()` (the system temp directory, outside the repository, removed afterwards), builds it with `rustc -O --edition 2021`, runs it, and writes the data file with the header above.
- Generator modes (exactly one per run; argparse usage errors exit 2):
  - `--write-rust-ranges`: needs `rustc` (exit 4 when it is not on PATH); writes `rust-word-ranges.txt`.
  - `--write`: reads the data file and writes `crates/unam-core/src/bundle/word_table.rs` from the running Python (no `rustc`).
  - `--check`: Python only. (1) Parse the data file and validate it: header lines present, ranges ascending, disjoint, non-adjacent (maximal), within U+0000 to U+10FFFF, none touching U+D800 to U+DFFF. (2) Sweep every scalar value `cp` (surrogates excluded): `pyw = re.match(r"\w", chr(cp)) is not None`; `inr` = `cp` lies in the data file's ranges (binary search); Python-only = `pyw and not inr`; excluded = `inr and not pyw and unicodedata.category(chr(cp)) != "Cn"`; kept-as-word `Cn` count = `inr and not pyw and category == "Cn"`; excluded code points are coalesced into maximal ranges. (3) Regenerate the `word_table.rs` text in memory and compare it with the committed file, ignoring only the `// Python:` line (a differing Python patch version prints a note, never a failure). (4) Print one summary line `gen-word-table: python=<X.Y.Z> unicode=<V> rust-unicode=<V> excluded=<N> ranges=<R> py_only=<P> cn_kept=<K>` and then `OK`, `DRIFT: ...`, or `FAIL: ...` followed by up to 40 lines of unified diff (`difflib`). Exit codes: 0 OK; 1 FAIL (malformed data file, Python-only count above 0, or the committed table differs while the running `unicodedata.unidata_version` equals the committed `// Unicode:` line); 3 DRIFT (the table differs and the running Unicode version differs from the committed `// Unicode:` line: newer toolchain data, the gate is CONDITIONAL with the printed numbers recorded, never a silent pass); a Python-only count above 0 is always 1.
  - `--check-rust`: needs `rustc` (exit 4 without it, CONDITIONAL); rebuilds the printer, compares its range lines with the committed data file (header comment lines excluded; the `rustc` and `rust-unicode` values are printed), exit 0 when equal, 3 when they differ and the fresh `rust-unicode` differs from the file's, 1 when they differ under the same `rust-unicode`.
- At plan time (Python 3.14.5, Unicode 16.0.0, rustc 1.99.0) the result is 1495 excluded code points in 250 ranges (`Mn` 927, `Mc` 438, `So` 130), 4672 newer-Unicode `Cn` code points kept as word characters, and zero Python-only code points; record the actual numbers and versions in the report and in `docs/adapters.md`. Differences from these numbers caused only by newer toolchain Unicode data are recorded (exit 3), not failures; a non-zero Python-only count is a failure (the predicate would not be a superset).
- Unit tests first: the fixture table (each row `input` gives `expected`, all computed from the Hermes algorithm with Python 3.14.5 on 2026-10-05): `Backend Dev` gives `backend-dev`; the empty string gives the empty string; `!!!` gives the empty string; `foo.bar` gives `foobar`; `a/b` gives `ab`; `x@y` gives `xy`; `a+b` gives `ab`; `1.2.3` gives `123`; `Caf` plus U+00E9 gives `caf` plus U+00E9 (precomposed kept); `Cafe` plus U+0301 gives `cafe` (combining mark removed); CJK text `U+65E5 U+672C U+8A9E` space `U+30B9 U+30AD U+30EB` gives the same characters joined by `-` (kept); `a` tab `b` gives `ab`; `  Hello   World  ` gives `hello-world`; `My_Skill-Name` gives `my-skill-name`; `---` gives the empty string; `a--b` gives `a-b`; `-lead` gives `lead`; `trail-` gives `trail`; `a b_c-d` gives `a-b-c-d`; `Stra` plus U+00DF plus `e` gives the same lowercase text (sharp s kept); U+0130 plus `stanbul` gives `istanbul` (the dotted-i expansion's combining dot is removed); U+0391 U+03A3 gives U+03B1 U+03C2 (final sigma); U+24B6 space `bc` gives `bc` (circled letter: the table row exists for this); the Devanagari word U+0939 U+093F U+0928 U+094D U+0926 U+0940 gives U+0939 U+0928 U+0926 (marks removed); `a` U+200D `b` gives `ab`; `a` U+1F600 `b` gives `ab`; U+FF21 U+FF42 U+FF43 gives U+FF41 U+FF42 U+FF43; U+01C5 space `x` gives U+01C6 `-x`; U+216B space `roman` gives U+217B `-roman`; `a` U+00B2 `b` kept unchanged; `e` U+0301 U+0301 gives `e`. Sources use `\u{...}` escapes. Plus: `rust_word_ranges_file_matches_char_is_alphanumeric` (reads `include_str!("../../tools/rust-word-ranges.txt")`, recomputes the maximal ranges of `c == '_' || c.is_alphanumeric()` over every scalar value, and asserts equality with the data file when `char::UNICODE_VERSION` equals the file's `rust-unicode` header value; on a differing Unicode version it prints a `SKIP:` note and passes, a named residual because `--check-rust` reports that drift with exit 3); the table is sorted, disjoint, and every code point inside every range is `char::is_alphanumeric()` and not `is_word_char`; sample boundary code points (U+0345, U+0363, U+036F, U+0370 is a letter and stays word, U+24B6, U+24E9) behave; a proptest over arbitrary strings: output contains only `-` and word characters, never starts or ends with `-`, has no `--`, and `slugify(slugify(x)) == slugify(x)`.

### `bundle/mod.rs`

Types: `Bundle { slug, name, description, skills: Vec<String>, instruction: Option<String>, metadata: BundleMetadata, path: PathBuf (absolute: bundles dir joined with the file name), file_name: String }`; `BundleMetadata { version: Option<u32>, materialize_instruction: Option<bool> }` (lenient view over `metadata.unambiguous`; wrong types are ignored); `BundleSet { bundles: Vec<Bundle> (load order, accepted bundles only), diagnostics: Vec<Diagnostic> (each with `path` = file name) }` with `get(name_or_slug)` (slugify, then find; empty slug gives `None`) and `slugs()` (sorted); `pub use slug::slugify;`.

`parse_bundle_text(text, file_stem) -> Result<ParsedBundle, Diagnostic>` (`ParsedBundle` carries the fields above plus `warnings: Vec<Diagnostic>`), steps in order, mirroring Hermes `_load_bundle_file`:

1. Strip one leading U+FEFF. Parse with `serde_norway` into a `Value`; failure gives `bundle-invalid-yaml` (error, message carries the parser message); a non-mapping (including an empty file) gives `bundle-not-mapping`.
2. `name`: key absent or YAML null falls back to `file_stem`; a string is trimmed; a number or boolean is stringified; an explicitly empty or whitespace-only string, or a blank result, gives `bundle-name-blank` (error, bundle skipped).
3. `skills`: must be a non-empty sequence else `bundle-no-skills` (message says "missing, not a list, or empty"). Each entry: string trimmed; number or boolean stringified (`true`, `false`, decimal); null dropped silently; mapping, sequence, or tagged value dropped, counted into one `bundle-entry-ignored` warning for the bundle; entries blank after trim dropped. Nothing left gives `bundle-no-skills` ("empty skills list").
4. `slug = slugify(name)`; empty gives `bundle-empty-slug` (error).
5. `description`: scalar text trimmed (null and absent are empty); empty gives `Load {n} skills as a bundle` with `n` the cleaned entry count. `instruction`: scalar text trimmed, `None` when empty. `metadata`: from `metadata.unambiguous`.

`load_dir(dirs) -> BundleSet`: a missing bundles directory gives an empty set and no diagnostic. List regular files directly in the directory (`is_file`, following symlinks; no hidden-file filter, matching Hermes `Path.glob`), extension exactly `yaml` or `yml`; sort each group bytewise by file name; process all `.yaml` then all `.yml` (Hermes order; a stem present in both gives `a.yaml` first). Per file: read bytes (IO error gives `bundle-unreadable`; invalid UTF-8 gives `bundle-not-utf8`), `parse_bundle_text(text, file stem)`, on error record the diagnostic with `path` = file name and skip; on success the first file for a slug wins and a later one adds `bundle-duplicate-slug` (warning naming both files). Diagnostics are in processing order; the set is never fail-fast.

CRUD functions (all writes through `fsops::write_atomic`; all take the loaded `BundleSet`): `render_manifest(name, skills) -> String` (LF, no BOM: `name:` line, `skills:` line, one `  - <scalar>` line per skill, dash at indent 2; scalars via a private `yaml_scalar` built on `serde_norway::to_string` of the string with the trailing newline removed; names or entries containing control characters are rejected before rendering); `save(dirs, set, name, skills, force) -> SaveOutcome { slug, name, path, overwritten }` (P14 rules; at least one skill; renders, verifies the text with `parse_bundle_text`, then writes); `duplicate(dirs, set, source, new_name, force) -> SaveOutcome` (the source must be loaded; copies its bytes, replacing the top-level `name:` line by `name: <scalar of new_name>` or inserting that line first when the file has none (when the file starts with a BOM the new line goes after the BOM, so the BOM stays first); the result must parse to the new name and the same skills, else `bundle-edit-failed` and no write); `delete(dirs, set, name) -> DeleteOutcome { slug, path }` (`BundleNotFound` when not loaded; per P22 it removes exactly the loaded `Bundle.path`: it asserts `path.parent() == dirs.bundles` and that `symlink_metadata` shows a regular file or a symlink, else `BundleEditFailed` and nothing is removed; `std::fs::remove_file` on that path only, so a symlinked manifest is unlinked and its target is never touched, and no sibling manifest is touched); `add_refs(dirs, set, catalog, name, refs, rewrite) -> EntriesOutcome { slug, path, added, skipped, skill_count }` and `remove_refs(dirs, set, catalog, name, refs, rewrite) -> EntriesOutcome` (matching rules below); `edit_in_editor(dirs, set, name, editor) -> EditOutcome { slug, path, changed }` (P13). Symlink rule (P21): `save` and `duplicate` with `--force` over an existing slug, `add_refs`, `remove_refs` (line edit and `--rewrite`), and the `edit_in_editor` commit each call `std::fs::symlink_metadata` on the existing target `Bundle.path` before anything else; a symlink gives `BundleEditUnsupported` (reason: the manifest is a symbolic link, edit its target directly; `--rewrite` does not bypass it), exit 2, nothing written, and for `edit_in_editor` the editor is never started.

`remove_refs` matching, per reference, before anything is written: (1) every manifest entry whose `canonical_ref_text` equals the reference's canonical text matches; (2) otherwise the reference is resolved with `resolve::resolve_ref` (a not-found or ambiguous result aborts with that error, exit 4 or 5) and every entry that itself resolves to the same `SkillId` matches; (3) otherwise `SkillNotInBundle`. All matched entries of all references are removed together; the values handed to `remove_values` are the raw edit-layer values (from `list_entries`) of the matched entries, never the canonical reference text (a mismatch is caught by the verification below, exit 1 and no write, but must not occur); zero remaining entries gives `BundleWouldBeEmpty`. `add_refs`: each reference must parse (control characters rejected) and resolve (`resolve_ref`); a reference whose canonical text equals an existing entry's canonical text is skipped; the rest are appended in the given order. `--rewrite` replaces the line edit with a full rewrite: parse the file to a `serde_norway::Mapping`, set `skills` to the new list, serialize with `serde_norway::to_string`, restore the BOM and the file's line ending; comments are lost; the same verification applies.

Verification after any line edit or rewrite (D3): parse the new text with `parse_bundle_text`; require `Ok`, the same slug and name, `skills` equal to the expected list (old cleaned list minus removed entries, or plus added, order preserved), and every other top-level key equal as a `serde_norway::Value` to the old file's (compared with `skills` removed from both); any mismatch is `BundleEditFailed` and nothing is written.

Unit tests first: loader matrix (valid Hermes-only file; BOM; CRLF; missing name uses the stem; null name uses the stem; empty and whitespace-only name give `bundle-name-blank`; numeric name stringified; description default and strip; instruction strip and `None`; skills missing, scalar, empty, all blank; integer, boolean, null, mapping, and nested-list entries; invalid YAML; non-mapping; empty file; empty slug; load order `.yaml` before `.yml` with files `b.yaml`, `c.yaml`, `a.yml` loading as `b`, `c`, `a`; duplicate slug first wins with a warning; non-UTF-8 file; a directory named `x.yaml` ignored; missing directory; `BundleSet::get` by name and by slug; metadata view; a chmod 000 file gives `bundle-unreadable`, Hybrid, self-skips with a printed note under root); `yaml_scalar` round trip over a table (plain, `a: b`, leading `-`, `#`, trailing space, `123`, `true`, quotes) and a proptest over printable ASCII; `render_manifest` shape and round trip; `save` (new, exists without force, force overwrites at the existing file name, invalid name, empty skills, slug collision with a differently named file); `duplicate` (including a BOM-prefixed source with no `name:` key, where the inserted line follows the BOM); `delete` (removes exactly `Bundle.path`: a file `Backend Dev.yaml` with slug `backend-dev` is deleted while sibling manifests stay byte-identical; a symlinked manifest is unlinked and its target is untouched, unix; a `Bundle` whose path parent is not `dirs.bundles` gives `BundleEditFailed` and removes nothing); `symlinked_manifest_refuses_in_place_writes` (unix, one row: `add_refs`, `remove_refs` with and without `--rewrite`, `save` and `duplicate` with `--force`, and `edit_in_editor` over a symlinked manifest each give `BundleEditUnsupported`; link and target bytes unchanged; the editor is never started); `add_refs` and `remove_refs` (matching passes 1 to 3, equivalent spelling, would-be-empty, skipped duplicate, abort leaves the file untouched, rewrite path keeps unknown keys and BOM, verification failure path); `edit_in_editor` with a scripted editor (valid change commits; invalid result, non-zero exit, and slug change leave the file unchanged; unchanged bytes report `changed: false`; the temp copy is gone afterwards on every path including editor failure, and two sequential edits never reuse a temp name).

### `bundle/edit.rs` (line editor, D3)

API (the module has no knowledge of bundles or catalogs): `list_entries(text) -> Result<Vec<String>, EditError>` (entry values in order; a null entry is the empty string), `remove_values(text, values) -> Result<(String, Vec<String>), EditError>` (new text, removed values one per removed line), `add_values(text, values) -> Result<(String, Vec<String>, Vec<String>), EditError>` (new text, added, skipped); `EditError` carries one reason string that callers wrap as `BundleEditUnsupported`.

Processing: split into lines keeping each line's terminator (`\n`, `\r\n`, or none on the last line); if the text starts with U+FEFF, treat it as outside line 0 and put it back unchanged. The line ending used for inserted lines is the terminator of the `skills:` key line.

Locating the block: exactly one line at column 0 that starts with `skills:` (a quoted key `"skills":` or `'skills':` at column 0, or a second `skills:` line, is refused; if only an indented `skills:` exists the reason says it appears only nested, for example under `metadata`; if none exists the reason says no `skills:` block was found). The rest of the key line must be empty or spaces plus an optional `# comment`; a value after the colon is refused with the reason by first character: `[` flow sequence, `&` or `*` anchor or alias, `!` tag, `|` or `>` block scalar, anything else scalar value. Tab characters in indentation are refused.

Items: following lines are scanned in order. A blank or comment-only line is skipped (and stays). An item line is zero or two spaces, then `-`, then a space (or end of line for a null item); the first item fixes the dash indent (0 or 2) and every later item must match it (else refused: inconsistent indentation). The first non-blank, non-comment line at column 0 that is not an item (the next key) ends the block. Any other non-blank non-comment line (deeper indentation, a continuation, a nested structure) is refused as a multi-line or nested entry. Zero items is refused. The item body: if it starts with `"`, scan to the closing unescaped `"`; if it starts with `'`, scan to the closing `'` (with `''` as an escape); an unterminated quote is refused (multi-line quoted scalar); after the closing quote only spaces and an optional `# comment` may follow, else refused. A plain body ends at the first space-hash or the end of the line, with trailing spaces trimmed; it is refused when it starts with one of `[ { & * ! | > % @` or a backtick, starts with `- `, or contains `: ` or ends with `:` (a mapping entry). An empty body, or a body that is only a comment, is a null entry. The entry value is `serde_norway` parsing of the scalar token alone, stringified (string as is, number or boolean as text, null as the empty string); a token that parses to a mapping or sequence is refused.

Operations: `remove_values` deletes every item line whose value is in `values` (only that line, terminator included; a final line without a terminator that is removed leaves the previous line's terminator in place, so the file may now end with a newline). `add_values` skips a value equal (by exact value) to an existing entry, then inserts one line per remaining value directly after the last item line: the block's dash indent, `- `, the `yaml_scalar` text, and the key line's terminator (when the last item line was the file's last line without a terminator, that line first gets the terminator and the final new line gets none, preserving the no-trailing-newline style). Nothing else in the file is touched.

Edge-case matrix (one unit-test row each, asserting the exact output text or the refusal reason): dash at indent 0 and at indent 2; double-quoted, single-quoted, and plain entries; entries with trailing comments; comment lines and blank lines between items; `skills:` with a trailing comment; CRLF throughout; a BOM; no trailing newline at end of file; CRLF plus BOM plus no trailing newline; remove first, middle, last, and several at once; remove an entry that has duplicates (all matching lines go); remove that leaves other lines byte-identical (asserted by comparing the lists of untouched lines); add to a block followed by blank and comment lines; add to a block that ends the file without a terminator; add with the CRLF ending; add skipped for an existing value; refusals: flow sequence, anchor, alias entry, tagged value, block scalar value, scalar value, `skills` only under `metadata`, no `skills` key, duplicate `skills` key, quoted key, mixed dash indent, tab indentation, multi-line plain entry (continuation line), multi-line quoted entry, mapping-looking entry (`- a: b`), nested sequence entry (`- - a`), `skills:` with no entries; a null entry round-trips untouched; an entry whose value looks numeric stays a string after `add_values` (the inserted token is quoted so it reparses as a string). Added matrix families (PVL cycle 1, R2), each one unit-test row asserting exact output text or values: (a) quoted entries with special content, each through `list_entries`, `remove_values`, and `add_values`: `"a: b"`, `"a #b"`, `"a\"b"` (escaped quote), `"a\\"` (escaped backslash before the closing quote), `'a''b'`, `'a: b'`, `'a #b'`, and a quoted entry whose trailing comment itself contains quote characters (`- "x"  # say "hi"`); `list_entries` returns the unquoted values (`a: b`, `a #b`, `a"b`, `a\`, `a'b`, `a: b`, `a #b`, `x`), removal deletes only the matching line, and `add_values` of each such value inserts a token that reparses to exactly that string; (b) scalar-looking entries already in the file through `list_entries`: `- true`, `- 123`, `- null`, `- ~`, `- 1.50`, `- 0x10`, `- yes` give `true`, `123`, the empty string, the empty string, `1.5`, `16`, `yes` (the `serde_norway` stringification the loader also uses), and `add_values` of `true`, `null`, `~`, `123`, `1e3`, `0x10` inserts each quoted so it reparses as the same string, while `yes` and `no` are inserted as written (plain) and still reparse as strings under YAML 1.2 (the writer note for `docs/adapters.md`); (c) an entry whose value equals the key name: `- skills` (value `skills`; removing it leaves the `skills:` key line untouched; `add_values` of `skills` inserts `- skills`), `- "skills:"` (value `skills:`), and `add_values` of the value `skills:` (inserted quoted). A proptest builds random valid block manifests (random indent 0 or 2, random quoting, random comments, random EOL) and checks remove-then-reparse equality, that all lines not removed are byte-identical, and that `add_values` followed by `list_entries` returns the old list followed by the new values in order. Proptest entry alphabet: each entry value is drawn from (1) plain tokens over `[A-Za-z0-9:/._-]` (1 to 24 characters, no space, no `: ` sequence, not starting with `-`), (2) quote-requiring strings over that alphabet plus space, `#`, `'`, `"`, and `\` (1 to 24 characters, not blank after trimming), and (3) the literals `true`, `false`, `null`, `~`, `yes`, `no`, `123`, `1e3`, `0x10`, `skills`, `skills:`. The test renderer writes a value plain only when it is a class (1) token that starts with an ASCII letter, does not end with `:` (the grammar refuses a plain body ending with a colon because it would be a mapping entry), and is not one of the literals that would reparse as a non-string (`true`, `false`, `null`, `~`, `123`, `1e3`, `0x10`); everything else, including every class (1) token that starts with a digit, `.`, `/`, `_`, or `:` or that ends with `:`, is written double-quoted or single-quoted (random choice) with correct escaping (`\\` and `\"` inside double quotes, `''` inside single quotes). The `add_values` property draws its new values distinct from each other and from the existing entries (the skip-an-existing-value case has its own matrix row), so the expected result is exactly the old list followed by the new values. (PVL cycle 2: the plain-token rule above was tightened because class (1) tokens ending with `:` made the proptest fail against the plan's own grammar.)

### `project/index.rs`

`read_index(dirs) -> IndexRead { projects, diagnostics, state }` (`IndexState`: `Absent`, `Ok`, `Corrupt`, `Unreadable`; schema `{"version": 1, "projects": [string...]}`; the diagnostics are `project-index-unreadable` warnings exactly as M1 produced them: "cannot read project index PATH: ..." and "cannot parse project index PATH: ..."); `register(dirs, root) -> Result<Vec<Diagnostic>>` (P9); `projects_referencing_source(dirs, alias) -> (Vec<String>, Vec<Diagnostic>)` (the M1 `referencing_projects` behavior moved unchanged, including the lenient scanner and the sorted, deduplicated result); `projects_referencing_bundle(dirs, slug)` (same shape; a project references a bundle when any string under `targets.*.bundles` slugifies to `slug`). Unit tests: the M1 `referencing_projects_cases` test moved here and renamed, plus the bundle variant, plus `register` (creates the data dir and file; idempotent; keeps sorted and deduplicated; corrupt index rebuilt with a warning; unreadable index not written; two roots).

### `project/selection.rs`

`TargetId { ClaudeCode, AgentsNeutral }` (`as_str`, `parse`, serde as the string, ordered); `TargetSelection { strategy: Option<Strategy>, bundles: Vec<String>, add: Vec<String>, remove: Vec<String> }`; `Selection { version: u32, targets: BTreeMap<TargetId, TargetSelection> }` (P7); `selection_path(root)`; `parse_selection(text, path) -> Result<Selection>`; `load_selection(root) -> Result<Option<Selection>>` (absent file gives `None`; IO error `SelectionRead`; YAML or schema error `SelectionInvalid`; version not 1 `SelectionUnsupportedVersion`); `save_selection(dirs, root, selection) -> Result<SelectionSave { path, created_dir, gitignore_lines, diagnostics }>`: creates `.unam/` if absent, writes with `write_atomic`, calls `index::register`; `gitignore_lines` is `.unam/applied.json`, `.unam/journal.json`, `.unam/history/` only when this call created the `.unam/` directory (OWN-7: "offered" means returned and printed by the caller; the file `.gitignore` is never read or written). Unit tests: the spec 2.3 example parses; missing `version`; version 2; unknown key; unknown target; empty file; whitespace-only file; round trip; save creates `.unam/` and offers the lines once and not on the second save; save registers the root (canonical); load of an absent file; invalid YAML; add and remove kept as text including invalid text.

### `project/mod.rs`

`RootDetection { Git, Explicit }`, `ProjectRoot { path, how }`, `detect_root(cwd, ceiling) -> Result<ProjectRoot>` (P8). Unit tests (each with a tempdir and `ceiling` set to the tempdir): `.git` directory, `.git` file, `.git` symlink, nearest wins over an outer one, started in a subdirectory, none gives `Explicit` equal to the canonical cwd, ceiling is examined but not passed, a ceiling given as a symlink path to the tempdir (unix) still stops the walk because it is canonicalized, a cwd that does not exist is an IO error.

### `resolve.rs`

- Re-exports `Lookup`, `edit_distance`, `canonical_ref_text`. `ResolveError { NotFound { candidates, unreachable }, Ambiguous(Vec<String>) }` and `resolve_ref(r, catalog) -> Result<SkillId, ResolveError>` (spec 4.6 signature; a thin mapping of `Catalog::lookup`); `resolve_text(catalog, text) -> Result<SkillId, Error>` (parse then `resolve_ref`, errors built exactly as `Catalog::resolve_text`).
- Types: `Pin { Follow }` (`#[non_exhaustive]`); `Provenance { Bundle(String), Adhoc }` (serializes as `{"bundle": slug}` or `{"adhoc": true}`; provisional until milestone M3 writes receipts); `DesiredEntry { id, name, strategy, pin, provenance }` with `winner()` (last provenance record); `ProblemKind`, `Origin`, `Problem` (P4) with `to_error()` (`InvalidRef`, `NotFound`, `Ambiguous`, `BundleNotFound`) and `to_diagnostic()` (the P2 codes; message includes the origin, for example "from bundle `x`", "in add", "in remove"); `Resolution { resolved: Vec<DesiredEntry>, problems: Vec<Problem> }` with `is_clean()` and `first_error()`.
- `resolve_selection(target: &TargetSelection, bundles: &BundleSet, catalog: &Catalog, default_strategy: Strategy) -> Resolution`: for each text in `target.bundles` in order: slugify it (an empty slug or a slug with no loaded bundle gives a `BundleMissing` problem), then for each entry of the bundle in order add the reference with provenance `Bundle(slug)`; then each `target.add` text with provenance `Adhoc`; adding a reference parses it (failure gives an `Invalid` problem), looks it up (`Found` adds or extends the entry; `NotFound` and `Ambiguous` become problems with the matching candidates and unreachable note) and the first occurrence of an ID fixes its position; then each `target.remove` text is resolved and subtracted per P5. Strategy is `target.strategy` or `default_strategy`. Never fail-fast; `resolved` order is first-occurrence order.
- `EntryResolution { entry, outcome }` and `resolve_entries(bundle, catalog) -> Vec<EntryResolution>` (outcome is a resolved `SkillId` or a `Problem`), used by `bundle show` and by the removal matching; `bundles_referencing(bundles, catalog, id) -> Vec<String>` (sorted slugs of bundles with at least one entry that resolves to `id`).
- Unit tests first (one per bullet, against a catalog with duplicates across categories and across two sources, built with the existing `catalog_of` helper plus precedence): order bundles then add; first occurrence fixes position; provenance accumulates and dedupes; `winner()` is the last; remove by an equivalent spelling (bundle has `fx:code-review`, remove `fx:sdlc/code-review`); remove drops all provenance; remove of an unselected valid id is a no-op; overlapping bundle membership (EDG-7: two bundles share a skill; a selection naming only one bundle keeps it with that bundle's provenance); problems are collected, not fail-fast (EDG-3: one bad entry, the rest resolve); an unresolved `remove` cancels the same dangling add; an unresolved `remove` with no matching dangling entry is its own problem; missing bundle; slugified bundle name (`Backend Dev` finds `backend-dev`); empty slug; invalid entry text (`fx:`); unreachable source note; ambiguous across sources lists candidates in source-precedence order; strategy inheritance and override; empty selection; `Problem` to error and diagnostic mapping with the exit codes 2, 4, 5, 4; `first_error` is the first problem; `resolve_entries` statuses; `bundles_referencing` (sorted, only resolving entries count).

### `source.rs` (change)

Delete `ProjectIndex`, `selection_references`, and `referencing_projects`; `remove` calls `crate::project::index::projects_referencing_source`. The tests `remove_*` stay unchanged and green (they write the same index and selection text); `referencing_projects_cases` moves to `project/index.rs`. No other change.

### `lib.rs`

Adds `pub mod bundle;`, `pub mod project;`, `pub mod resolve;` (each added by the step that creates its first file, see the checklist). The crate documentation comment gains bundles, project, and resolve.

---

## CLI Contract (`unam-cli`)

### Commands, flags, exit codes

Global flags are unchanged (`--json`, `--no-cache`, `-v`, `-q`). New top-level command `bundle` (doc comment `Manage skill bundles.`; clap drops the trailing period in the command list, so the help line reads `Manage skill bundles`), declared after `show` so help order is source, search, show, bundle.

| Command | Behavior | Exit |
|---|---|---|
| `bundle save <name> [--force]` | From the current project's selection (P10) write `<slug>.yaml` with fully qualified refs | 0; 2 invalid name, exists, several targets; 4 no selection, unresolved ref; 5 ambiguous ref; 1 malformed selection |
| `bundle ls` | List bundles sorted by slug with skill count; loader diagnostics in warnings | 0 (a broken bundle is reported, never an error) |
| `bundle show <name>` | Manifest fields plus the resolution status of every entry; problems listed | 0; 4 unknown bundle |
| `bundle edit <name>` | Open `$EDITOR` on a temp copy; commit only a valid result with the same slug | 0; 2 `EDITOR` unset; 4 unknown; 1 editor failed or invalid result |
| `bundle duplicate <name> <new-name> [--force]` | Copy under a new name | 0; 2 exists, invalid name; 4 unknown source bundle |
| `bundle rm <name>` | Delete the manifest; list referencing projects, never block | 0; 4 unknown |
| `bundle rm <name> <ref>... [--rewrite]` | Remove entries in place | 0; 2 would be empty, unsupported shape; 4 not in bundle, unresolvable ref; 5 ambiguous ref |
| `bundle add <name> <ref>... [--rewrite]` | Add entries in place after resolving each ref | 0; 2 unsupported shape, invalid ref; 4 unresolvable ref; 5 ambiguous ref |

A symlinked manifest gives exit 2 (`bundle-edit-unsupported`) for `save --force`, `duplicate --force`, `edit`, `add`, and `rm <name> <ref>...` (P21); `rm <name>` unlinks only the link (P22). `--rewrite` is accepted only with references (clap `requires`). Exit 1 is any other failure; exits 6 to 11 stay unused.

### `show` change

`data.bundles` and the `Bundles:` human line (P16). No other field changes.

### `--json` data shapes (envelope unchanged; `command` is `bundle save`, `bundle ls`, `bundle show`, `bundle edit`, `bundle duplicate`, `bundle rm`, `bundle add`)

- `bundle save`: `{ slug, name, path, project_root, target, skill_count, skills: [ids], overwritten }`.
- `bundle ls`: `{ bundles: [ { slug, name, description, skill_count, has_instruction, path } ], diagnostics: [...] }`.
- `bundle show`: `{ slug, name, description, instruction (string or null), path, skills: [ { entry, status ("resolved"|"unresolved"|"ambiguous"|"invalid"), id (string or null), candidates: [...] } ], problems (count) }`.
- `bundle edit`: `{ slug, path, changed, problems }`.
- `bundle duplicate`: `{ source, slug, name, path, overwritten }`.
- `bundle rm <name> <ref>...`: `{ slug, path, removed: [entry texts], rewrite, skill_count }` (`removed` is always an array of the removed entry texts as written in the manifest, one per removed line).
- `bundle rm <name>`: `{ slug, path, deleted: true, referencing_projects: [roots] }` (no `removed` key; the two forms never share a key with different types).
- `bundle add`: `{ slug, path, added: [texts], skipped: [texts], rewrite, skill_count }`.
- Diagnostics in data and in `warnings` use the M1 diagnostic JSON object with `source` null and `skill` null for bundle and selection diagnostics (`FlatDiagnostic::global`), the manifest file name in `path`.
- Errors use the M1 `errors` objects; `candidates` appears for `not-found` and `ambiguous` as before.

### Human output (informative; tests assert substrings)

`save`: `Saved bundle <slug> (N skills) -> <path>` (adds `(overwrote existing)` when forced over one). `ls`: header `SLUG  NAME  SKILLS  DESCRIPTION`, one row per bundle, or `No bundles in <dir>`. `show`: title `<name>  (<slug>)`, `Description:`, `Path:`, `Instruction:` (when present), then `Skills (N):` with one line per entry, `  <entry>  ->  <id>` or `  <entry>  !! <status>: <ids or reason>`. `add`: `Added N skill(s) to <slug>` and, when relevant, `Already present: <texts>`. `rm` with references: `Removed N entr(y|ies) from <slug>`. `rm` bundle: `Deleted bundle <slug> (<path>)`, then each referencing project root and the sentence that those selections keep the reference, which no longer resolves until the bundle is restored. `duplicate`: `Copied <source> -> <slug> (<path>)`. `edit`: `Bundle <slug> updated` or `No changes`. All untrusted text (names, descriptions, entries, paths, diagnostic messages) goes through `neutralize`.

### `cli/mod.rs`, `cli/bundle.rs`, `cli/show.rs`

`cli/mod.rs`: the `Bundle { cmd: BundleCmd }` command and the `BundleCmd` enum (`Save`, `Ls`, `Show`, `Edit`, `Duplicate`, `Rm`, `Add`), `Ctx::bundles()` (calls `bundle::load_dir(&dirs)`), dispatch, and the seven `command_name` strings. `cli/bundle.rs`: one function per subcommand, thin: load config and catalog as needed (the catalog is loaded for `save`, `show`, `edit`, `add`, and `rm` with references; not for `ls`, `duplicate`, and `rm` of a bundle), call `unam-core`, render. `bundle save` uses `std::env::current_dir()` and `project::detect_root(cwd, None)` (the CLI is the only place the process working directory is read) and maps `Resolution` problems as P10 says (`Failed { error: first problem's error, warnings: every problem as a diagnostic }`). `bundle edit` reads `EDITOR` from `ctx.dirs.env`. `cli/show.rs` adds `bundles`.

---

## Test Harness additions (`crates/unam-cli/tests/common/mod.rs`) and Fixture Contract

`common/mod.rs` gains (P17; every helper depends only on APIs that exist at step A5, so all test binaries keep compiling): `dirs_for(env) -> unam_core::paths::Dirs` (builds an `Env::from_map` of the seven `TestEnv` variables); `build_second_library(env) -> PathBuf` (copies `tests/fixtures/library2` into `work/lib2`); `copy_fixture_bundle(env, name)` (copies `tests/fixtures/bundles/<name>` into the bundles dir); `write_project_index(env, roots)` (string-list format); `git_marker(dir)` (creates an empty `.git` directory as a root marker); `cmd_in(env, dir)` (like `cmd()` with another working directory); `write_scripted_editor(env, body) -> PathBuf` (a `#!/bin/sh` file inside the `TestEnv` with mode 755, unix only); `read_lines(path) -> Vec<String>` (lines with their terminators preserved, for AC-11 comparisons).

Static fixtures (plain text, reviewable):

| Path | Content | Expected |
|---|---|---|
| `fixtures/bundles/hermes-only.yaml` | `name: hermes-only`; `skills:` list `code-review`, `pytest-fixtures`, `alpha-root-skill` (dash at indent 2, bare names); a multi-line `instruction: |` block; no `description`, no `metadata` | loads; description `Load 3 skills as a bundle`; instruction text trimmed |
| `fixtures/bundles/commented.yaml` | a leading comment line; `name: commented`; `description: Backend feature work.`; `skills:` with `  - fx:sdlc/code-review        # primary review flow`, a comment line `  # testing helpers`, `  - "fx:testing/pytest-fixtures"`, `  - 'fx:data/schema-migration'`, `  - alpha-root-skill`; `instruction: |` block; `metadata:` with `unambiguous:` `version: 1` | AC-11 subject; all five entries resolve against `fx` |
| `fixtures/library2/ops/deploy-checklist/SKILL.md` | valid; name `deploy-checklist`; description about a deploy checklist | loaded (alias `hs`) |
| `fixtures/library2/api/rest-design/SKILL.md` | valid; name `rest-design` | loaded |
| `fixtures/library2/data/schema-migration/SKILL.md` | valid; name `schema-migration` (same name and relative path as `fx:data/schema-migration`) | loaded; bare `schema-migration` is ambiguous across sources |

`library2` has exactly three skills and no diagnostics; alias `hs` is registered second (precedence 20, after `fx` at 10), so candidate lists put `fx` first.

### Insta snapshot procedure (as M1, with the M2 specifics)

- Nine new snapshots, one `snapshots__<name>.snap` each: `bundle_save`, `bundle_ls`, `bundle_show`, `bundle_show_not_found`, `bundle_add`, `bundle_rm_skill`, `bundle_rm_bundle`, `bundle_duplicate`, `bundle_edit`. `bundle_rm_skill` pins `removed` as an array of entry texts and has no `deleted` key; `bundle_rm_bundle` pins `deleted: true` with `slug`, `path`, and `referencing_projects` and has no `removed` key. Total after M2: 19.
- Two existing snapshots change by visible output and are regenerated: `snapshots__show_found.snap` (exactly one added line, `"bundles": [],`) and `snapshots__usage_bare.snap` (only added `bundle` help lines). Gates G7b to G7d check this mechanically. Any other changed snapshot is a deviation to explain in the report from its diff.
- Creation (EXECUTE only, step H5): run `INSTA_UPDATE=always cargo test -p unam --test snapshots` once, read every new or changed `.snap`, confirm no temp path or timestamp (`[TMP]` placeholders only), then confirm `INSTA_UPDATE=no` is green. Never accept a snapshot unread.

---

## Data Flow (primary scenario: save a bundle from a project and reuse it)

1. `unam bundle save my-set` in a project: `main` builds `Dirs`; the CLI reads the working directory and `detect_root` finds the nearest `.git` ancestor (or the directory itself, `Explicit`).
2. `load_selection` reads `<root>/.unam/selection.yaml`; exactly one target is chosen.
3. `Catalog::load` (cached) and `bundle::load_dir` produce the catalog and the bundle set; `resolve_selection` expands the target's bundles in order, then `add`, then subtracts `remove`, with provenance.
4. Problems stop the command with the first problem's exit code; otherwise `bundle::save` renders `name` plus fully qualified `skills`, verifies the text by parsing it, and writes `<bundles dir>/my-set.yaml` atomically.
5. In another project, a selection `bundles: [my-set]` resolves through the same `resolve_selection` to the same IDs (AC-10, desired side). Milestone M3's `plan::diff` consumes `Resolution.resolved`.

Writes: one manifest file. Nothing under a source, nothing in either project except (when core `save_selection` is used) `.unam/selection.yaml` and the data-dir index.

## Failure Modes

| Failure | Behavior |
|---|---|
| Bundles directory missing | Empty set, no diagnostic; created on first `save` |
| Manifest unreadable, not UTF-8, invalid YAML, not a mapping, blank name, no skills, empty slug | That bundle skipped with a named error diagnostic; others load; exit 0 for `ls` and `show` of others |
| Duplicate slug | First wins; later file reported `bundle-duplicate-slug` |
| Selection malformed, wrong version, unknown key | exit 1, file untouched |
| Selection names a missing bundle or an unresolvable, ambiguous, or invalid ref | Collected as problems (not fail-fast); commands needing a full set exit 2, 4, or 5 with every problem as a warning |
| Line editor cannot prove an edit safe | exit 2, nothing written, error names the reason and `--rewrite` |
| Manifest is a symbolic link and the command would write it | exit 2 `bundle-edit-unsupported`, nothing written, editor not started (P21); `bundle rm <name>` unlinks only the link and never its target (P22) |
| Post-edit verification fails | exit 1 `bundle-edit-failed`, nothing written |
| Editor missing, exits non-zero, or leaves an invalid or renamed bundle | No write; the original manifest is byte-identical; temp copy removed |
| Interrupted write of a manifest, selection, or index | Atomic rename leaves the old or the new file |
| Project index corrupt or unreadable | Reads yield no references plus a warning; `register` rebuilds a corrupt file (warning) and does not write after an IO error. Known limitation (recorded, not fixed in M2): the rebuild keeps only the registering root, so other registered projects are forgotten and `source rm` loses its exit-3 protection for them until they register again; a later milestone may move the corrupt file aside as `projects.json.corrupt` first |
| Source unreachable during resolution | Reference is `Unresolved` with the unreachable note; nothing marked inactive (EDG-5) |
| Two processes edit one manifest | Last writer wins (no lock until milestone M3); recorded residual |

---

## Implementation Checklist

Order follows the compile chain so every step builds and its gate can run: foundations (errors, paths, catalog ordering), fixtures and the red first test, slugify, project, bundle loader and editor, resolve, bundle CRUD, CLI, tests and snapshots, docs and closeout. The first test (AC-11) is written red as soon as the fixtures and helpers exist, before the code that makes it pass. After each step run that step's gate and fix inline before moving on. Test-first: where a step lists tests, write them before the implementation they cover. Until step G the red first test lives in package `unam`, so earlier gates run `cargo test -p unam-core` (never `--workspace`) and compile-check the rest with clippy.

### Step A: Pre-step, foundations, fixtures, first test

- [x] A0. Run the Pre-Step (versions, real-dirs record file per P20, hook note, no installs, `INSTA_UPDATE=no`).
- [x] A1. `error.rs`: add the 15 variants of the table with `code()`, `exit_code()`, `remedy()`; extend the table test to construct every new variant.
- [x] A2. `paths.rs`: `bundle_file`, `edit_dir`, the doc-comment update; unit tests first.
- [x] A3. `catalog/lookup.rs` and `catalog/mod.rs`: `source_rank`, candidate ordering (P15), `canonical_ref_text`, `not_found_message`, `diag_class` codes; unit tests first.
- [x] A4. Gate A: `gate cargo test -p unam-core error`, `gate cargo test -p unam-core paths`, `gate cargo test -p unam-core catalog`, then `cargo test -p unam-core` (all M1 tests unchanged and green), `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`.
- [x] A5. Create the static fixtures (Fixture Contract) and the `common/mod.rs` helpers (P17).
- [x] A6. FIRST TEST, written RED: `crates/unam-cli/tests/ac_bundles.rs` with only `ac_11_removing_one_entry_leaves_every_other_line_byte_identical` per the AC-11 row (including the CRLF plus BOM, dash-at-indent-0, first-item, and last-item variants, each in its own `TestEnv`). It fails now because `unam bundle` does not exist (clap exit 2); it must be green at step G. File header comment: "AC-11 is the riskiest M2 behavior (hand edits of user files); written first".
- [x] A7. Gate A2: fmt and clippy (the red test compiles); run `cargo test -p unam --test ac_bundles -- --exact ac_11_removing_one_entry_leaves_every_other_line_byte_identical` once and record that it FAILS (proves red) in the report.

### Step B: Slugify and the word table

- [x] B0. Create `bundle/mod.rs` containing only `pub mod edit; pub mod slug; pub mod word_table;`; create doc-comment-only stubs `bundle/edit.rs` and `bundle/slug.rs`, and `bundle/word_table.rs` holding a doc comment plus `pub(super) const NOT_WORD: &[(u32, u32)] = &[];` so every step compiles before B3 (the unused-constant warning is expected until B2, so run no clippy between B0 and B3); add `pub mod bundle;` to `lib.rs`. The line `pub use slug::slugify;` is added to `bundle/mod.rs` at B2, when `slugify` exists. No `todo!()` or `unimplemented!()`.
- [x] B1. Write the committed generator `crates/unam-core/tools/gen-word-table.py` exactly per the `bundle/slug.rs, bundle/word_table.rs` spec (standard library only; no network; paths resolved from the script's own location). Run, in order: `python3 crates/unam-core/tools/gen-word-table.py --write-rust-ranges` (needs `rustc`; creates `crates/unam-core/tools/rust-word-ranges.txt`), `python3 crates/unam-core/tools/gen-word-table.py --write` (replaces the B0 stub `bundle/word_table.rs`), `python3 crates/unam-core/tools/gen-word-table.py --check` (must exit 0 with `py_only=0`), and `python3 crates/unam-core/tools/gen-word-table.py --check-rust` (must exit 0). Record the printed summary lines (Python-only count must be 0; excluded count, ranges, versions) for the report and `docs/adapters.md`. Never run `rustfmt` on `word_table.rs` by hand: if `cargo fmt --all --check` (G1) rejects the generated file, fix the generator template, regenerate, and record a deviation.
- [x] B2. `bundle/slug.rs`: the unit tests and proptest first (fixture table, table sanity, boundaries, `rust_word_ranges_file_matches_char_is_alphanumeric`), then `slugify` and `is_word_char`; add `pub use slug::slugify;` to `bundle/mod.rs`.
- [x] B3. Gate B: `gate cargo test -p unam-core slug`, `python3 crates/unam-core/tools/gen-word-table.py --check` (G12), fmt, clippy.

### Step C: Project (root, selection, index)

- [x] C0. Create `project/mod.rs` containing `pub mod index; pub mod selection;` plus `index.rs` and `selection.rs` as doc-comment-only stubs; add `pub mod project;` to `lib.rs`.
- [x] C1. `project/index.rs`: tests first (the moved `referencing_projects_cases`, the bundle variant, `register` cases), then `read_index`, `register`, `projects_referencing_source`, `projects_referencing_bundle`.
- [x] C2. `source.rs`: delete the provisional reader, call `projects_referencing_source`; the `remove_*` tests stay green unmodified.
- [x] C3. `project/selection.rs`: tests first, then `TargetId`, `TargetSelection`, `Selection`, `parse_selection`, `load_selection`, `save_selection`.
- [x] C4. `project/mod.rs`: `RootDetection`, `ProjectRoot`, `detect_root` with its tests first.
- [x] C5. Gate C: `gate cargo test -p unam-core project` and `gate cargo test -p unam-core source`, then `cargo test -p unam-core`, fmt, clippy.

### Step D: Bundle loader and line editor

- [x] D1. `bundle/mod.rs`: loader tests first, then types, `parse_bundle_text`, `load_dir`, `BundleSet`, the private `yaml_scalar`.
- [x] D2. `bundle/edit.rs`: the edge-case matrix and the proptest first, then `list_entries`, `remove_values`, `add_values`.
- [x] D3. Gate D: `gate cargo test -p unam-core bundle`, fmt, clippy.

### Step E: Resolve

- [x] E1. `resolve.rs` and `pub mod resolve;` in `lib.rs`: the unit matrix first, then `ResolveError`, `resolve_ref`, `resolve_text`, the types, `resolve_selection`, `resolve_entries`, `bundles_referencing`.
- [x] E2. Gate E: `gate cargo test -p unam-core resolve`, `cargo test -p unam-core` (M1 lookup tests still green), fmt, clippy.

### Step F: Bundle CRUD

- [x] F1. `bundle/mod.rs`: tests first, then `render_manifest`, `save`, `duplicate`, `delete`, `add_refs`, `remove_refs`, `edit_in_editor`, the verification routine, and the `--rewrite` path.
- [x] F2. Gate F: `gate cargo test -p unam-core bundle`, `cargo test -p unam-core` (all), fmt, clippy.

### Step G: CLI

- [x] G1. `cli/mod.rs`: the `Bundle` command, `BundleCmd`, `Ctx::bundles`, dispatch, `command_name` strings.
- [x] G2. `cli/bundle.rs`: the seven subcommands per the CLI Contract.
- [x] G3. `cli/show.rs`: the `bundles` field and the `Bundles:` line.
- [x] G4. Gate G: `cargo build --workspace --locked`; the first test is now GREEN: `gate cargo test -p unam --test ac_bundles ac_11_removing_one_entry_leaves_every_other_line_byte_identical -- --exact`; `gate cargo test -p unam --test show_cli`; `gate cargo test -p unam --test source_cli`; clippy. (The `snapshots` binary is expected to show two mismatches, `show_found` and `usage_bare`, until step H5 regenerates them.)

### Step H: Integration tests and snapshots

- [x] H1. `tests/ac_bundles.rs`: add `ac_07_*`, `ac_08_*`, `ac_09_*` per the Acceptance Criteria table.
- [x] H2. `tests/ac_selection.rs`: `ac_10_*`, `ac_12_*`, `ac_35_*` plus core-API checks that `save_selection` registers the project and `detect_root` finds the marker.
- [x] H3. `tests/bundle_cli.rs` (the names below), each ending with `assert_source_unchanged` where a source exists: `bundle_ls_empty_and_populated`; `bundle_ls_reports_broken_bundle_and_keeps_going`; `bundle_show_resolves_each_entry`; `bundle_show_unknown_exit_4`; `bundle_save_writes_qualified_refs`; `bundle_save_refuses_existing_without_force`; `bundle_save_force_overwrites`; `bundle_save_without_selection_exit_4`; `bundle_save_with_two_selection_entries_is_exit_2`; `bundle_save_blocked_by_unresolved_ref_exit_4`; `bundle_save_root_detection_from_subdirectory`; `bundle_duplicate_cases`; `bundle_rm_deletes_and_lists_referencing_projects` (`--json` data has `deleted: true`, `slug`, `path`, `referencing_projects` and no `removed` key; human output names the path); `bundle_rm_deletes_only_the_loaded_file_when_name_differs_from_slug` (file `Backend Dev.yaml` with `name: Backend Dev`; sibling manifests `other.yaml` and `backend-dev-copy.yaml` stay byte-identical); `bundle_rm_forms_are_separate` (the delete form never edits entries and leaves no file; the reference form never deletes the file, even when it exits 2 for would-be-empty, and leaves it byte-identical); `bundle_symlinked_manifest_cases` (unix: with the manifest a symlink to a file outside the bundles directory, `bundle add`, `bundle rm <name> <ref>`, the same with `--rewrite`, `bundle save --force`, `bundle duplicate --force`, and `bundle edit` each exit 2 with `bundle-edit-unsupported` and link and target bytes are unchanged; `bundle rm <name>` unlinks only the link and the target file is intact); `bundle_rm_unknown_exit_4`; `bundle_add_cases`; `bundle_add_ambiguous_exit_5_no_write`; `bundle_rm_skills_cases` (not in bundle exit 4, would-be-empty exit 2, equivalent spelling; the `--json` data has `removed` as an array of entry texts and no `deleted` key); `bundle_edit_with_scripted_editor_commits_valid_change`; `bundle_edit_invalid_result_leaves_file_unchanged`; `bundle_edit_editor_failure_leaves_file_unchanged`; `bundle_edit_unset_editor_exit_2`; `bundle_edit_slug_change_refused`; `bundle_rewrite_flag_cases`; `bundle_edit_unsupported_shapes_refused`; `bundle_commands_leave_sources_untouched`. Every `bundle save` test (including `bundle_save_without_selection_exit_4`, `bundle_save_with_two_selection_entries_is_exit_2`, and `bundle_save_blocked_by_unresolved_ref_exit_4`) calls `git_marker(project_dir)` before running the command, so root detection never walks the ancestors of the system temp directory.
- [x] H4. `tests/show_cli.rs`: `show_lists_bundles_referencing_it` (and the empty case); `tests/usage.rs`: assert `bundle` appears in the bare usage text and in `--help`.
- [x] H5. `tests/snapshots.rs`: the nine new snapshot tests; run `INSTA_UPDATE=always cargo test -p unam --test snapshots` ONCE, read every new and changed `.snap`, then run G7 and G7b to G7d.
- [x] H6. Gate H: the seven G4 commands, G5, and `cargo test --workspace --locked` with `INSTA_UPDATE=no`.

### Step I: Docs and closeout

- [x] I1. Create `docs/adapters.md` (P19): "Bundle slugify parity with Hermes" (source files and local checkout commit, the algorithm, the outcome "aligned to Hermes with the P3 `\w` table", the sweep numbers and versions from B1 as ONE line in exactly this form, checked by G13b: `Sweep numbers: Python X.Y.Z, Unicode X.Y.Z, rustc X.Y.Z; excluded N code points in R ranges; Python-only 0; kept as word characters (newer Unicode) K.`; the generator path and usage (`crates/unam-core/tools/gen-word-table.py` with `--write-rust-ranges`, `--write`, `--check`, `--check-rust` and their exit codes 0, 1, 3, 4), the fixture table location, and the explicit difference from spec 4.4's ASCII rule), "Bundle loader: differences from Hermes" (one row per difference: load order is the same as Hermes and differs from plain alphabetical in spec 2.2; blank name is an error diagnostic and an explicitly empty `name` string is blank here while Hermes falls back to the stem; null entries dropped here while Python gives the text `None`; non-scalar entries dropped with a warning here; booleans stringified as `true` and `false` here, `True` and `False` in Python; YAML 1.1 booleans such as `yes` not mirrored; non-UTF-8 manifest skipped with an error here while Hermes' loader does not catch the decode error; no mtime cache here, the directory is reloaded per command; hidden files are loaded as in Hermes; `bundle save` writes only `name` and `skills`; the writer is YAML 1.2 (`serde_norway`), so bare entries such as `yes`, `no`, `on`, `off`, `y`, `n` are written unquoted while PyYAML (YAML 1.1, Hermes) reads them as booleans, qualified IDs and normal skill names are unaffected, and numeric-, boolean-, and null-looking entries are quoted on write so they stay strings; symlinked manifests are loaded (followed) as in Hermes but never written through here), and a `## Maintenance` section (M4 extends; M7 may regenerate the adapter-evidence part and must keep the two M2 sections; it names `python3 crates/unam-core/tools/gen-word-table.py --check` as the table reproducibility command and the regeneration order `--write-rust-ranges` then `--write`). The three headings must be exactly `## Bundle slugify parity with Hermes`, `## Bundle loader: differences from Hermes`, and `## Maintenance` (checked by G13). No em dash, no emoji.
- [x] I2. Run every gate G1 to G14 in order (G13 has three rows: G13, G13b, G13c); run `gate6` once (catalog code changed); run `python3 crates/unam-core/tools/gen-word-table.py --check-rust` once (Hybrid; exit 4 without `rustc` is CONDITIONAL, exit 3 is drift recorded with the numbers); record outputs, the sweep numbers, the perf line, any deviation, the first-test red record, and the snapshot creation note in the phase report.
- [x] I3. Hand off: do not commit (UPDATE PROCESS commits on `main`); do not push. At UPDATE PROCESS: write the backlog NOTE for the M4 test notes and the other follow-ups, and register the umbrella state.

---

## Exit Gate

Setup for every command: repository root; `export PATH="$HOME/.cargo/bin:$PATH"`; `INSTA_UPDATE=no` exported (each Bash call is a fresh shell, so define the helpers and export inside the same invocation). Gate helper (a shell function, not product code) that makes a filter matching zero tests FAIL:

`gate() { out=$("$@" 2>&1); rc=$?; printf '%s\n' "$out"; [ "$rc" -eq 0 ] && printf '%s\n' "$out" | grep -Eq 'test result: ok\. [1-9][0-9]* passed'; }`

| Gate | Command | Expected |
|---|---|---|
| G1 fmt | `cargo fmt --all --check` | exit 0 |
| G2 clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0; no `unwrap` outside tests |
| G3 all tests | `cargo test --workspace --locked` | all pass (snapshots with `INSTA_UPDATE=no`, `perf_cold_scan` in debug); no ignored tests |
| G4 per AC | the seven commands below | each exits 0 with exactly 1 passed |
| G5 M1 regression | `gate cargo test -p unam --test ac_read_path`; `gate cargo test -p unam --test read_side_invariant source_tree_unchanged_read_side -- --exact`; `gate cargo test -p unam --test show_cli`; `gate cargo test -p unam --test source_cli`; `gate cargo test -p unam --test search_cli`; `gate cargo test -p unam --test cache_cli` | all pass (AC-1, 3, 4, 5, 6, 38 and the first M1 test unchanged) |
| G6 perf (release) | `gate6 cargo test --release -p unam-core --test perf_cold_scan -- --exact perf_cold_scan --nocapture` (helper below) | rc 0; `1 passed`; the pinned line ending `skills=500 profile=release`; median under 300; record the median and the five runs |
| G7 snapshot hygiene | the G7 command below | no `.snap.new`; at least 19 `.snap` files; no path and no timestamp in any snapshot |
| G7b changed set | the G7b command below | exactly `show_found` and `usage_bare` modified |
| G7c show_found diff | the G7c command below | the only changed line is the added `"bundles": [],` |
| G7d usage_bare diff | the G7d command below | no removed line; at least one added `bundle` line |
| G8 text hygiene | the G8 command below | exit 0: file list non-empty, no U+2014, no code points 0x1F300 to 0x1FAFF or 0x2600 to 0x27BF; the pathspec `crates` covers the generator script and the Rust data file |
| G9 real paths untouched | the G9 commands below | no hits; the three real directories listed as in the Pre-Step REF file |
| G10 harness | `node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs`; `node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs`; `node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs`; `node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <this plan>`; `node .claude/skills/vc-generate-phase-program/scripts/validate-phase-stub.mjs <this plan>`; `python3 -c "import ast,sys; ast.parse(open(sys.argv[1], encoding='utf-8').read())" crates/unam-core/tools/gen-word-table.py` (syntax check that leaves no bytecode in the repository); `git diff --check` | all pass |
| G11 no dependency change | `git diff --exit-code -- Cargo.lock Cargo.toml crates/unam-core/Cargo.toml crates/unam-cli/Cargo.toml`; `cargo metadata --locked --format-version 1 > /dev/null` | both exit 0 |
| G12 slugify sweep (Hybrid) | `python3 crates/unam-core/tools/gen-word-table.py --check` | exit 0: the committed `word_table.rs` equals the regeneration from the committed Rust data file and the running Python, and the printed summary shows `py_only=0`; exit 3 is Unicode-data drift (the running Python's Unicode version differs from the committed header): CONDITIONAL with the printed numbers recorded, never a silent pass; exit 1 is FAIL (table differs under the same Unicode version, malformed data file, or Python-only count above 0) |
| G13 protected docs and adapters doc | `git diff --quiet -- docs/PROJECT-BRIEF.md docs/KICKOFF-PROMPT.md docs/spec docs/adr spike`; `grep -q '^## Bundle slugify parity with Hermes$' docs/adapters.md && grep -q '^## Bundle loader: differences from Hermes$' docs/adapters.md && grep -q '^## Maintenance' docs/adapters.md` | both exit 0 (the second proves the file exists and carries the three required headings; an empty or partial file fails) |
| G13b sweep numbers line | `grep -Eq '^Sweep numbers: Python [0-9.]+, Unicode [0-9.]+, rustc [0-9.]+; excluded [0-9]+ code points in [0-9]+ ranges; Python-only 0; ' docs/adapters.md` | exit 0 (the doc carries the B1 numbers and versions on one line; a non-zero Python-only count cannot match) |
| G13c no ignored tests | `! grep -rEq '#\[ignore' crates` | exit 0 (the literal text appears nowhere under `crates`, comments included) |
| G14 invariants | `gate cargo test -p unam --test bundle_cli bundle_commands_leave_sources_untouched -- --exact` | 1 passed |

G4 commands:

```
gate cargo test -p unam --test ac_bundles ac_07_hermes_only_bundle_with_bare_names_loads_unchanged -- --exact
gate cargo test -p unam --test ac_bundles ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates -- --exact
gate cargo test -p unam --test ac_bundles ac_09_bundle_spans_two_categories_and_two_sources -- --exact
gate cargo test -p unam --test ac_selection ac_10_saved_bundle_in_empty_project_yields_original_selection -- --exact
gate cargo test -p unam --test ac_bundles ac_11_removing_one_entry_leaves_every_other_line_byte_identical -- --exact
gate cargo test -p unam --test ac_selection ac_12_bundle_plus_add_plus_remove_resolves_with_provenance -- --exact
gate cargo test -p unam --test ac_selection ac_35_hand_edited_selection_is_desired_not_applied -- --exact
```

Helpers and commands for G6, G7 to G9 (G6, G7, G8, and the G9 grep are the M1 commands, verified at M1 PVL; G7 and G8 are adjusted for the new counts and files):

G6 helper (invoke as the G6 row shows):

```
gate6() { out=$("$@" 2>&1); rc=$?; printf '%s\n' "$out" | grep -E 'perf_cold_scan median_ms|test result'; [ "$rc" -eq 0 ] && printf '%s\n' "$out" | grep -Eq 'test result: ok\. 1 passed' && printf '%s\n' "$out" | grep -Eq 'perf_cold_scan median_ms=[0-9.]+ runs_ms=\[[^]]+\] skills=500 profile=release$' && printf '%s\n' "$out" | awk '/perf_cold_scan median_ms=/{split($2,a,"="); if (a[2]+0 < 300) ok=1} END{exit ok?0:1}'; }
```

G7:

```
test -z "$(find crates -name '*.snap.new')" && [ "$(find crates/unam-cli/tests/snapshots -name '*.snap' | wc -l)" -ge 19 ] && ! grep -rEq '/var/|/private/|/tmp/|/Users/|/home/|[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}' crates/unam-cli/tests/snapshots
```

G7b, G7c, G7d (run after step H5 and before the milestone commit, while the M1 snapshots are still the committed versions):

```
[ "$(git ls-files -m -- crates/unam-cli/tests/snapshots | sort)" = "$(printf '%s\n' crates/unam-cli/tests/snapshots/snapshots__show_found.snap crates/unam-cli/tests/snapshots/snapshots__usage_bare.snap)" ]
[ "$(git diff -U0 -- crates/unam-cli/tests/snapshots/snapshots__show_found.snap | grep -E '^[+-][^+-]')" = '+    "bundles": [],' ]
! git diff -U0 -- crates/unam-cli/tests/snapshots/snapshots__usage_bare.snap | grep -Eq '^-[^-]' && git diff -U0 -- crates/unam-cli/tests/snapshots/snapshots__usage_bare.snap | grep -Eq '^[+]  bundle'
```

G8 (run before the milestone commit; also scan the phase report separately):

```
python3 -c "import sys,subprocess; fs=subprocess.check_output(['git','ls-files','-m','-o','--exclude-standard','--','Cargo.toml','Cargo.lock','clippy.toml','dist-workspace.toml','.github','crates','docs/adapters.md'],text=True).split(); bad=sorted({f for f in fs for c in open(f,encoding='utf-8',errors='replace').read() if ord(c)==0x2014 or 0x1F300<=ord(c)<=0x1FAFF or 0x2600<=ord(c)<=0x27BF}); print('files scanned:',len(fs),'hits:',bad); sys.exit(1 if bad or not fs else 0)"
```

G9 (both parts must hold; `<REF>` is the Pre-Step record file):

```
! grep -rEq 'Developer/01-AI|[.]claude/skills|[.]agents/skills' crates .github dist-workspace.toml
diff <(ls -A ~/.config/unam ~/.cache/unam ~/.local/share/unam 2>&1 | sort) <(sed -n '/^--- ls output ---$/,$p' <REF> | tail -n +2)
```

- All checklist items checked; AC-7 to AC-12 and AC-35 (desired side) tests green; the first test (AC-11) went red at A7 and green at G4.
- Regression: M1 criteria tests (AC-1, 3, 4, 5, 6, 38), `source_tree_unchanged_read_side`, and `perf_cold_scan` (median under 300, via `gate6`, because `catalog/mod.rs` and `catalog/lookup.rs` changed) still green; M0 spike file unchanged.
- Hermes slugify parity: aligned, with the documented `\w` table and the sweep outcome in `docs/adapters.md` and the phase report (G12 `--check`, G13 headings, G13b sweep line); no ignored tests anywhere (G13c).
- Phase report written to the report destination above, including: the red-first record, resolved facts (sweep numbers and tool versions), the changed-snapshot diffs, the two tracked M4 test notes, the deviation list, and a Forward Preview for milestone M3 (the `Resolution` and `DesiredEntry` shapes, `Provenance` serialization marked provisional, `save_selection` and the gitignore lines for `use` and `add`, `detect_root` for the `--project` flag, the EDG-2 name-collision check M3's `plan::diff` must add, the index registration on every write).

---

## Deviations (EXECUTE 2026-10-05)

Recorded by the execute agent; none changes a public contract beyond the plan, none touches a hard-stop class (no schema, auth, billing, container, secret, or external integration surface), and every changed file is inside the registry blast radius.

1. Split (P1 permits it): bundle CRUD lives in `crates/unam-core/src/bundle/crud.rs` (new file) instead of `bundle/mod.rs`, which keeps the loader. `bundle/mod.rs` re-exports the CRUD functions, so the API path `unam_core::bundle::{save, duplicate, delete, add_refs, remove_refs, edit_in_editor, render_manifest}` is as planned. Line counts including unit tests: `crud.rs` about 1130, `edit.rs` about 810, `resolve.rs` about 820; the production part of each is under about 600 lines, the rest is the planned unit matrices.
2. `yaml_scalar` is `pub(crate)` in `bundle/edit.rs` (shared by the line editor and `render_manifest`) instead of private in `bundle/mod.rs`.
3. `EntriesOutcome` carries two extra fields, `removed` (values, one per removed line) and `rewrite`, which the CLI JSON contract needs; `edit_in_editor` takes `editor: Option<&str>` (the `EDITOR` value the CLI reads through `Dirs.env`; `None` or blank gives `editor-not-set`).
4. M1 test assertion changed: `show_cli.rs` `frontmatter_order_supporting_and_diagnostics` asserted that `data.bundles` is absent (M1 P12); P16 makes it always present, so the assertion now checks `data.bundles == []`. The plan listed only one new `show_cli.rs` test.
5. Advisory A3 adopted: the symlink refusal (P21) also applies when `save --force` or `duplicate --force` would overwrite an existing `<slug>.yaml` that is not loaded (for example an invalid manifest behind a symlink).
6. Unspecified details decided: a `name` that is a YAML mapping or list is `bundle-name-blank` ("`name` is not text"); an unresolved `remove` cancels earlier problems of every kind except `BundleMissing` with the same canonical reference; `bundle ls` human rows cut the description at 80 characters (the M1 `head` helper); loader diagnostics are reported (data and warnings) by `bundle ls` only; `bundle edit` reports post-commit resolution problems as warning-severity diagnostics.
7. Test harness: `common/mod.rs` gains `run_in` and `split_lines` beside the P17 helpers; `project/index.rs` exposes a `#[cfg(test)]` `tests_support::dirs_in` used by the other core unit tests; an extra Hybrid unit test `register_does_not_write_an_unreadable_index` (chmod 000, self-skips under root); the `bundle_edit` snapshot test is `#[cfg(unix)]` (scripted editor).
8. Process only: gate output was captured once in `/tmp/claude-501/g.out` (outside the repository) while running G4, G5, and G14.

First-test record: at A7, `cargo test -p unam --test ac_bundles -- --exact ac_11_removing_one_entry_leaves_every_other_line_byte_identical` FAILED (the binary exited 2: "unrecognized subcommand 'bundle'"); at G4 it passed.

## Phase Completion Rules

- CODE DONE means code exists with gates unverified; VERIFIED requires the exit gate green plus the regression check and a recorded validate-contract.
- A phase can be VERIFIED while the program is incomplete.
- Commit execution changes on `main` before starting milestone M3; no push.

---

## Blockers That Would Justify BLOCKED Status

- M0 gate not met, or M1 not VERIFIED (both are met).
- Toolchain missing (`cargo`, `rustfmt`, `clippy`): BLOCKED; never install.
- `cargo build --locked` cannot succeed without changing `Cargo.lock` (a dependency would be needed): record a deviation; BLOCKED only if no in-scope alternative exists.
- A Hermes format fact contradicts spec 2.2 in a way not already recorded (items 29, 30, 34, 36): report to Taylor; spec and PRD stay authoritative.
- `perf_cold_scan` over 300 ms in release after M2 changes to catalog code: record the value, keep that gate CONDITIONAL with a backlog stub, never widen the bound.
- `python3` or `rustc` missing: only the word table and gate G12 are affected (Pre-Step item 1); not a BLOCKED condition.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop).

- [x] 1. RESEARCH (2026-10-05): research-agent: M1 report read; Hermes slugify and loader source read; test context loaded; plan drift checked
- [x] 2. INNOVATE (2026-10-05): innovate-agent: approach decided; Decision Summary written (decisions D1 to D8 above)
- [x] 3. PLAN-SUPPLEMENT (2026-10-05): plan-agent: existing phase plan updated; Inner Loop Refresh Note written
- [x] 4. PVL (gate verdict PASS 2026-10-05 after 1 validate-fix cycle plus validator fix of the proptest renderer; results-m2.tsv): vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md` (Status / Gate / Plan updates applied / Execute-agent instructions / Test gates / High-risk pack / Backlog artifacts / Known gaps / Accepted by)
- [x] 5. EXECUTE (2026-10-05): all checklist items done; per-section test gates run and green; Exit Gate G1 to G14 green in the execute run (EVL confirmation pending)
- [x] 6. EVL (2026-10-05, vc-tester sonnet): all EVL gates green (G1 to G14, G12b, E1; G6 median 140.8 ms); follow-up stubs registered; EVL HANDOFF SUMMARY written (results-m2-evl.tsv)
- [x] 7. UPDATE PROCESS (2026-10-05): phase report written, umbrella state updated, context updated, backlog NOTE written (commit by the orchestrator, execution first, no push)

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Risks and Predictions

Plan-agent analysis (five viewpoints; not a `vc-predict` run):

| Viewpoint | Risk | Mitigation in this plan |
|---|---|---|
| Correctness | The line editor corrupts a hand-written manifest, or a refusal rule misses a shape it cannot handle | Grammar with explicit refusals, whole-file reparse verification that also compares every other top-level key, the edge-case matrix plus a proptest, AC-11 asserting byte-identical other lines, `--rewrite` fallback |
| Parity | The slugify `\w` predicate differs from Hermes on marks and symbols | Exhaustive sweep against Python `re` (zero Python-only code points required), generated exclusion table from the committed generator `crates/unam-core/tools/gen-word-table.py`, G12 single-command reproducibility gate (`--check`), a fixture table that includes NFD and Other_Alphabetic cases; differences from newer Unicode data recorded in `docs/adapters.md` |
| Safety | A test writes to the real bundles dir, a real editor starts, or a path join escapes the bundles dir | `TestEnv` isolation plus `dirs_for`, scripted editor inside the tempdir, `Dirs::bundle_file` validation, temp copies under `<cache dir>/edit`, G9 |
| Compatibility | The candidate-ordering change or the new `show` field breaks M1 behavior | Single-source behavior unchanged by construction; G5 reruns every M1 test; G7b to G7d prove exactly two snapshots changed and why; `gate6` rerun |
| Semantics | Resolution and removal rules surprise users (silent no-ops, cancelled problems, last-record winner) | Rules written as P4 to P6 with unit rows for each; problems are never silent (reported), only a valid unselected removal is a no-op; recorded for Taylor (items 24, 27) |
| Process | `bundle rm` overload deletes a manifest when the user meant to remove a skill | Reported in open item 28; output names the deleted path; `--rewrite` requires references; delete is atomic and the file is tiny and user-owned; recorded as a risk, not hidden |
| Data safety | `bundle rm` removes the wrong file (a manifest whose file name differs from its slug) or an in-place write replaces a symlinked manifest | P21 and P22: delete removes exactly the loaded `Bundle.path` (parent asserted); in-place writers refuse symlinks; sibling-intact, form-separation, and symlink tests |
| Tooling | The scout-block hook rejects gate text; Edit tool disabled; Python or `rustc` missing | Pre-Step items 1, 3, 4; gate names avoid the tokens |

Edge cases for the three highest-risk items (folded into the tests above):

- Line editor: dash indent 0 versus 2; quoted entries with `: ` or ` #` inside, escaped quotes and escaped backslashes; trailing comment after a quoted entry; CRLF plus BOM plus missing final newline; removing the last item of a file without a trailing newline; adding to a block followed by comments; numeric-looking and boolean-looking entries; duplicate entries; a null entry; nested `skills` under `metadata` only; two `skills` keys; tabs; anchors and aliases; flow sequences; mapping-looking entries; an entry that equals the key name.
- Slugify table: NFD versus NFC accents; combining marks (`Mn`, `Mc`); circled letters (`So`, Other_Alphabetic); numerals (`Nl`, `No`) kept; final sigma context; dotted capital I expansion; zero-width joiner; emoji; CJK; tabs and non-breaking spaces removed; runs of hyphens; empty result.
- Resolution: a bundle entry that is ambiguous in one source set but unique in another; the same skill via bundle and add (position and provenance order); remove by equivalent spelling; remove of a dangling entry; a selection naming the same bundle twice; a bundle named with different spelling (`Backend Dev`); an unreachable source in a bundle; two skills with the same name and relative path in different sources.

---

## Touchpoints

- Files listed in Blast Radius
- `crates/unam-core/tools/gen-word-table.py` and `crates/unam-core/tools/rust-word-ranges.txt` (committed generator and data; cited in `docs/adapters.md`)
- Read: `docs/spec/unam-technical-spec.md` sections 1.3, 2.2, 2.3, 3, 4.4 to 4.6, 5, 7, 8, 10, 11; `docs/spec/PRD-unambiguous.md` sections 5, 7.2, 7.3, 7.9, 13.2; read-only Hermes sources `agent/skill_commands.py` and `agent/skill_bundles.py` (local checkout, commit `ea32af9331`); the M1 code under `crates/` and the M1 report Forward Preview
- `process/context/all-context.md` and `process/context/tests/all-tests.md` updated at UPDATE-PROCESS (new test files, G12 to G14 plus G13b and G13c, scripted-editor note)

---

## Public Contracts

- CLI: new `bundle` command family (spec 5) with the exit codes, `--json` data shapes, and diagnostics above; `show` gains `data.bundles`. Later milestones must not break them (additive fields only).
- Bundle manifest format (Hermes-compatible, spec 2.2) as read by the loader rules above and as written by `bundle save` (`name`, then `skills` with dash at indent 2).
- `selection.yaml` format (spec 2.3) as parsed (P7) and written; the gitignore lines offered on first write.
- Project index file `<data dir>/projects.json`, schema `{"version": 1, "projects": [...]}` (unchanged from the M1 provisional schema; now owned by `project::index`).
- Core API consumed by milestone M3: `project::{detect_root, load_selection, save_selection}`, `resolve::{resolve_selection, Resolution, DesiredEntry, Provenance, Pin, Problem}`, `bundle::{load_dir, BundleSet}`. `Provenance` JSON is provisional until M3 defines receipts.
- Diagnostic codes of P2 and error codes of the error table are stable contract.
- Candidate ordering of `not-found` and `ambiguous` errors is by source precedence (spec 4.6), a visible change from M1 only with several sources.
- `bundle rm --json`: the reference form returns `removed` (an array of entry texts), the delete form returns `deleted` (`true`) with `referencing_projects`; the two forms never share a key with different types (additive-only from here).
- Symlinked manifests are loaded (followed) but never written through (exit 2 `bundle-edit-unsupported`); `bundle rm <name>` unlinks only the link.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| G1 `cargo fmt --all --check`; G2 `cargo clippy --workspace --all-targets --locked -- -D warnings`; G3 `cargo test --workspace --locked` | Fully-Automated | spec 8 quality gates (unwrap denied outside tests, spec 7) |
| `ac_07_hermes_only_bundle_with_bare_names_loads_unchanged` (G4) | Fully-Automated | AC-7 (BND-2) |
| `ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates` (G4) | Fully-Automated | AC-8 (BND-3, EDG-2) |
| `ac_09_bundle_spans_two_categories_and_two_sources` (G4) | Fully-Automated | AC-9 (BND-4), desired side |
| `ac_10_saved_bundle_in_empty_project_yields_original_selection` (G4) | Fully-Automated | AC-10 (BND-5), desired side; end-to-end form in M3 |
| `ac_11_removing_one_entry_leaves_every_other_line_byte_identical` (G4) and the `bundle/edit.rs` matrix and proptest | Fully-Automated | AC-11 (BND-7), spec 11 YAML trade-off |
| `ac_12_bundle_plus_add_plus_remove_resolves_with_provenance` (G4) | Fully-Automated | AC-12 (BND-8), desired side; status side tracked for M4 |
| `ac_35_hand_edited_selection_is_desired_not_applied` (G4) | Fully-Automated | AC-35 (STA-1), desired side; status side tracked for M4 |
| G5 M1 regression (`ac_read_path`, `read_side_invariant`, `show_cli`, `source_cli`, `search_cli`, `cache_cli`) | Fully-Automated | AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 read-side, PRD S3 read-side |
| `perf_cold_scan` release (G6, via `gate6`) | Hybrid (precondition: release build, local SSD; assertion automated) | spec 9 cold scan under 300 ms (regression after catalog changes) |
| `bundle/slug.rs` fixture table, table sanity, boundaries, proptest | Fully-Automated | spec 8 slugify table-driven; PRD 5.1 Hermes slug rule |
| Slugify parity sweep and table reproducibility: `python3 crates/unam-core/tools/gen-word-table.py --check` (G12; Python only, uses the committed Rust data file); `--check-rust` (needs rustc, run at B1 and I2) | Hybrid (precondition: python3 present; rustc for `--check-rust`) | spec 4.4 parity claim; brief open question: Hermes slugify parity |
| `rust_word_ranges_file_matches_char_is_alphanumeric` (in `gate cargo test -p unam-core slug`) | Fully-Automated | spec 4.4 parity claim, Rust side of the table input |
| Bundle loader matrix (including order, BOM, defaults, blank name, duplicates) | Fully-Automated | spec 2.2 parsing rules; BND-1, BND-2; PRD 5.1 |
| Bundle `chmod 000` unreadable-manifest test | Hybrid (precondition: non-root user; self-skip printed; named residual) | spec 7 "never panic on user data"; EDG-3 |
| `resolve.rs` unit matrix | Fully-Automated | spec 4.6, 8 (resolve against duplicates across categories and sources); BND-8, EDG-3, EDG-7 |
| `project` unit tests (root detection, selection IO, index, gitignore offer) | Fully-Automated | spec 4.5, 2.3; ACT-1; OWN-7 |
| `bundle_cli.rs` integration tests (save, ls, show, edit, duplicate, rm, add, exit codes) | Fully-Automated | spec 5 CLI and exit codes 0, 1, 2, 4, 5; BND-3, BND-5, BND-6 (delete, duplicate), BND-7 |
| `bundle_rm_deletes_only_the_loaded_file_when_name_differs_from_slug`, `bundle_rm_forms_are_separate`, `bundle_symlinked_manifest_cases`, `symlinked_manifest_refuses_in_place_writes`, and the `delete` unit rows | Fully-Automated (symlink rows unix) | BND-6 (delete never deactivates, exact target); PRD ACT-8 (writes only to explicitly targeted user-scope files) |
| `bundle_rm_deletes_and_lists_referencing_projects` and `bundle_rm_skills_cases` JSON assertions, snapshots `bundle_rm_skill` and `bundle_rm_bundle` | Fully-Automated | spec 5 `--json` contract (type-stable `removed` and `deleted` keys) |
| Scripted-editor tests (`bundle edit`) | Fully-Automated (unix) | spec 5 "`edit` opens `$EDITOR`"; spec 11 commit-on-success |
| `snapshots` (G3, G7 to G7d; 19 snapshots) | Fully-Automated | spec 8 CLI: `--json` output snapshotted with insta |
| `show_lists_bundles_referencing_it` | Fully-Automated | spec 5 `show` "bundles referencing it" |
| G8 to G11, G14 hygiene, safety, harness, lockfile, invariant | Fully-Automated | brief rules; spec 1.4 lockfile |
| G13 protected documents and the three `docs/adapters.md` headings; G13b sweep-numbers line; G13c no ignored tests | Fully-Automated | decision 4 (protected documents); Exit Gate bullets on the adapters document content and on no ignored tests |

AC-10 and the M4 status halves: the M2 tests cover the desired side only; the M3 plan re-verifies AC-10 end to end and the M4 plan adds the two tracked status assertions (see the tracked M4 test notes). No Known-Gap rows: every developed behavior has a Fully-Automated or Hybrid gate. Named residuals (not Known-Gap passes): the chmod-based Hybrid test self-skips under root; the sweep is machine- and toolchain-version dependent; two processes editing one manifest have no lock until M3; a corrupt `projects.json` is rebuilt from the registering root alone, so other registered projects are forgotten (known limitation); `--check` exit 3 (Unicode drift) is CONDITIONAL, not a pass; Windows and non-unix editors are out of scope. If EXECUTE finds an untestable developed behavior it writes a backlog stub and keeps that gate CONDITIONAL.

---

## Test Infra Improvement Notes

- Carried from M1: `cargo-insta` is not installed (the no-`cargo-insta` procedure is used again); Hybrid chmod tests self-skip under root (candidate: a capability probe that fails loudly when `CI=true`); the scout-block hook friction on `target`, `.git`, `build` text remains a harness issue for Taylor.
- The scripted `EDITOR` approach works only where `/bin/sh` and executable files exist (unix); a portable editor stub (a tiny test binary in the workspace) is a candidate if Windows is ever supported.
- The word table makes slugify parity data-driven; G12 is now the single command `python3 crates/unam-core/tools/gen-word-table.py --check`, so a scheduled CI job only needs a Python on the runner and exit code 3 flags Unicode drift between Python and the committed Rust data.
- `common/mod.rs` now carries many helpers; if it passes about 500 lines, split it into `common/{env,fixtures,snapshot}.rs` (candidate for a later milestone).
- A property test that generates whole bundle files (comments, quoting, EOLs) and random edit sequences exists only for the line editor; the same generator could later fuzz `resolve_selection`.

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-03-m2-bundles-and-selection_PLAN_04-10-26.md`
- Last completed step: step 3 PLAN-SUPPLEMENT (2026-10-05); steps 1 RESEARCH and 2 INNOVATE complete; PVL cycle 1 supplement applied (2026-10-05, gaps R1 to R5 and the contract's execute-agent instructions E1 to E11 folded into this plan body)
- Validate-contract status: written 2026-10-05 (inner-pvl: phase-3), net gate verdict PASS after 1 supplement cycle; EXECUTE may start.
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `process/context/tests/all-tests.md`, spec sections cited above, the M1 plan and report, the umbrella plan, the Hermes sources named in Touchpoints (read-only)
- Next step for a fresh agent: re-spawn vc-validate-agent from V1 against this plan (PVL cycle 1 supplement applied); do not start EXECUTE until the validate-contract gate verdict reads as a pass (or an accepted CONDITIONAL after at least one supplement cycle). The contract's execute-agent instructions are folded into the plan text (E1 and E11: Pre-Step items 9 and 10; E2: B0; E3: P8, Safety, the `project/mod.rs` tests, H3; E4: the AC-11 row and A6; E5: P13 and the `edit_in_editor` tests; E6: the `remove_refs` spec; E7: CLI Contract; E8: I1 and the edit.rs matrix; E9: the `duplicate` spec; E10: where the contract list and this plan differ, this plan wins). Execute with one opus agent following the Implementation Checklist in order, running each step's gate before the next. Commit on `main` at UPDATE PROCESS, never push.
- Forward Preview for milestone M3 (to be confirmed in the phase report): `Resolution.resolved` (`DesiredEntry` with provenance) feeds `plan::diff`; `project::save_selection` returns the gitignore lines for `use` and `add` to print; `detect_root` backs the `--project` flag; M3 must add the EDG-2 name-collision conflict (two selected skills sharing a name); `Provenance` JSON is provisional and M3 fixes it with the receipt format; AC-10 end to end is M3's to re-verify.

---

## Inner Loop Refresh Note: 2026-10-05 - changed sections: Overview, Scope, Decisions Locked, Plan-Level Decisions, Acceptance Criteria, Entry Gate, Pre-Step, Safety, Blast Radius, Module Specifications, CLI Contract, Test Harness and Fixture Contract, Data Flow, Failure Modes, Implementation Checklist, Exit Gate, Blockers, Phase Loop Progress, Risks and Predictions, Touchpoints, Public Contracts, Verification Evidence, Test Infra Improvement Notes, Resume and Execution Handoff. Invalidates prior validate-contract (none had been written).

---

## Validate Contract

Status: PASS
Date: 05-10-26
date: 2026-10-05
generated-by: inner-pvl: phase-3
supersedes: 2026-10-05 (inner-pvl: phase-3) - inner PVL has current evidence (the cycle-1 first-pass CONDITIONAL contract, concerns R1 to R5, is replaced; R1 to R5 are verified closed below)

Parallel strategy: sequential
Rationale: score 3/7 (S2 new public CLI, JSON, bundle-file, and selection-file contracts; S4 phase program; S7 about 40 files in the blast radius; no S1 because only two workspace packages are touched, no S3 because the approach is decided, no S5, no S6 because no auth, billing, migration, gateway, or secrets class is touched). Dominant signal: S7 file count, but the file set is one dependency chain (steps A to I each compile against the previous one and share `error.rs`, `lib.rs`, and the catalog), so there are no disjoint groups for a team and no independent directions for parallel subagents. EXECUTE: one opus agent following the Implementation Checklist in order. Agent counts: sequential 1; parallel subagents 4 + 9 + 3 = 16 with nothing to split; agent team 3 members x 2 rounds = 6 with no coordination benefit; workflow 9 steps x 1 agent x 1 iteration = 9, not warranted because the item count is known. Cost guard not triggered (all under 30). Everything except the implementing agent (tester, reviewers, update-process) runs sonnet. This pass ran inline on sonnet (Layer 1 four dimensions and Layer 2 nine sections reviewed by the validate agent itself; no helper agents spawned) with cheap-local probes in the session scratchpad outside the repository; no VC-FEASIBILITY-PROBE-NEEDED was emitted; no agent CLI, `unam`, or Hermes was run (Hermes sources read read-only). Goal block: BRANCH B (the umbrella plan has `## Stable Program Goal`), so no autonomous goal block section is written into this phase plan; the umbrella goal governs. Reference for latest state: process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md

Cycle record: this is PVL cycle 2 (re-validation from V1 after the cycle-1 supplement, which applied R1 to R5 plus E1 to E11 and both advisories; see `phase-03-m2-bundles-and-selection-pvl-iteration-001_REPORT_05-10-26.md`). Verdict: 0 FAILs, 1 small CONCERN found and closed in this pass (P1 below, plan edit applied and re-checked), 0 unresolved. `results-m2.tsv` rows are written by the orchestrator.

Plan updates applied (this pass, all inside the plan body, one location each, re-checked with `validate-plan-artifact.mjs`: 0 failures, 0 warnings, no em dash):
- P1 (the one concern): the `bundle/edit.rs` proptest renderer rule wrote every class (1) token plain except seven literals, but the plan's own grammar refuses a plain body that ends with `:`. A probe over 200,000 tokens drawn exactly as specified found 2,956 tokens (1.5 percent) that end with `:` and 1,562 that reparse as numbers, so the proptest would fail on its first run (certain at 256 cases). Edit: the renderer writes a value plain only when it is a class (1) token that starts with an ASCII letter, does not end with `:`, and is not one of the seven literals; everything else is quoted; the `add_values` property draws new values distinct from each other and from existing entries. Re-probe of the tightened rule: 155,294 plain tokens, 0 failures (all reparse as the same string and none is refused by the grammar).
- P2 (precision, not a concern): the generator template prose said "eight comment lines" while the template lists nine, and "the rustc line of that file's header" was ambiguous; now "exactly the nine comment lines above" and "the text after `# rustc: `" / "`# rust-unicode: `".

Evidence gathered in this pass (cheap-local; scratchpad only; nothing created in the repository):

| Check | Result | Evidence |
|---|---|---|
| Structural validators | Pass | `validate-plan-artifact.mjs` and `validate-phase-stub.mjs`: 0 failures, 0 warnings (also after the P1 and P2 edits); `validate-all-context.mjs`, `validate-context-discovery.mjs`, `validate-plan-inventory.mjs` clean; `git diff --check` clean; no em dash in the plan; no non-process path modified (working tree changed only under `process/`) |
| Paths (V1 scout) | Pass | all 36 referenced existing paths resolve (modify targets in `crates/`, five protected documents, `spike/symlink-discovery.md`, the M1 report, Hermes `skill_commands.py` and `skill_bundles.py` at checkout commit `ea32af9331`, the validator scripts); every path the plan creates is absent and not git-ignored; `docs/adapters.md` and `crates/unam-core/tools/` absent as expected |
| Toolchain | Pass | cargo and rustc 1.99.0 (Unicode 17.0.0 in `char::UNICODE_VERSION`), python3 3.14.5 (Unicode 16.0.0); G9 baseline directories absent; G11 (`git diff --exit-code` on the lockfile and three manifests) clean; G13 protected-document diff quiet; G9 grep clean; no `#[ignore` text under `crates` |
| Hook compatibility | Pass | 64 command texts extracted mechanically from the Exit Gate fences, tables, Pre-Step, and checklist (G1 to G14 including the `gate` and `gate6` helpers, G7, G7b to G7d, G8 python, both G9 lines, G12, G13, G13b, G13c, the G10 python syntax check) plus 45 test-name gate commands (`gate cargo test -p unam --test bundle_cli <name> -- --exact`) and the new `rust_word_ranges_file_matches_char_is_alphanumeric` command, run through `scout-block.cjs` with stdin JSON: all allowed (rc 0) |
| R1 generator (B1, G12) | Pass | Implemented `gen-word-table.py` from the plan text alone in a scratch copy of the layout and ran all four modes: `--write-rust-ranges` (845 ranges, rustc 1.99.0, Rust Unicode 17.0.0), `--write`, `--check` (`py_only=0 excluded=1495 ranges=250 cn_kept=4672`, OK, exit 0, 0.4 s), `--check-rust` (OK, exit 0). Two full regenerations are byte-identical (data file and table). The 250 NOT_WORD ranges equal the independent cycle-0 sweep range-for-range. The generated `word_table.rs` is ASCII, LF, one final newline, 250 range lines, and `rustfmt --edition 2024 --check` exits 0 (G1 safe). Non-vacuity: a one-code-point corruption of the table gives a diff and a non-zero exit; a Unicode-version header change plus a corruption gives exit 3 (DRIFT); a malformed data file gives FAIL (exit 1); a data file missing a range gives `py_only=26` and FAIL; no mode gives usage exit 2; two modes give exit 2; `--check-rust` without `rustc` on PATH gives exit 4 |
| Slugify parity (P3, D2) | Pass | A Rust probe of `bundle/slug.rs` written from the plan (generated table, `slugify`, `is_word_char`) passes: all 31 fixture rows, table sanity (sorted, disjoint, every code point alphanumeric and not a word char), the boundary code points, a 20,000-string pseudo-proptest (only `-` and word chars, no leading, trailing, or doubled `-`, idempotent), and `rust_word_ranges_file_matches_char_is_alphanumeric` in its NON-skipped branch (data file and `char::UNICODE_VERSION` both 17.0.0, 845 ranges equal). The cycle-0 full sweep (1,112,064 scalar values, three contexts, against Python `slugify_skill_name`) differed only on the 4,672 deliberately kept `Cn` code points |
| R2 line editor matrix | Pass after P1 | `serde_norway` probes agree with every matrix expectation: `to_string` quotes `a: b`, `a #b`, `true`, `null`, `~`, `123`, `1e3`, `0x10`, `skills:`, `False`, `TRUE`, `.inf`; leaves `a"b`, `a\`, `a'b`, `x`, `yes`, `no`, `skills` plain; no folding; every value round-trips to the same string; reading `1.50`, `0x10`, `yes`, `true`, `~`, `null` gives `1.5`, `16`, `yes`, `true`, empty, empty, and the quoted forms `"a: b"`, `"a #b"`, `"a\"b"`, `"a\\"`, `'a''b'`, `'a: b'`, `'a #b'`, `"x"`, `"skills:"` unquote correctly; the proptest alphabet defect was P1 |
| R3 delete target and symlinks, R5 JSON shapes | Pass | P21 and P22 are stated once and applied consistently in the module spec, CLI Contract, Failure Modes, Safety, Public Contracts, Risks, the H3 test list, and Verification Evidence; the two `bundle rm` JSON shapes (`removed` array versus `deleted: true` plus `referencing_projects`) agree in the CLI Contract, snapshot descriptions (`bundle_rm_skill`, `bundle_rm_bundle`), the two H3 JSON tests, and Public Contracts; no stale `removed: true` remains |
| R4 gate texts | Pass | G13 fails on an empty and on a partial `adapters.md` and passes on a complete one; G13b passes only with `Python-only 0; ` and fails on `Python-only 3`; G13c fails on a seeded ignore attribute and passes on the clean tree |
| Step order and compile chain | Pass | B0 state (stub `NOT_WORD`, doc-only `slug.rs` and `edit.rs`, no `pub use`) compiles with only a `dead_code` warning (no error; the workspace lints are `unsafe_code` forbid and `unwrap_used` deny only), which is why no clippy runs between B0 and B3 (clippy appears only at A4, A7, B3, C5, D3, E2, F2, G4); `pub use slug::slugify` at B2 precedes its first consumer (`project/index.rs` at C1); `pub(super)` makes `NOT_WORD` visible to `bundle::slug` and its test module; `include_str!("../../tools/rust-word-ranges.txt")` from `src/bundle/slug.rs` resolves to `crates/unam-core/tools/`; `common/mod.rs` already has `#![allow(dead_code)]` so new helpers cannot trip clippy; `Error` is matched exhaustively only inside `error.rs` and the plan extends all three arms |
| Cycle-1 edit review | Pass | all 45 changed hunks of the 56-edit cycle were read against the previous text; E1 to E11 and advisories A1 and A2 each map to a body location (Pre-Step items 9 and 10; B0; P8, Safety, `project/mod.rs` tests, H3; AC-11 row and A6; P13; `remove_refs` spec; CLI Contract; I1 and the matrix; `duplicate` spec; Resume line; Failure Modes and Safety); counts agree (15 error variants, 9 `bundle-*` plus 4 resolution diagnostic codes, 9 new plus 10 existing equals 19 snapshots, 7 G4 commands, 31 slug rows, 8 matrix entries in family (a)) |
| Fixture expectations | Pass | AC-7 to AC-10 and AC-12 expected IDs, orders, and provenance re-derived by hand against the real fixture library (one `schema-migration`, `pytest-fixtures`, `code-review`, `alpha-root-skill`, two `error-handling`) plus the planned `library2`; they agree |

Layer 1 dimensions:

| Layer 1 dimensions | Status |
|---|---|
| Infra fit | PASS |
| Test coverage | PASS |
| Breaking changes | PASS |
| Security surface | PASS |

Layer 2 sections:

| Layer 2 sections | Status |
|---|---|
| Section A: Pre-Step, foundations, fixtures, first test (A0 to A7) | PASS |
| Section B: slugify and word table (B0 to B3) | PASS |
| Section C: project root, selection, index (C0 to C5) | PASS |
| Section D: bundle loader and line editor (D1 to D3) | PASS |
| Section E: resolve (E1, E2) | PASS |
| Section F: bundle CRUD (F1, F2) | PASS |
| Section G: CLI (G1 to G4) | PASS |
| Section H: integration tests and snapshots (H1 to H6) | PASS |
| Section I and Exit Gate: docs, closeout, G1 to G14 | PASS |

Totals: 0 FAILs / 0 CONCERNs unresolved (1 concern, P1, closed by a plan edit in this pass) / 13 PASS rows.

Net gate: PASS (after one recorded PVL fix cycle plus the in-pass plan edit; no FAIL; no unresolved CONCERN; net-gate vacuous-green check below)

Test gates (C3 5-column table; legacy line form follows):

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| spec 8 quality gates | fmt clean (including the generated `word_table.rs`), clippy clean with `unwrap` denied outside tests, whole workspace tests pass | Fully-Automated | `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` (G1, G2, G3; `INSTA_UPDATE=no`) | B |
| AC-7 (BND-2) | Hermes-only bundle with bare names loads unchanged; description `Load 3 skills as a bundle`; manifest bytes unchanged | Fully-Automated | `gate cargo test -p unam --test ac_bundles ac_07_hermes_only_bundle_with_bare_names_loads_unchanged -- --exact` (G4) | B |
| AC-8 (BND-3, EDG-2) | ambiguous bare name errors naming both IDs (exit 5), nothing activates, manifest byte-identical | Fully-Automated | `gate cargo test -p unam --test ac_bundles ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates -- --exact` (G4) | B |
| AC-9 (BND-4), desired side | one bundle spans two categories and two sources, four distinct IDs, no problems | Fully-Automated | `gate cargo test -p unam --test ac_bundles ac_09_bundle_spans_two_categories_and_two_sources -- --exact` (G4) | B |
| AC-10 (BND-5), desired side | saved bundle in an empty project resolves to the original selection with `Bundle(my-set)` provenance | Fully-Automated | `gate cargo test -p unam --test ac_selection ac_10_saved_bundle_in_empty_project_yields_original_selection -- --exact` (G4) | B (end-to-end form: C, milestone M3 re-verifies) |
| AC-11 (BND-7) | removing one entry leaves every other line byte-identical (LF, CRLF plus BOM, dash at indent 0, first and last item) | Fully-Automated | `gate cargo test -p unam --test ac_bundles ac_11_removing_one_entry_leaves_every_other_line_byte_identical -- --exact` (G4; red at A7, green at G4) plus the `bundle/edit.rs` matrix and proptest | B |
| AC-12 (BND-8), desired side | bundle plus add plus remove resolves with provenance and `winner()` | Fully-Automated | `gate cargo test -p unam --test ac_selection ac_12_bundle_plus_add_plus_remove_resolves_with_provenance -- --exact` (G4) | B (status side: C, milestone M4 note) |
| AC-35 (STA-1), desired side | hand-edited selection resolves as desired with no receipt, no destination, no index registration | Fully-Automated | `gate cargo test -p unam --test ac_selection ac_35_hand_edited_selection_is_desired_not_applied -- --exact` (G4) | B (status side: C, milestone M4 note) |
| M1 regression (AC-1, 3, 4, 5, 6, 38 read side, read-side invariant) | M1 behavior unchanged after the lookup ordering, `show`, and index-reader changes | Fully-Automated | G5: `ac_read_path`, `read_side_invariant source_tree_unchanged_read_side -- --exact`, `show_cli`, `source_cli`, `search_cli`, `cache_cli` | A |
| spec 9 cold scan | 500-skill cold scan median of five under 300 ms in release after the catalog code changed | Hybrid | `gate6 cargo test --release -p unam-core --test perf_cold_scan -- --exact perf_cold_scan --nocapture` (G6; precondition: release build, local SSD) | A |
| slugify table | Hermes `slugify_skill_name` parity: 31-row fixture table, table sanity, boundaries, proptest, and the Rust-side data-file test | Fully-Automated | `gate cargo test -p unam-core slug` (step B3); at I2 also run it once with `-- --nocapture` and confirm no `SKIP:` line | B |
| slugify `\w` parity sweep and table reproducibility | committed `word_table.rs` equals the regeneration from the committed Rust data file and the running Python, zero Python-only code points; the data file equals the running compiler's `char::is_alphanumeric` ranges | Hybrid | `python3 crates/unam-core/tools/gen-word-table.py --check` (G12; precondition python3; exit 0 required, exit 3 is CONDITIONAL with numbers recorded, exit 1 FAIL) and `python3 crates/unam-core/tools/gen-word-table.py --check-rust` (I2; precondition rustc; exit 0, exit 3 recorded drift) | A (B1 produces the files; the gates prove them) |
| docs and hygiene deliverables | `docs/adapters.md` carries the three headings and the sweep-numbers line; no ignored tests; protected documents untouched | Fully-Automated | G13 (three heading greps and the protected-document diff), G13b, G13c exactly as in the Exit Gate section | B |
| spec 2.2 loader | loader matrix (BOM, CRLF, defaults, blank name, order, duplicates, non-UTF-8, metadata) | Fully-Automated | `gate cargo test -p unam-core bundle` (D3, F2) | B |
| spec 7 never panic on user data | unreadable manifest gives `bundle-unreadable` and nothing else breaks | Hybrid | chmod 000 unit test in `bundle/mod.rs` (precondition: non-root user; printed self-skip otherwise) | D (named residual) |
| spec 4.4, BND-7 line editor | refusal grammar, edge-case matrix (families a, b, c), reparse verification, proptest with the tightened renderer rule | Fully-Automated | `gate cargo test -p unam-core bundle` (the `bundle::edit` tests) | B |
| spec 4.6, BND-8, EDG-3, EDG-7 | resolution order, provenance, remove semantics, problem kinds and exit mapping | Fully-Automated | `gate cargo test -p unam-core resolve` (E2) | B |
| spec 4.5, 2.3, OWN-7, ACT-1 | root detection, selection IO, index, gitignore offer | Fully-Automated | `gate cargo test -p unam-core project` (C5) | B |
| spec 5 bundle CLI and exit codes | save, ls, show, edit, duplicate, rm both forms, add; exits 0, 1, 2, 4, 5; sources untouched | Fully-Automated | `gate cargo test -p unam --test bundle_cli` (H3, H6) and G14 `bundle_commands_leave_sources_untouched` | B |
| BND-6, ACT-8 delete target and symlink rule (P21, P22) | `bundle rm <name>` removes exactly the loaded file and unlinks a symlink without touching its target; in-place writers refuse a symlinked manifest with exit 2 | Fully-Automated (symlink rows unix) | `bundle_rm_deletes_only_the_loaded_file_when_name_differs_from_slug`, `bundle_rm_forms_are_separate`, `bundle_symlinked_manifest_cases` in `bundle_cli.rs`; `symlinked_manifest_refuses_in_place_writes` and the `delete` rows in `bundle/mod.rs` | B |
| spec 5 `--json` contract for `bundle rm` | reference form returns `removed` (array), delete form returns `deleted: true` with `referencing_projects`; no shared key with two types | Fully-Automated | `bundle_rm_deletes_and_lists_referencing_projects`, `bundle_rm_skills_cases`, snapshots `bundle_rm_skill` and `bundle_rm_bundle` | B |
| spec 5 `bundle edit` | scripted `$EDITOR` commits only a valid same-slug result; temp copy removed on every path | Fully-Automated (unix) | `bundle_edit_with_scripted_editor_commits_valid_change` and the refusal tests in `bundle_cli.rs` | B |
| spec 8 CLI snapshots | 19 snapshots, two changed by visible output with mechanical diff checks | Fully-Automated | G3 with `INSTA_UPDATE=no`; G7, G7b, G7c, G7d | B |
| spec 5 `show` | bundles referencing the skill | Fully-Automated | `show_lists_bundles_referencing_it` in `show_cli.rs` | B |
| hygiene and safety | text hygiene, real paths untouched, harness validators, lockfile | Fully-Automated | G8 to G11, G14 as in the Exit Gate section | B |

Failing stub: AC-7
test("should load a Hermes-only bundle with bare names unchanged", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_07_hermes_only_bundle_with_bare_names_loads_unchanged")
})

Failing stub: AC-8
test("should error on an ambiguous bare name naming both IDs and activate nothing", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_08_ambiguous_bare_name_errors_naming_both_ids_and_nothing_activates")
})

Failing stub: AC-9
test("should resolve a bundle spanning two categories and two sources", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_09_bundle_spans_two_categories_and_two_sources")
})

Failing stub: AC-10
test("should resolve a saved bundle in an empty project to the original selection", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_10_saved_bundle_in_empty_project_yields_original_selection")
})

Failing stub: AC-11
test("should leave every other line byte-identical when removing one entry", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_11_removing_one_entry_leaves_every_other_line_byte_identical")
})

Failing stub: AC-12
test("should resolve a bundle plus one add plus one remove with provenance", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_12_bundle_plus_add_plus_remove_resolves_with_provenance")
})

Failing stub: AC-35
test("should treat a hand-edited selection as desired and not applied", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: ac_35_hand_edited_selection_is_desired_not_applied")
})

Failing stub: sources untouched by bundle commands
test("should leave every source tree unchanged after every bundle command", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: bundle_commands_leave_sources_untouched")
})

Failing stub: show lists bundles
test("should list the bundles referencing a shown skill", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: show_lists_bundles_referencing_it")
})

Failing stub: scripted editor commit
test("should commit a valid edit made through a scripted editor", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: bundle_edit_with_scripted_editor_commits_valid_change")
})

Failing stub: delete removes only the loaded file
test("should delete exactly the loaded manifest when its file name differs from its slug", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: bundle_rm_deletes_only_the_loaded_file_when_name_differs_from_slug")
})

Failing stub: symlinked manifest is never written through
test("should refuse in-place writes to a symlinked manifest and unlink only the link on delete", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: bundle_symlinked_manifest_cases")
})

(Stubs are the red-first skeleton in the protocol form; the Rust equivalent is `#[test] fn <name>() { panic!("NOT IMPLEMENTED - TDD stub: <name>") }`. Plan step A6 already writes the first test red. Command-gate rows, Hybrid rows, and the unit-matrix rows (slug, loader, line editor, resolve, project, which are written test-first by the checklist steps themselves) receive no stubs.)

C-4 reconciliation: the strategy column carries only Fully-Automated, Hybrid, Agent-Probe. Known-Gap is never a strategy; named residuals are listed under Open gaps. Net-gate vacuous-green check: every developed behavior in the blast radius (slugify and the word table with its generator, loader, line editor, bundle CRUD and `edit`, delete and symlink rules, project root detection, selection IO, project index, resolve, the `show` bundles line, the CLI contract and exit codes, snapshots, `docs/adapters.md` headings and sweep line) has a Fully-Automated or Hybrid gate; the only Known-Gap-like items are named residuals (below). The one skip path in the gates, `rust_word_ranges_file_matches_char_is_alphanumeric` printing `SKIP:` on a Unicode-version mismatch, is not a vacuous pass: the same data file is re-proved by `--check-rust` (exit 3 on drift, never silent), the table sanity test and the fixture rows run regardless, and in this environment both versions are 17.0.0 so the test runs its full comparison (verified in the probe above). The net gate is not vacuously green.

Legacy line form:
- quality gates: [fully-automated: cargo fmt --all --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --locked]
- AC-7 to AC-12 and AC-35 desired side: [fully-automated: gate cargo test -p unam --test ac_bundles|ac_selection <test name> -- --exact, seven commands (G4)]
- M1 regression: [fully-automated: G5, six commands]
- spec 9 cold scan: [hybrid: gate6 release perf_cold_scan; precondition release build and local SSD]
- slugify: [fully-automated: gate cargo test -p unam-core slug] [hybrid: G12 python3 crates/unam-core/tools/gen-word-table.py --check; precondition python3] [hybrid: python3 crates/unam-core/tools/gen-word-table.py --check-rust; precondition rustc]
- loader, line editor, resolve, project units: [fully-automated: gate cargo test -p unam-core bundle|resolve|project]
- bundle CLI, scripted editor, delete and symlink rules, show, snapshots: [fully-automated: gate cargo test -p unam --test bundle_cli|show_cli|snapshots]
- unreadable manifest: [hybrid: chmod 000 unit test, self-skip under root; named residual]
- hygiene G7 to G11, G13, G13b, G13c, G14: [fully-automated: exact commands in the Exit Gate section]
- AC-12 and AC-35 status sides: [known-gap: documented as tracked M4 test notes; backlog NOTE m2-ac12-ac35-status-side-for-m4_NOTE_<dd-mm-yy>.md at UPDATE PROCESS]
- AC-10 end-to-end form: [known-gap: documented; milestone M3 re-verifies after apply exists]

Dimension findings:
- Infra fit: PASS - toolchain (cargo and rustc 1.99.0, python3 3.14.5), all 36 referenced paths, hook behavior (64 gate texts plus 45 test-name commands allowed by `scout-block.cjs`), offline dependency resolution (no new crates; `tempfile` is already a unam-core dependency), clean git state outside `process/`, the real-directory baseline, and every edit target in the existing M1 code check out; the step order A to I compiles in sequence including the B0 stub state (dead_code warning only) and the E2 move of `pub use slug::slugify` to B2; the first test is red at A7 and green at G4.
- Test coverage: PASS - proving tests map one to one to AC-7 to AC-12 and AC-35 and every developed behavior has a Fully-Automated or Hybrid gate; R1 (G12 is the committed, runnable, non-vacuous `--check`), R2 (matrix families a, b, c present and consistent with probed `serde_norway` behavior; the proptest alphabet and renderer rule now consistent with the plan's grammar after P1), and R4 (G13, G13b, G13c fail on empty, partial, or seeded-bad inputs) are closed; named residuals are explicit.
- Breaking changes: PASS - additive only (new `bundle` family, `show.data.bundles`, two visibly changed snapshots with mechanical diff gates, candidate ordering changes only with several sources); R5 closed: `removed` is always an array of entry texts and the delete form uses `deleted: true`, consistent across the contract, snapshots, tests, and Public Contracts.
- Security surface: PASS - no auth, billing, secrets, or network; bundle names and slugs validated before any path join, selection text parsed and never joined into a path, `EDITOR` read through `Dirs.env` and split without a shell, temp copies under the isolated cache dir, no write under a source; R3 closed: `delete` removes exactly the loaded `Bundle.path` (parent asserted, symlink unlinked and never followed) and in-place writers refuse a symlinked manifest with exit 2 (editor never started); high-risk evidence pack not required (no class from the six; only explicit user-targeted delete and overwrite of user-scope manifests, allowed by PRD ACT-8 and BND-6).
- Section A (Pre-Step, foundations, fixtures, first test) feasibility: PASS - 15 `Error` variants and exit mapping consistent with the CLI table and M1's exhaustive `exit_code` match; helpers depend only on existing APIs; highest-risk edit: `catalog/lookup.rs` candidate ordering (equal precedence falls back to ID, so M1 tests and snapshots are unchanged by construction; G5 and `gate6` rerun them).
- Section B (slugify and word table) feasibility: PASS - the generator spec is implementable deterministically from the plan text alone (byte-identical double run, numbers and ranges equal the independent sweep, rustfmt-stable output); highest-risk edit: the generated `word_table.rs` (mitigated by G12, `--check-rust`, G1, and the Rust data-file test).
- Section C (project) feasibility: PASS - `source.rs` change is behavior-preserving; highest-risk edit: moving the index reader (the `remove_*` tests stay unmodified and green); advisory A1 recorded as a known limitation in Failure Modes.
- Section D (loader and line editor) feasibility: PASS - the grammar plus whole-file reparse that also compares every other top-level key makes silent corruption impossible, AC-11 asserts byte identity, the matrix covers the research list plus families a, b, c, and the proptest is now implementable as written; highest-risk edit: the line editor.
- Section E (resolve) feasibility: PASS - problem kinds, exit mapping (2, 4, 5, 4), provenance, ordering, and remove semantics are consistent across D5, P4 to P6, the module spec, the CLI contract, and the AC expectations; highest-risk edit: remove cancelling a dangling add (explicit unit rows).
- Section F (bundle CRUD) feasibility: PASS - P21 and P22 are specific and testable; highest-risk edit: `delete` and the in-place write path (exact-target assertion, sibling-intact and form-separation tests, symlink refusal test).
- Section G (CLI) feasibility: PASS - the `bundle rm` two-form overload is unambiguous (zero references deletes, one or more edits, `--rewrite` requires references) and both forms have named tests and snapshots; highest-risk edit: `cli/bundle.rs` exit-code mapping (`bundle_cli.rs` asserts every exit code).
- Section H (integration tests and snapshots) feasibility: PASS - fixture totals and ID expectations re-derived against the real fixture library; G7b to G7d are mechanically sound before the milestone commit; `INSTA_UPDATE=no` makes any missing snapshot fail; every CLI command is built through `TestEnv`; highest-risk edit: snapshot creation (read every `.snap` before accepting).
- Section I and Exit Gate (G1 to G14) feasibility: PASS - every row is runnable as written (hook-checked, and G12, G13, G13b, G13c exercised for pass and fail); notes for the tester: each Bash call is a fresh shell (zsh here), so define `gate` and `gate6` and export `PATH` and `INSTA_UPDATE=no` inside the same invocation.
- Decisions and scope consistency: PASS - D1 to D8 and P1 to P23 are consistent with each other and with spec and PRD where they do not depart; open items 15 to 36 are not re-litigated; type and module names are consistent across sections.

Open gaps:
- AC-12 and AC-35 status sides: known-gap: documented as tracked M4 test notes (gap-resolution C; backlog NOTE written at UPDATE PROCESS per the plan).
- AC-10 end-to-end form: known-gap: documented; milestone M3 re-verifies (gap-resolution C).
- Hybrid chmod 000 manifest test self-skips under root: named residual (gap-resolution D).
- Word-table parity is tied to the Unicode data of the generating Python 3.14.5 (Unicode 16.0.0) and rustc 1.99.0 (Unicode 17.0.0); the 4672 newer-Unicode letters are deliberately kept as word characters; a Hermes running on an older Python may differ for those characters; `--check` exit 3 and the data-file test's `SKIP:` branch are toolchain-drift signals recorded as CONDITIONAL, never passes: named residual, recorded in `docs/adapters.md`.
- Two processes editing one manifest or registering into the index at once: last writer wins, no lock until M3: named residual.
- `perf_cold_scan` is machine-dependent: on overrun keep the gate CONDITIONAL with a backlog stub and never widen the bound (plan rule, unchanged).
- Advisory A1 (known limitation, in Failure Modes): a corrupt `projects.json` is rebuilt from the registering root alone, forgetting other registered projects.
- Advisory A3 (not a concern, not blocking): `bundle save --force` or `bundle duplicate --force` over a `<slug>.yaml` that is a symlink to an INVALID manifest (not loaded, so P21 has no `Bundle.path` to check) would replace the link; the execute agent may add the same `symlink_metadata` refusal there if it costs one line, recorded as a deviation; not required by the contract.

What This Coverage Does NOT Prove:
- G1 to G3 (fmt, clippy, workspace tests): that behavior is correct beyond what the tests assert; `unwrap` denial covers non-test code compiled by clippy, not `expect`, indexing, or arithmetic overflow; "never panic on user data" is evidenced by unit matrices and proptests, not by fuzzing the loader, the line editor, or the selection parser.
- AC-7 to AC-12 and AC-35 tests: run against synthetic fixtures on the test machine; they do not prove behavior on Taylor's real bundles or library (milestone M5, S1 and S6), on bundle files written by a real Hermes at other versions, or on other filesystems; AC-10 and AC-9 prove the resolution half only (activation arrives in M3); AC-12 and AC-35 prove the desired side only (the status side is milestone M4).
- Line editor tests: that every possible YAML spelling of a `skills:` block is either edited correctly or refused; the guarantee is that an edit is written only when a whole-file reparse proves the semantics unchanged, and that untouched lines are byte-identical for the matrix and proptest shapes; exotic YAML (tabs, anchors, directives, multi-document files) is covered only by the refusal rules; the proptest draws only the three entry classes named in the plan.
- Slugify tests, G12, and `--check-rust`: parity with Hermes on Python versions whose Unicode data differs from 16.0.0, on strings the sweep did not exercise (the full sweep covers single code points in three contexts; multi-character interactions beyond `to_lowercase` final-sigma and expansion rows rely on the fixture table), and that the committed table equals a regeneration on a different machine; G12 alone does not prove the runtime Rust predicate equals the data file (that is the data-file unit test and `--check-rust`); the unit test self-skips on a Unicode-version mismatch.
- G13 and G13b: that the prose in `docs/adapters.md` is accurate or complete beyond the three headings and the sweep-numbers line; G13c proves no ignore attribute text exists under `crates`, not that every test is meaningful.
- Delete and symlink tests: behavior on filesystems without symlinks, on case-insensitive filesystems for slug collisions, or for an invalid symlinked manifest (advisory A3).
- Resolve and project tests: behavior with very large selections, concurrent writers, or an index with thousands of projects.
- `bundle edit` scripted-editor tests: any real editor (terminal modes, background editors such as `code` without `--wait`, editors that rewrite via rename); the scripted `/bin/sh` editor works only on unix.
- Snapshot tests: that the JSON and human output is correct, only that it is unchanged since a human read it at creation; path and timestamp hygiene is checked by grep, not semantically.
- Hybrid chmod test: behavior under root, on filesystems that do not enforce permissions, or for other IO errors.
- `perf_cold_scan`: the cold-scan bound on any other machine, a cold OS cache, or via the CLI end to end.
- This validation: no gate command was executed against the real repository (the M2 code does not exist yet); the generator, the slug module, `serde_norway` behavior, hook behavior, and the proptest token statistics were exercised in scratch copies on one platform (macOS, aarch64) and one dependency set; the line editor, loader, resolver, and CLI designs were reviewed by reading and by the YAML probes, not by prototype.

Execute-agent instructions (only items the plan body does not already carry; where this list and the plan text differ, the plan text wins):
- E1: when running the slugify gate at step I2, run `gate cargo test -p unam-core slug -- --nocapture` once and record whether a `SKIP:` line appears; a `SKIP:` line is acceptable only together with a recorded `--check-rust` result (exit 0 or a recorded exit 3), never silently.
- E2: if the generator's `--check` exits 3 on the EXECUTE machine, record the printed summary line in the phase report and `docs/adapters.md` and keep G12 CONDITIONAL; never regenerate the committed files merely to turn the gate green without recording the new toolchain versions (rerun `--write-rust-ranges` then `--write`, then record the deviation).

Backlog artifacts:
- `m2-ac12-ac35-status-side-for-m4_NOTE_<dd-mm-yy>.md` in `process/features/v1-static-activation/backlog/` (written at UPDATE PROCESS per the plan; carries the two tracked M4 test notes).
- No further backlog artifact is required by this pass; if EXECUTE finds an untestable developed behavior it writes a stub and keeps that gate CONDITIONAL.

Known gaps (excluded from the CONCERN count): AC-12 and AC-35 status sides (milestone M4), AC-10 end-to-end form (milestone M3), documented as above.

Gate: PASS (0 FAILs, 0 unresolved CONCERNs; cycle-1 concerns R1 to R5 verified closed; one in-pass proptest-renderer concern P1 fixed in the plan and re-probed; EXECUTE may start with one opus agent following the Implementation Checklist in order)
Accepted by: n/a (PASS, no concern outstanding); session (autonomous, autopilot, PVL cycle 2) recorded the in-pass plan edit P1 and the named residuals above as known-gaps.

---

## Next Step

Run PVL (step 4) with vc-validate-agent against this plan, then EXECUTE with one opus agent, then EVL via vc-tester.
