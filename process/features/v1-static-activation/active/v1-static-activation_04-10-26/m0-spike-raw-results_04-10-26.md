# Milestone M0 spike: raw results as reported by Taylor

Transcribed into `spike/symlink-discovery.md` once all runs are in.

| Run | Agent | Destination | probe-a (symlink) at start | probe-b (copy) at start | probe-c mid-session | Refresh needed | Date | Notes |
|---|---|---|---|---|---|---|---|---|
| 1 | Claude Code 2.1.285 | `.claude/skills` (.agents hidden) | Y | Y | Y | start: none; mid: none | 2026-10-05 | probe-c appeared on Question 2 with no reload; agent did not report running tools |
| 2 | Claude Code 2.1.285 | `.agents/skills` (.claude hidden) | N | N | N | start: not visible even after restart; mid: not visible | 2026-10-05 | Valid redo at 11:24 after an invalid first attempt (probe-c linked before Question 1). Neither the symlink nor the real copy was listed, so Claude Code does not scan .agents/skills at all; not a symlink effect. Folder state verified: .claude hidden, probe-c linked 11:25 |
| 3 | Codex codex-cli 0.160.0 | `.agents/skills` (both folders present) | Y | Y | Y | start: none; mid: none | 2026-10-05 | probe-c listed on Question 2 with no restart; agent did not report running tools |
| 4 | Cursor (cursor-agent CLI) v2026.10.01-e373342 | `.claude/skills` (.agents hidden) | Y | Y | N before restart; Y after restart | start: none; mid: restart | 2026-10-05 | Workspace Trust prompt on first start, accepted (cursor-agent residue outside skill dirs). probe-c linked 11:32 between questions, not listed on Question 2; listed after quitting and restarting cursor-agent (restart check used a shorter yes/no prompt without the no-tools clause; answer came instantly) |
| 5 | Cursor (cursor-agent CLI) v2026.10.01-e373342 | `.agents/skills` (.claude hidden) | Y | Y | N before restart; Y after restart | start: none; mid: restart | 2026-10-05 | Folder state verified: .claude hidden, probe-c linked 11:37 between questions. Not listed on Question 2; listed after restarting cursor-agent (restart check used the shorter yes/no prompt) |
| 6 | Hermes Agent v0.21.5+7344.gea32af9 (2026.9.24) | `.agents/skills` (both folders present) | before trust N; after trust Y | before trust N; after trust Y | N before restart; Y after restart | start: trust step; mid: restart | 2026-10-05 | probe-c linked 11:44. Hermes prefixes project skill descriptions with "[project]". Restart check used the shorter yes/no prompt |
