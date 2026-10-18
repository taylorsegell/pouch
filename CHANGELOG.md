# Changelog

All notable changes to this project are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project intends to follow [Semantic Versioning](https://semver.org/) once it is published.

## [0.1.0] - Unreleased

The first version. Nothing has been tagged, pushed as a release, or published; see the README for what exists and what is untested.

### Added

- Milestone M0, discovery spike: a recorded test of how Claude Code, Codex, the `cursor-agent` CLI, and Hermes discover a symlinked skill directory compared with a copied one (`spike/symlink-discovery.md`). Result: every tested agent finds a symlink, so `symlink` is the default strategy for both adapters.
- Milestone M1, read path: `unam source add|list|rm` to register skill libraries, `unam search` (ranked, filterable by category and source), and `unam show` (metadata, supporting files, absolute path). A catalog cache keeps repeat scans fast; `--no-cache` bypasses it. Ambiguous skill references are an error that lists the candidates.
- Milestone M2, bundles and selection: `unam bundle save|ls|show|edit|duplicate|rm|add` for named sets of skills, each project's desired selection, and resolution of bundles plus additions minus removals with a record of why each skill is selected. Edits to a bundle file change only the affected lines and keep comments. `docs/adapters.md` records the differences from Hermes bundle loading.
- Milestone M3, activation: the `claude-code` adapter and `unam add`, `rm`, `use`, `deactivate`, `resume`, and `rollback`. A change shows its plan first, writes a journal before touching the project, records what it placed in a receipt, keeps a capped history, and holds a project lock while applying. Only receipted entries are ever removed; foreign files are never touched. After each change it says what the agent needs to see it, and after each removal it says that a running conversation keeps what it already loaded.
- Milestone M4, verification: `unam status` (selected, applied, active, shadowed, foreign, orphaned), `unam reconcile [--repair]`, rollback from history, `unam doctor [--evidence]` (which prints every adapter claim with its source), and `unam source relocate`. `unam source rm` now counts bundle members that reference the source.
- Milestone M5, shared directory and briefs: the `agents-neutral` adapter for `.agents/skills`, read by Codex, Cursor, Windsurf and Devin, and Hermes; and generated bundle briefs, a small skill written from a bundle's instruction, with an opt-out per bundle.
- Milestone M6, picker: running `unam` with no arguments in a terminal opens a full-screen picker. Search, mark with space, confirm, and accept the plan; bundles overlay; interrupted-apply recovery. Without a terminal, bare `unam` prints usage and status.
- Milestone M7, release readiness: this changelog, a rewritten README, `docs/json.md` documenting the `--json` output of every command (checked against the recorded outputs by tests), a refreshed `docs/adapters.md` (its evidence tables are checked against `unam doctor --evidence` by tests), `LICENSE-MIT` and `LICENSE-APACHE`, package metadata, and a cargo-dist release workflow for macOS and Linux on x86_64 and aarch64 (never run).

### Changed

- The command-line package is named `unambiguous`; the binary is still `unam`, and the library crate is still `unam-core`.
- Licensed under MIT OR Apache-2.0.
- `publish = false` stays in place: nothing is published to crates.io.

### Known limits

- Linux is not yet verified; the continuous-integration workflow has never run.
- The Cursor editor and Windsurf are not tested; Cursor evidence is for the `cursor-agent` CLI only.
- The picker uses `termion` instead of `crossterm`; a resize shows on the next key press and an external SIGTERM leaves the terminal raw.
- A universal macOS binary is not built: cargo-dist 0.32.0 does not support that target spelling.
- The release workflow, the shell installer, and the Homebrew formula have never run, and the Homebrew tap does not exist.
