---
name: project
description: Use Pouch to activate or remove skills in a project, apply or switch bundles, or deactivate the project's managed skill selection.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required. Resolve the requested project root and target before applying. Pass `--project <root> --target <target>`. Use `claude-code` for Claude Code and `agents-neutral` for Codex or another agent that uses `.agents/skills`. If the target is unclear, inspect the current selection with `pouch status` and resolve any ambiguity before applying. Respect repository restrictions on managing project skills.

| Request | Command |
| --- | --- |
| Add individual skills | `pouch add <skill>...` |
| Remove individual skills | `pouch rm <skill>...` |
| Add a bundle to the existing selection | `pouch use <bundle> --add` |
| Make a bundle the entire Pouch selection for the target | `pouch use <bundle> --replace` |
| Clear the Pouch selection for the target | `pouch deactivate` |

Use `pouch search` and `pouch bundle show` to resolve names if needed. Prefer exact skill IDs. Default to `--add` when asked to activate a bundle. Use `--replace` for a requested replacement. To clear multiple targets, run `deactivate` separately for each. In Pouch 0.2.0, `--all` still uses the single-target path.

For an authorized change, pass `--yes --json`. Inspect the result and report conflicts or partial applies from the JSON. If Pouch reports an interrupted operation, surface its recovery choices. Verify the resulting selection with `pouch status --json` for the affected target or targets. Report applicable refresh instructions and any removal caveat from the CLI output.
