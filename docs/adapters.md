# Adapters and Hermes compatibility

This document records how unam stays compatible with tools it reads from or writes for. Milestone M2 created it with the two bundle sections below; milestone M4 extends it with adapter evidence (see Maintenance).

## Bundle slugify parity with Hermes

TL;DR: unam slugifies bundle names exactly like Hermes `slugify_skill_name`, including Unicode, using a generated exclusion table. This intentionally differs from the ASCII-only rule in spec 4.4.

Source read (read-only, never executed): `agent/skill_commands.py` (`slugify_skill_name`) and `agent/skill_bundles.py` (`_load_bundle_file`, `bundle_path_for`, `save_bundle`) in a local Hermes checkout at commit `ea32af9331`, read on 2026-10-05.

Hermes algorithm (Python):

1. `name.lower()`
2. replace each space and each `_` with `-`
3. delete every character that is not Python `\w` (Unicode `re`) and not `-`
4. collapse runs of `-` to one `-`
5. strip `-` from both ends

An empty result is an error in unam (`invalid-bundle-name` when saving, `bundle-empty-slug` when loading). The slug is used for lookup (bundle names in a selection are slugified), for the saved file name (`<slug>.yaml`), and for duplicate detection. The slug is validated as a single path component before any path join.

Outcome: aligned to Hermes. The Rust port (`crates/unam-core/src/bundle/slug.rs`) uses `str::to_lowercase` (the same full-string lowercasing, final sigma included) and a word predicate `c == '_' || (c.is_alphanumeric() && !NOT_WORD.contains(c))`. Rust's `char::is_alphanumeric` is a superset of Python `\w` minus `_`; the surplus (combining marks and Other_Alphabetic symbols such as circled letters) is listed in the generated table `crates/unam-core/src/bundle/word_table.rs`. Letters that are unassigned (`Cn`) in the running Python but assigned in Rust's newer Unicode data are kept as word characters, because a Hermes on a newer Python would treat them as `\w`.

Sweep numbers: Python 3.14.5, Unicode 16.0.0, rustc 1.99.0; excluded 1495 code points in 250 ranges; Python-only 0; kept as word characters (newer Unicode) 4672.

The excluded code points by category: Mn 927, Mc 438, So 130. Rust's `char::UNICODE_VERSION` at the time of the sweep was 17.0.0.

Generator and checker: `crates/unam-core/tools/gen-word-table.py` (Python 3.10 or newer, standard library only, no network). The Rust side of the comparison is the committed data file `crates/unam-core/tools/rust-word-ranges.txt` (maximal ranges of `c == '_' || c.is_alphanumeric()`), because Python alone cannot compute Rust's ranges. Modes, exactly one per run:

| Mode | Needs | Does | Exit codes |
|---|---|---|---|
| `--write-rust-ranges` | `rustc` | builds and runs a tiny Rust printer in a temp dir, writes `rust-word-ranges.txt` | 0 written; 4 `rustc` not on PATH |
| `--write` | Python | writes `word_table.rs` from the data file and the running Python | 0 written; 1 malformed data or Python-only code points |
| `--check` | Python | validates the data file, sweeps every scalar value, regenerates the table in memory, compares with the committed file | 0 OK; 1 FAIL; 3 DRIFT (Unicode data of this Python differs from the committed header) |
| `--check-rust` | `rustc` | recomputes the Rust ranges and compares them with the data file | 0 OK; 1 FAIL; 3 DRIFT (Rust Unicode version differs); 4 `rustc` not on PATH |

Usage errors (no mode or two modes) exit 2. Exit 3 is a toolchain drift signal and is recorded, never treated as a pass.

Fixture table: the 31-row table in the unit test `fixture_table` in `crates/unam-core/src/bundle/slug.rs` (Hermes test vectors `Backend Dev` gives `backend-dev`, empty gives empty, `!!!` gives empty, plus the rows that differ from the spec 4.4 ASCII rule: `foo.bar` gives `foobar`, `a/b` gives `ab`, precomposed accents and CJK kept, combining marks removed, tabs removed). The test `rust_word_ranges_file_matches_char_is_alphanumeric` proves the data file matches the compiler running the tests (it prints `SKIP:` and passes only when the compiler's Unicode version differs from the file header; `--check-rust` then reports the drift).

Difference from spec 4.4: spec 4.4 describes an ASCII approximation (lowercase, replace any run of characters outside `[a-z0-9]` with `-`, trim `-`) and asks for the Hermes parity check recorded here. unam follows Hermes instead (umbrella open item 25), so an accented or CJK letter is kept, and `foo.bar` becomes `foobar` rather than `foo-bar`.

## Bundle loader: differences from Hermes

TL;DR: unam reads every manifest Hermes reads, in the same order, with the same defaults; the rows below are the deliberate differences.

| Area | Hermes | unam |
|---|---|---|
| Load order | `*.yaml` sorted, then `*.yml` sorted | Same. This differs from the plain alphabetical order described in spec 2.2 (umbrella open item 29). A stem present with both extensions loads `a.yaml` first. |
| Blank name | an empty `name` falls back to the file stem; a blank result is skipped | An explicitly empty or whitespace-only `name` string is an error diagnostic (`bundle-name-blank`) and the bundle is skipped and reported; an absent or null `name` falls back to the stem (umbrella open item 30). |
| Null entries | `str(None)` gives the text `None` | dropped silently |
| Non-scalar entries (mappings, lists, tagged values) | stringified | dropped, with one `bundle-entry-ignored` warning per bundle |
| Booleans | stringified as `True` and `False` | stringified as `true` and `false` |
| YAML 1.1 booleans (`yes`, `no`, `on`, `off`) | read as booleans by PyYAML | read as strings (YAML 1.2, `serde_norway`); not mirrored |
| Non-UTF-8 manifest | the decode error is not caught by the loader | skipped with `bundle-not-utf8` |
| Caching | an mtime cache of the directory | none; the directory is reloaded per command |
| Hidden files | loaded (`Path.glob`) | loaded |
| Description default | `Load {n} skills as a bundle` | same |
| Duplicate slug | first wins | first wins, later files reported with `bundle-duplicate-slug` |
| What `bundle save` writes | `name`, `skills`, plus `description` and `instruction` when given | only `name` and `skills`; `bundle edit` adds the rest |
| Writer quoting | PyYAML (YAML 1.1) | YAML 1.2 (`serde_norway`): bare entries such as `yes`, `no`, `on`, `off`, `y`, `n` are written unquoted and PyYAML (Hermes) would read them as booleans. Qualified IDs and normal skill names are unaffected; numeric-, boolean-, and null-looking entries (`123`, `true`, `null`, `~`, `0x10`, `1e3`) are quoted on write so they stay strings. |
| Symlinked manifests | followed | followed when loading; never written through (in-place edits, `--force` overwrites, and `bundle edit` refuse with `bundle-edit-unsupported`); `bundle rm <name>` unlinks only the link |
| Path-like entries | not special | not special: parsed as references per spec 4.6 |

## Maintenance

- Milestone M4 extends this document with adapter evidence; milestone M7 may regenerate the adapter-evidence part from `unam doctor --evidence` and must keep the two M2 sections above.
- Table reproducibility command: `python3 crates/unam-core/tools/gen-word-table.py --check` (Python only).
- Regeneration order after a toolchain change: `python3 crates/unam-core/tools/gen-word-table.py --write-rust-ranges`, then `python3 crates/unam-core/tools/gen-word-table.py --write`; record the new versions and the new sweep line above.
