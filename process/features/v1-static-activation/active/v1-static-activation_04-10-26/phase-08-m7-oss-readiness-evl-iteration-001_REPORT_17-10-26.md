---
domain: tests
phase: milestone M7 EVL
iteration: 1
date: 2026-10-17
loop_status: CONTINUE
---

# M7 EVL iteration 001

Independent vc-tester run at HEAD 5d5505a (code 9897bfe, base fadca4b).

## Gate results

- G1, G2: exit 0. G3: 710 passed, 0 failed, 0 ignored.
- G4: `dist plan`, `dist generate --check`, `cargo check --profile dist` all exit 0; four targets, `[bin] unam`, Homebrew formula, shell installer.
- G5: all new tests present; 45 snapshots identical in name and blob to fadca4b.
- G6: protected docs unchanged; lockfile only renames `unam` to `unambiguous`; no `panic = "abort"`; no tags; no added `.spawn(` or `#[ignore`.
- Controls proven by scratch-copy mutation; README honesty and `docs/json.md` spot checks pass.

## Defect found

`unam --json search skill | head -c 50` panics with "failed printing to stdout: Broken pipe" and exits 101 (3 of 3 runs). Violates "never panic on user data".

## Fix scope for cycle 1

Treat a broken-pipe write as a clean exit in the CLI output path, with a regression test; also add `version = "0.1.0"` to the `unam-core` workspace dependency so a later `cargo publish` is not blocked (Taylor's standing "fix findings").
