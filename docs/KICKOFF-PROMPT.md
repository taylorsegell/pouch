# Kickoff prompt

Renamed: this project is now called pouch (binary `pouch`) as of 2026-10-20; this document keeps the original name as history.

Paste everything in the block below into a new Claude Code session started in the project folder, after the toolkit installer has run.

```text
This repository is the start of Unambiguous, a Rust CLI invoked as `unam`. The design is finished and lives in docs/. There is no code yet. Your job in this session is to set up the development harness around that design and produce the plans. You will not write application code.

Read these first, in full, before doing anything else:
1. docs/PROJECT-BRIEF.md
2. docs/spec/PRD-unambiguous.md
3. docs/spec/unam-technical-spec.md
4. docs/adr/ADR-0001-dynamic-skill-routing.md
5. spike/symlink-discovery.md

Preconditions. Check each and stop with a clear message if one fails:
- All five documents above exist. If one is missing, tell me which and stop.
- vibecode-pro-max-kit is installed: .claude/skills/vc-setup exists and process/development-protocols/ exists. If not, tell me to run the installer and stop.
- This is a git repository on branch main.
- `rustc --version` and `cargo --version` work on the stable toolchain, and Node.js is 22 or later. Report the versions.

Then do the following, in order.

Step 1: Run vc-setup.
Follow its full flow. Answer its discovery questions from docs/PROJECT-BRIEF.md, the PRD, and the spec, and show me your answers for confirmation; ask me only what those documents do not cover. There is no Cargo.toml yet, so tell it the stack is Rust (stable, edition 2024, Cargo workspace with crates unam-core and unam-cli) as described in spec sections 1.3 and 1.4. When it populates process/context/all-context.md, route to the documents in docs/ and summarize them; do not copy them. Record the "Rules for anyone working in this repository" section of the brief as standing project rules.

Step 2: Create the phase program for product Phase 1.
Use the toolkit's phase program flow to create an umbrella plan for "Phase 1: selection and activation" with one toolkit phase per spec milestone, M0 through M5, in that order. Build the Program Goal Charter from PRD sections 3, 12, and 13 and spec sections 9 and 10; its done condition is the PRD's: every P0 and P1 acceptance criterion passes and the tool is in daily use. For each toolkit phase, carry over the milestone's scope and the acceptance criteria it closes from spec section 10, and reference PRD and spec sections by ID or number instead of restating them. M0 is the gate: every later milestone depends on its results table.

Step 3: File the later work in the toolkit's backlog.
Add backlog entries that point to their source documents without duplicating them: product Phase 2 (spec milestones M6 and M7, then the items in spec section 11), and product Phase 3 (PRD section 12 and ADR-0001, gated by PRD decision D8). Do not create entries for context offloading; PRD decision D6 excludes it from this product.

Step 4: Prepare M0.
Take the M0 phase through planning and validation. The spike is a manual procedure that needs me to launch each agent, so do not try to execute it yourself. Give me the exact commands to create the throwaway repository and probe skills, tell me what to ask each agent, and tell me how to report results so you can fill in the table in spike/symlink-discovery.md.

Step 5: Commit and stop.
Make one commit containing the harness setup, context, plans, and backlog entries. Then show me: what vc-setup recorded, the umbrella plan, the M0 plan, and the list of open questions from the brief that block any milestone. Do not start M1.

Rules for this session:
- The documents in docs/ are the source of truth, and the PRD outranks the spec and the ADR. Do not move, rename, or rewrite them. If you find a contradiction between them, or between them and what the toolkit expects, tell me; do not resolve it yourself.
- "Phase 1", "Phase 2", and "Phase 3" mean the product phases in the PRD. The toolkit's phases map to spec milestones and are a different thing. In everything you write, say "milestone M3" or "product Phase 1" so the two are never confused.
- No application code, no Cargo workspace, and no crates in this session. The workspace is the first task of M1.
- Do not write to the skill library, and do not run any skill-management tool against this repository.
- Do not publish, push, or add a license file without asking me.
- No em dashes and no emojis in any file you write.
```
