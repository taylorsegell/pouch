# Unambiguous: Project Brief

Renamed: this project is now called pouch (binary `pouch`) as of 2026-10-20; this document keeps the original name as history.

The standing context for this project: what it is, what is decided, and what is still open. Requirements live in the PRD, design in the spec and the ADR; this brief points to them and does not repeat them.

## What it is

Unambiguous is a Rust command-line tool, invoked as `unam`, that controls which agent skills a project is exposed to. A skill is a folder containing a `SKILL.md` that an AI coding agent can load on demand.

One library of skills lives in one place outside every project. `unam` activates a chosen subset in a given project by placing them where that project's agent looks for skills, tracks what it placed, and can verify, repair, and undo it.

## Why it exists

The goal is accuracy. When an agent can see every skill in a large library, it picks plausible wrong ones, overuses some, and wastes tool calls. Shrinking the set a model chooses from is the most direct fix. Token cost savings are a side effect, not the goal.

## Who uses it

Taylor, solo, on macOS and Linux, across several agent harnesses (Claude Code, Codex, Cursor, Hermes). It is a personal tool built to open-source-ready standards: tests, docs, reproducible builds, no telemetry, no network access.

## Roadmap

The PRD defines three product phases (PRD section 12).

| Phase | Scope | Where it is defined | Status |
|---|---|---|---|
| Phase 1: selection and activation | Sources, catalog, bundles, per-project selection, two adapters, journaled apply, verify, CLI | PRD; spec milestones M0 to M5 | Designed, not started |
| Phase 2: surface, reproducibility, release | Picker, release mechanics, then pinning, isolated adapters, adopt, bundle composition, remote sources | PRD; spec milestones M6 and M7, and spec section 11 | M6 and M7 designed; the rest listed only |
| Phase 3: intent-driven exposure | Per-message routing of skills from the registry, using author metadata and an atlas | PRD section 12; `docs/adr/ADR-0001-dynamic-skill-routing.md` | Proposed; gated by PRD decision D8 |

**Vocabulary.** "Phase 1, 2, 3" always means these product phases. The spec's units of work are milestones, M0 to M7. The development toolkit also calls its unit of work a phase; one toolkit phase maps to one milestone.

**Order of authority.** The PRD decides what and why. The spec decides how for Phases 1 and 2. The ADR decides how for Phase 3. Where two disagree, the PRD wins and the disagreement is reported to Taylor.

## Architecture in one paragraph

A Cargo workspace with two crates. `unam-core` is a library holding all logic; `unam-cli` builds the single `unam` binary, containing a `clap` command line and a `ratatui` picker that both call the core and hold no logic of their own. Three state stores are kept separate and compared on every status report: desired (`selection.yaml`), applied (`applied.json`), and observed (a filesystem scan). Source trees are read-only by construction. Details: spec sections 1 to 4.

## Decisions already made

- Rust, stable toolchain, edition 2024. macOS and Linux only in v1.
- One binary. Later stages add commands to it; they do not become separate tools.
- Skill identity is `alias:relative/path`, never the bare name. An ambiguous reference is always an error with candidates listed; nothing is ever picked silently.
- Bundle files are compatible with Hermes Skill Bundles.
- The skill library is deliberately kept outside every project so no project sees the whole library. `unam` must preserve that.
- Phase 3 skill metadata lives in frontmatter under `metadata.unambiguous`, with a user-scope overlay file as the alternative. Phases 1 and 2 preserve it untouched and do not act on it. PRD section 9; spec section 2.6.
- Phase 3 is gated on evidence (PRD D8) and its router favors precision over recall (PRD D9).
- Deterministic mechanisms are preferred over model judgment wherever they are sufficient.
- This is not an agent harness and will not become one. It sits in front of existing harnesses.

## Rules for anyone working in this repository

- **No application code before the M0 spike table is filled in.** `spike/symlink-discovery.md`.
- **The skill library is read-only.** It lives at `~/Developer/01-AI/skillzone/` (confirm the path with Taylor). Inspect it freely; never write to it.
- **Never run `unam` against this repository.** The development toolkit owns `.claude/skills/` here and links `.agents/skills` to it, so both of `unam`'s adapters would resolve to the same toolkit-owned directory. All `unam` tests use temporary directories and fixtures.
- **The PRD and spec are authoritative.** Where a plan disagrees with them, they win until Taylor changes them.
- **Adapter claims need evidence.** Every statement about how an agent discovers skills is backed by a source or a spike result before code depends on it.
- No publishing to crates.io and no pushing to a remote without Taylor's explicit go-ahead.
- No em dashes and no emojis in any file.

## Testing and quality

- `cargo fmt --check`, `cargo clippy` with `unwrap` denied outside tests, `cargo test`, on macOS and Linux in CI.
- Each PRD acceptance criterion maps to one integration test named `ac_NN_*`.
- Two invariants are asserted in every mutating test: the source fixture's tree hash is unchanged, and foreign files in the destination are byte-identical afterward.
- Details: spec section 8.

## Out of scope

- **Phase 3** until Phase 1 is in daily use and the baseline week exists (PRD D8).
- **Phase 2** until Phase 1 is done by the PRD's definition: every P0 and P1 acceptance criterion passes and the tool is in daily use.
- **Context offloading.** Excluded from this product by PRD D6.
- **A repository map for orientation** (files and symbols served to the agent). Not the atlas, and not part of any phase.

## Open questions

| Question | Options | Needed by |
|---|---|---|
| Crate name and license (PRD Q11) | `unam`, or `unambiguous` with a `unam` binary. Both were unregistered on 2026-10-02; neither is reserved. License not chosen. | Before M7 |
| Hermes `slugify` parity | Read the Hermes implementation and align, or document the difference | M2 |
| Default strategy per adapter (PRD Q1) | Decided by the M0 spike | M0 |
| First real project to adopt `unam` | Taylor to name one | End of M3 |

The PRD's own open questions (section 15) cover the rest: Q1 symlink or copy default, Q3 committed selection files, Q5 to Q10.
