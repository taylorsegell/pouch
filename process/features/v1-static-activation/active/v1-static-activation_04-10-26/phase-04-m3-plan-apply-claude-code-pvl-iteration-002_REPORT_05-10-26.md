---
iteration: 2
domain: plan
plan: phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md
gaps_found: 2
fail_count: 0
concern_count: 2
applied: 2
loop_status: CONTINUING
date: 2026-10-05
---

# Milestone M3 PVL iteration 002

Input: cycle-2 validate verdict Gate CONDITIONAL, 0 FAILs; 11 of 12 cycle-1 gaps verified closed; two new gaps.

| Gap | Area | Disposition |
|---|---|---|
| G13 | M3-activated projects never registered in projects.json (source rm guard would miss live links) | APPLIED: change::run registers after every selection write; warning project-index-failed; CLI assertion in AC-18 test |
| G14 | AC-31 scenario A unbuildable; first-apply path never fault-injected | APPLIED: scenario C (first add, copy strategy, empty project) across the full matrix |
| Nits | exit lists, test placement, checklist id collision, status lines, safety list | APPLIED |

Residual confirmed by probe: R4 lock-window ancestor swap of .unam can overwrite an outside file named lock; std-only code cannot close it (unsafe forbidden, no openat/O_NOFOLLOW). Accepted as a named known-gap.

Next: re-run vc-validate-agent from V1.
