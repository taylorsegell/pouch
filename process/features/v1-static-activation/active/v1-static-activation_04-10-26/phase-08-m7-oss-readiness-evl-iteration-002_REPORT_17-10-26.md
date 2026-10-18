---
domain: tests
phase: milestone M7 EVL
iteration: 2
date: 2026-10-17
loop_status: HALTED_SUCCESS
---

# M7 EVL iteration 002

Independent vc-tester run at HEAD bae2268 after fix cycle 1 (code 15764ef).

## Gate results

- G1, G2: exit 0. G3: 712 passed, 0 failed, 0 ignored.
- G4: `dist plan`, `dist generate --check`, `cargo check --profile dist` exit 0.
- G5: 45 snapshots identical in name and blob to fadca4b.
- G6: protected docs unchanged; lockfile rename-only, no denied packages; no `panic = "abort"`; no tags; no added `.spawn(` or `#[ignore` in src; `git diff --check` clean.
- Original defect: `unam --json search skill | head -c 50` now exits 0 with no panic (3 of 3).
- No `print!`/`println!` remains in CLI src outside TUI tests.

## Known gaps

- `human_output_to_closed_pipe_exits_zero` is vacuous (33 KB output is under the 64 KB pipe buffer). Strengthened in a follow-up quick fix.
- `eprintln!` to a closed stderr could still panic (low risk).
- `LICENSE-APACHE` ends with a trailing blank line (stock text, verbatim copy).

Closeout classification: WITH_GAPS (Linux/CI, first release run, Taylor's manual runs).
