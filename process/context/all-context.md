# Unambiguous - All Context

Last updated: 2026-10-17

This file is the root context entrypoint for the repo.

Use it for two things:

1. quick routing to the right context pack or root file
2. broad architecture and repository understanding

Start here before loading deeper context files.

---

## Project

Unambiguous is a Rust command-line tool, invoked as `unam`, that controls which agent skills a project is exposed to. One skill library lives outside every project; `unam` activates a chosen subset in a given project by placing it where that project's agent looks for skills, tracks what it placed, and can verify, repair, and undo it. The goal is accuracy: a model choosing from four relevant skills picks better than one choosing from a whole library. Token savings are a side effect.

- **User:** Taylor, solo, on macOS and Linux, across Claude Code, Codex, Cursor, and Hermes.
- **Standard:** personal tool built to open-source-ready standards: tests, docs, reproducible builds, no telemetry, no network.
- **Status:** milestones M0 (symlink discovery spike, symlink default for both adapters), M1 (core read path: `source add|list|rm`, `search`, `show`), M2 (bundles, project selection, resolve: `bundle save|ls|show|edit|duplicate|rm|add`, `show` lists referencing bundles), M3 (adapter trait, claude-code plan and apply: `add`, `rm`, `use`, `deactivate`, `resume`, `rollback` with journal, receipt, history, and project lock), M4 (`verify`/`status`, `reconcile [--repair]`, rollback from history, `doctor [--evidence]`, `source relocate`, `source rm` counts bundle members), and M5 (`agents-neutral` adapter writing `.agents/skills`, ancestor-skill warnings, generated per-bundle instruction briefs with journaled `write_generated`/`remove_generated` steps) are done and VERIFIED with gaps. The program is now at the Product Phase 1 Gate (AWAITING TAYLOR): M6 and M7 are held until Taylor confirms product Phase 1 is done. Progress and open items: umbrella plan `## Current Execution State` in `process/features/v1-static-activation/`.
- **Not a harness.** It sits in front of existing agent harnesses (PRD D7).

Full product and design context: `process/context/product/all-product.md`.

---

## Standing Project Rules

Recorded verbatim from `docs/PROJECT-BRIEF.md`, "Rules for anyone working in this repository". These bind every agent in every session.

- **No application code before the M0 spike table is filled in.** `spike/symlink-discovery.md`.
- **The skill library is read-only.** It lives at `~/Developer/01-AI/skillzone/` (confirm the path with Taylor). Inspect it freely; never write to it.
- **Never run `unam` against this repository.** The development toolkit owns `.claude/skills/` here and links `.agents/skills` to it, so both of `unam`'s adapters would resolve to the same toolkit-owned directory. All `unam` tests use temporary directories and fixtures.
- **The PRD and spec are authoritative.** Where a plan disagrees with them, they win until Taylor changes them.
- **Adapter claims need evidence.** Every statement about how an agent discovers skills is backed by a source or a spike result before code depends on it.
- No publishing to crates.io and no pushing to a remote without Taylor's explicit go-ahead.
- No em dashes and no emojis in any file.

Confirmed by Taylor on 04-10-26: the skill library path `~/Developer/01-AI/skillzone/` is correct.

Additional rules set for the harness-setup session (04-10-26), kept as standing practice:

- The five design documents that existed at kickoff are protected: `docs/PROJECT-BRIEF.md`, `docs/KICKOFF-PROMPT.md`, `docs/spec/PRD-unambiguous.md`, `docs/spec/unam-technical-spec.md`, `docs/adr/ADR-0001-dynamic-skill-routing.md`. Never move, rename, or rewrite them; an edit needs Taylor's explicit permission for that specific edit. New files the spec calls for (for example `docs/adapters.md`, `docs/json.md`) may be created and maintained. Report contradictions between documents, or between them and the toolkit, to Taylor; do not resolve them.
- Product Phase 1 is done when AC-1 to AC-43 and AC-53 pass (PRD 13.2: every criterion is Phase 1 unless under its "Phase 2" or "Phase 3" heading) and Taylor uses the tool daily. Milestones M6 and M7 are product Phase 2: not researched, planned further, or executed until Taylor clears the hard stop after M5.
- Do not run any skill-management tool against this repository.
- Do not add a license file without asking (PRD Q11 is open).
- Commit on `main` (see CLAUDE.md commit policy); never push without explicit approval.

