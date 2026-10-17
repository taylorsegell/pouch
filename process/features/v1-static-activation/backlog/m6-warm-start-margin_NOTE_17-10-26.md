---
name: note:m6-warm-start-margin
description: "Backlog note from M6: the warm catalog load is about 42 ms of the 50 ms picker start budget; the margin is thin and one release run under load reached a median of 49.9 ms"
date: 17-10-26
feature: v1-static-activation
---

# M6 follow-up: warm-start margin (owner: a later change to `unam-core`)

Priority: low; the spec 9 target (warm start under 50 ms on a 500-skill catalog) is met, but the margin is small.

Measured (release, 500-skill generated fixture, M6 EXECUTE, local SSD): `perf_warm_start_picker` medians 43.8, 42.7, 42.7, and 49.9 ms (the last under other load). A scratch timing split showed `Catalog::load` with a warm cache takes about 42 ms; the picker itself adds about 1 ms (`App` setup, `verify`, one draw to a `TestBackend` 0.5 ms).

Where the time goes: the warm path still stats every file of every skill to validate the cache fingerprints. A cheaper validity check (for example per-directory mtimes first) would cut it, but it touches `crates/unam-core/src/catalog/` and the M1 cache semantics, so it was out of scope for M6 (catalog code was not touched, so `perf_cold_scan` was not rerun).

Risk: the assertion in `perf_warm_start_picker` could flake on a loaded CI runner. If it does, first re-run on an idle machine before changing the limit; spec 9 sets 50 ms.

Source: `phase-07-m6-tui_REPORT_17-10-26.md`, Test Gate Outcomes.
