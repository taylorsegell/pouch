# ADR-0001: Dynamic Skill Routing (Phase 3)

**Status:** Proposed
**Date:** 2026-10-03
**Deciders:** Taylor
**Project:** Unambiguous (command: `unam`)

## Scope

The PRD (`docs/spec/PRD-unambiguous.md`) defines what Phase 3 must do: requirements RTE-1 to RTE-12, DIS-11, DIS-12, SES-5, EDG-14, EDG-15, the four tiers, the atlas, and the measurement that decides whether the phase worked. This ADR records the technical decisions the PRD leaves open, mainly PRD questions Q8 and Q9. It does not restate the requirements.

Phase 3 is gated by PRD decision D8. Nothing here is built until Phase 1 is in daily use and the baseline week has been recorded.

## Context

The goal is accuracy, measured as fewer wasted tool calls per turn. Cost savings are a side effect.

A project selection shrinks the set a model chooses from. It cannot fix two things:

1. **Overused skills.** Some skills are useful but get used when they should not be. Their problem is being visible at all. A file on disk is either visible or absent; there is no "visible only when the request warrants it." This is the `excluded` tier.
2. **Long-tail skills.** Domain skills relevant to one request in fifty either pay catalog cost in every session or are never available.

Constraints:

- Not a harness (PRD D7). The router sits in front of existing harnesses and inherits their loop, tools, and permissions.
- Harnesses differ in which events expose hooks, whether hook output reaches the model, and where that output lands.
- The hook decides discoverability for one turn. It never writes skill bodies into the prompt and never instructs the model to use a skill (PRD section 12).
- Per-message overhead must be unnoticeable on the common path (PRD S14).
- Deterministic by default (PRD RTE-5). Precision over recall (PRD D9).

## Decision

One router in the same binary, reading the same registry, driven by a per-message hook.

### 1. Exposure mechanism (PRD Q9)

Use the hook's output channel to deliver a catalog delta: for each exposed skill, its name, its description, and its path, in the same shape the harness uses for its own skill catalog. The model then decides whether to load a skill, exactly as it does for an installed one.

This is the second of the three options in Q9. It is proposed, not settled: it depends on each target documenting a pre-model hook whose output reaches the model, which must be verified per target before the phase is scoped (PRD A6). A target without such a hook is unsupported for Phase 3 and keeps Phase 1 behavior (PRD RTE-1).

Rejected for now:

- **Writing and removing symlinks around each turn.** Works with any directory-scanning target but races with file watchers and turns a per-turn decision into filesystem churn.
- **Writing skills into a rules or agent configuration file.** Violates PRD OWN-5.

### 2. Evidence pipeline

Evaluated in order. Each step can add candidates; none can remove a skill that the project selection already holds (PRD RTE-4).

1. Explicit skill name in the prompt.
2. Path-like tokens in the prompt, matched against the atlas (`paths`).
3. Topic match against the atlas (`topics`).
4. Tier rules: `global` always; `excluded` only on step 1 or a high-specificity topic match; `domain` on steps 2 or 3; `project` never routed.
5. `not_when` filter: a match removes the candidate.
6. Rank by `importance`; cap by the exposure budget in `size_tokens`.
7. Confidence bar: candidates below it are dropped (PRD RTE-11). An empty result is normal and exposes the global tier only.

### 3. Optional classifier (PRD Q8)

Off by default. If enabled, it runs only when steps 1 to 3 produce nothing, and it answers one closed question: which of about eight task types (debug, build, refactor, review, document, plan, research, operate) the prompt is. Skills that declare a matching `task_types` value become candidates and then pass through steps 4 to 7 like any other.

A closed-set classification is chosen over open-ended skill selection because it is faster, testable, and fails toward exposing nothing. It has a hard timeout; on timeout or error the router proceeds without it.

Whether to build it at all is decided after the deterministic baseline is measured.

### 4. Session state

A small per-session file records what has been exposed and the last turn's exposure set. It serves PRD RTE-12: a follow-up with no intent of its own inherits the previous set, and a skill already exposed in the session is not announced again. It is not the receipt and is never written to it.

### 5. Shape of the router

A command that takes the hook's JSON on stdin and writes the catalog delta and the one-line summary (PRD RTE-7) to stdout. Each target gets a thin hook adapter that translates its event format. `unam route --dry-run` calls the same code path outside any agent (PRD RTE-9, AC-51).

### 6. Logging

One JSONL record per turn in the exposure log (PRD RTE-10): which pipeline steps fired, what was exposed and why, time taken, and whether each exposed skill was later loaded where the target makes that observable.

### Latency budget

| Path | Target |
|---|---|
| Deterministic pipeline | Under 50 ms (PRD S14) |
| Classifier, when enabled | Under 1 s, hard timeout |
| Prompts under a length threshold | Skip routing, reuse session state |

## Options Considered

### Option A: Stop at project selection

