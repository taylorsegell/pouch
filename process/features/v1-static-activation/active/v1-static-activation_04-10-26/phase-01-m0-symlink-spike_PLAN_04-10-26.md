---
name: plan:v1-static-activation-phase-01-m0-symlink-spike
description: "v1: static activation, phase 1 of 8: milestone M0, gating symlink discovery spike (executable manual runbook for Taylor, gate for everything)"
date: 04-10-26
metadata:
  node_type: memory
  type: plan
  feature: v1-static-activation
  phase: phase-01
---

# Milestone M0: Gating spike (symlink discovery)

**Program:** v1-static-activation
**Umbrella plan:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/v1-static-activation-umbrella_PLAN_04-10-26.md
**Date**: 04-10-26
**Complexity**: SIMPLE (one manual runbook, no code)
**Phase:** Phase 1 of 8: milestone M0
**Status**: PLANNED
**Phase status:** VALIDATED (Gate: PASS 2026-10-04 after 2 PVL fix cycles; EXECUTE is Taylor running the spike)
**Report destination:** process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_REPORT_04-10-26.md (flat in the program task folder)
**Primary execute anchor:** this file. Supporting phase files: none.

TL;DR: Taylor runs 6 runs by hand (Claude Code x2, Codex, Cursor x2, Hermes; Windsurf not installed), which is 7 short agent sessions because Hermes starts twice (before and after trust), pastes one fixed report block per run, and the orchestrator transcribes the results into `spike/symlink-discovery.md` and applies the decision rule. Nothing is written to user-scope skill directories or to this repository's skill folders. The one sanctioned write outside `$SPIKE` is the `skills.trusted_project_dirs` entry that `hermes skills trust` puts in `~/.hermes/config.yaml`; it is backed up first, removed by `hermes skills untrust` during cleanup, and the file is diffed against the backup before anything is deleted.

---

## Overview

Milestone M0 is the GATE for every other milestone in this program. It answers, per agent, whether a symlinked skill directory is discovered the same way as a real one, and it sets the default activation strategy per adapter (PRD Q1, risk R1, assumption A1; spec section 10 M0; brief rule "No application code before the M0 spike table is filled in").

The spike is a MANUAL procedure that Taylor runs by hand against real agents. The procedure of record is `spike/symlink-discovery.md` (9 steps). This plan turns it into copy-pasteable zsh commands for macOS. It is not a test and it is not automated. No application code and no Cargo files exist before it is done. The orchestrator never launches the agents; Taylor does.

---

## Scope

- Run the procedure in `spike/symlink-discovery.md` using the commands in "Runbook" below.
- Fill the results table: adapter by strategy, with agent version and date on every row; Windsurf is "not tested".
- Fill the Outcome section: default strategy per adapter (`claude-code`, `agents-neutral`), and any adapter claim in the spec that the results contradict.
- Apply the decision rule in the spike file (symlink not discovered means that adapter drops `symlink`, defaults to `copy`, and the reason is recorded).
- Record any claim change for later propagation to `docs/adapters.md` (does not exist yet; the M0 Outcome may be recorded there once it exists, created by whichever milestone first has something to record (M2 for the slugify outcome, otherwise M4); it is a new spec-called document, not one of the five protected design documents) and `process/context/product/all-product.md` (update trigger).

---

## Research Findings (orchestrator scout, 04-10-26, read-only)

| Agent | Installed | Version at scout time | Start command | Reads (source) |
|---|---|---|---|---|
| Claude Code | yes | 2.1.285 (`claude`) | `claude` | `.claude/skills` (may also read `.agents/skills`) (PRD 10.1) |
| Codex | yes | 0.160.0 (`codex`) | `codex` | `.agents/skills` only (also user scope `~/.agents/skills`, untouched) (PRD 10.1) |
| Cursor | yes (app 3.23.14 plus `cursor-agent`) | 2026.09.08-6caf4ff (`cursor-agent`) | `cursor-agent` (no arguments, interactive; never use `-p` or `--print`) | `.claude/skills` and `.agents/skills` (PRD 10.1) |
| Hermes Agent | yes (`~/.hermes` exists) | v0.21.5 (`hermes`) | `hermes chat` from `$SPIKE/repo` | `.agents/skills` only; project-local skills need `hermes skills trust` once per repo (PRD 5.1 and 10.2, spec 4.7) |
| Windsurf | NO (no app, not on PATH; `~/.codeium` exists but no Windsurf) | n/a | n/a | row is "not tested" |

User-scope skill dirs `~/.claude/skills` and `~/.agents/skills` already exist and hold Taylor's own skills (for example `block-organizer`). The probes use the spike's names `probe-a`, `probe-b`, `probe-c` (no collision) and each has a distinctive description beginning `UNAM SPIKE PROBE`. Nothing in this runbook writes to any user-scope skill directory.

---

## Design Gap and Resolution (sequencing only; procedure unchanged)

The procedure places `probe-a` and `probe-b` in BOTH `.claude/skills` and `.agents/skills`. An agent that reads both locations (Claude Code may; Cursor does per PRD 10.1) would list the same name once, so the table could not tell which destination was discovered.

Resolution (same probes, same placement, sequencing only): for Claude Code and Cursor, run twice.

- Run 1: move `.agents` aside (`mv .agents .agents.off`) so only `.claude/skills` is visible. Fills the `.claude/skills` rows.
- Run 2: restore `.agents`, move `.claude` aside (`mv .claude .claude.off`). Fills the `.agents/skills` rows.

Codex and Hermes read only `.agents/skills` (PRD 10.1), so they get one run each with both directories in place (the `.claude` copy is inert for them). This deviation is to be listed under contradictions and open items for Taylor in the phase report.

---

## Acceptance Criteria

- Closes no PRD acceptance criterion (spec section 10 lists none for M0).
- Supports PRD S9 (no adapter claim without evidence) and resolves PRD Q1 (symlink-first or copy-first default).
- AC-M0-1: results table in `spike/symlink-discovery.md` has every row filled (version, date, outcomes) or marked "not tested" with no empty cell. Proven by: gate G1 and G2 (document checks). Strategy: Hybrid (precondition: Taylor ran the sessions).
- AC-M0-2: Outcome section names the default strategy per adapter (`claude-code`, `agents-neutral`) per the decision rule, and lists any contradicted spec claim. Proven by: gate G4a, G4b-claude-code, G4b-agents-neutral plus orchestrator read. Strategy: Hybrid.
- AC-M0-3: Status line no longer reads "not run". Proven by: gate G3. Strategy: Fully-Automated (grep).
- AC-M0-4: `docs/` untouched. Proven by: gate G5. Strategy: Fully-Automated.
- AC-M0-5: nothing written to user-scope skill dirs; Hermes trust entry removed; spike dirs deleted. Proven by: Taylor's cleanup confirmation block and gate G6. Strategy: Hybrid.

