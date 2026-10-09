---
name: ref:v1-static-activation-open-items-decision-sheet
description: "Decision sheet for the open umbrella items 7, 10, and 15 to 55 plus M3 plan D16, each with a proposed answer"
date: 09-10-26
feature: v1-static-activation
---

# Open items decision sheet (umbrella items 7, 10, 15 to 55)

TL;DR: 7 rows need your decision, 23 are accepted as built, 9 are closed or superseded. Mark only the rows you disagree with; everything else counts as accepted as proposed.

## How to use

- Rows are grouped. Numbers in the # column are umbrella "Contradictions and Open Items" numbers; "D16" is the M3 plan deviation; "M3-C" is the M3 plan conflicts table.
- Write your override next to a row (or reply "row X: no, do Y"). Silence means accepted.
- Proposed answers apply your 2026-10-09 threat model: same user, trusted local filesystem, untrusted input is only skill file content and cloned-repo files. Hardening beyond that is dropped. Protections against your own mistakes stay.
- v1 is symlink-only going forward (existing copy code stays; no new copy branches).
- No protected design document is edited by any answer here.

## Accept as built (no change)

| # | Item | Current provisional choice | Proposed answer | Why | Affects |
|---|---|---|---|---|---|
| 15, 16 | Config, cache, data paths on macOS; data dir override | PRD paths (`~/.config/unam` etc.); `directories` crate dropped; no data-dir override | Keep | PRD and spec section 2 agree; tests isolate via `HOME` | M1 |
| 17 | Diagnostics type | `Diagnostic` with stable codes, always-present JSON arrays | Keep | Spec requires named diagnostics but defines no type | M1 |
| 19, 20 | `dist` config targets; search crate | Per-arch targets, no universal claim; `nucleo-matcher` only | Keep; reconcile "universal" wording when M7 is researched | Nothing before M7 depends on it | M1, M7 |
| 22 | Bare `unam` | Prints usage only | Keep; M4 adds status | `status` does not exist until M4 | M1, M4 |
| 23 | `SKILL.md` at a source root | Skipped with warning `root-skill-skipped` | Keep | A skill ID needs a non-empty relative path | M1 |
| 24 | AC-12 and AC-35 status halves | Desired side tested in M2; status side in M4 | Keep | `status` is an M4 command | M2, M4 |
| 27 | Duplicate skill in several bundles | Keep all provenance records; last is the winner | Keep | Nothing varies by winner yet | M2, M3 |
| 28 | Bundle command scope | No rename; validate folded into `show`/`edit`; `--force` overwrite flag | Keep | Smallest surface that meets the PRD | M2 |
| 29, 30, 34 | Bundle loader details | Mirrors Hermes (yaml then yml order; blank name skipped with error diagnostic) | Keep | Hermes compatibility is the contract | M2 |
| 31 | Candidate order in errors | Distance, then source precedence, then ID | Keep | Spec says precedence only orders candidates | M2 |
| 35 | `docs/adapters.md` M2 sections | M7 must preserve them when regenerating | Keep | Process note only; M7 is held | M7 |
| 37 | Failure semantics | Continue past a failed step; exit 7; journal kept | Keep | Needed for AC-32 and STA-4; spec 4.10 "break" cannot meet them | M3 |
| 38, 49 | State layout; typed write paths | `.unam/<target>/...`, `version` field, JSONL journal; sealed `Writable` trait | Keep; amend spec 2.1, 2.5, 4.5 only if you want | One file cannot hold two targets | M3 |
| 39 | Lock | std `File::try_lock`, `rust-version = "1.89"`, no `fs4` | Keep | One fewer dependency | M3 |
| 40, 41 | "Changes nothing" and selection write timing | Lock path excluded from assertions; selection written after confirm, before apply | Keep | Matches user intent staying after partial failure | M3 |
| 42 | Selection semantics (`rm`, `use --replace`, `deactivate --all`) | As built; `--replace` clears ad hoc add/remove | Keep | Predictable; one adapter so `--all` equals default | M3 |
| 43 | Confirmation and exit codes | No TTY or `--json` without `--yes` is exit 2; decline is exit 0 | Keep | Every repair previews first (PRD 11.5) | M3, M4 |
| 44 | Nothing detected | Default to `claude-code`, create `.claude/skills`, record `created_dirs` | Keep | Case the PRD leaves open | M3 |
| 45 | Name validation; conflicts | Invalid names are blocking conflicts; no `--skip-conflicts` | Keep | Names become paths; this protects from bad skill content | M3 |
| 46 | Removing owned entries | Symlink unlinked only if link text matches receipt; copies via trash with hash check | Keep as built; no extension | Protects against your own edits; copy code frozen | M3 |
| 47 | Hash prefix and provenance shape | `blake3:` and `{"bundle": slug}` per spec | Keep | Matches spec forms | M3 |
| 48 | Enterprise skills paths | Assumed paths; `UNAM_ENTERPRISE_SKILLS_DIRS` test seam | Keep | Labeled Assumption in the evidence table | M3 |
| 51 | Spec text mismatches (a, c, d, e) | Remedies reworded; `active` halves added in M4 | Keep | Spec edits are not needed | M3, M4 |

