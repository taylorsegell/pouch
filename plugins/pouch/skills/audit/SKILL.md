---
name: audit
description: Diagnose why a Pouch skill is missing or shadowed, check project drift and adapter evidence, or repair Pouch-managed links when requested.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required. Pass `--project <root> --target <target> --json` to each command. Use `claude-code` for Claude Code and `agents-neutral` for Codex or another agent that uses `.agents/skills`.

- Run `pouch doctor` for source, destination, receipt, and journal checks. Add `--evidence` when the issue involves discovery or refresh behavior.
- Run `pouch reconcile` to inspect drift, findings, repair steps, and conflicts. Diagnostic commands can return a nonzero exit with useful findings in the JSON.
- When the user requests repair, inspect the proposed steps and run `pouch reconcile --repair --yes` within the requested scope. Report any blockers. Then run `pouch status` and `pouch reconcile` to verify the result. Report refresh guidance if the CLI returns it.

Report the affected skills and the evidence for each finding. Treat foreign entries as findings. For an interrupted apply, surface the CLI's resume and rollback choices.
