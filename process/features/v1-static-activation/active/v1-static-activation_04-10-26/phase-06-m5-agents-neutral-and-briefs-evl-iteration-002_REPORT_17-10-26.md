---
domain: tests
phase: milestone M5 EVL
iteration: 2
date: 2026-10-17
loop_status: HALTED_SUCCESS
---

# M5 EVL iteration 002

Independent vc-tester re-run at HEAD 5ba6888 after fix cycle 1.

## Fix applied in cycle 1

Commit 5ba6888 (`apply.rs`): resume now re-evaluates a `WriteGenerated` skipped as dest-occupied when its same-name `RemoveGenerated` is not terminal, and a `RemoveGenerated` whose destination holds a different-hash brief with a trash copy present counts as done, so the purge runs. Regression test `brief_replace_interrupted_at_call_seven_resumes_to_clean_state` runs without the env var.

## Gate results

- G1, G2: exit 0. G3: 643 passed, 0 failed, 0 ignored.
- G4: not applicable (catalog numstat all 0 0).
- G5 (a) to (d): pass (14 `brief_*` names). G5 (e): literal exits 1 on mode only; numstat 0 0 for every existing snapshot; 3 new snapshots are additions.
- G6: literal exits 1 on mode only; `--stat` 0 insertions, 0 deletions.
- Full fault matrix: `ac_briefs` 13 passed, `ac_recovery` 14 passed.
- `git diff --check`: clean.

## Result

All gates green. Remaining known gap: literal G5 (e) and G6 diff commands exit 1 only because of the repo-wide 644 to 755 mode change from commit 4956007. Closeout classification: CLEAN.
