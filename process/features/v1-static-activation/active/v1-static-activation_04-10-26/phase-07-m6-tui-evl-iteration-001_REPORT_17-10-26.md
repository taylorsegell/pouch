---
domain: tests
phase: milestone M6 EVL
iteration: 1
date: 2026-10-17
loop_status: HALTED_SUCCESS
---

# M6 EVL iteration 001

Independent vc-tester run at HEAD 0425429 (code 37292c2).

## Gate results

- G1, G2: exit 0. G3: 692 passed, 0 failed, 0 ignored.
- G4a warm start (release, 3 runs): medians 47.4, 47.4, 47.5 ms (limit 50). G4b peak RSS 8.9 MB (limit 50).
- G5 (a) to (e): pass; 39 existing snapshots unchanged; `wanted` list untouched.
- G6: protected docs unchanged; no `crossterm`, `async_stdin`, `.spawn(` in product source; no denied package in Cargo.lock.
- `git diff --check`: clean.

## Probes

Parity and no-write scans are non-vacuous (scratch-copy mutations turned each red); untrusted text neutralized; the picker takes the project lock only inside `change::run` and recovery.

## Known gaps

Taylor's manual AC-44 real-terminal run outstanding; warm-start margin thin (about 47 of 50 ms); parity check is substring-based (a doc comment satisfies three needles).

Closeout classification: WITH_GAPS. A code-review fix round follows (findings 1 to 5, Taylor's standing "fix findings" choice from M5).