## Threat Model (set by Taylor, 2026-10-09)

`unam` is a single-user local tool. Assume the same user owns the machine and the filesystem is trusted. Untrusted input is limited to skill file content (frontmatter and supporting files) and files that arrive in a cloned repository. Plans and validators must not add hardening against attackers beyond this (for example a concurrent hostile process swapping paths). Safety properties that protect the user from their own mistakes stay: the source library is never written, only receipted entries are removed, foreign files are never touched, and skill names are validated before they become paths.

## Vocabulary

- **Product Phase 1, 2, 3** are the PRD phases. "Phase 3" means intent-driven exposure, also called dynamic skill routing.
- **Milestones M0 to M7** are the spec's units of work. One toolkit phase maps to one milestone.
- Always write "milestone M3" or "product Phase 1". A bare "phase 3" is ambiguous and not allowed in plans.

---

## How This File Works (the `all-*.md` Convention)

Every `process/context/` directory has one `all-*.md` entrypoint that acts as an attachable quick router for that domain. This root file is the top-level router. Context groups each have their own `all-{group}.md` entrypoint.

**How agents use it:**

1. Read `all-context.md` first (this file)
2. Find the relevant context group from the routing tables below
3. Read that group's `all-{group}.md` entrypoint
4. Only then load the specific deep doc needed

Never load the whole `process/context/` tree.

**What each `all-{group}.md` must contain:** scope, read-when rules, quick procedures or decision rules, source paths, update triggers, routing to deeper docs.

---

## Quick Start

For most substantial tasks:

1. read this file first
2. choose the smallest relevant root file or context group from the tables below
3. only then load deeper files

---

## Current Root Entry Points

<!-- The two tables below (Root Entry Points + Context Groups) are GENERATED from each
     context doc's frontmatter by `discover-context.mjs --emit-routing`. Do NOT hand-edit
     between the GENERATED markers. To change a row, edit the owning doc's frontmatter
     (description / keywords) and re-emit. `--check-routing` fails lint if this block drifts. -->

<!-- GENERATED:routing -->
| File | Read when |
|---|---|
| `process/context/all-context.md` | any substantial planning, research, review, or implementation task |
| `process/context/planning/all-planning.md` | Plan shapes and SIMPLE vs COMPLEX calibration for implementation plans |
| `process/context/product/all-product.md` | Product and design source documents in docs/ and spike/: what each decides and which section answers which question |
| `process/context/tests/all-tests.md` | Test runners, commands, and verification gates for unam (cargo suite exists since milestone M1, grown through M5 (643 tests); CI written, not yet run) |

## Current Context Groups

| Group | Entry point | Scope |
|---|---|---|
| `planning/` | `process/context/planning/all-planning.md` | Plan shapes and SIMPLE vs COMPLEX calibration for implementation plans |
| `product/` | `process/context/product/all-product.md` | Product and design source documents in docs/ and spike/: what each decides and which section answers which question |
| `tests/` | `process/context/tests/all-tests.md` | Test runners, commands, and verification gates for unam (cargo suite exists since milestone M1, grown through M5 (643 tests); CI written, not yet run) |
<!-- /GENERATED:routing -->

## Task Routing Table

| Task type | Load first | Then load |
|---|---|---|
| any milestone planning or implementation | `all-context.md`, `product/all-product.md` | the PRD and spec sections the milestone plan cites |
| requirement or acceptance-criterion question | `product/all-product.md` | PRD section 7 or 13.2 |
| "how is X designed" for product Phase 1 or 2 | `product/all-product.md` | spec section named in its section map |
| product Phase 3 (routing, atlas, hooks) | `product/all-product.md` | ADR-0001, PRD section 12; note D8 gate |
| testing or verification | `tests/all-tests.md` | spec section 8 |
| creating a new plan | `planning/all-planning.md` | `process/features/v1-static-activation/` |
| milestone M0 spike | `product/all-product.md` | `spike/symlink-discovery.md`, the M0 plan |

## Current Features

