# Unambiguous

`unam` is a command-line tool that controls which agent skills a project is exposed to. You keep one skill library in one place. `unam` activates only the skills a project needs, so the agent chooses from a short, relevant list instead of the whole library. Choosing from four relevant skills is more accurate than choosing from four hundred; saving tokens is a side effect.

It places skills where your project's agent looks for them (symlinks by default), remembers what it placed, and can verify, repair, and undo it. It never writes to your library and never touches files it did not place.

## First run: pick two skills

```
unam source add lib ~/skills     # once: register your skill library under the alias "lib"
cd ~/code/my-project             # any project; its root is the nearest directory with a .git
unam                             # opens the picker
# Type a few letters to search the library. Press space on two skills to mark them.
# Press enter to confirm. The plan shows the two adds, the destination, and which
# agents will see them. Press enter again to accept. Both skills are now active.
```

Undo it with `unam deactivate`, or remove one skill with `unam rm lib:path/to/skill`.

## Install

From a clone of this repository (needs a Rust toolchain, 1.89 or newer):

```
cargo install --path crates/unam-cli --locked
```

This installs the `unam` binary into `~/.cargo/bin`. Prebuilt release archives and installers, and `cargo install unambiguous`, will exist only after the first release and publication. Neither has happened yet, so do not expect them.

## First activation without the picker

Everything the picker does is also a plain command, with `--json` output for scripts (see `docs/json.md`).

```
unam source add lib ~/skills            # register the library
unam search review                      # find skills; rows show name, category, source
unam show lib:sdlc/code-review          # one skill: metadata, files, absolute path
cd ~/code/my-project
unam add lib:sdlc/code-review --yes     # plan, then apply without a prompt
unam status                             # the six-state table for this project
```

`--yes` skips the confirmation prompt; the plan is still printed. Without it, and without a terminal, `unam` refuses rather than guessing. Without `--target`, a change uses the target already in the project's selection; a new project gets the agent `unam` detects there, or Claude Code (`.claude/skills`) when it detects none. Pass `--target agents-neutral` for the shared `.agents/skills` directory that Codex, Cursor, Windsurf and Devin, and Hermes read.

## Commands

| Command | What it does |
|---|---|
| `unam` | Open the picker (needs a terminal); without one it prints usage and status |
| `unam source add\|list\|rm\|relocate` | Register, list, unregister, or move a skill library |
| `unam search [words]` | Search skills across sources |
| `unam show <skill>` | Show one skill: metadata, supporting files, absolute path |
| `unam bundle save\|ls\|show\|edit\|duplicate\|rm\|add` | Manage named sets of skills |
| `unam add <skill>...` | Add skills to this project's selection and apply them |
| `unam rm <skill>...` | Remove skills from the selection and apply the change |
| `unam use <bundle> --replace\|--add` | Use a bundle in this project |
| `unam deactivate` | Remove every skill `unam` placed for the target and clear its selection |
| `unam resume`, `unam rollback` | Finish or undo an interrupted apply; `rollback` also restores the previous state |
| `unam status` | Show whether each selected skill is selected, applied, active, shadowed, foreign, or orphaned |
| `unam reconcile [--repair]` | Report where the destination differs from the selection, and optionally repair it |
| `unam doctor [--evidence]` | Check symlinks, the destination, sources, the receipt, and the journal; `--evidence` lists every adapter claim with its source |

Global flags: `--json`, `--project <PATH>`, `--target <ID>`, `--yes`, `--no-cache`, `-v`, `-q`. A skill is named `alias:relative/path`; a bare name works when it is unambiguous, and an ambiguous one is an error that lists the candidates.

Per-user settings live in `~/.config/unam/` (override with `UNAM_CONFIG_DIR`); each project keeps its own state in `<root>/.unam/`. `unam` makes no network connections and sends no telemetry.

## What activation does and does not do

- **no network, no telemetry.** `unam` has no network dependency and runs no background process.
- After each change `unam` says what the agent needs to see it: nothing, a reload command, a restart, or "not documented". It reads this from the adapter, not from a guess.
- After each removal it says that a conversation which already loaded the instructions keeps them until it ends.
- "Verified" means discoverable under the target's documented scan rules. It does not prove a running agent loaded the skill.
- A bundle can carry an instruction. By default `unam` writes it as a small generated skill, `<bundle>-brief`, next to the bundle's skills. It is discoverable; `unam` cannot make an agent read it. A bundle opts out with `metadata.unambiguous.materialize_instruction: false` (see `docs/adapters.md`).

## Platforms and evidence

- Linux: not yet verified. The tool is built and tested on macOS. The CI workflow in `.github/workflows/ci.yml` has never run, and no Linux test run has completed.
- The `claude-code` and `agents-neutral` adapters rest on a recorded discovery spike (`spike/symlink-discovery.md`) and on vendor documentation; every claim and its source is listed in `docs/adapters.md` and printed by `unam doctor --evidence`.
- Evidence for Cursor is for the `cursor-agent` CLI only. The Cursor editor and Windsurf are not tested; their rows rest on documentation.
- Windows is not a target.

## Known limits

- The picker uses `termion`, not `crossterm`: a window resize shows on the next key press, and an external SIGTERM leaves the terminal in raw mode (run `reset`).
- The picker has no `e` key to switch strategy; version 1 is symlink-only.
- The picker shows a static "Loading catalog..." frame and does not highlight matched letters.
- The project lock is held only while a change is applied, never while the picker is open.
- When a project's selection names more than one target and `--target` is not given, the picker falls back to the first adapter and says so in its footer; the command line errors instead.
- The warning summary in the picker footer covers source and config warnings only, not a malformed `SKILL.md`.
- For `agents-neutral`, the search for ancestor `.agents/skills` directories stops at the nearest `.git`.
- The picker applies additions and removals one kind at a time; a mixed pending set asks you to apply them separately.

## Documents

| Document | What it is |
|---|---|
| `docs/json.md` | The `--json` output of every command |
| `docs/adapters.md` | How each agent discovers skills, with the evidence for every claim; Hermes compatibility notes |
| `CHANGELOG.md` | What each version contains |
| `docs/PROJECT-BRIEF.md` | What the project is, the decisions already made, and the open questions |
| `docs/spec/PRD-unambiguous.md` | Product requirements, phases, and acceptance criteria |
| `docs/spec/unam-technical-spec.md` | Technical design for product Phases 1 and 2: architecture, data formats, module specs, CLI, and milestones M0 to M7 |
| `docs/adr/ADR-0001-dynamic-skill-routing.md` | Technical decisions for product Phase 3: per-message skill routing (proposed, not built) |
| `spike/symlink-discovery.md` | The discovery spike: procedure and results |

## Development

Rust (stable, edition 2024) for macOS and Linux. The workspace has two crates: `unam-core` (all logic) and `unambiguous` (the `unam` binary). Run the checks with `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --locked -- -D warnings`, and `cargo test --workspace --locked`. Tests use temporary directories and never touch your config or your skill library.

Do not run `unam` against this repository: the development toolkit owns `.claude/skills/` here.

## License

Licensed under either of

- Apache License, Version 2.0 (`LICENSE-APACHE`)
- MIT license (`LICENSE-MIT`)

at your option. Unless you state otherwise, any contribution you submit for inclusion is dual licensed as above, with no additional terms or conditions.
