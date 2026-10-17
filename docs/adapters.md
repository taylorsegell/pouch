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
| `agents-neutral` | `.agents/skills` | `symlink` | Codex 0.160.0, the cursor-agent CLI, and Hermes (after `hermes skills trust`) all found the symlinked probe. The adapter arrived in milestone M5. |

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

## Adapter evidence: the agents-neutral claims

TL;DR: `agents-neutral` writes the shared `.agents/skills` directory that Codex, Cursor, Windsurf and Devin, and Hermes read. Every claim carries a source, and `unam doctor --evidence` prints this same list. The Cursor editor and Windsurf were never tested; the Cursor rows rest on the documentation, with the `cursor-agent` CLI spike as supporting evidence only.

The table lists the 16 entries of `Adapter::evidence()` for `agents-neutral` (`crates/unam-core/src/adapter/agents_neutral.rs`), in source order. A unit test (`adapter_claims_have_evidence`) fails when a destination, scan location, strategy, refresh, or consumer claim has no entry, for either adapter.

| Claim | Evidence | Status | What it says |
|---|---|---|---|
| `destination` | https://learn.chatgpt.com/docs/build-skills.md | verified | Project skills live in .agents/skills/<name>/SKILL.md (Codex and Cursor document it) |
| `scan:project` | https://cursor.com/docs/skills | verified | <project>/.agents/skills is read by Codex and Cursor |
| `scan:ancestor` | https://learn.chatgpt.com/docs/build-skills.md | verified | Codex reads every .agents/skills from the working directory up to the repository root |
| `scan:user` | https://learn.chatgpt.com/docs/build-skills.md | verified | ~/.agents/skills is read as the user scope |
| `precedence` | https://agentskills.io/client-implementation/adding-skills-support.md | verified | A project skill beats a user skill of the same name; the order between ancestor directories is not documented (an assumption, so a same-name skill there is a warning, not a shadow) |
| `strategy:symlink` | spike/symlink-discovery.md | verified | A symlinked skill directory in .agents/skills was discovered by Codex 0.160.0, the cursor-agent CLI 2026.10.01, and Hermes v0.21.5 after trust (run 2026-10-05) |
| `strategy:copy` | spike/symlink-discovery.md | verified | A copied skill directory in .agents/skills is discovered like any project skill |
| `refresh:unknown` | spike/symlink-discovery.md | verified | Whether a running agent notices a skill added to .agents/skills differs per agent, so unam reports it per consumer and never claims a refresh is unnecessary |
| `refresh:codex` | https://learn.chatgpt.com/docs/build-skills.md | verified | Codex detects skill changes automatically; restart it if the skill does not appear |
| `refresh:cursor` | spike/symlink-discovery.md | verified | Cursor does not document refresh; the cursor-agent CLI showed a mid-session addition only after a restart (the Cursor editor was not tested) |
| `refresh:windsurf-devin` | https://docs.devin.ai/desktop/cascade/skills | assumption | Windsurf and Devin do not document refresh (Windsurf was not tested); this is an assumption from the absence of documentation |
| `refresh:hermes` | https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md | verified | Hermes lists project skills only after `hermes skills trust` is run once per repository, and needed a restart for a mid-session addition |
| `consumers:codex` | https://learn.chatgpt.com/docs/build-skills.md | verified | Codex reads .agents/skills |
| `consumers:cursor` | https://cursor.com/docs/skills | verified | Cursor reads .agents/skills (documented; the cursor-agent CLI spike is supporting evidence, the editor is untested) |
| `consumers:windsurf-devin` | https://docs.devin.ai/desktop/cascade/skills | verified | Windsurf and Devin read .agents/skills and ~/.agents/skills |
| `consumers:hermes` | https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md | verified | Hermes reads .agents/skills once the repository is trusted |

Notes:

- Scan locations and ranks (lower wins): the project's `.agents/skills` is rank 0 (Project), each ancestor's `.agents/skills` is rank 1 (Ancestor), `~/.agents/skills` is rank 2 (User). A same-name skill in a location that does not outrank the project one never marks unam's skill `shadowed`: the user location ranks below the project, and ancestors are reported as a warning (`ancestor-skills`), not a shadow, because the documentation does not state their precedence.
- Ancestor stop rule (a reading, not in the spec): the ancestors are every parent directory of the project root up to and including the nearest one holding a `.git` entry. When the project root is itself the Git root there are none, and when no `.git` is found above the root there are none either; the walk never runs on to the filesystem root. Codex documents "from the working directory up to the repository root", so this matches it for projects inside a Git repository.
- Refresh is always `unknown`. The refresh sentence has one clause per consumer, and unam never prints "no restart or reload required" for this destination. The Hermes followup `hermes skills trust <root>` is printed (when `~/.hermes` exists) and is never run.
- Not tested: Windsurf (not installed) and the Cursor editor. The consumers list is documentation, not a test result for those two.
- Cross-adapter duplicate: Cursor and Windsurf/Devin also read `.claude/skills`, so a skill placed for both targets is reported once per skill as `duplicate for cursor and windsurf-devin via compat read of .claude/skills`.

## Generated bundle briefs

TL;DR: a bundle with an `instruction` gets a small generated skill, `<slug>-brief`, next to the skills it activates. It is discoverable; unam cannot make an agent read it.

- Written for each bundle named in the target selection, unless the bundle sets `metadata.unambiguous.materialize_instruction: false`. An absent value means on (PRD 5.3 "opt-out per bundle"); a bundle without an `instruction` has no brief. There is no config key.
- Shape: the directory `<destination>/<slug>-brief/` holding only `SKILL.md`: a frontmatter block (`name`, a one-line `description`, built with a YAML serializer) and the trimmed instruction.
- Ownership: the receipt records the brief (`kind: generated`, `generated: bundle-brief`, the bundle, the name, the destination, and the `content_hash` of the exact bytes written). An unchanged brief is left alone and its record carried verbatim. A brief whose content no longer matches the receipt is never overwritten or deleted: it blocks the change while the bundle still wants it, and is left and released from the receipt when it does not. A name already taken by something unam did not write is a conflict.
- `unam status` and `unam reconcile` report a receipted brief that is missing or edited (`brief-missing`, `brief-modified`); `reconcile --repair` writes a missing one again.

## Maintenance

- The two M2 sections (Hermes slugify parity and the bundle loader differences), the two M4 evidence sections, and the M5 `agents-neutral` and brief sections are all kept.
- When an adapter changes its evidence entries, regenerate the claim table from `unam doctor --evidence` (run it only against a temporary `UNAM_CONFIG_DIR` and `HOME`, never against a real project) and keep the M0 outcome section in step with `spike/symlink-discovery.md`. Milestone M5 added the `agents-neutral` claims and the brief section; milestone M7 regenerates the adapter-evidence part and must keep the two M2 sections.