---

## Entry Gate

- None. This is the first milestone. It does not depend on any other milestone.
- Taylor available to run the spike by hand (6 runs, 7 short agent sessions because Hermes starts twice, two terminals each).

---

## Blast Radius

- `spike/symlink-discovery.md` (results table, Outcome, Status line). Transcribed by the orchestrator from Taylor's pasted report blocks (editing this file is authorized by Taylor; `spike/` is not `docs/`).
- A throwaway Git repository and a scratch directory at `$HOME/unam-spike-2026-10-04`, outside this repository and outside `~/Developer/01-AI/skillzone`, deleted at the end.
- ONE sanctioned exception outside `$SPIKE`: `hermes skills trust "$SPIKE/repo"` writes a `skills.trusted_project_dirs` entry into `~/.hermes/config.yaml` (Hermes `save_config`). The file is copied to `$SPIKE/scratch/hermes-config.before.yaml` first; cleanup runs `hermes skills untrust "$SPIKE/repo"` before `rm -rf "$SPIKE"`. `save_config` can rewrite the WHOLE `~/.hermes/config.yaml` (parsed and re-serialized, so reformatting or default-stripping of unrelated keys is possible), not just add one entry. Cleanup therefore diffs the file against the backup BEFORE the backup is deleted, and Taylor keeps a copy outside `$SPIKE` if the diff shows more than reformatting.
- Known residue (not provable offline): each agent writes its own trust and session records outside skill dirs on first run in a new directory (see Open Items 6).
- No files under `docs/`, `crates/`, the skill library, `~/.claude/skills`, `~/.agents/skills`, the `~/.hermes` skills store, or this repository's `.claude/skills`.

---

## Runbook (Taylor runs these by hand; zsh on macOS)

Use a fresh terminal. Terminal 1 is the agent session. Terminal 2 is the shell that runs the mid-session step. Both start with the lines in their own block below. Every block here is paste-safe: no trailing comments, and the first line enables comments for zsh. Explanatory text lives in the prose around the blocks.

### R0. Preflight and setup (once)

Record the four version lines into the first report block (the version cell is the first line of each `--version` output). The two path checks must print OK lines; the existence check must print "OK: path free".

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
claude --version
codex --version
cursor-agent --version
hermes --version
git -C "$HOME" rev-parse --show-toplevel 2>/dev/null && echo "STOP: HOME is inside a git repo" || echo "OK: no enclosing git repo above spike"
case "$SPIKE" in
  "$HOME/Developer/01-AI/arkitect/unambiguous"*|"$HOME/Developer/01-AI/skillzone"*) echo "STOP: spike path is inside a protected project" ;;
  *) echo "OK: spike path is outside protected projects" ;;
esac
[ -e "$SPIKE" ] && echo "STOP: $SPIKE already exists" || echo "OK: path free"
mkdir -p "$SPIKE/repo" "$SPIKE/scratch"
git -C "$SPIKE/repo" init -b main
git -C "$SPIKE/repo" -c user.name=spike -c user.email=spike@example.invalid commit --allow-empty -m "spike root"
```

If any STOP line prints, do not continue; report it.

### R1. Create the three probe sources in scratch

probe-c is linked only mid-session (step M below).

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
for p in a b c; do mkdir -p "$SPIKE/scratch/probe-$p"; done

cat > "$SPIKE/scratch/probe-a/SKILL.md" <<'EOF'
---
name: probe-a
description: UNAM SPIKE PROBE A (symlink probe). Spike marker only; do not load.
---

This is probe A for the unam symlink discovery spike.
EOF

cat > "$SPIKE/scratch/probe-b/SKILL.md" <<'EOF'
---
name: probe-b
description: UNAM SPIKE PROBE B (copy probe). Spike marker only; do not load.
---

This is probe B for the unam symlink discovery spike.
EOF

cat > "$SPIKE/scratch/probe-c/SKILL.md" <<'EOF'
---
name: probe-c
description: UNAM SPIKE PROBE C (mid-session symlink probe). Spike marker only; do not load.
---

This is probe C for the unam symlink discovery spike.
EOF
```

### R2. Place probe-a (symlink) and probe-b (copy) in both destinations

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
cd "$SPIKE/repo"
mkdir -p .claude/skills .agents/skills
ln -s "$SPIKE/scratch/probe-a" .claude/skills/probe-a
ln -s "$SPIKE/scratch/probe-a" .agents/skills/probe-a
cp -R "$SPIKE/scratch/probe-b" .claude/skills/probe-b
cp -R "$SPIKE/scratch/probe-b" .agents/skills/probe-b
ls -l .claude/skills .agents/skills
```

Check: `probe-a` shows as `->` pointing at the absolute scratch path; `probe-b` is a real directory in both.

### Reset between runs (run before every run below)

The reset removes only the probe-c symlink (never its source) and restores a moved-aside directory ONLY if the original name is free. If both a directory and its `.off` twin exist (Claude Code run 2 may recreate `.claude/`), the block prints a STOP line and does not move anything: inspect both, then merge or remove by hand before continuing. Check the final `ls -la`: it must show `.agents` and `.claude` and no `.off` entries.

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
cd "$SPIKE/repo"
rm -f .claude/skills/probe-c .agents/skills/probe-c
if [ -d .agents.off ]; then
  if [ -e .agents ]; then echo "STOP: .agents exists, inspect"; else mv .agents.off .agents; fi
fi
if [ -d .claude.off ]; then
  if [ -e .claude ]; then echo "STOP: .claude exists, inspect"; else mv .claude.off .claude; fi
fi
ls -la
```

### Prompts (paste verbatim)

Start prompt (P1):

```
List every skill available to you in this session: name and one-line description. Do not load, open, or run any skill. Then say whether you see any skill whose description starts with UNAM SPIKE PROBE. Answer only from the skill list already in your session context; do not run shell commands, list directories, or read files.
```

Optional cross-check (P1b), paste after P1:

```
Is a skill named exactly probe-a, probe-b available to you right now? Answer from your loaded skill list only.
```

Follow-up prompt after the mid-session step (P2):

```
I may have just added another skill while you were running. List every skill available to you again: name and one-line description. Do not load, open, or run any skill. Tell me whether any skill whose description starts with UNAM SPIKE PROBE is new compared with your previous list. Answer only from the skill list already in your session context; do not run shell commands, list directories, or read files.
```

Contamination rule: if an agent runs a tool (shell, file read, directory listing) anyway, Taylor records that in Notes and that result counts as unverified (not a valid Y or N for the table; re-ask once, else transcribe as "unverified").

