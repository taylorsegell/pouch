---
name: note:product-phase-3-dynamic-skill-routing
description: "Deferred pointer: product Phase 3 (intent-driven exposure, dynamic skill routing), including the atlas; gated by PRD D8"
date: 04-10-26
metadata:
  node_type: memory
  type: note
---

# Product Phase 3: dynamic skill routing

**Status:** deferred. Pointer only; the source documents hold all scope.

**Summary:** a per-message hook plus a deterministic router that exposes skills for one turn, using author metadata and a per-project atlas (RTE-2, which is part of this entry).

## Why deferred

Product Phase 3 is out of scope until product Phase 1 is in daily use and a baseline week exists (brief "Out of scope"; PRD D8).

## Gate to revisit

PRD D8: product Phase 1 in daily use, plus one recorded baseline week with wasted tool calls tagged by cause. Stop if the gate fails (PRD section 12 "Entry gate"; ADR-0001 action item 1).

## Sources

- `docs/adr/ADR-0001-dynamic-skill-routing.md`: status Proposed; Action Items list (items 1 to 8); Decision sections 1 to 6; "To revisit".
- `docs/spec/PRD-unambiguous.md`: section 12 "Phase 3"; section 7.11 (RTE-1 to RTE-12); DIS-11, DIS-12 (7.1); SES-5 (7.8); EDG-14, EDG-15 (7.9); section 0 (D8, D9); section 9 (atlas and metadata schema); section 15 Q5, Q7, Q8, Q9, Q10.
- `docs/spec/unam-technical-spec.md`: section 2.6 (reserved metadata paths).

## Not in this entry

Repository map and context offloading are separate entries (see siblings in this folder).
