---
name: search
description: Search the local Pouch catalog, find a skill by name or topic, or inspect a Pouch skill and its supporting files.
---

Use the installed `pouch` CLI. If it is missing, report that the local Pouch binary is required.

- Run `pouch search "<query>" --json`. Use `--source <alias>` or `--category <prefix>` when the request narrows the search. An empty query lists everything, so use it with a filter when browsing a large library.
- Run `pouch show "<skill>" --json` for details. Use the exact `alias:relative/path` returned by search. If a short name is ambiguous, show the candidates and resolve the intended one before using it.
- Return the relevant matches and their IDs. Include the absolute path and supporting files when the user asks for them.
