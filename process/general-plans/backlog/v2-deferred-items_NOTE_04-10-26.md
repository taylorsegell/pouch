---
name: note:v2-deferred-items
description: "Deferred pointer: v2 items (rest of product Phase 2 beyond milestones M6 and M7, plus Windows); one note with a sub-list"
date: 04-10-26
metadata:
  node_type: memory
  type: note
---

# v2 deferred items

**Status:** deferred. Pointer only; each item is specified at its source.

**Scope boundary:** milestones M6 (TUI) and M7 (OSS readiness) are NOT backlog; they are in the v1 phase program at `process/features/v1-static-activation/`. Product Phase 3 is covered by its own note, not here.

## Why deferred

Spec section 11 "What to revisit" lists these as v2 themes; PRD section 12 places them in product Phase 2 after milestones M6 and M7, and product Phase 2 itself waits on product Phase 1 being done (brief "Out of scope").

## Gate to revisit

Product Phase 1 done by PRD section 12: every P0 and P1 acceptance criterion passes and the tool is in daily use.

## Items

| Item | Sources |
|---|---|
| Pin mode and drift | PRD VSN-3 to VSN-6, AC-45; spec section 11 bullet "Pin mode" |
| Isolated destination policy; `cursor` and `windsurf-devin` adapters | PRD D3, section 10.2, section 12 product Phase 2; spec section 11 |
| Bundle composition and export/import | PRD BND-11, BND-12; spec section 11 |
| Team use of committed selections; alias indirection | PRD Q3 (section 15); spec section 11 |
| Native Hermes adapter | PRD Q6 (section 15); spec section 11 |
| Windows support | PRD D2; spec section 11 "Windows" |
| Adopt hand-placed skills (in PRD section 12, not in spec section 11) | PRD OWN-6 |
| Remote sources (in PRD section 12, not in spec section 11) | PRD SRC-7 |