### Mid-session step (M), terminal 2

Run in terminal 2 while the agent session stays open. First the terminal-2 setup block, then exactly ONE of the two M blocks, depending on the destination under test in that run.

Terminal-2 setup:

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
```

M-claude (run when the run tests `.claude/skills`: runs 1 and 4):

```zsh
ln -s "$SPIKE/scratch/probe-c" "$SPIKE/repo/.claude/skills/probe-c"
```

M-agents (run when the run tests `.agents/skills`: runs 2, 3, 5, 6):

```zsh
ln -s "$SPIKE/scratch/probe-c" "$SPIKE/repo/.agents/skills/probe-c"
```

If a probe is not visible after P1 or P2, try in this order and record which was needed: Claude Code `/reload-skills` then re-ask; then fully quit and restart the agent and re-ask. Codex: quit and restart. Cursor: quit and restart `cursor-agent`. Hermes: run the trust step, then restart. Record the first thing that made it appear. "Not visible even after restart" is a valid result.

### Run order

| Run | Agent | Isolation before start | Destination under test | Start command (terminal 1, in `$SPIKE/repo`) | Table row(s) filled in `spike/symlink-discovery.md` |
|---|---|---|---|---|---|
| 1 | Claude Code run 1 | reset block, then `mv .agents .agents.off` | `.claude/skills` | `claude` | row 1 (Claude Code, `.claude/skills`, symlink: probe-a start, probe-c mid-session) and row 2 (Claude Code, `.claude/skills`, copy: probe-b start) |
| 2 | Claude Code run 2 | reset block, then `mv .claude .claude.off` | `.agents/skills` | `claude` | row 3 (Claude Code, `.agents/skills`, symlink). probe-b copy result goes in Notes (no table row) |
| 3 | Codex | reset block (both dirs in place) | `.agents/skills` | `codex` | row 4 (Codex symlink: probe-a start, probe-c mid-session) and row 5 (Codex copy: probe-b start) |
| 4 | Cursor (cursor-agent CLI) run 1 | reset block, then `mv .agents .agents.off` | `.claude/skills` | `cursor-agent` (no arguments; do not use `-p` or `--print`) | row 7 (Cursor (cursor-agent CLI), `.claude/skills`, symlink) |
| 5 | Cursor (cursor-agent CLI) run 2 | reset block, then `mv .claude .claude.off` | `.agents/skills` | `cursor-agent` (no arguments; do not use `-p` or `--print`) | row 6 (Cursor (cursor-agent CLI), `.agents/skills`, symlink) |
| 6 | Hermes | reset block (both in place) | `.agents/skills` | `hermes chat` | row 9 (Hermes, `.agents/skills`, symlink). Record before-trust and after-trust results |
| - | Windsurf | n/a | n/a | not installed | row 8: "not tested" (version `n/a`, date = the run date Taylor reports in the other report blocks) |

Note: the table in `spike/symlink-discovery.md` lists rows in this order: 1 Claude `.claude` symlink, 2 Claude `.claude` copy, 3 Claude `.agents` symlink, 4 Codex symlink, 5 Codex copy, 6 Cursor `.agents` symlink, 7 Cursor `.claude` symlink, 8 Windsurf, 9 Hermes.

### Per-run steps

1. Run the reset block. Apply the isolation in the run-order table.
2. Terminal 1: `cd "$SPIKE/repo"` and start the agent with the start command (always from `$SPIKE/repo`). Cursor: `cursor-agent` with no arguments; do not use `-p` or `--print`. Hermes: run `hermes chat` first WITHOUT trust and paste P1 (record before-trust result); then run the Hermes trust sequence (below), restart `hermes chat` from `$SPIKE/repo`, paste P1 again (record after-trust result).
3. Paste P1 (optionally P1b). Record: probe-a seen (Y/N), probe-b seen (Y/N). If neither is seen, apply the "not visible" ladder and record what was needed.
4. Terminal 2: run the terminal-2 setup block, then the M block for the destination under test. The M step is MANDATORY for runs 2 and 5 (their mid-session cell would otherwise be `not run`) and recommended for all runs so the mid-session column is filled in every symlink row.
5. Terminal 1: paste P2. Record: probe-c seen mid-session (Y/N); if N, apply the ladder and record what was needed.
6. Quit the agent. Fill in the report block (below) and keep it for pasting.

Hermes trust sequence (run 6 only, in a shell, after the before-trust P1 result is recorded):

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
cp ~/.hermes/config.yaml "$SPIKE/scratch/hermes-config.before.yaml"
hermes skills trust "$SPIKE/repo"
```

Hermes extra: `hermes skills trust` writes `skills.trusted_project_dirs` into `~/.hermes/config.yaml`. That single entry is the one sanctioned write outside `$SPIKE` (named in Blast Radius and Hard Stops). Always use the explicit path above, never a bare `hermes skills trust`. The command does not prompt: it prints `Trusted: <path>`. Check that the printed path is `$SPIKE/repo` and nothing else; if it prints any other path, or reports a write anywhere other than `~/.hermes/config.yaml`, stop and report. Hermes `save_config` can rewrite the whole file, not only that entry, which is why the backup exists and why Cleanup diffs against it. Cleanup removes the entry with `hermes skills untrust "$SPIKE/repo"` BEFORE the directory is deleted (untrust fails with "Not a directory" once the path is gone).

---

## Report Format (Taylor pastes one block per run)

```
AGENT: <Claude Code | Codex | Cursor (cursor-agent CLI) | Hermes | Windsurf>
VERSION: <first line of its --version output>
RUN: <1 | 2 | single>
DESTINATION: <.claude/skills | .agents/skills>
PROBE-A (symlink) SEEN AT START: <Y | N | unverified>
PROBE-B (copy) SEEN AT START: <Y | N | unverified>
PROBE-C (symlink added mid-session) SEEN MID-SESSION: <Y | N | unverified | not run>
REFRESH-AT-START: <none | /reload-skills | restart | trust step | not visible even after restart>
REFRESH-MID-SESSION: <none | /reload-skills | restart | not visible even after restart | n/a>
DATE: <YYYY-MM-DD>
NOTES: <optional; verbatim agent reply excerpt; for Hermes, before-trust vs after-trust result; any tool the agent ran anyway>
```

Field RUN is the per-agent run number (`1` or `2` for Claude Code and Cursor, `single` for Codex and Hermes). The run table numbers 1 to 6 are global across agents; DESTINATION disambiguates when mapping a block to its run-table row.

### Transcription rules (orchestrator, when filling the results table)

