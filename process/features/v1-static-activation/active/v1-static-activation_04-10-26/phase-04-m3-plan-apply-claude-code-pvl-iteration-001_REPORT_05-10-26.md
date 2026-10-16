---
iteration: 1
domain: plan
plan: phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md
gaps_found: 12
fail_count: 0
concern_count: 12
applied: 12
loop_status: CONTINUING
date: 2026-10-05
---

# Milestone M3 PVL iteration 001

Input: first-pass validate verdict Gate CONDITIONAL, 0 FAILs, 12 gaps (G1 and G2 high-severity safety, both probed).

| Gap | Area | Disposition |
|---|---|---|
| G1 | Lock file and state dir followed symlinks (probed overwrite outside project) | APPLIED: lstat refusal, fd vs path identity check, StateDirInvalid |
| G2 | Destination root only classified at plan time | APPLIED: ensure_dest_root before first fs op in execute, resume, rollback, and after the prompt; DestRootInvalid |
| G3 | created_dirs removal algorithm missing | APPLIED: Step::RemoveDir, dir_removed records |
| G4 | Fault injection scope too narrow | APPLIED: every mutating FsOps method, two scenarios, four modes |
| G5 | Rollback left empty state dirs | APPLIED: remove_state_dir_if_empty bottom-up |
| G6 | Torn journal tail, orphan history, dir record timing | APPLIED |
| G7 | Project root inside a source root | APPLIED: project-overlaps-source before the lock |
| G8 | Checklist compile order | APPLIED |
| G9 | Deletion manifest vs hash skip lists | APPLIED |
| G10 | Receipt target and journal root validation | APPLIED |
| G11 | Unchanged selection fields | APPLIED |
| G12 | Residuals R1 to R9 and adversarial scenarios | APPLIED |

Orchestrator decision: accept 20 error variants (StateDirInvalid, ProjectOverlapsSource, DestRootInvalid) rather than the validator's estimate of 18; DestRootInvalid is needed because plan Conflicts cannot be returned from apply.

Next: re-run vc-validate-agent from V1; it must refresh the contract's stale counts.