| Feature | Folder | Scope |
|---|---|---|
| v1: static activation | `process/features/v1-static-activation/` | Phase program for milestones M0 to M7 (product Phase 1, then the M6 and M7 part of product Phase 2, with a hard gate after M5) |

Later work (product Phase 3, v2 items, out-of-scope pointers) is filed in `process/general-plans/backlog/`.

---

## Repository Structure

```
unambiguous/
  docs/
    PROJECT-BRIEF.md                  standing context, rules, open questions
    KICKOFF-PROMPT.md                 harness-setup session prompt (history)
    spec/
      PRD-unambiguous.md              requirements, phases, acceptance criteria (v0.3)
      unam-technical-spec.md          design for product Phases 1 and 2, milestones M0 to M7
    adr/
      ADR-0001-dynamic-skill-routing.md   product Phase 3 decisions (Proposed)
  spike/
    symlink-discovery.md              milestone M0 procedure and results table
  process/
    context/                          this context system
    development-protocols/            RIPER-5 methodology (toolkit-managed)
    features/                         feature-scoped plans
    general-plans/                    general plans and backlog
    _seeds/                           toolkit seed templates (read-only)
  .claude/ .codex/ .agents/           toolkit-managed; .agents/skills -> ../.claude/skills
  CLAUDE.md AGENTS.md                 toolkit-managed protocol files, no project content
  README.md START-HERE.md .gitignore
```

Source layout as of milestone M5 (M1 created 2026-10-05, M2 added the desired-state side, M3 added plan and apply, M4 added verify, reconcile, and doctor, M5 added the agents-neutral adapter and generated briefs; the rest of spec section 1.3 arrives in later milestones):

```
Cargo.toml Cargo.lock clippy.toml     virtual workspace, resolver = "3", unwrap denied
dist-workspace.toml                   hand-written, unverified (dist not installed)
.github/workflows/ci.yml              fmt, clippy, test on ubuntu and macos; written, not yet run
crates/
  unam-core/src/                      config, paths, error, fsops (FsOps seam, typed paths), hash, source,
                                      resolve, observe, state, plan, apply, change,
                                      verify, reconcile, doctor, generated (+ generated/tests),
                                      adapter/{mod,claude_code,agents_neutral},
                                      catalog/{mod,cache,frontmatter,lookup,scan,search},
                                      bundle/{mod,crud,slug,edit},
                                      project/{mod,selection,index}
  unam-core/tests/                    perf_cold_scan.rs (500-skill fixture, 300 ms limit)
  unam-cli/src/                       main.rs, output.rs, cli/{mod,search,show,source,bundle,change,status,doctor}
  unam-cli/tests/                     ac_read_path, ac_bundles, ac_selection, ac_activation,
                                      ac_recovery, ac_verify, ac_reconcile, doctor_cli, activation_cli,
                                      ac_agents_neutral, ac_briefs, phase1_checks, phase1_exit,
                                      bundle_cli, read_side_invariant,
                                      search_cli, show_cli, source_cli, cache_cli, usage, snapshots
                                      (+ common/{mod,faults}.rs, fixtures/)
.unam/<target>/                       per-project state written by unam (not in this repo): receipt,
                                      JSONL journal, history, lock; target is the adapter name
docs/adapters.md                      spec-called doc (M2): Hermes slugify differences and bundle loader
                                      differences; M4 added the claude-code evidence table and M0 Outcome;
                                      M7 regenerates the adapter-evidence part and keeps the M2 sections
```

Still to come: `tui/` (M6, held behind the Product Phase 1 Gate); `docs/json.md` (M7). `docs/adapters.md` exists since M2 (M4 added the claude-code table, M5 the agents-neutral table and a generated-brief section; M7 regenerates the adapter-evidence part and must keep the two M2 sections).

## Technology Stack

