---
phase: phase-01-m0-symlink-spike
date: 2026-10-05
status: COMPLETE_WITH_GAPS
feature: v1-static-activation
plan: process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_PLAN_04-10-26.md
---

# Milestone M0 phase report: gating symlink discovery spike

TL;DR: M0 is VERIFIED (WITH_GAPS). Symlink is the default strategy for both adapters (`claude-code`, `agents-neutral`). All gates G1 to G6 passed. Gaps: Windsurf and the Cursor editor were not tested, and gates check presence and format, not truth of values. The M1 entry gate is met. Two process incidents are recorded below.

## What Was Done

- Taylor ran the spike by hand on 2026-10-05 against Claude Code 2.1.285, Codex 0.160.0, cursor-agent CLI 2026.10.01-e373342, and Hermes v0.21.5. Windsurf is not installed.
- `spike/symlink-discovery.md` was filled: all 9 results rows (version, date, outcomes), the Status line, and the Outcome section. Raw results are in `m0-spike-raw-results_04-10-26.md` in this task folder.
- Outcome (decision rule applied):
  - `claude-code` (`.claude/skills`): symlink. Discovered at start and mid-session, no reload needed. cursor-agent CLI also discovered it.
  - `agents-neutral` (`.agents/skills`): symlink. Codex, cursor-agent CLI, and Hermes (after trust) discovered it.
  - Claude Code does not scan `.agents/skills` at all (a copy probe was also not listed, so this is not a symlink effect).
  - Refresh behavior: Claude Code none; Codex none; cursor-agent CLI restart needed for mid-session additions; Hermes needs `hermes skills trust` once per repo and a restart for mid-session additions.
- PRD Q1 (default strategy per adapter) is answered: symlink for both adapters. No spec or PRD claim is contradicted. Two refinements are noted in the Outcome (cursor-agent CLI and Hermes refresh need a restart; spec 4.7 says "not documented" for Cursor).
- Process artifacts updated: M0 plan checked off, umbrella state rewritten, blast-radius registry marked DONE, M1 Entry Gate ticked, context pointers added, one feedback memory written.

## What Was Skipped/Deferred

- Windsurf: not installed, row recorded as "not tested". Backlog NOTE: re-run the spike for Windsurf if it is ever installed or before any Windsurf adapter claim.
- Cursor editor: not tested. The cursor-agent CLI rows are evidence for that CLI only. Backlog NOTE: same.
- `docs/adapters.md`: not created in M0 (it is a new spec-called document, created by M2 or M4). Follow-up: record the M0 outcome and per-agent refresh behavior there when it is created. Source of truth until then: `spike/symlink-discovery.md`.
- Known residue named in the plan (not cleaned, not provable): agent-owned trust and session records, such as the cursor-agent Workspace Trust acceptance.

## Test Gate Outcomes

Gates were run by vc-tester (EVL, 2026-10-05) against the validate-contract commands.

| Gate | Result |
|---|---|
| G1 results table has 9 filled rows | PASS |
| G2 no empty cells | PASS |
| G3 Status line no longer "not run" | PASS |
| G4a Outcome names both adapters and a default | PASS |
| G4b-claude-code default matches the evidence rows | PASS |
| G4b-agents-neutral default matches the evidence rows | PASS |
| G5 `docs/` untouched | PASS |
| G6 spike dirs removed, user-scope skill dirs unchanged | PASS |