| Case | Rule |
|---|---|
| Copy rows (rows 2 and 5): mid-session cell | `n/a (copy probe; probe-c is symlink-only)` |
| Hermes row 9: start cell | the AFTER-trust result; before-trust result goes in Notes; Refresh = `trust step` when trust was needed |
| M step in runs 2 and 5 | mandatory; if skipped the mid-session cell = `not run` |
| Refresh needed cell (every row) | `start: <REFRESH-AT-START>; mid: <REFRESH-MID-SESSION>`. Copy rows use the start value with `mid: n/a`. Hermes uses `trust step` as the start value when trust was needed |
| Cursor rows (6 and 7) | The Agent cell is `Cursor (cursor-agent CLI)`. The Outcome section states that the Cursor editor was not tested and that these rows are not evidence for the editor |
| Version cell | first line of the agent's `--version` output |
| Agent ran a tool despite the prompt | cell = `unverified`, reason in Notes |
| Any cell | no cell may be empty; use `n/a`, `not run`, `unverified`, or `not tested` |

The orchestrator then (after all blocks are in) fills the results table and the Outcome section in `spike/symlink-discovery.md` (spike/ is not docs/; editing that file is authorized by Taylor), updates the Status line, and applies the decision rule. The orchestrator does not launch agents and does not run the sessions.

---

## Cleanup (after results are reported)

Only after Taylor has pasted all report blocks and the orchestrator has confirmed it has them. Order is fixed: Hermes untrust and config diff, Taylor confirms the diff, then guarded delete, then checks. Cleanup is two blocks so the diff is reviewed before the backup is deleted.

