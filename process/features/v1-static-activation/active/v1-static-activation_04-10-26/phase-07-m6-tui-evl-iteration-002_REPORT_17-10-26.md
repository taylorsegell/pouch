---
domain: tests
phase: milestone M6 EVL (post-review fixes)
iteration: 2
date: 2026-10-17
loop_status: HALTED_SUCCESS
---

# M6 EVL iteration 002

Independent vc-tester run at HEAD 9658536 after the review fix round (code bd97432).

## Gate results

- G1, G2: exit 0. G3: 699 passed, 0 failed, 0 ignored.
- G4a warm start (release, 3 runs alone): medians 46.8, 46.7, 45.0 ms (limit 50). G4b peak RSS 9.1 MB.
- G5 (a) to (d): pass; 39 CLI snapshots unchanged vs 9a4e9df; 6 TUI snapshots unchanged vs 37292c2.
- G6: protected docs unchanged; no `crossterm`, `async_stdin`, `.spawn(`; no denied package in Cargo.lock.
- `git diff --check`: clean.

## Spot checks

All six new review-fix tests exist and pass. `Session::apply` declines on plan mismatch (`tui/actions.rs:222-226`).

## Known gaps

Taylor's manual AC-44 real-terminal run; warm-start headroom about 3 to 5 ms; Linux and CI unrun.

Closeout classification: WITH_GAPS.
