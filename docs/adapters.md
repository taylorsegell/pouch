# Adapters and Hermes compatibility

This document records how unam stays compatible with tools it reads from or writes for. Milestone M2 created it with the two bundle sections; milestone M4 added the adapter evidence sections after them (see Maintenance).

## Bundle slugify parity with Hermes

TL;DR: unam slugifies bundle names like Hermes `slugify_skill_name`, including Unicode, using Rust's alphanumeric test as the word-character rule. This intentionally differs from the ASCII-only rule in spec 4.4.

Source read (read-only, never executed): `agent/skill_commands.py` (`slugify_skill_name`) and `agent/skill_bundles.py` (`_load_bundle_file`, `bundle_path_for`, `save_bundle`) in a local Hermes checkout at commit `ea32af9331`, read on 2026-10-05.

Hermes algorithm (Python):

1. `name.lower()`
2. replace each space and each `_` with `-`
3. delete every character that is not Python `\w` (Unicode `re`) and not `-`
4. collapse runs of `-` to one `-`
5. strip `-` from both ends

An empty result is an error in unam (`invalid-bundle-name` when saving, `bundle-empty-slug` when loading). The slug is used for lookup (bundle names in a selection are slugified), for the saved file name (`<slug>.yaml`), and for duplicate detection. The slug is validated as a single path component before any path join.

Outcome: aligned to Hermes. The Rust port (`crates/unam-core/src/bundle/slug.rs`) uses `str::to_lowercase` (the same full-string lowercasing, final sigma included) and the word predicate `c == '_' || c.is_alphanumeric()`.

Documented differences: unam uses Rust `char::is_alphanumeric` plus `_` where Hermes uses Python `\w`. They differ only on some combining marks and symbol letters (for example circled letters), which bundle names do not use in practice. There is no generated table and no parity sweep (simplified on 2026-10-09).

Fixture table: the table in the unit test `fixture_table` in `crates/unam-core/src/bundle/slug.rs` (Hermes test vectors `Backend Dev` gives `backend-dev`, empty gives empty, `!!!` gives empty, plus the rows that differ from the spec 4.4 ASCII rule: `foo.bar` gives `foobar`, `a/b` gives `ab`, precomposed accents and CJK kept, tabs removed).

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

## Adapter evidence: the M0 outcome

TL;DR: every tested agent that scans a destination discovered a symlinked skill directory exactly as it discovered a real copy, so both adapters default to `symlink`. The full run is `spike/symlink-discovery.md`; this section keeps the conclusions each adapter claim rests on.

| Adapter | Destination | Default strategy | Rests on |
|---|---|---|---|
| `claude-code` | `.claude/skills` | `symlink` | Claude Code 2.1.285 (run 2026-10-05): the symlinked probe was listed at start, and a symlink added mid-session appeared with no reload. The cursor-agent CLI, which reads `.claude/skills` for compatibility, found it too. |
| `agents-neutral` | `.agents/skills` | `symlink` | Codex 0.160.0, the cursor-agent CLI, and Hermes (after `hermes skills trust`) all found the symlinked probe. The adapter itself arrives in milestone M5. |

Refresh behavior observed: Claude Code needs nothing for an existing `.claude/skills` directory; Codex needs nothing; the cursor-agent CLI shows a mid-session addition only after a restart; Hermes needs `hermes skills trust <repo>` once and a restart for a mid-session addition. Claude Code does not scan `.agents/skills` at all (no discovery even after a restart, copy or symlink).

Not tested: Windsurf (not installed) and the Cursor editor. The Cursor results are evidence for the `cursor-agent` CLI only; the Cursor editor is not tested, so its behavior stays "not documented".

## Adapter evidence: the claude-code claims

TL;DR: every claim the `claude-code` adapter makes carries a source, and `unam doctor --evidence` prints this same list. A claim marked `assumption` is a guess that has not been checked against the vendor documentation.

The table lists the 13 entries of `Adapter::evidence()` for `claude-code` (`crates/unam-core/src/adapter/claude_code.rs`), in source order, as `unam doctor --evidence` prints them. A unit test fails when a destination, scan location, strategy, refresh, or consumer claim has no entry.

| Claim | Evidence | Status | What it says |
|---|---|---|---|
| `destination` | https://code.claude.com/docs/en/skills | verified | Project skills live in .claude/skills/<name>/SKILL.md |
| `scan:user` | https://code.claude.com/docs/en/skills | verified | Personal skills live in ~/.claude/skills |
| `scan:project` | https://code.claude.com/docs/en/skills | verified | Project skills live in .claude/skills |
| `scan:enterprise` | https://code.claude.com/docs/en/skills | assumption | The enterprise location outranks personal and project scope; the platform paths used are an assumption |
| `scan:nested` | https://code.claude.com/docs/en/skills | verified | Nested .claude/skills directories are discovered (not enumerated in v1) |
| `precedence` | https://code.claude.com/docs/en/skills | verified | Enterprise beats personal beats project when names collide |
| `strategy:symlink` | spike/symlink-discovery.md | verified | A symlinked skill directory is discovered at start and mid-session with no reload (Claude Code 2.1.285, 2026-10-05) |
| `strategy:copy` | spike/symlink-discovery.md | verified | A copied skill directory is discovered like any project skill |
| `refresh:none` | https://code.claude.com/docs/en/skills | verified | Existing skill directories are watched; no restart or reload is needed (except in bare mode) |
| `refresh:command` | https://code.claude.com/docs/en/skills | verified | Run /reload-skills once when the skills directory did not exist at startup |
| `consumers:claude-code` | https://code.claude.com/docs/en/skills | verified | Claude Code reads .claude/skills |
| `consumers:cursor` | https://cursor.com/docs/skills | verified | Cursor reads .claude/skills for compatibility |
| `consumers:windsurf-devin` | https://docs.devin.ai/desktop/cascade/skills | verified | Windsurf and Devin read .claude/skills when Claude config reading is enabled |

Notes:

- `scan:nested`: Claude Code also discovers nested `.claude/skills` directories. unam does not enumerate them in v1, so a skill placed there is neither listed in `status` nor treated as a shadow.
- `scan:enterprise` is the one assumption: the documentation says the enterprise location outranks personal and project scope, but the platform paths unam checks are not specified there. Override them with `UNAM_ENTERPRISE_SKILLS_DIRS`.
- The `consumers:*` rows are why the plan header says "visible to" more than one agent. They say these tools read `.claude/skills`; the M0 run tested only the cursor-agent CLI among them.

## Maintenance

- The two M2 sections (Hermes slugify parity and the bundle loader differences) and the two M4 evidence sections are all kept.
- When an adapter changes its evidence entries, regenerate the claim table from `unam doctor --evidence` (run it only against a temporary `UNAM_CONFIG_DIR` and `HOME`, never against a real project) and keep the M0 outcome section in step with `spike/symlink-discovery.md`. Milestone M5 adds the `agents-neutral` claims.
