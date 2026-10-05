# v1 static activation: autopilot goal block

Emitted: 2026-10-05. This is the provisional block. V7 of VALIDATE will emit the (UPDATE) variant.

```
SESSION GOAL: v1 static activation: complete milestones M0 to M5, stop at the product Phase 1 gate
ENTRY PHASE: milestone M1, step 1 RESEARCH (M0 VERIFIED 2026-10-05; harness, plans, backlog done)
REMAINING PHASES:
  [ ] M1 core read path: sequential (sonnet; EXECUTE opus)
  [ ] M2 bundles and selection: sequential
  [ ] M3 plan and apply for claude-code: sequential
  [ ] M4 verify, status, reconcile, rollback: sequential
  [ ] M5 agents-neutral and briefs: sequential
  [ ] HARD STOP: product Phase 1 gate (M6, M7 held)
CLARIFICATIONS LOCKED:
  1. M3 first adopting project: recorded as open (this repo excluded by brief rule)
  2. No extra pauses beyond hard stops; commit each milestone on main
  3. M5 may read ~/Developer/01-AI/skillzone read-only for S1/S6
  4. No push: CI workflow written but not run; Linux unverified = known gap
  5. Strategy: sequential, ~55 spawns
EXECUTE CONSENT: standing-granted via autopilot trigger (2026-10-05)
DECISION POLICY: self-select recommended approach; PVL/EVL auto-run to 10 cycles, remaining gaps become known-gaps; BLOCKED becomes backlog note and the run continues; per-milestone 7-step loop R,I,P,PVL,E,EVL,UP; commit per milestone, never push
HARD STOPS:
  - Irreversible/outward-facing actions require manual confirmation (push, publish, deletes outside temp dirs, writes to skill library)
  - Cascade BLOCKED (2 consecutive phases BLOCKED-skipped)
  - needs-live-provider feasibility probe
  - High-risk evidence pack (manual-first)
  - Product Phase 1 gate after M5
TEST GATES: TBD: populated after VALIDATE
START: milestone M1, step 1 RESEARCH: spawn vc-research-agent with the M1 plan
LANE: full
```

## (UPDATE) 2026-10-05

```
SESSION GOAL: (UPDATE) v1 static activation: complete milestones M0 to M5, stop at the product Phase 1 gate
ENTRY PHASE: milestone M1, step 5 EXECUTE (M0 VERIFIED; M1 R, I, P, PVL done; PVL Gate PASS after 3 cycles)
REMAINING PHASES:
  [ ] M1 core read path: EXECUTE (1 opus agent), EVL (sonnet vc-tester), UPDATE PROCESS (sonnet)
  [ ] M2 bundles and selection: sequential
  [ ] M3 plan and apply for claude-code: sequential
  [ ] M4 verify, status, reconcile, rollback: sequential
  [ ] M5 agents-neutral and briefs: sequential
  [ ] HARD STOP: product Phase 1 gate (M6, M7 held)
CLARIFICATIONS LOCKED:
  1. M3 first adopting project: recorded as open (this repo excluded by brief rule)
  2. No extra pauses beyond hard stops; commit each milestone on main
  3. M5 may read ~/Developer/01-AI/skillzone read-only for S1/S6
  4. No push: CI workflow written but not run; Linux unverified = known gap
  5. Strategy: sequential; sonnet everywhere except code-writing EXECUTE (opus); never fable
EXECUTE CONSENT: standing-granted via autopilot trigger (2026-10-05)
DECISION POLICY: self-select recommended approach; PVL/EVL auto-run to 10 cycles, remaining gaps become known-gaps; BLOCKED becomes backlog note and the run continues; per-milestone 7-step loop R,I,P,PVL,E,EVL,UP; commit per milestone, never push
HARD STOPS:
  - Irreversible/outward-facing actions require manual confirmation (push, publish, deletes outside temp dirs, writes to skill library)
  - Cascade BLOCKED (2 consecutive phases BLOCKED-skipped)
  - needs-live-provider feasibility probe
  - High-risk evidence pack (manual-first)
  - Product Phase 1 gate after M5
TEST GATES (M1): G1 cargo fmt --all --check; G2 cargo clippy --workspace --all-targets -- -D warnings; G3 cargo test --workspace; G4 per-AC gate cargo test -p unam --test ac_read_path <ac_NN> -- --exact; G5 read-side invariant; G6 gate6 release perf_cold_scan under 300 ms; G7 snapshot hygiene; G8 CI parse; G9 dist parse; G10 text hygiene; G11 real paths untouched; G12 harness validators; G13 lockfile (exact commands in the M1 plan Exit Gate)
START: milestone M1, step 5 EXECUTE: spawn vc-execute-agent (opus) with the M1 plan
LANE: full
```
