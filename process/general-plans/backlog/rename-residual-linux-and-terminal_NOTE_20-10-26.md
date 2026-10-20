---
name: note:rename-residual-linux-and-terminal
description: "Residual gaps after the pouch rename: aarch64 Linux never executed, Linux skill discovery not spike-tested, AC-44 real-terminal run; notes the stale CI statement in the M7 backlog note"
date: 20-10-26
---

# Residual gaps after the pouch rename

Priority: low. None of these blocks the rename (VERIFIED, plan archived at `process/general-plans/completed/rename-to-pouch_20-10-26/`).

## Gaps

1. **aarch64 Linux never executed.** CI ran only x86_64 Linux (ubuntu-latest) and macOS. The rename changed no logic. Fix option: add an aarch64 Linux runner or run a release artifact on one when available.
2. **Agent skill discovery on Linux not spike-tested.** Spike M0 (`spike/symlink-discovery.md`) ran on macOS only. Fix option: repeat the spike procedure on Linux for claude-code and agents-neutral.
3. **AC-44 real-terminal TUI run.** Taylor's manual run of the picker in a real terminal; unchanged by the rename. See `process/features/v1-static-activation/backlog/m6-ac44-manual-terminal-run_NOTE_17-10-26.md`.

## Stale note

`process/features/v1-static-activation/backlog/m7-linux-ci-verification_NOTE_17-10-26.md` is stale about CI: Linux CI has run green (run 39345121170; ubuntu-latest and macos-latest, earlier run 39179076283). It is feature history and was not edited. Trust this note for current CI status. Still true there: aarch64 Linux and Linux skill discovery are unexercised.
