---
name: status
description: Report which skills Pouch has selected, placed, or made discoverable for a project and which are shadowed, foreign, or orphaned.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required.

Run `pouch status --project <root> --target <target> --json`. Use `claude-code` for Claude Code and `agents-neutral` for Codex or another agent that uses `.agents/skills`. If the user wants both, query the targets separately.

Report the project, target, and relevant records with their states and reasons. An orphaned entry can cause a nonzero exit with useful records still in the JSON. Pouch's active state means the skill is discoverable under the adapter's scan rules. A running agent's loaded instructions are outside this check.
