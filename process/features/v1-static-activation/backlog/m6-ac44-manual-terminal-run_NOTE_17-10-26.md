---
name: note:m6-ac44-manual-terminal-run
description: "Backlog note from M6: the real-terminal half of AC-44 (Hybrid, owner Taylor); the phase closeout is CONDITIONAL until it is recorded"
date: 17-10-26
feature: v1-static-activation
---

# M6 follow-up: AC-44 real-terminal run (owner Taylor)

Priority: required before M6 is called VERIFIED without gaps; not a code blocker.

Gate: AC-44 real-terminal half, Hybrid (precondition: a real terminal, a temp project, a temp or fixture library). Status until done: CONDITIONAL, with this note as its recorded path. It is not a Known-Gap pass. The automated half (`ac_44_query_space_enter_enter_yields_active_skill`, synthetic keys, `TestBackend`) is green.

What: run the primary scenario once in a real terminal, never in this repository and never against `~/Developer/01-AI/skillzone` with writes. The exact steps are in the "For Taylor" block of `phase-07-m6-tui_REPORT_17-10-26.md` (task folder `process/features/v1-static-activation/active/v1-static-activation_04-10-26/`).

Evidence to record: that the picker opened, the four-keystroke scenario worked, quit restored the terminal (echo and line editing back, cursor visible), and, optionally, that a deliberate resize shows on the next key press.

Not covered by any test, by design: raw mode and termios restore on a real terminal, key decoding from real bytes, resize on the next key, external SIGTERM or SIGINT leaving the terminal raw (documented limit).

Source: `phase-07-m6-tui_PLAN_04-10-26.md`, D-TEST and step D1.