## Needs a real decision

| # | Item | Current provisional choice | Proposed answer | Why | Affects |
|---|---|---|---|---|---|
| D16 | A copy or symlink edited before the command: exit 0 or exit 7? | CLI: release ownership, exit 0, nothing deleted; exit 7 `skipped modified` only for an edit during the confirm window (core API) | Keep both as built | Your edits are never deleted and the plan already showed them | M3, M4 |
| 26, 55 | `source rm` ignores bundle membership | Guard counts only literal `add`/`remove` entries; bundle-only selections are invisible | Fix in M4: guard resolves the selection (bundle members count); `bundle rm` still never blocks | Without it `source rm` can break live symlinks, which is a mistake-protection case, not hardening | M4 |
| 54 | First adopting project and D4 gate | Unnamed; D4 (one real activation) is a backlog NOTE owned by you | You name the project when ready; D4 stays CONDITIONAL and does not block M4 or M5 | The wave plan already proceeds; D4 needs your machine | M3, M4, M5 |
| 52 | Evidence pack decision vocabulary | Pack carries both scripts' fields; you record `review-decision.json` | Use `approved`, `approved-with-concerns`, or `rejected` (what `validate-risk-artifacts.mjs` accepts); do not edit `.claude/`; log the script mismatch as backlog | One value must be chosen; changing tooling is out of scope | M3 close, M4 |
| 21 | Source `rename` (PRD SRC-4) | Not built; no spec command | Drop from v1; workaround is `source rm` then `source add` | Not in spec section 5; rarely needed; lighter process | M4 |
| M3-C | `.claude` symlinked outside the project | Blocking conflict: destination root outside the project; nothing written | Keep the refusal, no override flag in v1; tell me if your dotfile setups need one | A cloned repo can ship such a symlink, so it falls inside the threat model | M3 |
| 51(b) | Copy fallback when symlinks fail | No automatic fallback; failure is `symlink-unsupported` | Keep; with symlink-only this is now the final answer | Matches v1 symlink-only decision | M3 |

## Superseded or closed by later decisions

| # | Item | Current provisional choice | Proposed answer | Why | Affects |
|---|---|---|---|---|---|
| 7, 10 | Module-to-milestone map; toolkit template notes | Decided per milestone; templates worked around | Close both | Process notes with no open product effect | All |
| 18 | Temporary `lookup` and provisional `projects.json` reader | Replaced by M2 `resolve.rs` and `project::index` | Close | Already superseded in M2 | M1, M2 |
| 25, 36 | Slugify exact `\w` port and Unicode table | Generated exclusion table, exact Python parity | Superseded: wave 1 uses `c == '_' \|\| c.is_alphanumeric()` plus a differences note in `docs/adapters.md` | Parity beyond that is hardening nobody needs | M2 |
| 32, 33 | `--project`/`--target` not in M2; `bundle save` interim behavior | Deferred to M3 | Close | M3 built both flags | M2, M3 |
| 50 | Rename onto an empty directory race | Named residual | Close, no further work | Needs a hostile concurrent process; outside the threat model | M3 |
| 53 | Deferred M3 extras | Nested `scan_locations`, VCS-tracked warning (EDG-12), case-insensitive collisions (EDG-13), `--skip-conflicts` | Drop all four from M4; M4 keeps `status`, rollback from history, `doctor --evidence` | Extras beyond the lighter M4 scope; revisit after v1 | M4 |

## Status

- File: `/Users/databased/Developer/01-AI/arkitect/unambiguous/process/features/v1-static-activation/active/v1-static-activation_04-10-26/open-items-decision-sheet_REF_09-10-26.md`
- Rows: Accept as built 23; Needs a real decision 7 (5 umbrella rows plus D16 and M3-C, with 26 and 55 sharing one row); Superseded or closed 6 (covering 9 item numbers). Items 15 to 55, 7, and 10 each appear once; item 51 appears in two groups (parts a, c, d, e accepted; part b decided).
