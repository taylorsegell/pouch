---
name: source
description: Use Pouch to list, register, unregister, or relocate local skill libraries and inspect source health.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required.

- Run `pouch source list --json` to identify aliases, paths, and scan health.
- Register an existing directory with `pouch source add <alias> <path> --json`. Report the registered alias and scan results.
- Run `pouch source relocate <alias> <new-path> --json` to point an existing alias at a library already at a new path. This can repair links in projects that use the source. Report affected projects and any failures. If the user only wants to register another library, use `source add`.
- Run `pouch source rm <alias> --json` to unregister. If Pouch reports project or bundle references, surface them before using `--force`. Force is for a request that explicitly accepts unregistering a referenced source.

After a change, verify the source registry with `pouch source list --json`.