Cleanup block 1 (untrust and diff):

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
hermes skills untrust "$SPIKE/repo"
diff "$SPIKE/scratch/hermes-config.before.yaml" ~/.hermes/config.yaml | head -40
```

Taylor, read the diff output. Empty output, or output that is reformatting only (whitespace, key order, quoting), means go on to block 2. If the diff shows anything else (a changed or removed value, a missing key), do NOT run block 2 yet. Copy the backup outside `$SPIKE` first with this exact command, then report the diff to the orchestrator before deleting anything:

```zsh
cp "$SPIKE/scratch/hermes-config.before.yaml" "$HOME/hermes-config.before-unam-spike.yaml"
```

If trust was never applied (run 6 skipped), the backup file does not exist and the untrust and diff lines may print errors; that is fine, go on to block 2.

Cleanup block 2 (guarded delete and checks). The delete runs only when `$SPIKE` is exactly `$HOME/unam-spike-2026-10-04`. Expected output: `CLEANED`, no probe names from the `ls | grep`, and `0` from the final count.

```zsh
setopt interactive_comments
export SPIKE="$HOME/unam-spike-2026-10-04"
case "$SPIKE" in "$HOME"/unam-spike-2026-10-04) rm -rf "$SPIKE" ;; *) echo "STOP: unexpected SPIKE=$SPIKE" ;; esac
[ -e "$SPIKE" ] && echo "STILL EXISTS" || echo "CLEANED"
ls ~/.claude/skills ~/.agents/skills 2>/dev/null | grep -ixE 'probe-[abc]'
grep -c 'unam-spike-2026-10-04' ~/.hermes/config.yaml
```

Reminder: the only write outside `$SPIKE` was the sanctioned Hermes trust entry in `~/.hermes/config.yaml`, removed by the untrust line in cleanup block 1, and the config diff there is reviewed before block 2 deletes the backup. Nothing was written to `~/.claude/skills`, `~/.agents/skills`, the `~/.hermes` skills store, or the skill library. `rm -rf "$SPIKE"` removes only the throwaway repo and scratch (symlinks inside it are removed, not followed). Agent-owned trust and session records are known residue (Open Items 6).

---

## Implementation Checklist

### Step A: Prepare (agent-assisted, read-only)

- [x] A1. Re-read `spike/symlink-discovery.md`; installed agents and versions listed (see Research Findings).
- [ ] A2. Taylor runs R0 preflight; both OK lines print (spike path outside this repo, outside `~/Developer/01-AI/skillzone`, no enclosing git repository above the spike) and "OK: path free" prints.

### Step B: Run the spike (Taylor, manual)

- [ ] B1. R1 and R2 executed; `ls -l` confirms probe-a is a symlink and probe-b is a real directory in both destinations.
- [ ] B2. Runs 1 to 6 from the run-order table executed, each with the reset block first; one report block per run.
- [ ] B3. Windsurf row recorded as "not tested".
- [ ] B4. Taylor pastes all report blocks to the orchestrator.

### Step C: Record the outcome (orchestrator transcribes; Taylor confirms)

- [ ] C1. Fill all 9 rows of the results table by the transcription rules: version and date on every row (Windsurf: "not tested").
- [ ] C2. Fill the Outcome section: default strategy per adapter (`claude-code`, `agents-neutral`) by the decision rule; state that the Cursor editor was not tested and that the `Cursor (cursor-agent CLI)` rows are not evidence for the editor; record the sequencing deviation (two-run isolation for Claude Code and Cursor).
- [ ] C3. List any spec or PRD adapter claim the results contradict; report to Taylor (do not edit `docs/`). Note `docs/adapters.md` does not exist yet; the Outcome may be recorded there once it exists (milestones M4/M7).
- [ ] C4. Update the spike Status line from "not run" to the actual state (for example "run 2026-10-04; Windsurf not tested"), keeping the `**Status:**` line format.
- [ ] C5. Cleanup per the Cleanup section (Hermes untrust first, guarded delete, then checks); Taylor confirms.

---

## Exit Gate

M0 has no cargo gate. Its gate is the filled table, and the Outcome must name the Cursor editor as not tested (the Cursor rows are labeled `Cursor (cursor-agent CLI)` and are not evidence for the editor; this is a prose requirement, not one of the gate commands). Run from the project root (`/Users/databased/Developer/01-AI/arkitect/unambiguous`). Each gate prints an explicit PASS or FAIL line. The Status line format in `spike/symlink-discovery.md` is `**Status:** not run` (line 3).

```bash
n=$(grep -cE '^\| (Claude Code|Codex|Cursor|Windsurf|Hermes) ' spike/symlink-discovery.md); [ "$n" -eq 9 ] && echo "G1 PASS" || echo "G1 FAIL: rows=$n expected 9"
grep -E '^\| (Claude Code|Codex|Cursor|Windsurf|Hermes) ' spike/symlink-discovery.md | grep -qE '\|[[:space:]]*\|' && echo "G2 FAIL: empty cell" || echo "G2 PASS"
s=$(grep -c '^\*\*Status:\*\*' spike/symlink-discovery.md); r=$(grep -c '^\*\*Status:\*\* not run' spike/symlink-discovery.md); [ "$s" -eq 1 ] && [ "$r" -eq 0 ] && echo "G3 PASS" || echo "G3 FAIL: status_lines=$s not_run=$r"
p=$(sed -n '/^## Outcome/,$p' spike/symlink-discovery.md | grep -c 'To be filled in'); [ "$p" -eq 0 ] && echo "G4a PASS" || echo "G4a FAIL: placeholder count=$p"
c=$(sed -n '/^## Outcome/,$p' spike/symlink-discovery.md | grep -ci 'claude-code'); [ "$c" -ge 1 ] && echo "G4b-claude-code PASS" || echo "G4b-claude-code FAIL"
a=$(sed -n '/^## Outcome/,$p' spike/symlink-discovery.md | grep -ci 'agents-neutral'); [ "$a" -ge 1 ] && echo "G4b-agents-neutral PASS" || echo "G4b-agents-neutral FAIL"
if git rev-parse --verify HEAD >/dev/null 2>&1; then [ -z "$(git status --porcelain -- docs/)" ] && echo "G5 PASS" || echo "G5 FAIL: docs/ changed"; else echo "G5 PRECONDITION FAIL: no commit yet (harness-setup commit must exist)"; fi
d=0; [ -e "$HOME/unam-spike-2026-10-04" ] && d=1; l=$(ls ~/.claude/skills ~/.agents/skills 2>/dev/null | grep -icxE 'probe-[abc]'); h=$(grep -c 'unam-spike-2026-10-04' ~/.hermes/config.yaml 2>/dev/null); [ "$d" -eq 0 ] && [ "$l" -eq 0 ] && [ "$h" -eq 0 ] && echo "G6 PASS" || echo "G6 FAIL: spike_dir=$d probes=$l hermes_entries=$h"
```

Notes on the gates:

- All eight lines (G1, G2, G3, G4a, G4b-claude-code, G4b-agents-neutral, G5, G6) running together are the single PASS/FAIL report for G1 to G6.
- G5 is meaningful only once the harness-setup commit exists (precondition `git rev-parse --verify HEAD` succeeds). It lists untracked or modified files under `docs/` after that commit.
- G6 checks for the Hermes trust entry only; whole-file changes to `~/.hermes/config.yaml` are reviewed through the cleanup block 1 diff, not by G6.
- Every row filled or marked "not tested" with version and date.
- Outcome states the default strategy per adapter per the decision rule; contradicted spec claims are listed for Taylor.
- Taylor explicitly confirms the spike is complete.
- Phase report written to the report destination above.

---

## Hard Stops

- No application code, no Cargo files.
- No writes to `~/.claude/skills`, `~/.agents/skills`, the `~/.hermes` skills store, or the skill library. The ONE sanctioned exception: the `skills.trusted_project_dirs` entry that `hermes skills trust "$SPIKE/repo"` writes into `~/.hermes/config.yaml`, backed up first to `$SPIKE/scratch/hermes-config.before.yaml`, removed by `hermes skills untrust "$SPIKE/repo"` before `rm -rf "$SPIKE"`, and diffed against the backup before deletion (keep a copy outside `$SPIKE` if the diff is more than reformatting).
- Never run any probe step against this repository's `.claude/skills`.
- The orchestrator never launches the agents; Taylor does.
- Do not edit `docs/`; contradictions are reported to Taylor only.
- If an agent prompts to write outside `$SPIKE/repo` (other than the one sanctioned Hermes trust entry), stop and report.
- No yolo or bypass-permissions modes in any agent; deny any write prompt during the spike.
- Never use `-p` or `--print` with `cursor-agent`.

---

## Phase Completion Rules

- CODE DONE does not apply (no code). M0 is VERIFIED only when the exit gate holds and Taylor has confirmed.
- No later milestone may start RESEARCH-to-EXECUTE work that depends on adapter defaults until M0 is VERIFIED. Every later plan lists this as an entry dependency.

---

## Blockers That Would Justify BLOCKED Status

- PRD Q1 (default strategy per adapter) is decided by this spike; until the table is filled it is unresolved. Needed by: M0 itself.
- No supported agent runnable by Taylor in the time available.
- Results contradict a spec claim in a way that changes the adapter design: report to Taylor; the PRD and spec stay authoritative until Taylor changes them.

---

## Contradictions and Open Items for Taylor

1. RESOLVED 2026-10-04 (decisions 6 and 7). Procedure deviation (sequencing only; procedure unchanged): Claude Code and Cursor are run twice with the other destination moved aside (one folder hidden per run), because the procedure places the same probe names in both destinations and the table could not otherwise attribute discovery to a destination. Taylor accepted this run order. Spec 10 M0 now points to `spike/symlink-discovery.md`, which is authoritative for the procedure, so the spec/spike procedure contradiction is closed.
2. STILL OPEN (named residual). Windsurf is not installed; its row is "not tested", which leaves its behavior "not documented" per the decision rule.
3. RESOLVED 2026-10-04 (decision 4). `docs/adapters.md` does not exist yet; the decision rule says to record reasons there. Only the five kickoff design documents are protected; `docs/adapters.md` is a new spec-called document that agents may create and maintain (milestones M4/M7). The M0 Outcome may be recorded there once it exists. No Taylor permission is needed for it.
4. STILL OPEN (noted in the results). Cursor is tested through `cursor-agent` (CLI) only; the Agent cell reads `Cursor (cursor-agent CLI)` and the Outcome states the Cursor editor was not tested. These rows are not evidence for the editor; if the editor behaves differently, that is outside this spike.
5. RESOLVED 2026-10-04 (decision 12): Taylor accepted the Hermes trust write as planned (backup, single write, remove in cleanup, show the diff before deleting); no runbook change. Hermes project-local skills need `hermes skills trust`; the before/after result is recorded because it affects the `agents-neutral` default. The trust write to `~/.hermes/config.yaml` is the one sanctioned exception to "nothing outside `$SPIKE`" and is reversed by untrust at cleanup.
6. STILL OPEN (named residual). Known residue (not provable offline): each agent writes its own trust and session records outside skill dirs on first run in a new directory (for example Claude Code project trust and session records, Codex and Cursor trust or session records). The spike cannot prevent or prove removal of these; they are named here, not cleaned.

---

## Phase Loop Progress

Orchestrator reads this before deciding which subagent to spawn next. The canonical 7-step inner loop `R -> I -> P -> PVL -> E -> EVL -> UP` SKIPS SPEC (SPEC runs once in the outer program loop). For M0, the EXECUTE step is Taylor's manual spike.

- [x] 1. RESEARCH: orchestrator scout done 04-10-26 (spike file read; installed agents and versions listed; context loaded)
- [x] 2. INNOVATE: n/a (mechanical procedure fixed by the spike file; the only design gap is resolved by sequencing, see Design Gap and Resolution)
- [x] 3. PLAN-SUPPLEMENT: plan-agent: stub taken through planning; Inner Loop Refresh Note written below
- [x] 4. PVL (Gate PASS 2026-10-04, 2 fix cycles; results.tsv): vc-validate-agent: full V1-V7; validate-contract written per `.claude/skills/vc-validate-findings/references/example-validate-output.md`
- [ ] 5. EXECUTE: Taylor runs the spike; table and Outcome filled
- [ ] 6. EVL: orchestrator reads the filled file against the exit gate; EVL HANDOFF SUMMARY written
- [ ] 7. UPDATE PROCESS: phase report written, umbrella state updated, context updated, commit done

**Validate-contract required before execute.** If step 4 (PVL) is unchecked or `## Validate Contract` reads "(placeholder - vc-validate-agent writes this section before EXECUTE)", the orchestrator must spawn vc-validate-agent first. A partial contract missing Plan updates applied / Execute-agent instructions / Test gates is treated as a placeholder.

