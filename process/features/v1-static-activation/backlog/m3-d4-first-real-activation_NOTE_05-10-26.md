---
name: note:m3-d4-first-real-activation
description: "Backlog note from M3: Hybrid gate D4, one real activation in the first adopting project (owner Taylor)"
date: 05-10-26
feature: v1-static-activation
---

# M3 follow-up: Hybrid gate D4, first real activation (owner Taylor)

Priority: required before M3 is called VERIFIED for daily use (PRD 12 daily-use path); not a code blocker.

Gate: D4, Hybrid (precondition: a project named, a real Claude Code installed). Status until done: CONDITIONAL, with this note as its recorded path. It is not a Known-Gap pass.

What: one real activation in the first adopting project. That project is still unnamed (umbrella open item 54). This repository is excluded by the brief rule (its `.agents/skills` is a symlink to the toolkit-owned `.claude/skills`).

Commands to run in that project (by Taylor, outside every test harness):

1. `unam add <ref> --yes`
2. Start a fresh Claude Code session in the project and check that the skill is discovered.
3. `unam deactivate --yes`

Evidence to record: the plan output of step 1, `ls -l .claude/skills` after step 1, the discovery result in the fresh session, and the output of step 3.

Source: phase-04-m3-plan-apply-claude-code_PLAN_04-10-26.md, P23 and step J3.