| Dimension | Assessment |
|---|---|
| Complexity | Low |
| Per-message latency | None |
| Accuracy risk | Low; cannot make things worse |
| Portability | High; only moves files |

**Pros:** Works on every harness today. No hooks, nothing to break silently.
**Cons:** No answer for overused skills or long-tail skills.

### Option B: Deterministic router with an optional closed-set classifier (chosen)

| Dimension | Assessment |
|---|---|
| Complexity | Medium |
| Per-message latency | Low on the common path |
| Accuracy risk | Medium; bounded by the confidence bar and by failing toward exposing nothing |
| Portability | Medium; one hook adapter per supported target |

**Pros:** Handles both gaps. Decisions are reproducible and can be replayed with `--dry-run`. A model is a tiebreaker on a small closed question, if it is used at all.
**Cons:** Depends on authors annotating skills. Hook adapters need maintenance as harnesses change.

### Option C: Model-first router

A small model reads the full index and picks skills on every message.

| Dimension | Assessment |
|---|---|
| Complexity | Medium |
| Per-message latency | High; a model call on every message |
| Accuracy risk | High; a weaker model makes open-ended picks that the main agent treats as endorsed |
| Portability | Medium |

**Pros:** Needs little metadata. Handles phrasing that keywords miss.
**Cons:** Non-deterministic, slow on every turn, and its misses are silent. Contradicts PRD RTE-5.

### Option D: Pull only

Expose a search command and let the main agent decide when to call it.

| Dimension | Assessment |
|---|---|
| Complexity | Low |
| Per-message latency | None unless called |
| Accuracy risk | Medium; the agent will not search for what it does not know exists |
| Portability | High |

**Pros:** Simple, and needs no hook.
**Cons:** Relies on the same judgment that is failing today, and adds a tool call instead of removing some.

## Trade-off Analysis

- **B over A.** A is Phase 1, the prerequisite and not the alternative. Whether the waste left after Phase 1 justifies B is an empirical question answered by the D8 gate.
- **B over C.** C puts a weaker model in charge of an open-ended choice on every message. B asks a model one narrow question, only when cheaper evidence runs out, and only if enabled.
- **D as a later addition.** A pull command could serve targets with no usable hook. It is not part of this decision, because PRD RTE-1 says unsupported targets are reported as unsupported, not approximated.
- **Precision over recall.** Exposed skills read as endorsed. A wrong exposure is worse than a missed one, so the bar is set to stay silent when unsure.

## Consequences

**Easier**
- Overused skills become usable without being always visible.
- The registry can grow without growing any project's standing catalog cost.
- Routing can be inspected and replayed from the log and with `--dry-run`.

**Harder**
- Authors must maintain tier, topics, paths, and `not_when`.
- Each supported target needs a hook adapter and a test that proves its output reached the model.
- A skill can now reach the model two ways, through the selection and through the router; reporting must keep `active` and `exposed` distinct (PRD section 4).

**To revisit**
- Where each target places hook output, and whether that placement invalidates prompt caching.
- Mid-turn triggers. A single prompt can start dozens of tool calls, and a relevant skill may only become apparent when the agent touches a path. A tool-event hook could expose skills then. Out of scope until per-message routing is measured.
- Whether native deferred skill search in the major harnesses makes the hook redundant.

## Risks

| Risk | Mitigation |
|---|---|
| A target discards hook output, indistinguishable from success | Per-target verification test before the adapter ships |
| Installing a hook means writing agent configuration | PRD Q10; decided at Phase 3 scoping |
| Authors do not maintain metadata | PRD EDG-15 default; DIS-12 reporting; log exposed-but-never-loaded skills and suggest demotions |
| Atlas goes stale as the tree changes | Rebuild on source or tree change (PRD RTE-2) |
| Classifier latency or outage | Hard timeout; proceed without it |

## Action Items

1. [ ] After Phase 1 is in daily use, record the baseline week and tag wasted calls by cause (PRD D8). Stop here if the gate fails.
2. [ ] Verify, per target, whether a pre-model per-message hook exists and whether its output reaches the model (PRD A6). Record evidence in `docs/adapters.md`.
3. [ ] Settle PRD Q9 and Q10 from the results of item 2.
4. [ ] Annotate the author's own library; build the atlas and `unam atlas show`.
5. [ ] Build `unam route --dry-run` with the deterministic pipeline.
6. [ ] Add session state and the exposure log.
7. [ ] Write the first hook adapter and its verification test.
8. [ ] Measure against the baseline (PRD S12, S13, S15). Consider the classifier only if a measurable gap remains.

## Related ideas, out of scope

Recorded so they are not lost. Neither is a commitment, and neither is part of this product's phases.

- **Context offloading.** Excluded from this product by PRD D6 and researched separately. The idea: keep the full working context of a session outside the main agent's window, with one process recording everything and a second distilling it into a short, unambiguous state document that points back into the record.
- **Repository map for orientation.** A map of a repository's files and symbols served to the agent so it spends fewer calls finding its way. Distinct from the atlas, which maps paths and topics to skills.