---

## Inner Loop Refresh Note

Date: 04-10-26. Sections changed in PLAN-SUPPLEMENT: Overview, Scope, Research Findings (new), Design Gap and Resolution (new), Acceptance Criteria (AC-M0-1 to AC-M0-5), Blast Radius, Runbook (new), Report Format (new), Cleanup (new), Implementation Checklist, Exit Gate (runnable gates G1 to G6), Hard Stops (new), Contradictions and Open Items (new), Verification Evidence, Resume and Execution Handoff. Cause: orchestrator scout found Windsurf not installed and a probe-placement ambiguity in the spike procedure.

---

### Amendment 2026-10-04 (label-only, approved by Taylor)

Cursor labeling: the Agent cell for rows 6 and 7 is `Cursor (cursor-agent CLI)`; the Outcome states that the Cursor editor was not tested. Changed text: Transcription rules (Cursor rows), run-order table and Report Format AGENT values, Step C2, Exit Gate prose, Out-of-scope note, Contradictions item 4. These were label-only changes approved by Taylor: no command, prompt, Reset, Cleanup, or gate command changed (G1 and G2 verified against a scratch copy with relabeled cells: both PASS, 9 rows), so no re-validation is required and the Validate Contract text is untouched.

## Touchpoints

- `spike/symlink-discovery.md` (read; transcribed from Taylor's report blocks)
- `$HOME/unam-spike-2026-10-04` (throwaway, created and deleted by Taylor)
- `~/.hermes/config.yaml` (sanctioned exception: `skills.trusted_project_dirs` entry added by `hermes skills trust`, removed by `hermes skills untrust`; backed up first)
- `process/context/product/all-product.md` (update trigger if an adapter claim changes)
- `process/context/tests/all-tests.md` (no cargo gate exists for M0; document checks only)

---

## Public Contracts

- None exposed. The Outcome feeds the `strategies()` and default-strategy contract of the adapters built in M3 and M5.

---

## Verification Evidence

| Gate / Scenario | Strategy | Proves SPEC criterion |
|---|---|---|
| G1, G2: 9 rows present, no empty cell (AC-M0-1) | Fully-Automated (grep on the filled file) after Hybrid data entry | PRD S9 evidence basis; spec 10 M0 table filled |
| G3: exactly one Status line and none reading "not run" (AC-M0-3) | Fully-Automated | spec 10 M0 outcome |
| G4a, G4b-claude-code, G4b-agents-neutral: Outcome not placeholder, names each adapter (AC-M0-2) | Fully-Automated plus orchestrator read (Hybrid) | PRD Q1 resolved |
| G5: `git status --porcelain -- docs/` empty, precondition `git rev-parse --verify HEAD` (AC-M0-4) | Fully-Automated | brief rule: spec and PRD stay authoritative |
| G6 (spike dir gone, no probes in user scope, 0 Hermes trust entries) plus Taylor's cleanup confirmation (AC-M0-5) | Hybrid | spike procedure step 9 |
| Runs 1 to 6 on real agents (Taylor, report blocks) | Hybrid (precondition: agents installed; Taylor runs them; an agent that runs a tool anyway makes the result unverified) | PRD Q1, risk R1, assumption A1 |

No cargo or `ac_NN_*` gate exists for M0. Known-Gap is not used for developed behavior; Windsurf "not tested" is a named residual recorded in Contradictions item 2, not a pass.

---

## Test Infra Improvement Notes

(none identified yet)

---

## Resume and Execution Handoff

- Selected plan file path: `process/features/v1-static-activation/active/v1-static-activation_04-10-26/phase-01-m0-symlink-spike_PLAN_04-10-26.md`
- Last completed step: PLAN-SUPPLEMENT (loop step 3), PVL supplement cycle 1 applied to the plan body
- Validate-contract status: pending (CONDITIONAL contract from cycle 0 awaits re-issue by the validator)
- Supporting context files: `process/context/all-context.md`, `process/context/product/all-product.md`, `spike/symlink-discovery.md`
- Next step: re-run vc-validate-agent from V1 (PVL cycle 2). After PASS, hand Taylor the Runbook; Taylor runs runs 1 to 6 and pastes report blocks; the orchestrator transcribes into `spike/symlink-discovery.md` and runs gates G1 to G6 (G5 only after the harness-setup commit). No agent runs real agents for Taylor.

---

## Validate Contract

Status: PASS
Date: 04-10-26
date: 2026-10-04
generated-by: inner-pvl: phase-1
supersedes: 2026-10-04 (inner-pvl: phase-1, PVL cycle 2 CONDITIONAL contract with 2 concerns C1 and C2) - inner PVL cycle 3 has current evidence

Parallel strategy: sequential
Rationale: score 0/7 (no signal present: single plan file, one spike file edited, no high-risk class, manual procedure, planning-doc review). Dominant signal: none. No EXECUTE agent exists for M0; Taylor runs the spike by hand. Validate ran as inline Layer 1 and Layer 2 review scoped to the cycle-2 edits (no helper agents, no live-agent or live-provider probes). Model: sonnet. Goal block: BRANCH B (umbrella plan has `## Stable Program Goal`), so no autonomous goal block section is written to this phase plan (the umbrella's goal governs).

Test gates (C3 5-column table; legacy line form follows). Gates G1 to G6 are unchanged since cycle 2, where each was run in a scratch copy under `bash` and interactive `zsh -f -i`: unfilled copy gave G2, G3, G4a, G4b-claude-code, G4b-agents-neutral FAIL (G1 PASS by design, presence only); hand-filled copy gave G1 to G4b PASS; G5 PASS on a committed clean `docs/`, FAIL on a modified and on an untracked `docs/` file; G6 FAIL for each of spike dir present, probe name in a user-scope skill dir, Hermes trust entry present. In this repository today G5 prints `PRECONDITION FAIL` (no commit yet, `git rev-parse --verify HEAD` fails) as expected; it passes once the harness-setup commit exists.

| criterion id | behavior | strategy | proving test | gap-resolution |
|---|---|---|---|---|
| AC-M0-1 | results table has 9 rows, every cell filled or marked "not tested" (Refresh needed cell now has a combining rule) | Hybrid | G1 plus G2 on `spike/symlink-discovery.md` after Taylor's runs (precondition: Taylor ran the sessions; verified: G2 FAIL unfilled, PASS filled) | B (report template and transcription rules from cycle 1; Refresh cell rule from cycle 2) |
| AC-M0-2 | Outcome names default strategy for `claude-code` and `agents-neutral` and lists contradicted spec claims | Hybrid | G4a plus G4b-claude-code plus G4b-agents-neutral plus orchestrator read (verified: all three FAIL unfilled, PASS filled) | B |
| AC-M0-3 | Status line no longer reads "not run" | Fully-Automated | G3 (verified: FAIL on current file with `status_lines=1 not_run=1`, PASS when changed) | B |
| AC-M0-4 | `docs/` untouched | Fully-Automated | G5 using `git status --porcelain -- docs/` expecting empty output, valid only after the harness-setup commit exists (verified in a scratch repo: PASS clean, FAIL modified, FAIL untracked) | B |
| AC-M0-5 | nothing written to user-scope skill dirs; Hermes trust entry removed; whole-file Hermes config change reviewed before backup deletion; spike dir deleted | Hybrid | G6 plus cleanup block 1 diff review plus Taylor's cleanup confirmation (precondition: Taylor ran cleanup; untrust and diff precede `rm -rf`; verified: G6 FAIL for each of three leftovers; cleanup blocks run under `zsh -f -i` in a scratch HOME) | B (two-block cleanup with diff review added by cycle 2) |
| runs 1 to 6 | per-agent symlink and copy discovery, mid-session discovery, refresh needed | Hybrid | Taylor's report blocks from the Runbook (precondition: agents installed; versions confirmed: claude 2.1.285, codex 0.160.0, cursor-agent 2026.09.08-6caf4ff, hermes v0.21.5) | A for tooling presence; B for P1/P2 wording |

Failing stub: AC-M0-3
test("should report Status line not equal to 'not run' after the spike", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: Status line not 'not run'")
})

Failing stub: AC-M0-4
test("should show no changes under docs/ after the spike", () => {
  throw new Error("NOT IMPLEMENTED - TDD stub for: docs/ untouched")
})

(Stubs are advisory: M0 has no test runner and no cargo gate; the proving commands above are grep and git checks. Hybrid rows receive no stubs.)

C-4 reconciliation: the strategy column carries only Fully-Automated, Hybrid, Agent-Probe. Known-Gap is never a strategy; named residuals are listed under Open gaps (Windsurf not tested; agent-owned state residue; runtime discovery behavior only the spike can measure). Net-gate vacuous-green check: every developed behavior in the blast radius (table fill, Outcome, Status line, docs untouched, cleanup, agent runs) has a Fully-Automated or Hybrid gate, so PASS is not vacuous.

Legacy line form:
- results table (AC-M0-1): [hybrid: G1 + G2 grep after Taylor's runs]
- Outcome section (AC-M0-2): [hybrid: G4a + G4b-claude-code + G4b-agents-neutral grep + orchestrator read]
- Status line (AC-M0-3): [fully-automated: G3 grep]
- docs untouched (AC-M0-4): [fully-automated: `git status --porcelain -- docs/` after setup commit]
- cleanup (AC-M0-5): [hybrid: G6 + cleanup block 1 diff review + Taylor confirmation + `hermes skills untrust` before `rm -rf`]
- agent runs 1 to 6: [hybrid: Taylor-run report blocks]
- Windsurf: [known-gap: documented; row recorded as "not tested"]

Cycle-2 gap closure check (re-read against the plan text, not the iteration report):
- C1 Refresh needed cell mapping: CLOSED. The Transcription rules table has a "Refresh needed cell (every row)" rule: `start: <REFRESH-AT-START>; mid: <REFRESH-MID-SESSION>`, copy rows use the start value with `mid: n/a`, Hermes uses `trust step` as the start value when trust was needed. The Report Format fields (REFRESH-AT-START includes `trust step`; REFRESH-MID-SESSION includes `n/a`) cover every value the rule can emit, and the skipped-M case yields `mid: n/a` with the mid-session cell `not run`. Two transcribers now produce the same cell.
- C2 Hermes config diff before backup deletion: CLOSED. Cleanup is two blocks: block 1 runs untrust then `diff` against the backup; the prose tells Taylor to proceed only on empty or reformatting-only output, otherwise copy the backup to `$HOME/hermes-config.before-unam-spike.yaml` and report before deleting; block 2 is the guarded delete and checks. Blast Radius, TL;DR, Hermes extra, Hard Stops, the G6 note and the closing Reminder all state the diff-before-delete order consistently. The stale "follow its prompt" wording is gone from the plan body (the command prints `Trusted: <path>` and does not prompt).
- Nits from cycle 2 (7-session wording, RUN field meaning, Windsurf date): all applied (TL;DR and Entry Gate explain Hermes starts twice; report-format note says RUN is per agent and DESTINATION disambiguates; Windsurf row date is "the run date Taylor reports in the other report blocks").

Cycle-2 edit regression scan (Python replacement script):
- Markdown structure: 32 fence lines, all paired (16 blocks: 11 zsh, 1 bash, 4 plain); no unclosed fence; tables intact.
- No duplicated or orphaned sentences from the replacement (only expected repeats such as table separators and the `EOF` terminators); no `#` inside any zsh block; no tabs or trailing whitespace; no em dash, en dash, emoji or any non-ASCII character in the plan body or the cycle reports.
- Run numbering consistent: run table 1 to 6; row mapping (run 1 rows 1 and 2, run 2 row 3, run 3 rows 4 and 5, run 4 row 7, run 5 row 6, run 6 row 9, Windsurf row 8) matches the table order note and `spike/symlink-discovery.md`; M-claude for runs 1 and 4, M-agents for runs 2, 3, 5, 6 matches the run table; "6 runs, 7 short agent sessions" is used consistently.
- zsh paste-safety under `zsh -f -i` with HOME redirected into the session scratchpad (agent, `hermes`, `claude`, `codex`, `cursor-agent` lines skipped; Hermes trust and untrust emulated by file edits on a fake `~/.hermes/config.yaml`; real `~/.hermes` never touched): R0 printed three OK lines and made the repo; R1 and R2 created the probes, symlink and copy as described; Reset restored both moved-aside dirs and printed `STOP: .claude exists, inspect` and moved nothing on the collision case; M blocks create the probe-c symlink; Cleanup scenario A (config restored by untrust) gave an empty diff, then block 2 printed `CLEANED` and `0`; scenario B (config changed beyond the trust entry) showed the diff, and block 2 printed `CLEANED` and `1` (the leftover trust entry is caught); scenario C (trust never applied) printed the expected diff error only. Scratch deleted afterward; real `~/unam-spike-2026-10-04` absent, no `probe-[abc]` in `~/.claude/skills` or `~/.agents/skills`.
- `validate-plan-artifact.mjs` and `validate-phase-stub.mjs`: 0 failures, 0 warnings.

Dimension findings:
- Infra fit: PASS - all fenced zsh blocks including the new two-block Cleanup are paste-safe in interactive zsh; path guards behave as described; real HOME has no enclosing repo, no existing spike dir, no probe names in user-scope skill dirs.
- Test coverage: PASS - G1 to G6 unchanged and correct; the Refresh needed cell now has a deterministic transcription rule (C1 closed); G5 is precondition-gated by design.
- Breaking changes: PASS - only `spike/symlink-discovery.md` is edited (`docs/` untouched, G5); no public contract changes; Outcome feeds the M3 and M5 adapter `strategies()` default.
- Security surface: PASS - all runbook commands write only under `$SPIKE` except the one sanctioned Hermes trust entry, which is backed up, removed by untrust, and whole-file-diffed against the backup before the backup is deleted (C2 closed); `rm -rf` is path-guarded.
- Section 1 feasibility (Runbook R0 to R2, Reset, prompts, run order, M step): PASS - unchanged and consistent; highest-risk edit: Claude Code run 2 recreating `.claude/`, handled by the Reset collision guard (re-verified).
- Section 2 feasibility (Report Format and transcription rules): PASS - every table cell, including Refresh needed, has a rule.
- Section 3 feasibility (Exit Gate G1 to G6): PASS - unchanged; PASS/FAIL semantics explicit.
- Section 4 feasibility (Cleanup and Hard Stops): PASS - two-block order, diff review, guard and final counts verified in scratch; highest-risk edit: block 2 deleting the backup, mitigated by the block 1 review and the copy-out command.

Open gaps:
- Windsurf not installed: row 8 recorded as "not tested"; leaves its behavior "not documented" per the decision rule (named residual, gap-resolution D if a later milestone needs it).
- Agent-owned state written outside skill directories on first run in the spike repo (Claude Code project trust and session records, Codex and Cursor trust or session records): cannot be undone or proven offline; named in Contradictions and Open Items item 6.
- Whether each agent discovers symlinked skills, whether mid-session additions appear, and whether Claude Code `/reload-skills` exists cannot be verified offline; that is exactly what the spike measures (no VC-FEASIBILITY-PROBE-NEEDED emitted by instruction).
- Hermes trust behavior is taken from a read-only source read of the installed v0.21.5 checkout; actual runtime behavior is confirmed only by Taylor's run 6.
- Cursor is tested through `cursor-agent` (CLI) only; rows are labeled `Cursor (cursor-agent CLI)` and the Cursor editor is not tested (Contradictions item 4).
- Non-blocking nits (cosmetic, not counted as concerns): checklist item C5 summarizes cleanup as "Hermes untrust first, guarded delete, then checks" without naming the diff review (the Cleanup section it points to governs); the one-line `cp` copy-out command uses `$SPIKE` and so assumes it is pasted in the same terminal as cleanup block 1 (a fresh terminal gives a harmless `cp` error, not data loss); the Resume and Execution Handoff block and `**Phase status:**` line still describe the pre-cycle-3 state and are for the orchestrator to refresh on this PASS.

What This Coverage Does NOT Prove:
- G1/G2: that the recorded Y/N values are true observations; the gates prove only that cells are filled, not that Taylor's runs were correct or that an agent did not answer from filesystem inspection. G2 flags empty cells only, not a cell filled with a wrong value (including a wrongly combined Refresh needed cell).
- G3: that the status text is accurate, only that it is not "not run".
- G4a/G4b: that the default strategy chosen follows the decision rule or is justified; that is the orchestrator read. G4b only counts the adapter names, so a mention of `claude-code` that states the wrong default still passes.
- G5: that no other repository path was touched; it covers `docs/` only, and only changes made after the setup commit.
- G6: that agent-owned state (trust records, session transcripts) outside skill dirs is absent; it checks the spike dir, probe names in user-scope skill dirs, and the Hermes trust entry only. It does not prove `~/.hermes/config.yaml` is otherwise unchanged; that is the cleanup block 1 diff review by Taylor, which relies on Taylor reading the diff correctly.
- The validation itself: Hermes `skills trust` and `untrust` were emulated, not executed, so the real command output and the exact shape of the config diff are unproven until run 6; the real agents were never launched.
- The whole spike: behavior of the Cursor app UI, Windsurf, other agent versions, or behavior after upgrades; symlink behavior for skills with nested relative paths (probes are single-file).

Plan updates applied: none by the validate agent (instruction: do not edit the plan body beyond this section).

Execute-agent instructions (for Taylor and the orchestrator; there is no execute agent for M0):
- E1: Hand Taylor the Runbook as written; Taylor runs runs 1 to 6, pastes one report block per run, and the orchestrator transcribes using the Transcription rules table.
- E2: The orchestrator transcribes only from pasted report blocks; it never launches agents and never runs the sessions.
- E3: Run G5 only after the harness-setup commit exists (`git rev-parse --verify HEAD` succeeds); before that, `docs/` is untracked and G5 prints `PRECONDITION FAIL`.
- E4: At cleanup, run block 1 (untrust and diff), have Taylor review the diff, copy the backup out if the diff is more than reformatting, then run block 2; confirm `~/.hermes/config.yaml` has no `unam-spike` entry (block 2 final count is `0`).
- E5: If any agent prompts to write outside `$SPIKE/repo` (other than the single sanctioned Hermes trust entry), stop and report; do not use yolo or bypass-permission modes.

Gate: PASS (0 FAILs, 0 unresolved CONCERNs; C1 and C2 closed; no regressions from the cycle-2 edits; PVL cycle 3)
Accepted by: n/a (Gate is PASS; no concerns to accept)

---

## Next Step

Take this plan through PVL (RIPER-5). Do not write code.