What green proves: the spike file is complete, well formed, and its stated defaults follow from its own rows; `docs/` was not edited. What it does not prove: that the recorded values are true (gates check presence and format; the evidence is Taylor's pasted report blocks).

Closeout classification: WITH_GAPS (from the EVL handoff).

## Plan Deviations

- Two-run isolation for Claude Code and Cursor (hide `.agents` for run 1, hide `.claude` for run 2) was a planned sequencing deviation from the spike procedure, accepted by Taylor on 2026-10-04 (decision 7).
- Hermes trust write to `~/.hermes/config.yaml` was the one sanctioned write outside the spike directory (decision 12). After cleanup, the Hermes config diff showed only an added empty `trusted_project_dirs: []`.
- Taylor's first Run 2 attempt was invalid and was redone (see Process incidents).
- Otherwise none.

## Process incidents

1. Dry run broke the Hermes install (during PVL cycle 1, repaired 2026-10-05).
   - A plan agent's paste-safety dry run invoked `hermes skills untrust` with HOME redirected to the session scratchpad.
   - Hermes rewrote its launcher scripts (`~/.hermes/hermes-agent/.hermes/bin/hermes` and `hermes-acp`) to point at a scratch Python that was later deleted. `hermes` stopped working until it was repaired on 2026-10-05.
   - The orchestrator's verification checked only `~/.hermes/config.yaml` and missed the launcher change.
   - Lessons: dry runs must never invoke real agent CLIs, even with HOME redirected, because they can rewrite their own install. Verification after any such step must cover the whole tool install, not a single file. Recorded as a feedback memory (see Memory below).
2. Invalid first Run 2 attempt.
   - Taylor linked probe-c before Question 1, so the attempt was invalid and was redone.
   - The runbook was hard to follow. Two simplified guides were written under `.brain/walkthroughs/`; they are not part of the plan record.
   - Lesson for later manual runbooks: keep the order-sensitive steps explicit and numbered per terminal, and prefer one short guide over a long reference runbook.

## Test Infra Gaps Found

- Gates G1 to G6 check structure only; there is no automated way to confirm the recorded agent behavior is true. Accepted for a manual spike. The values are re-testable by re-running the runbook.
- No agent-version drift check: behavior can change between releases (versions are recorded on every row).
- Backlog NOTE: when M4 builds `unam doctor`, consider whether any adapter claim from M0 can be rechecked automatically (not in scope for M1).

## SPEC Achievement

No locked `*_SPEC_*.md` exists for this phase (phase-program inner loop; the umbrella plan governs). Scored against the plan's acceptance criteria instead:

| Criterion | Score | Proven by |
|---|---|---|
| AC-M0-1 table rows filled or "not tested" | met | G1, G2 PASS |
| AC-M0-2 Outcome names default per adapter and lists contradicted claims | met | G4a, G4b-claude-code, G4b-agents-neutral PASS plus read |
| AC-M0-3 Status line updated | met | G3 PASS |
| AC-M0-4 `docs/` untouched | met | G5 PASS |
| AC-M0-5 nothing in user-scope skill dirs; trust entry removed; spike dirs deleted | met with named residue | G6 PASS plus Taylor's cleanup confirmation; agent-owned trust and session records remain (known gap) |

## SPEC Gaps

No unmet criteria. Residual gaps (each carried as a backlog NOTE above): Windsurf not tested; Cursor editor not tested; agent-owned trust and session residue.

## Closeout Packet

1. Selected plan path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_PLAN_04-10-26.md`
2. Closeout classification: Ready for UPDATE PROCESS archival, with gaps (EVL classification WITH_GAPS). The task folder stays in `active/` because it holds the whole program (umbrella and M1 to M7 plans); the M0 plan file stays in place and is marked VERIFIED (WITH_GAPS).
3. What was finished: see What Was Done.
4. Verified: gates G1 to G6 (EVL). Unverified: truth of recorded values; Windsurf; Cursor editor.
4b. Validate-contract: present in the plan, Gate PASS 2026-10-04 after 3 PVL cycles (`results.tsv`).
5. Cleanup done: spike dirs deleted, Hermes untrust applied, launcher repaired. Still needed: process commit by the orchestrator.
6. Next valid state: milestone M1 (phase 2 of 8), loop step 1 RESEARCH, not started; awaiting Taylor.
7. Commit checkpoint: process commit belongs after UPDATE PROCESS (no execution commit exists; M0 has no code).
8. Regression status: first phase, no prior verified surfaces; not applicable.
9. SPEC achievement: see above (no SPEC; plan criteria all met).

Drift score: 3 signals (files touched >= 1; 3+ memory-worthy observations; feature-folder structural change via backlog NOTEs and registry update). MEDIUM. Recommend UPDATE PROCESS -- significant changes detected.

## Forward Preview

### Test Infra Found

- No cargo or code test infra exists yet. M1 creates the workspace, CI, and `source_tree_unchanged_read_side`.
- Spike evidence format (per-row version, date, notes) can be reused for later adapter claims.

### Blast Radius Changes

- M0 touched only `spike/symlink-discovery.md` (as planned), plus process artifacts in this task folder and two context files. `docs/` untouched.
- Registry: Phase 1 entry marked `status: DONE`. Phase 2 (M1) ownership of `Cargo.toml`, `crates/**`, CI is unchanged.

### Commands to Stay Green

```
node .claude/skills/vc-generate-context/scripts/validate-all-context.mjs
node .claude/skills/vc-audit-context/scripts/validate-context-discovery.mjs
node .claude/skills/vc-audit-plans/scripts/validate-plan-inventory.mjs
node .claude/skills/vc-generate-plan/scripts/validate-plan-artifact.mjs <plan>
git diff --check
```

M1 adds the cargo gates (`cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`).

### Dependency Changes

- None added or removed. M1 entry gate is now met: the spike table is filled and the Outcome records the default strategy per adapter.
- Inputs M1 and later milestones should consume from the Outcome: default `symlink` for both adapters; refresh output per adapter (Claude Code none, Codex none, cursor-agent CLI restart, Hermes trust then restart); Hermes needs `hermes skills trust <repo>` once per repo; Claude Code does not read `.agents/skills`.
- First M1 task: workspace and the read-side source-tree-unchanged test, per the M1 plan. M1 research has not started; it waits for Taylor.
- Reminder for every milestone: never invoke real agent CLIs in dry runs (see Process incidents).
