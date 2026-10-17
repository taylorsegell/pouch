---
domain: tests
phase: milestone M5 EVL
iteration: 1
date: 2026-10-17
loop_status: CONTINUE
---

# M5 EVL iteration 001

Independent vc-tester run at HEAD bd616b9 (code 22a2c72).

## Gate results

- G1 fmt, G2 clippy, G3 workspace tests (642 passed, 39 snapshots): pass.
- G4 perf: not applicable (catalog diff since 8d688e1 is mode-only, 0 content lines).
- G5 (a) to (d): pass. G5 (e): pass in substance; the literal command exits 1 on the repo-wide 644 to 755 mode change from commit 4956007 only (numstat shows 0 content changes; 3 new snapshots are additions).
- G6 protected docs: literal and content-only forms exit 0. Invariant calls present in the new suites (count sample).
- `git diff --check`: pass.

## Failing check

Full fault matrix (`UNAM_FULL_FAULT_MATRIX=1`) for the brief tests: 2 of 12 fail.

- `brief_interruption_resume_and_rollback` and `brief_interruption_resume_and_rollback_claude_code`, at `crates/unam-cli/tests/ac_briefs.rs:716`.
- Case: scenario `replace claude-code`, injection `FailBefore(7)`, recovery `resume`.
- Observed: resume returns Recovered, but the brief `mx-brief/SKILL.md` is not written and its `generated` receipt row is absent. Expected end state has both.
- The strided sample does not reach this call index, which is why EXECUTE reported green.

## Other findings

- `history_is_capped_at_twenty_entries`: 5 of 5 passed; the earlier `Locked` failure is unreproduced and looks pre-existing.
- Agent-probe wording (PRD G12, AC-29): no overclaim found.

## Fix scope for cycle 1

vc-execute-agent supplement, scoped to the replace / `FailBefore(7)` / resume path for generated briefs. Decide whether the resume logic or the matrix expectation is wrong, with evidence. Re-gate: full matrix on `ac_briefs`, then G1 to G3.
