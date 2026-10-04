# M0 spike: do agents discover symlinked skills?

**Status:** not run

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
| Claude Code | | `.claude/skills` | symlink | | | | | |
| Claude Code | | `.claude/skills` | copy | | | | | |
| Claude Code | | `.agents/skills` | symlink | | | | | |
| Codex | | `.agents/skills` | symlink | | | | | |
| Codex | | `.agents/skills` | copy | | | | | |
| Cursor | | `.agents/skills` | symlink | | | | | |
| Cursor | | `.claude/skills` | symlink | | | | | |
| Windsurf | | `.agents/skills` | symlink | | | | | |
| Hermes | | `.agents/skills` | symlink | | | | | |

## Decision rule

- If symlinks are discovered for an adapter, its default strategy is `symlink`.
- If symlinks are not discovered for an adapter, that adapter drops `symlink` from its supported strategies, its default becomes `copy`, and the reason is recorded here and in `docs/adapters.md`.
- A row marked "not tested" leaves that consumer's behavior as "not documented" in the adapter's output.

## Outcome

To be filled in after the run: the default strategy per adapter, and any adapter claim in the spec that the results contradict.
