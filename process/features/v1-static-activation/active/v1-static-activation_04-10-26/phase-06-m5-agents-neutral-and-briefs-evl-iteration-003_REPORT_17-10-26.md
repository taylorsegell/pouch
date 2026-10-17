---
domain: tests
phase: milestone M5 EVL (post-review fixes)
iteration: 3
date: 2026-10-17
loop_status: HALTED_SUCCESS
---

# M5 EVL iteration 003

Independent vc-tester run at HEAD 4832e31 after Taylor-approved review fixes: e4bf5b2 (file modes restored to 100644), 476b81c (R1 to R4), 0d5c56b (adopted brief survives rollback; zero-step plans drop stale brief rows), with report commits d9c357b and 4832e31.

## Gate results

- G1, G2: exit 0. G3: 655 passed, 0 failed, 0 ignored.
- G5 (a) to (d): pass (17 `brief_*` names). G5 (e) literal: exit 0. G6 literal: exit 0 (mode noise gone).
- Full fault matrix: `ac_briefs` 19 passed, `ac_recovery` 14 passed.
- `git diff --check` clean; 0 mode changes since `4956007^`.

## Safety review

- `release_stale_brief_rows` has one call site (`change.rs:640`), under the project lock, only on a zero-step plan; read-only commands (`verify`, `status`, `reconcile` without `--repair`) never reach it. It rewrites only the receipt, dropping rows for briefs already missing on disk. It runs without a prompt because there is no step to confirm (design note, not a bug).
- Adopt flag defaults to false for old journals; rollback skips adopted writes.
- Brief removal never deletes a directory holding `.git`.

Closeout classification: CLEAN.
