---
name: bundle
description: Use Pouch to save, list, inspect, duplicate, edit, or delete named skill bundles, or change their membership.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required.

| Request | Command |
| --- | --- |
| Save the project's current selection | `pouch bundle save <name> --project <root> --target <target>` |
| List or inspect bundles | `pouch bundle ls` or `pouch bundle show <name>` |
| Duplicate a bundle | `pouch bundle duplicate <name> <new-name>` |
| Add members | `pouch bundle add <name> <skill>...` |
| Remove members | `pouch bundle rm <name> <skill>...` |
| Delete the bundle | `pouch bundle rm <name>` |
| Edit bundle metadata or instructions | `pouch bundle edit <name>` |

Use `--json` to inspect results. `bundle save` snapshots a project's resolved selection and accepts only the bundle name as a positional argument. If asked to create a bundle from a new list of skills without changing the project, explain the save requirement and agree on which selection to use. For existing bundles, resolve member names with `pouch search` and use exact skill IDs. Ensure at least one member argument is present for member removal.

`bundle edit` opens the configured `EDITOR`. For interactive editing, run it in a terminal. For a specific requested metadata edit in a shell-only session, a temporary non-interactive editor can edit the file passed by the CLI so Pouch still validates and commits the change. Use `--force` on save or duplicate only for an intended overwrite. Verify edits with `pouch bundle show <name> --json`. Applying a bundle to a project uses `pouch use`.
