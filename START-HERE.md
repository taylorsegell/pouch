# Start here

This folder is the starter for the Unambiguous (`unam`) repository. It holds the design documents and nothing else: no code, no toolkit files.

## What is in it

| Path | Purpose |
|---|---|
| `README.md` | Project readme |
| `.gitignore` | Rust, `unam` state, and toolkit entries |
| `docs/PROJECT-BRIEF.md` | What the project is, decisions made, open questions. The toolkit's setup step reads this. |
| `docs/spec/PRD-unambiguous.md` | Product requirements, version 0.3: phases, requirements, acceptance criteria |
| `docs/spec/unam-technical-spec.md` | Technical design for Phases 1 and 2, with milestones M0 to M7 |
| `docs/adr/ADR-0001-dynamic-skill-routing.md` | Phase 3 technical decisions, gated on Phase 1 |
| `spike/symlink-discovery.md` | M0 procedure with an empty results table |
| `docs/KICKOFF-PROMPT.md` | The prompt to paste into Claude Code |

## What is deliberately not here

- **`CLAUDE.md` and `AGENTS.md`.** vibecode-pro-max-kit installs and manages both, and they hold no project content. Project context goes in `process/context/`, which the toolkit's setup step builds from `docs/`.
- **`.claude/`, `.codex/`, `.agents/`, `process/`.** The toolkit installer creates these.
- **`Cargo.toml` and crates.** Creating the workspace is the first task of milestone M1, after the M0 spike.

## Steps

1. Copy everything in this folder, including the hidden `.gitignore`, into the project folder.
2. In the project folder, run `git init -b main`.
3. Install the toolkit (needs Node.js 22 or later, git, and bash):
   `curl -fsSL https://raw.githubusercontent.com/withkynam/vibecode-pro-max-kit/main/install.sh | bash`
4. Start Claude Code in the project folder and paste the contents of `docs/KICKOFF-PROMPT.md`.

Install the toolkit before starting Claude Code, not from inside a session: the toolkit's `CLAUDE.md` and hooks are read when a session starts.