- **Language:** Rust, stable toolchain, edition 2024. Local toolchain at setup: rustc 1.99.0, cargo 1.99.0 (stable-aarch64-apple-darwin).
- **Platforms:** macOS and Linux. Windows is not a v1 build target.
- **Workspace:** Cargo virtual workspace, crates `unam-core` (library) and `unam-cli` (binary `unam`). Created in milestone M1; M2 added no crates (lockfile unchanged).
- **Dependencies in use (M1; lockfile committed):** clap 4.6.7 (derive), serde 1.0.229, serde_json 1.0.151, serde_norway 0.9.42, toml 1.1.6, ignore 0.4.33, blake3 1.8.7, nucleo-matcher 0.3.1, jiff 0.2.37, thiserror 2.0.21 (core), anyhow 1.0.104 (binary), tracing 0.1.44 + tracing-subscriber 0.3.23. Test: assert_cmd 2.2.2, predicates 3.1.4, insta 1.49.0, tempfile 3.27.0, proptest 1.11.0. Departures from spec 1.4: `directories` dropped (paths resolved in `paths.rs`), `nucleo-matcher` instead of full `nucleo` (umbrella open items 15 and 20). Still planned: `ratatui` + `crossterm` (M6), `fs4` (M3).
- **Deliberately absent in v1:** any Git library (shell out to `git` when present).
- **Release:** `cargo-dist` (milestone M7). `dist` and `cargo-insta` are not installed locally; `dist-workspace.toml` is parse-checked only.
- **Harness:** vibecode-pro-max-kit 3.2.5; Node.js v24.11.1 runs its validators.

## Key Patterns and Conventions

- **Core holds all logic.** CLI and TUI only call `unam-core` and render (spec 1.1).
- **Typed paths.** `SourcePath`, `DestPath`, `StatePath` newtypes; no write function accepts a `SourcePath` (spec 4.9).
- **Skill identity** is `alias:relative/path`. Ambiguous references are always errors with candidates; nothing is picked silently (spec 4.6).
- **Journaled apply.** Journal before mutation, receipt after, history capped at 20, advisory project lock (spec 4.10).
- **Errors:** `thiserror` enum in core with a `remedy` string per user-fixable variant; `anyhow` at the edge; never panic on user data; `unwrap` denied by clippy outside tests (spec 7).
- **Evidence:** every adapter claim carries an evidence URL or a spike result; an adapter without evidence fails a unit test (spec 4.7).
- **Deterministic mechanisms over model judgment** wherever sufficient (brief).

## Environment and Configuration

No environment is needed to develop yet. The product itself reads (spec section 2):

- Config: `~/.config/unam/config.toml`; bundles: `~/.config/unam/bundles/`
- Env overrides: `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`; logging: `UNAM_LOG`
- Per project: `<root>/.unam/` (selection, receipt, journal, history, lock)

Tests must point every one of these at temporary directories, never at the real user config or this repository.

## Open Questions That Block Milestones

From `docs/PROJECT-BRIEF.md`, "Open questions":

| Question | Needed by |
|---|---|
| Default strategy per adapter (PRD Q1) | milestone M0. ANSWERED 2026-10-05: `symlink` for both `claude-code` and `agents-neutral` (`spike/symlink-discovery.md`) |
| Hermes `slugify` parity | milestone M2 |
| First real project to adopt `unam` | end of milestone M3 |
| Crate name and license (PRD Q11) | before milestone M7 |

---

## Context Group Lifecycle

Context groups are durable knowledge domains, not feature folders.

Create a group when a topic has 3+ durable docs, a single doc exceeds roughly 800 lines with separable subtopics, multiple agents repeatedly need one slice of a large file, or the topic maps to a stable operational domain. Do not create a group for temporary reports, plans, or feature-specific content.

Move or split one group at a time. Use `all-{group}.md` entrypoints. Run the `vc-audit-context` skill after every context organization change.

## Naming Convention

There are no `README.md` files inside `process/context/`. Canonical entrypoints use `all-*.md`: root `process/context/all-context.md`; group `process/context/{group}/all-{group}.md`.

## Context Update Protocol

When durable project knowledge changes:

1. update the smallest relevant context file
2. update this file if routing, ownership, naming, or groups changed
3. update the owning `all-{group}.md` entrypoint when a group exists
4. run `vc-audit-context`

---

## Scan Metadata

- Generated: 2026-10-04; delta update 2026-10-17 (milestone M5 closeout)
- HEAD: commit 5ba6888 or later (the M5 process commit follows it)
- Mode: delta (status, repository structure refreshed; no dependency change in M5, `Cargo.lock` unchanged)
- Package manager: cargo (workspace exists since milestone M1)
