---
name: context:all-product
description: "Product and design source documents in docs/ and spike/: what each decides and which section answers which question"
keywords: product, prd, spec, adr, brief, requirements, acceptance criteria, milestone, phase, adapter, bundle, selection, receipt, journal, atlas, router, spike, unam
related: []
date: 04-10-26
---

# Product Context

This file is the product and design entrypoint for Unambiguous. It routes to the source documents and summarizes them. It does not restate them: when a detail matters, open the cited section.

## Scope

This group covers:

- where each product and design decision lives (`docs/`, `spike/`)
- the order of authority between those documents
- a section map so an agent can open the one section it needs

It does not cover:

- plans, milestone progress, or backlog (see `process/features/v1-static-activation/`)
- test commands (see `process/context/tests/all-tests.md`)

## Read When

Read this entrypoint when:

- planning or implementing any milestone (M0 to M7)
- answering "what does the product require" or "how is X designed"
- checking an acceptance criterion (`AC-NN`) or requirement ID (`SRC-`, `DIS-`, `BND-`, `ACT-`, `VER-`, `VSN-`, `OWN-`, `STA-`, `SES-`, `EDG-`, `UX-`, `RTE-`)

## Order of Authority

1. `docs/spec/PRD-unambiguous.md` decides what and why. It wins every disagreement.
2. `docs/spec/unam-technical-spec.md` decides how for product Phases 1 and 2.
3. `docs/adr/ADR-0001-dynamic-skill-routing.md` decides how for product Phase 3.
4. `docs/PROJECT-BRIEF.md` is standing context and rules; it points to the three above.

A disagreement between documents is reported to Taylor, never resolved by an agent. The five kickoff design documents (the brief, the kickoff prompt, the PRD, the spec, ADR-0001) are never moved, renamed, or rewritten by an agent without Taylor's explicit permission for that specific edit. New files the spec calls for, such as `docs/adapters.md` and `docs/json.md`, may be created and maintained.

Decision log: Taylor's 2026-10-04 decisions edited spec sections 8, 10, 11, 12 and PRD section 12 (commit ae120c5). The rationale for each is recorded in the v1 umbrella plan's "Contradictions and Open Items" section.

## Vocabulary

- **Product Phase 1, 2, 3** are the PRD phases (PRD section 12). "Phase 3" always means intent-driven exposure (dynamic skill routing).
- **Milestones M0 to M7** are the spec's units of work (spec section 10).
- **Toolkit phases** are units of a vibecode phase program. One toolkit phase maps to one milestone. Write "milestone M3" or "product Phase 1", never a bare "phase 1".
- **v1** means milestones M0 to M7 (spec preamble). **v2** means the rest of product Phase 2 (spec section 11).

## Documents

| Document | What it decides | Open it for |
|---|---|---|
| `docs/PROJECT-BRIEF.md` | Purpose, roadmap, decisions made, standing rules, open questions | Rules, out-of-scope list, open questions table |
| `docs/spec/PRD-unambiguous.md` | Requirements, phases, acceptance criteria (v0.3) | Requirement IDs, AC-NN, decisions D1 to D9, open questions Q1 to Q11 |
| `docs/spec/unam-technical-spec.md` | Architecture, formats, module specs, CLI, tests, milestones | Anything "how" for product Phases 1 and 2 |
| `docs/adr/ADR-0001-dynamic-skill-routing.md` | Product Phase 3 technical decisions (status: Proposed) | Router pipeline, exposure mechanism, action items |
| `spike/symlink-discovery.md` | Milestone M0 procedure, results table, decision rule | Running or reading the spike |
| `docs/KICKOFF-PROMPT.md` | The session prompt that set up this harness | History only |

