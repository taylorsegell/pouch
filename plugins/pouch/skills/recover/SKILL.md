---
name: recover
description: Resume an interrupted Pouch apply, undo an interrupted operation, or roll back the last applied change in a project.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required. Resolve the affected project and adapter. Use `--project <root> --target <target> --json` with each command. Use `claude-code` for Claude Code and `agents-neutral` for Codex or another agent that uses `.agents/skills`.

Inspect `pouch doctor` and any prior interrupted-operation error when recovery state is unclear. Use `pouch resume --yes` to finish the pending operation. Use `pouch rollback --yes` to undo it. With no interruption, rollback restores the previous applied state for that target. If the request leaves the recovery direction unclear, report the available actions so the user can choose.

Inspect the result for partial progress, conflicts, or a lock. Verify with `pouch status` and report the resulting state and any refresh instructions.
