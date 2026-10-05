# M0 spike: do agents discover symlinked skills?

**Status:** run 2026-10-05 by Taylor; Windsurf and the Cursor editor not tested

## Question

`unam` defaults to activating a skill by symlinking its directory from the library into the project's skill folder. That only works if each agent discovers a symlinked skill the same way it discovers a real directory. This spike answers that per agent before any application code is written. Its results set the default strategy for each adapter.

## Procedure

1. Create a throwaway Git repository outside this project.
2. In a scratch directory outside that repository, create two probe skills, `probe-a` and `probe-b`. Each is a directory with a `SKILL.md` that has `name` and `description` frontmatter and one sentence of body text. Give each a distinctive description so it is easy to spot in a skill list.
3. Symlink `probe-a` (absolute symlink) into `.claude/skills/probe-a` and `.agents/skills/probe-a` in the throwaway repository.
4. Copy `probe-b` into `.claude/skills/probe-b` and `.agents/skills/probe-b`.
5. Start Claude Code in the repository and ask it to list its available skills. Record whether each probe appears, and whether a reload command or a restart was needed.
6. With Claude Code still running, add a third probe, `probe-c`, as a symlink. Record whether it appears without a restart.
7. Repeat steps 5 and 6 in Codex.
8. Repeat in Cursor, Windsurf, and Hermes if installed. Otherwise record "not tested".
9. Delete the throwaway repository and the scratch directory.

## Results

Record the agent version with every row. Behavior changes between releases.

| Agent | Version | Destination | Strategy | Discovered at start | Discovered when added mid-session | Refresh needed | Date | Notes |
|---|---|---|---|---|---|---|---|---|
| Claude Code | 2.1.285 | `.claude/skills` | symlink | yes | yes | start: none; mid: none | 2026-10-05 | Run 1, `.agents` hidden |
| Claude Code | 2.1.285 | `.claude/skills` | copy | yes | n/a (copy probe; probe-c is symlink-only) | start: none; mid: n/a | 2026-10-05 | Run 1 |
| Claude Code | 2.1.285 | `.agents/skills` | symlink | no | no | start: not visible even after restart; mid: not visible | 2026-10-05 | Run 2, `.claude` hidden. The copy probe was not listed either, so Claude Code does not scan `.agents/skills` at all; this is not a symlink effect |
| Codex | codex-cli 0.160.0 | `.agents/skills` | symlink | yes | yes | start: none; mid: none | 2026-10-05 | Run 3 |
| Codex | codex-cli 0.160.0 | `.agents/skills` | copy | yes | n/a (copy probe; probe-c is symlink-only) | start: none; mid: n/a | 2026-10-05 | Run 3 |
| Cursor (cursor-agent CLI) | 2026.10.01-e373342 | `.agents/skills` | symlink | yes | yes, after restart | start: none; mid: restart | 2026-10-05 | Run 5, `.claude` hidden. Tested via cursor-agent CLI; not evidence for the Cursor editor |
| Cursor (cursor-agent CLI) | 2026.10.01-e373342 | `.claude/skills` | symlink | yes | yes, after restart | start: none; mid: restart | 2026-10-05 | Run 4, `.agents` hidden. Tested via cursor-agent CLI; not evidence for the Cursor editor. Workspace Trust prompt on first start |
| Windsurf | n/a | `.agents/skills` | symlink | not tested | not tested | not tested | 2026-10-05 | Not installed |
| Hermes | v0.21.5+7344.gea32af9 (2026.9.24) | `.agents/skills` | symlink | yes, after trust | yes, after restart | start: trust step; mid: restart | 2026-10-05 | Run 6. Before `hermes skills trust`, neither probe was listed; after trust, both were. Hermes prefixes project skill descriptions with "[project]" |

## Decision rule

- If symlinks are discovered for an adapter, its default strategy is `symlink`.
- If symlinks are not discovered for an adapter, that adapter drops `symlink` from its supported strategies, its default becomes `copy`, and the reason is recorded here and in `docs/adapters.md`.
- A row marked "not tested" leaves that consumer's behavior as "not documented" in the adapter's output.

## Outcome

Run on 2026-10-05 by Taylor. Every tested agent that scans a destination discovered a directory symlinked into it exactly as it discovered a real directory copy. No tested agent needed copy instead of symlink.

**Default strategy per adapter (decision rule applied):**

- `claude-code` (`.claude/skills`): **symlink**. Claude Code discovered the symlinked probe at start and picked up a symlink added mid-session with no reload. cursor-agent CLI, which reads `.claude/skills` for compatibility, also discovered the symlink.
- `agents-neutral` (`.agents/skills`): **symlink**. Codex, cursor-agent CLI, and Hermes (after trust) all discovered the symlinked probe.

**Refresh behavior observed** (feeds each adapter's `refresh` output, spec 4.7):

- Claude Code: none needed for an existing `.claude/skills` directory, at start or mid-session.
- Codex: none needed; a mid-session addition appeared automatically.
- cursor-agent CLI: a mid-session addition appeared only after restarting cursor-agent, in both destinations.
- Hermes: project skills appear only after `hermes skills trust <repo>`; a mid-session addition appeared only after a restart.

**Not tested:** Windsurf (not installed) and the Cursor editor. The Cursor rows are evidence for the cursor-agent CLI only, so the Cursor editor's behavior stays "not documented".

**Spec and PRD claims compared with the results** (reported to Taylor; documents not edited):

- Consistent: Claude Code does not read `.agents/skills` (PRD 10.1 says "Not documented"; this run found no discovery at all, even after restart). Claude Code watches `.claude/skills` (spec 4.7 `refresh` = none). Codex detects changes automatically (spec 4.7). Hermes needs `hermes skills trust` once per repository (spec 4.7 followups).
- Refinement, not contradiction: spec 4.7 renders Cursor refresh as "not documented". For the cursor-agent CLI, this run observed "restart required" for skills added mid-session. Hermes also needed a restart for a mid-session addition, which spec 4.7 does not state.
- No result contradicts symlink-first (PRD 11.2, Q1, assumption A1 for the tested agents).

Recording these claims with their evidence in `docs/adapters.md` falls to whichever milestone creates that file first (M2 or M4).
