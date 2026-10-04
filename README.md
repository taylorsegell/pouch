# Unambiguous

`unam` is a command-line tool that controls which agent skills a project is exposed to. It keeps one skill library in one place and activates only the skills a given project needs, so the agent chooses from a short, relevant list.

## Status

Design complete. No code yet. The first milestone is a gating spike (`spike/symlink-discovery.md`); no application code is written before its results table is filled in.

## Documents

| Document | What it is |
|---|---|
| `docs/PROJECT-BRIEF.md` | What the project is, the decisions already made, and the open questions |
| `docs/spec/PRD-unambiguous.md` | Product requirements, phases, and acceptance criteria |
| `docs/spec/unam-technical-spec.md` | Technical design for Phases 1 and 2: architecture, data formats, module specs, CLI, and milestones M0 to M7 |
| `docs/adr/ADR-0001-dynamic-skill-routing.md` | Technical decisions for Phase 3: per-message skill routing |
| `spike/symlink-discovery.md` | M0 procedure and results table |

## Development

Built with Rust (stable, edition 2024) for macOS and Linux. Development workflow is driven by vibecode-pro-max-kit; project context for agents lives in `process/context/`.