M0 results: milestone M0 ran on 2026-10-05; its Outcome is in `spike/symlink-discovery.md` (default strategy `symlink` for both adapters, per-agent refresh behavior, Windsurf and the Cursor editor not tested). That file is the source of truth for the M0 spike procedure and raw results. `docs/adapters.md` (spec-called document, created in milestone M2, not one of the five protected documents) records the Hermes slugify parity outcome and the bundle loader differences from Hermes; milestone M4 extended it with the claude-code adapter evidence table and the M0 Outcome, and milestone M7 regenerates the adapter part from `doctor --evidence` while keeping the two M2 sections.

## Section Map

### PRD (`docs/spec/PRD-unambiguous.md`)

| Section | Content |
|---|---|
| 0 | Locked decisions D1 to D9 (Rust, macOS and Linux, shared `.agents/skills` destination, three phases, D6 excludes context offloading, D7 not a harness, D8 Phase 3 evidence gate, D9 precision over recall) |
| 3 | Goals G1 to G16 by phase; non-goals |
| 4 | Terminology; the four conditions for "active"; the six reported states; discoverability versus loading |
| 5 | Hermes bundle behavior: verified, adopted, adapted, extended |
| 6 | User journeys A to H |
| 7 | Functional requirements with priorities (P0, P1 = product Phase 1; P2 = Phase 2; P3 = Phase 3) |
| 8 | Command surface, plan rendering, removal caveat text |
| 9 | Conceptual data model, storage layout, proposed schemas incl. Phase 3 metadata |
| 10 | Verified agent discovery locations; adapter scope by phase; adapter interface; compatibility limits |
| 11 | Activation mechanism comparison; symlink-first recommendation |
| 12 | Product Phases 1 to 3: scope and done conditions; development approach |
| 13 | Success criteria S1 to S15; acceptance criteria AC-1 to AC-56 |
| 14 | Risks R1 to R12; assumptions A1 to A7 |
| 15 | Open questions Q1, Q3, Q5 to Q11 |
| 16 | Sources (evidence URLs) |

### Technical spec (`docs/spec/unam-technical-spec.md`)

| Section | Content |
|---|---|
| 1.1 to 1.4 | Architecture, primary data flow, crate layout, dependencies |
| 2 | Storage paths and formats: config, bundle, selection, receipt, journal; 2.6 reserves Phase 3 metadata |
| 3 | Domain model (Rust types; field names are the contract) |
| 4.1 to 4.12 | Module specs: config, source, catalog, bundle, project, resolve, adapter, plan, fsops, apply, verify, reconcile |
| 5 | CLI commands and exit codes 0 to 11; `--json` envelope |
| 6 | TUI layout, keys, state machine |
| 7 | Error model and remedy texts |
| 8 | Testing strategy: unit, integration, interruption, CLI snapshots, spike |
| 9 | Non-functional targets |
| 10 | Milestones M0 to M7 with the acceptance criteria each closes |
| 11 | Decisions and trade-offs; "What to revisit" (v2 items) |
| 12 | Kickoff commands for the workspace (milestone M1, not before) |

## Architecture Summary

One binary, `unam`, from a Cargo workspace with two crates. `unam-core` holds all logic. `unam-cli` (package `unambiguous` since M7) holds a `clap` CLI and a `ratatui` picker that only call the core. Three state stores are never merged: desired (`<root>/.unam/selection.yaml`), applied (`<root>/.unam/applied.json`), observed (a filesystem scan). Source paths are a read-only newtype that no write function accepts. Two adapters in product Phase 1: `claude-code` (`.claude/skills`) and `agents-neutral` (`.agents/skills`). Details: spec sections 1 to 4.

## Source Paths

- `docs/PROJECT-BRIEF.md`
- `docs/spec/PRD-unambiguous.md`
- `docs/spec/unam-technical-spec.md`
- `docs/adr/ADR-0001-dynamic-skill-routing.md`
- `spike/symlink-discovery.md`

## Update Triggers

Update this group when:

- a document in `docs/` gains, loses, or renumbers a section
- the order of authority or the vocabulary changes
- milestone M0 results change an adapter claim (record the change in `docs/adapters.md` once it exists, and note it here)
