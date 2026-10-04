# PRD: Unambiguous

**Project-scoped skill activation, and eventually intent-driven skill exposure, for a local skills library**

| | |
|---|---|
| Product name | Unambiguous |
| Command | `unam` |
| Earlier working name | Loadout. Survives only as an informal term (section 4). |
| Version | 0.3 |
| Date | 2026-10-03 |
| Author | Taylor Segell |
| Reference model | Hermes Skill Bundles |
| Implementation | Rust. Personal tool built to open-source-ready standards. macOS and Linux. |
| Companions | `unam-technical-spec.md` (Phase 1 and 2 technical design); `ADR-0001-dynamic-skill-routing.md` (Phase 3 technical decisions) |

**How to read this document.** Claims are labeled so recommendations are never mistaken for facts.

- **[Verified]** confirmed against official documentation or source code, cited in section 16.
- **[Adopt]** existing Hermes behavior to copy as-is.
- **[Adapt]** existing Hermes behavior to change, with the reason.
- **[New]** capability Hermes does not have and this product requires.
- **[Decided]** a product decision that is closed. Listed in section 0.
- **[Recommendation]** a design choice with a stated alternative, still open to revision.
- **[Assumption]** believed true, not verified, with the verification path.

---

## 0. Decisions locked

| ID | Decision | Consequence |
|---|---|---|
| D1 | **Rust.** | Single binary, no runtime dependency, typed state model. Slower first milestone than a scripting language; TUI is more work. |
| D2 | **Personal tool, open-source-ready.** macOS and Linux only in v1. | No Windows build target, no multi-user state, no telemetry, no network. Nothing in the design precludes Windows later. |
| D3 | **Shared destination by default** for the neutral `.agents/skills/` adapter. | A selection written there is visible to every agent that reads the convention. The tool reports which agents those are on every plan. Isolated per-vendor destinations are a Phase 2 option, not the default. |
| D4 | **Name: Unambiguous, command `unam`.** | All paths, config directories, and state directories use `unam`. |
| D5 | **Three phases.** Phase 1 selection and activation. Phase 2 interactive surface, reproducibility, more adapters, release. Phase 3 intent-driven skill exposure. | Phase 3 is part of this product, not a separate tool. It changes one boundary this document otherwise holds firm; section 12 says which and why. |
| D6 | **Context offloading is out of scope** for this product. | The idea (keeping full context outside the orchestrating agent's window in a condensed form) is being researched separately against current compaction practice. It does not appear in any phase here. |
| D7 | **Not building a harness.** | Existing harnesses are far ahead. Unambiguous shapes what they see; it never replaces them. |
| D8 | **Phase 3 is gated on evidence.** It starts only after Phase 1 is in daily use and a baseline week of tool-call data has been recorded and tagged by cause. | If the baseline shows the remaining waste is not caused by wrong, missing, or overused skills, Phase 3 is not built. |
| D9 | **The router favors precision over recall.** When the evidence for a skill is weak, it exposes nothing beyond the global tier. | A missed exposure costs one explicit invocation. A wrong exposure misleads the model, because exposed skills read as endorsed. |

---

## 1. Product summary

Unambiguous sits between a local skills-development library and the coding agents working in individual projects. In its first two phases it lets a developer register local skill sources, browse and search them across whatever folder structure already exists, select any combination of skills, and activate that selection for one project and one coding-agent environment. The source library stays where it is and stays canonical. What the tool manages is which subset of the library a given agent in a given project can currently see.

In its third phase it stops relying on the developer to choose. A per-message hook and a directory atlas expose skills from the central registry based on what the current prompt is about, so the agent sees the four skills relevant to this message rather than the twelve relevant to this project. The goal of that phase is accuracy, measured in wasted tool calls avoided, more than context cost.

The bundle concept comes from Hermes Skill Bundles: a declarative, named group of skill references with a description and optional bundle-level instruction. Hermes bundles compose skills into a single conversation invocation. Unambiguous reuses the manifest shape and the pointer-not-container philosophy, then applies it to making a chosen set of skills discoverable on disk, per project, per agent.

The product is explicit about the boundary it cannot cross in Phases 1 and 2: it controls discoverability, not what a running conversation has already read. Phase 3 deliberately steps across that boundary through a hook, and section 12 states exactly how far.

---

## 2. Problem statement and target users

### The problem, in three layers

**Layer one: everything or nothing.** A developer who maintains a skills library of any real size can expose the whole library to every agent through a global directory, or expose none of it and wire skills by hand. The first costs roughly 50 to 100 tokens per skill in every session before anything is typed **[Verified]**, and gives the model a hundred descriptions to disambiguate between. The second is where the author of this document currently sits, having moved the library out of `.agents/` for exactly that reason: it works, and it is entirely manual, with no record of what is active and no way to name a recurring combination.

**Layer two: a project is the wrong grain.** Even a well-chosen project selection is static while the work inside a project changes with every message. A selection tuned for a migration task is wrong for the documentation task an hour later. The developer either re-selects constantly or lives with a superset.

**Layer three: the library has tiers, and the tools do not know that.** In practice the library sorts itself into a small set of skills that should always be loaded, a set that nearly made that cut but gets overused by the agent when present, domain-specific skills not tied to any one project, and project-specific or one-time skills. Which tier a skill belongs to is knowledge the author has and no tool consumes. The agent treats all discovered skills as equally eligible.

Phases 1 and 2 address layer one. Phase 3 addresses layers two and three.

### Target users

**Primary: the solo library maintainer.** Dozens to hundreds of skills in a category structure of their own design. Several coding agents, often more than one on the same project. Switches projects daily and tasks hourly. Wants the agent to see four relevant skills, not the library, and would rather not be the one choosing the four every time.

**Secondary: the team lead standardizing a project.** Wants a named, committed set of skills that anyone cloning the repository gets, without vendoring the skill content.

**Out of scope as a user:** the skill consumer who does not author skills and would be better served by a registry or marketplace.

---

## 3. Goals and non-goals

### Goals

| ID | Phase | Goal |
|---|---|---|
| G1 | 1 | Register one or more local directories as skill sources without moving or restructuring them |
| G2 | 1 | Browse and search every skill in a source across arbitrary nested categories |
| G3 | 1 | Select any combination of skills, from any number of categories, with no taxonomy constraint |
| G4 | 1 | Create, edit, duplicate, and reuse named bundles built on the Hermes manifest model |
| G5 | 1 | Activate a bundle or an ad hoc selection for one project and one agent environment |
| G6 | 1 | Add or remove a single skill without rebuilding the whole selection |
| G7 | 1 | Show what is active, where each active skill came from, and whether the target agent can discover it |
| G8 | 1 | Switch selections in seconds as work changes |
| G9 | 1 | Keep every project's selection independent of every other project's |
| G10 | 1 | Never mutate, relocate, or delete anything in a source library |
| G11 | 1 | Never overwrite or delete skills and configuration the tool does not own |
| G12 | 1 | Be honest about what activation does and does not affect in a running conversation |
| G13 | 2 | Make the primary workflow a single interactive step, and make the tool releasable |
| G14 | 3 | Expose skills per message based on prompt intent, drawing from one central registry, without the developer selecting |
| G15 | 3 | Let skill authors declare tier, category, importance, and size as metadata the router consumes |
| G16 | 3 | Reduce wasted tool calls caused by irrelevant or overused skills being visible |

### Non-goals

- **Not a skill authoring or editing tool.** Authoring stays in the library and the editor. Phase 3 asks authors to add metadata; it does not help them write it.
- **Not a registry, marketplace, or package manager.** No remote install, publishing, or dependency resolution. Sources are local paths.
- **Not a harness, and not a replacement for one** (D7). Unambiguous never runs the agent loop, never calls a model on its own behalf in Phases 1 and 2, and in Phase 3 calls one only if the intent classifier needs it (open question Q8).
- **Not a conversation participant in Phases 1 and 2.** No text injected into any conversation, no observation of agent sessions. Phase 3 revises this for one narrowly defined hook; section 12.
- **Not a context-offloading or compaction tool** (D6).
- **Not a replacement for any existing sync script.** Separate initiative.
- **Not a library reorganizer.** The source taxonomy is read-only input.
- **Not a Hermes feature.** Borrows the bundle model; does not depend on Hermes at runtime.
- **Not a context budget optimizer.** Reporting estimated catalog cost is in scope. Trimming a selection to fit a budget automatically is not, in any phase.

---

## 4. Terminology, and what "active" means

| Term | Definition |
|---|---|
| **Source** | A registered local directory containing skills. Need not be a Git repository. Has a user-assigned alias. |
| **Category** | The relative directory path between a source root and a skill directory. Derived, never declared. Purely organizational. |
| **Skill** | A directory containing a `SKILL.md` file, discovered by recursive walk of a source. |
| **Skill ID** | `<source-alias>:<relative/path/to/skill-dir>`. Path-based, so two skills named `code-review` in different categories stay distinct. |
| **Skill name** | The frontmatter `name`, falling back to the directory name. Used for display and collision analysis, never as the key. |
| **Bundle** | A declarative, named, reusable group of skill references with a description and optional bundle-level instruction. Hermes-derived. User scope, portable across projects. |
| **Project** | A directory tree identified by its root: nearest ancestor containing `.git`, or the given path. |
| **Target** | A coding-agent environment, defined by the destination contract it reads rather than by vendor. Section 10. |
| **Selection** | The *intended* set of skill references for one (project, target) pair, with provenance. Declarative, human-editable. |
| **Loadout** | Informal noun for the active selection of a (project, target) pair. Retained from the earlier working name because it is a useful word. |
| **Materialization** | The filesystem operation that makes a selected skill present at a destination the target scans. |
| **Receipt** | The record of what the tool created at a destination; the basis for ownership and cleanup. |
| **Tier** (Phase 3) | An author-declared exposure class for a skill. Section 12 defines the four tiers. |
| **Atlas** (Phase 3) | A per-repository or per-directory map from paths and topics to the skills relevant to them. Generated and cached; consumed by the router. |
| **Router** (Phase 3) | The per-message component that reads the prompt, the atlas, and skill metadata and decides which skills to expose for this turn. |
| **Exposure** (Phase 3) | Making a skill discoverable for one turn rather than for the life of a project selection. |

### The meaning of "active"

A skill is **active** for a (project, target) pair when all four hold:

1. **Selected.** It appears in that project's selection for that target.
2. **Materialized.** A readable `SKILL.md` resolves at a destination path the target's adapter documents as a scan location.
3. **Attributed.** That destination entry appears in the receipt as tool-owned, bound to its source skill ID.
4. **Unshadowed.** No higher-precedence skill with the same `name` exists for that target, per that target's documented precedence rules.

**Active means discoverable by that agent's next discovery pass.** Not loaded; not read.

Six reported states, never collapsed in the UI:

| State | Meaning |
|---|---|
| `selected` | In the selection, not yet applied |
| `applied` | Materialized, verification not run or inconclusive |
| `active` | All four conditions hold |
| `shadowed` | Materialized but a higher-precedence same-name skill exists |
| `foreign` | Present in the destination, not tool-owned; reported, never touched |
| `orphaned` | In the receipt, missing or altered on disk |

Phase 3 adds one more, scoped to a single turn: `exposed`, meaning the router made the skill discoverable for the current message only. It is reported in the hook's own output, never written to the receipt, and never confused with `active`.

### Discoverability versus loading

**Discoverability** is a filesystem and configuration property. An agent scans known directories, at session start or on a file-watch event, and builds a catalog of name and description pairs **[Verified]**. Phases 1 and 2 operate entirely here.

**Loading** is a conversation event. The model, or the user via a slash command, pulls a skill's full body into context, where it stays for the rest of that conversation **[Verified]**; some implementations deliberately protect loaded skill content from compaction **[Verified]**.

Consequence, stated at the moment of every deactivation: **removing a skill from discovery does not remove instructions a live conversation already loaded.** The conversation keeps behaving as that skill directs until it ends.

Phase 3 operates one layer closer to the conversation than Phases 1 and 2, because a per-message hook can place text in front of the model. Section 12 defines what the router is permitted to place there, and the same caveat applies: whatever a hook exposes in turn N and the model loads stays loaded in turn N+1 regardless of what the router does next.

---

## 5. Hermes reference analysis

### 5.1 Verified Hermes behavior

From `agent/skill_bundles.py` and the official skills documentation **[Verified]**:

**Bundle storage and loading**
- Bundles live in `$HERMES_BUNDLES_DIR` if set, otherwise `<HERMES_HOME>/skill-bundles`, documented as `~/.hermes/skill-bundles/<slug>.yaml`.
- `*.yaml` and `*.yml` accepted. Files sorted alphabetically before processing. Loader caches by mtime of the directory and every bundle file.

**Bundle fields**

| Key | Required | Behavior |
|---|---|---|
| `name` | Effectively yes | Stripped. Falls back to file stem when absent. Blank raises `ValueError("Bundle name is required")`. |
| `skills` | Yes | Non-empty list of skill identifiers. |
| `description` | No | Stripped. Defaults to `"Load {len(skills)} skills as a bundle"`. |
| `instruction` | No | Stripped. Empty when absent. |
| `slug` | Derived | `_slugify(name)`, imported from `skill_commands`. Empty slug rejects the bundle with a warning. |
| `path` | Derived | Filesystem path of the manifest. |

**Resolution and invocation**
- Skills referenced by bare name in a flat namespace.
- Unknown or missing skills skipped with a note when invocations are built. Disabled skills filtered at scan and again at invocation.
- Bundle `instruction` merged into the invocation header alongside any user instruction.
- No maximum skill count in bundle code. The separate chained slash-command path caps at five leading `/skill` tokens.
- No per-bundle deduplication. Duplicate bundle slugs resolve to first in alphabetical order, with a warning.
- Bundles take precedence over individual skills when slugs collide.

**Hermes skill discovery and conditional activation, for context**
- Primary store `~/.hermes/skills/`, plus `skills.external_dirs`, plus project-local `.hermes/skills/` and `.agents/skills/`. Precedence project, then local, then external.
- Project-local skills require a one-time `hermes skills trust` per repository.
- Frontmatter under `metadata.hermes` can declare `requires_toolsets`, `fallback_for_toolsets`, `requires_tools`, `fallback_for_tools`, hiding or showing a skill based on which toolsets are available in the environment.
- Three-tier progressive disclosure: `skills_list()` at roughly 3k tokens, `skill_view(name)`, `skill_view(name, path)`.

### 5.2 Adopt

| What | Why |
|---|---|
| Declarative YAML, one file per bundle, in a directory | Diffable, hand-editable, version-controllable. |
| The four-field manifest: `name`, `description`, `skills`, `instruction` | The right minimum. A Hermes bundle file is a valid Unambiguous bundle file. |
| `skills` required and non-empty | An empty bundle is a user error. |
| Generated default description | One less required field on the fast path. |
| **Bundle as pointer, not container** | Bundles reference, never copy. The property that keeps the source library canonical. |
| Alphabetical deterministic load order, warnings on duplicate slugs | Predictable and debuggable. |
| Tolerant loading: a broken bundle is skipped and reported | One bad file must not take down the tool. |
| mtime-based cache invalidation | Cheap, sufficient, makes hand-editing behave. |
| **Vendor-namespaced metadata** (`metadata.hermes.*`) | The pattern Phase 3 reuses as `metadata.unambiguous.*` for tier, importance, and size. |

### 5.3 Adapt

| Hermes | Unambiguous | Why |
|---|---|---|
| Bare names, flat namespace | Source-qualified refs (`skillzone:sdlc/code-review`). Bare name accepted as sugar when unambiguous; ambiguity errors with candidates, never guesses. | A categorized library has duplicate names across categories by design. |
| Missing skills silently skipped at invocation | Missing references block activation by default, with a precise report. `--allow-missing` proceeds and records the gap. | A skipped skill here mis-provisions a project for days, not one conversation. |
| `instruction` injected into the conversation header | Materialized as a generated brief skill in the destination, plus displayed on activation. Opt-out per bundle. | The tool is not in the conversation in Phases 1 and 2. Discoverable is the honest ceiling. Phase 3 may revisit (Q5). |
| Bundles take precedence over individual skills on slug collision | Bundle members and ad hoc additions merge into one flat selection, last-write-wins at the skill ID level, provenance recorded per entry. | The unit of activation is a skill, not a bundle. |
| No maximum skill count | No hard maximum; warning past a configurable catalog budget. | A large selection means the user recreated the problem. |
| Five-skill cap on chained invocation | Not applicable. | No invocation path. |
| Bundles are user-scope only | Bundles stay user-scope and reusable; a project selection references a bundle plus deltas. | Reuse and per-project customization both required. |
| **Conditional activation by toolset availability** | **Conditional exposure by prompt intent and author-declared tier** (Phase 3). | Hermes answers "is the environment able to use this skill." Phase 3 answers "is this message about something this skill is for." Different question, same shape: metadata the skill carries, consumed by the host to decide visibility. |

### 5.4 Extend

Hermes bundles solve composition inside one agent's conversation. None of the following exists in that model.

1. **Source registry.** Multiple aliased roots with derived categories and provenance.
2. **Project scoping.** Independent selections per project.
3. **Target scoping.** Independent selections per agent within a project.
4. **Materialization engine.** Placing skills where a *different* tool will look: links versus copies, ownership, cleanup.
5. **Ownership tracking.** Writing into directories shared with hand-maintained content.
6. **Verification.** Measuring the gap between "written" and "discoverable."
7. **Incremental mutation.** Add one, remove one.
8. **Version pinning and drift detection.**
9. **Adapters.** Per-target destination contracts and follow-up steps.
10. **Search and preview** for humans.
11. **Intent-driven, per-message exposure** from a central registry, using an atlas and author-declared tiers (Phase 3).

---

## 6. Primary user journeys

### Journey A: the primary scenario, ad hoc activation (Phase 1)

Developer is in `~/code/acme-api` and needs two skills, one from an SDLC category and one from somewhere else.

1. Runs `unam` in the project directory.
2. Tool resolves the project root, detects present agent environments, loads the current (empty) selection.
3. Developer types a few characters; results stream from all sources, each row showing name, category path, source alias, first line of description.
4. Marks two skills with the space bar. Two different categories; no extra step, not surfaced as a distinction.
5. Confirms. Plan shows two adds, destination, strategy, estimated catalog cost, and which agents will see the destination.
6. Accepts. Tool materializes, verifies, writes selection and receipt.
7. Result lists each skill `active` with source path, destination path, and the follow-up step for the detected agent.

**Acceptance:** both skills active; source library byte-identical; nothing foreign touched.

### Journey B: incremental change mid-task (Phase 1)

8. `unam rm sdlc/code-review` then `unam add testing/pytest-fixtures`.
9. Exactly those two entries change; the untouched skill's receipt entry is preserved verbatim.
10. Removal output states the skill leaves future discovery and that a conversation already holding it is unaffected until it ends.

### Journey C: save as a bundle (Phase 1)

11. `unam bundle save api-hardening` writes a manifest from the current selection with fully qualified refs.
12. Developer adds `description` and `instruction`.
13. Selection is rewritten to reference the bundle, preserving non-member skills as explicit additions.

### Journey D: switch bundles (Phase 1)

14. `unam use backend-dev --replace` diffs and applies only the delta.
15. `--add` merges instead.

### Journey E: project independence (Phase 1)

16. In `~/code/other-service`, `unam` shows that project's own prior selection; nothing under the first project is touched.

### Journey F: a second agent, same project (Phase 1)

17. `unam --target claude-code` and `unam --target agents-neutral` hold different selections, tracked and reported separately.

### Journey G: things are broken (Phase 1)

18. A source has moved. `unam status` reports unreachable, marks entries `orphaned`, offers `unam source relocate`, which repairs every affected project.

### Journey H: the router chooses (Phase 3)

19. The project selection holds twelve skills for a data-platform repo. Developer types a prompt about fixing a failing Alembic migration.
20. The hook fires. The router reads the prompt, consults the atlas for the files and topics it maps to, and reads tier metadata.
21. Six global-tier skills are exposed as always. Of the twelve project skills, the two migration-related ones are exposed for this turn. The other ten are not. A documentation skill the author marked "almost made the cut" is not exposed, because nothing in the prompt is about documentation.
22. The hook's output names what it exposed and why, in one line the developer can see.
23. Next prompt is about README structure. The documentation skill is exposed; the migration skills are not. Anything the model loaded in turn 21 is still in its context, and the hook says nothing to suggest otherwise.

---

## 7. Functional requirements

Priorities: **P0** required for Phase 1, **P1** wanted in Phase 1, **P2** Phase 2, **P3** Phase 3.

### 7.1 Source registration and discovery

| ID | Pri | Requirement |
|---|---|---|
| SRC-1 | P0 | Register a local directory as a source with a user-supplied alias and an absolute or `~`-relative path. No path is hardcoded or assumed. |
| SRC-2 | P0 | Multiple sources simultaneously, with stable precedence for display ordering. |
| SRC-3 | P0 | Git not required. Detected when present; enables extra features, never required. |
| SRC-4 | P0 | List, rename, relocate, unregister. Relocating repairs references in every project selection using that alias. |
| SRC-5 | P0 | Sources are read-only. No operation writes to, moves, renames, or deletes anything inside one. |
| SRC-6 | P1 | Validate at registration: exists, readable, has at least one skill. Warn rather than refuse on zero. |
| SRC-7 | P2 | Remote sources (Git URL, HTTP skill endpoint). |
| DIS-1 | P0 | Discover skills by recursive walk: any directory containing a file named exactly `SKILL.md`. |
| DIS-2 | P0 | Derive category from the relative path. Arbitrary depth, arbitrary names, no taxonomy, no reserved names. |
| DIS-3 | P0 | Lenient frontmatter parsing: load on name/dir mismatch or over-length name with a warning; skip only on unparseable YAML or missing description; record diagnostics either way. |
| DIS-4 | P0 | Path-based skill ID, stable across frontmatter edits. |
| DIS-5 | P0 | Skip `.git/`, `node_modules/`, and comparable directories; bound the walk by depth and count. |
| DIS-6 | P0 | Search across all sources and categories by name, description, category path, alias. Category is a filter, never a constraint on combination. |
| DIS-7 | P0 | Preview: full description, frontmatter, category path, absolute source path, supporting file inventory. |
| DIS-8 | P0 | Enumerate supporting content (`scripts/`, `references/`, `assets/`, anything else) so activation carries it and preview reports it. |
| DIS-9 | P1 | Catalog cache with mtime invalidation. |
| DIS-10 | P1 | Report duplicate `name` values across categories as information at scan time. |
| DIS-11 | P3 | Read Phase 3 metadata from frontmatter `metadata.unambiguous.*` and from the user-scope overlay file, the overlay winning per field: `tier`, `importance`, `size_tokens`, `topics`, `paths`, `task_types`, `not_when`. Absent fields take documented defaults. Unknown keys ignored. Category is never declared; it stays derived from the path. |
| DIS-12 | P3 | Report skills lacking Phase 3 metadata so authors can backfill. Never infer a tier the author did not state. |

### 7.2 Bundle management

| ID | Pri | Requirement |
|---|---|---|
| BND-1 | P0 | YAML, one file per bundle, in a configurable user-scope directory. |
| BND-2 | P0 | Accept the Hermes field set with Hermes defaulting for `name` and `description`. |
| BND-3 | P0 | Accept qualified and bare refs; bare resolves only when unambiguous, errors with candidates otherwise. |
| BND-4 | P0 | Reference skills from any number of categories and sources. |
| BND-5 | P0 | Create a bundle from the current project selection in one command, fully qualified. |
| BND-6 | P0 | Edit, duplicate, rename, delete. Deleting never deactivates; it breaks the reference and reports affected projects. |
| BND-7 | P0 | Add or remove single skills without rewriting unrelated entries or reordering. |
| BND-8 | P0 | A selection may reference bundles plus explicit additions and removals, resolved to a flat set with per-entry provenance. |
| BND-9 | P1 | Validate: refs resolve, no duplicates, non-empty, slug unique. |
| BND-10 | P1 | Report every project referencing a bundle. |
| BND-11 | P2 | Bundle composition with cycle detection. |
| BND-12 | P2 | Export and import with alias rewriting. |

### 7.3 Project activation

| ID | Pri | Requirement |
|---|---|---|
| ACT-1 | P0 | Resolve project root via Git root, falling back to the given path, with an override flag. |
| ACT-2 | P0 | Require an explicit target; default to the single detected one; prompt when several. |
| ACT-3 | P0 | One independent selection per (project, target). |
| ACT-4 | P0 | Activate a bundle, an ad hoc set, or both. |
| ACT-5 | P0 | `--replace` and `--add` as distinct, explicitly chosen operations. |
| ACT-6 | P0 | Add and remove single skills, applying only the delta. |
| ACT-7 | P0 | Deactivate everything tool-owned for a (project, target), leaving foreign content intact. |
| ACT-8 | P0 | An operation scoped to one project writes no file outside that project's destination and state, except user-scope bundle files the user explicitly edited. |
| ACT-9 | P0 | Preview every mutation as a plan. `--yes` skips the prompt, never the plan. |
| ACT-10 | P0 | Idempotent: re-applying an unchanged selection changes nothing and says so. |
| ACT-11 | P1 | Report estimated catalog cost, labeled as an estimate. |
| ACT-12 | P1 | Warn past a configurable size or cost threshold. |
| ACT-13 | P0 | On every plan against a shared destination, name the agents that will see it (D3). |

### 7.4 Status, inspection, and verification

| ID | Pri | Requirement |
|---|---|---|
| VER-1 | P0 | `status` reports per entry: ID, name, category, absolute source path, destination, provenance, state, strategy. |
| VER-2 | P0 | Verify each applied entry: resolves, `SKILL.md` readable, frontmatter parses, destination is a documented scan location. |
| VER-3 | P0 | Detect and report shadowing by name against documented precedence, including foreign skills in other scan locations. |
| VER-4 | P0 | Distinguish verified, unverifiable, failed. Never report `active` on the strength of a write alone. |
| VER-5 | P0 | State that verification proves discoverability under documented rules, not that the agent read anything. |
| VER-6 | P1 | `doctor`: symlink capability, destination writability, adapter detection, source reachability, receipt integrity. |

### 7.5 Source ownership, versioning, updates

| ID | Pri | Requirement |
|---|---|---|
| VSN-1 | P0 | The source is the only canonical copy. Any copy the tool makes is receipted with origin and hash. |
| VSN-2 | P0 | Default is **follow**: source edits visible with no re-activation. |
| VSN-3 | P2 | Optional **pin**: commit SHA for Git sources, content hash otherwise. Materializes a labeled managed copy. |
| VSN-4 | P2 | Drift reporting for pinned entries; change reporting for followed entries when a hash was recorded. |
| VSN-5 | P2 | `refresh` re-materializes, re-verifies, reports changes. |
| VSN-6 | P2 | Restore a pinned version from Git history. |

### 7.6 Coexistence

| ID | Pri | Requirement |
|---|---|---|
| OWN-1 | P0 | Receipt every created entry: destination, source ID, strategy, hash where applicable, timestamp. |
| OWN-2 | P0 | Remove only receipted entries. Never delete or modify an unrecorded path. |
| OWN-3 | P0 | On collision with foreign content, refuse by default, report both paths, offer skip or an explicit recorded override. Never overwrite silently. |
| OWN-4 | P0 | Leave the destination directory in place if it holds foreign content; remove only if tool-created and empty. |
| OWN-5 | P0 | Never write agent configuration files the tool does not own. Required changes are printed as manual steps. |
| OWN-6 | P2 | `adopt` for hand-placed skills after matching and confirmation. |
| OWN-7 | P1 | Offer `.gitignore` lines for tool-owned artifacts. |

### 7.7 State, failure, recovery

| ID | Pri | Requirement |
|---|---|---|
| STA-1 | P0 | Three distinct states: desired (selection), applied (receipt), observed (scan). Never conflated. |
| STA-2 | P0 | Journal written before mutation, cleared on success. |
| STA-3 | P0 | Detect an interrupted run; offer resume or rollback; default to neither. |
| STA-4 | P0 | Precise partial-failure reporting; one failure does not abort already-applied entries. |
| STA-5 | P0 | Roll back to the previous applied state. |
| STA-6 | P0 | `reconcile` compares all three states, reports divergence, can repair toward desired. |
| STA-7 | P1 | Bounded applied-state history. |
| STA-8 | P1 | Selection file safe to hand-edit; unresolvable refs produce a clear error and no partial apply. |
| STA-9 | P1 | Two invocations against one project never interleave mutations. A mutating command that finds another in progress refuses and says so. |

### 7.8 Agent-session behavior

| ID | Pri | Requirement |
|---|---|---|
| SES-1 | P0 | After every mutation, state what is required for discoverability: nothing, a reload command, a restart, or "not documented." Derived from the adapter. |
| SES-2 | P0 | After every removal, state that loaded instructions in an existing conversation persist until it ends. |
| SES-3 | P0 | Never claim a change took effect inside a running session. |
| SES-4 | P1 | Report per-adapter follow-ups, such as a trust step, as commands. |
| SES-5 | P3 | The Phase 3 hook output names what was exposed for this turn and never implies anything was unloaded. |

### 7.9 Edge cases

| ID | Pri | Requirement |
|---|---|---|
| EDG-1 | P0 | Missing or renamed source skill: report ID, last known path, candidates; offer repoint; block by default. |
| EDG-2 | P0 | Duplicate names across categories: permitted; IDs distinct; bare-name ambiguity errors; two selected skills sharing a name in one destination is a hard conflict. |
| EDG-3 | P0 | Broken ref inside a bundle: bundle loads, entry reported and blocked. |
| EDG-4 | P0 | Moved source: entries `orphaned`; `source relocate` repairs all affected projects. |
| EDG-5 | P0 | Unavailable source: report unreachable, do not mutate, do not mark inactive on that basis. |
| EDG-6 | P0 | Local source edits: supported under follow; drift under pin; never reverted. |
| EDG-7 | P0 | Overlapping bundle membership: one entry, multiple provenance records; removing one bundle keeps a skill another still needs. |
| EDG-8 | P0 | Foreign content: never touched; reported `foreign`. |
| EDG-9 | P1 | Mechanically detectable conflicts (name, path, shadowing, unresolvable refs, duplicate slugs, drift) reported as conflicts. |
| EDG-10 | P1 | Semantic incompatibility cannot be detected and is never claimed. Heuristic hints, if any, are labeled hints. |
| EDG-11 | P1 | Nested projects under ancestor-scanning targets: warn and name the ancestor. |
| EDG-12 | P1 | Destination under version control: warn before materializing. |
| EDG-13 | P2 | Case-insensitive filesystem collisions. |
| EDG-14 | P3 | A prompt that matches nothing in the atlas exposes only global-tier skills and says so. |
| EDG-15 | P3 | A skill with no Phase 3 metadata is treated as project-tier (exposed when selected, never routed), and reported once per session as unannotated. |

### 7.10 Developer experience

| ID | Pri | Requirement |
|---|---|---|
| UX-1 | P0 | One binary; scriptable CLI and interactive picker share one core; neither has behavior the other lacks. |
| UX-2 | P2 | Bare `unam` opens the picker pre-loaded with the project's selection. |
| UX-3 | P2 | Picker: incremental search, cross-category multi-select, preview, visible pending-change set. |
| UX-4 | P0 | Every mutation previews before applying. |
| UX-5 | P0 | `--json` for every read command. |
| UX-6 | P1 | Full non-interactive coverage. |
| UX-7 | Later | Editor extension. |
| UX-8 | Later | Graphical interface. |

### 7.11 Intent-driven exposure (Phase 3)

| ID | Pri | Requirement |
|---|---|---|
| RTE-1 | P3 | Provide a per-message hook for each supported target that runs before the model sees the prompt, where the target offers such a hook. Targets without one are reported as unsupported for Phase 3, not approximated. |
| RTE-2 | P3 | Maintain an atlas per project: a cached map from file paths, directories, and topics to skill IDs, built from skill `paths` and `topics` metadata plus the repository tree. Rebuilt on source or tree change. |
| RTE-3 | P3 | On each message, the router computes an exposure set: all global-tier skills, plus skills whose metadata matches the prompt's intent or the files it references, minus excluded-tier skills unless the prompt explicitly names them. A skill whose `not_when` conditions match the prompt is not exposed. |
| RTE-4 | P3 | Exposure draws from the central registry, not only from the project selection. The project selection remains the Phase 1 floor; routing adds to it per turn and never removes from it. |
| RTE-5 | P3 | The router is deterministic by default: keyword, path-glob, and topic matching against author metadata. A model-backed classifier is opt-in (Q8). |
| RTE-6 | P3 | Exposure is a turn-scoped mechanism, never written to the receipt. How the hook makes a skill discoverable for one turn is adapter-specific and documented per adapter (Q9). |
| RTE-7 | P3 | The hook emits one line naming exposed skills and the matching reason. `--quiet` suppresses it. |
| RTE-8 | P3 | Authors declare metadata under `metadata.unambiguous`, or in the overlay file for skills they would rather not edit: `tier` (one of four), `importance` (1 to 5), `size_tokens` (estimate; computed from the skill body when absent), `topics` (list), `paths` (globs), `not_when` (topics or phrases that rule the skill out), `task_types` (from a closed set; consumed only by the opt-in classifier, Q8). All optional; defaults documented. |
| RTE-9 | P3 | `unam atlas show` renders the atlas; `unam route --dry-run "<prompt>"` shows what would be exposed and why, outside any agent. |
| RTE-10 | P3 | Measure: log exposures and, where the target exposes it, subsequent skill loads, so wasted-exposure rate (exposed, not loaded) and missed-exposure rate (loaded via explicit invocation after not being exposed) are computable. Local file, no network. |
| RTE-11 | P3 | Precision over recall (D9): exposure requires a match above a configurable confidence bar. Below it, only global-tier skills are exposed. |
| RTE-12 | P3 | A short follow-up that carries no intent of its own ("yes, do it") inherits the previous turn's exposure set. A skill already exposed in the session is not announced again. |

---

## 8. Recommended UX and interaction flow

**[Decided] One binary, CLI plus interactive terminal picker.** The CLI is Phase 1; the picker is Phase 2. No editor extension or GUI in any planned phase.

### Command surface

```
unam                                   # picker (Phase 2); in Phase 1 prints status and usage
unam source add <alias> <path>
unam source list | relocate | rm
unam search <query> [--category <path>] [--source <alias>]
unam show <ref>
unam add <ref>...
unam rm <ref>...
unam use <bundle> --replace | --add
unam status [--json] [--target <t>]
unam deactivate [--all]
unam bundle save <name> | ls | show | edit | duplicate | rm | add | rm
unam reconcile [--repair]
unam rollback
unam resume
unam doctor [--evidence]
unam atlas build | show                 # Phase 3
unam route --dry-run "<prompt>"         # Phase 3
unam hook install <target> | uninstall  # Phase 3
```

### Plan rendering

```
Project   ~/code/acme-api  (git root)
Target    agents-neutral  ->  .agents/skills/    visible to: codex, cursor, windsurf-devin, hermes
Strategy  symlink

  + sdlc/code-review           skillzone   ->  .agents/skills/code-review
  + testing/pytest-fixtures    skillzone   ->  .agents/skills/pytest-fixtures
  = docs/api-reference         skillzone   (unchanged)
  - legacy/old-linter          skillzone   (remove, tool-owned)
  ~ backend-dev-brief          generated   (bundle instruction, unchanged)

  ! foreign    .agents/skills/house-style   (not managed, untouched)
  ! ancestor   ../.agents/skills contributes 2 skills to this project (codex walks upward)

Catalog estimate   4 skills, ~310 tokens (estimate)
After apply        codex: detects changes automatically, restart if not visible
                   cursor, windsurf-devin: refresh behavior not documented
                   hermes: run `hermes skills trust` once in this repository
```

### Result and follow-up

Six-state list, one follow-up block per consumer, and on any removal:

> Removed from future discovery. A conversation that already loaded these instructions keeps them until that conversation ends.

### Phase 3 hook output

One line, on the hook's own channel, never injected as skill content:

```
unam: exposed data/alembic-migrations, data/schema-diff (paths: migrations/**; topic: alembic) + 6 global
```

---

## 9. Conceptual data model

Entities and their meaningful fields. The technical specification holds the Rust types.

**Source** `alias`, `path`, `kind` (directory, git-repo), `precedence`, `last_scanned`, `diagnostics`.

**Category** (derived) `source_alias`, `relative_path`, `depth`, `skill_count`.

**Skill** (derived from scan) `id`, `source_alias`, `relative_path`, `category_path`, `name`, `description`, `frontmatter`, `supporting_paths`, `tree_hash`, `git_commit` (nullable), `diagnostics`, and from Phase 3 metadata: `tier`, `importance`, `size_tokens`, `topics`, `paths`, `task_types`, `not_when`.

**Bundle** `slug`, `name`, `description`, `skills` (ordered refs), `instruction`, `path`, `metadata`.

**Project** `root_path`, `detection_method`, `detected_targets`.

**Target** `id`, `destination_contract`, `scan_locations`, `strategy_support`, `precedence_rules`, `refresh_requirement` (none, command, restart, unknown), `followup_steps`, `consumers`, `evidence`, and from Phase 3: `hook_support` (none, or the hook kind and install location).

**Selection** `project_root`, `target_id`, `bundle_refs`, `additions`, `removals`, `resolved_entries` (computed), `updated_at`.

**ReceiptEntry** `project_root`, `target_id`, `skill_id`, `destination_path`, `strategy`, `source_path_at_apply`, `hash_at_apply`, `git_commit_at_apply`, `pin_mode`, `provenance`, `applied_at`.

**ObservedEntry** (never stored) `destination_path`, `resolves`, `parses`, `name`, `points_to`, `ownership`.

**Journal** `operation_id`, `project_root`, `target_id`, `previous_receipt`, `planned_steps`, `completed_steps`, `started_at`.

**Atlas** (Phase 3, cached) `project_root`, `built_at`, `tree_hash`, entries of `{ pattern | topic } -> [skill_id]`, derived from skill `paths` and `topics` metadata intersected with the repository tree.

**Exposure** (Phase 3, per turn, logged not stored) `timestamp`, `project_root`, `target_id`, `prompt_hash`, `exposed` (skill IDs with reasons), `later_loaded` (when observable).

### Storage layout

| What | Where | In VCS |
|---|---|---|
| User config | `~/.config/unam/config.toml` | n/a |
| Bundles | `~/.config/unam/bundles/*.yaml` | user's choice |
| Project selection (desired) | `<project>/.unam/selection.yaml` | recommended |
| Receipt, journal, history (applied) | `<project>/.unam/applied.json`, `journal.json`, `history/` | gitignore |
| Skill metadata overlay (Phase 3) | `~/.config/unam/skills.yaml` | user's choice |
| Atlas (Phase 3) | `<project>/.unam/atlas.json` | gitignore |
| Exposure log (Phase 3) | `~/.local/state/unam/exposures/<project-hash>.jsonl` | n/a |
| Project index | `~/.local/share/unam/projects.json` | n/a |
| Catalog cache | `~/.cache/unam/catalog/<alias>.json` | n/a |

Environment overrides `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`, mirroring the Hermes `HERMES_BUNDLES_DIR` pattern.

### Proposed schema: bundle manifest

**Proposed, not an established Hermes schema, not a final contract.** A Hermes bundle is valid input unchanged.

```yaml
# ~/.config/unam/bundles/backend-dev.yaml
name: backend-dev
description: Backend feature work: review, test, PR workflow.
skills:
  - skillzone:sdlc/github-code-review
  - skillzone:sdlc/github-pr-workflow
  - skillzone:testing/test-driven-development
  - house:conventions/error-handling
instruction: |
  Always start by writing failing tests, then implement.
metadata:
  unambiguous:
    version: 1
    materialize_instruction: true
```

### Proposed schema: project selection

```yaml
# <PROJECT_ROOT>/.unam/selection.yaml
version: 1
targets:
  claude-code:
    strategy: symlink
    bundles: [backend-dev]
    add: [skillzone:data/schema-migration]
    remove: [skillzone:sdlc/github-pr-workflow]
  agents-neutral:
    strategy: symlink
    bundles: []
    add: [skillzone:sdlc/code-review]
    remove: []
```

Resolution: expand `bundles` in order, apply `add`, apply `remove`, deduplicate by skill ID keeping all provenance.

### Proposed schema: skill metadata for Phase 3

Lives in the skill's own `SKILL.md` frontmatter, under a vendor namespace the way Hermes uses `metadata.hermes`. The spec defines `metadata` as a string-to-string map **[Verified]**; nested maps and lists exceed that, exactly as Hermes's do, so a strict validator may warn. Documented, accepted. For skills whose frontmatter must stay strictly conformant, or that the author does not want to edit, the same fields can be given in the overlay file `~/.config/unam/skills.yaml`, keyed by skill ID; the overlay wins per field.

```yaml
---
name: alembic-migrations
description: Diagnose and repair failing Alembic migrations. Use when a migration errors, autogenerate drifts, or revision history is inconsistent.
metadata:
  unambiguous:
    tier: domain            # global | excluded | domain | project
    importance: 4           # 1 to 5
    size_tokens: 1800       # author estimate of SKILL.md body
    topics: [alembic, migrations, sqlalchemy, schema]
    paths: ["migrations/**", "alembic.ini", "**/env.py"]
    not_when: [writing documentation]              # optional
    task_types: [debug]                            # optional; closed set, used only by the opt-in classifier
---
```

---

## 10. Coding-agent integration strategy and compatibility limits

### 10.1 Verified discovery locations

All rows **[Verified]** against official documentation, cited in section 16.

| Agent | Project-scope skill paths | User-scope skill paths | Reads neutral `.agents/skills`? | Refresh |
|---|---|---|---|---|
| **Claude Code** | `.claude/skills/<name>/SKILL.md`; nested `<subdir>/.claude/skills/` | `~/.claude/skills/` | Not documented | None; directories are watched, except in bare mode. `/reload-skills` once if the directory did not exist at startup. |
| **Codex** | `.agents/skills` in every directory from cwd up to the repository root | `$HOME/.agents/skills`; `/etc/codex/skills` | Yes, native | Automatic; restart if not visible. |
| **Cursor** | `.agents/skills/`, `.cursor/skills/`, anywhere in the repo; `.claude/skills/` and `.codex/skills/` for compatibility | `~/.agents/skills/`, `~/.cursor/skills/`, `~/.claude/skills/`, `~/.codex/skills/` | Yes | Not documented. |
| **Windsurf / Devin** | `.devin/skills/`, legacy `.windsurf/skills/`, `.agents/skills/`; `.claude/skills/` when Claude config reading is enabled | `~/.codeium/windsurf/skills/`, `~/.config/devin/skills/`, `~/.agents/skills/` | Yes | Not documented. |

**Adapters are keyed by destination contract, not vendor.** Three of four read `.agents/skills/`; one destination provisions Codex, Cursor, and Windsurf/Devin. Claude Code does not document reading it and needs `.claude/skills/`.

**The neutral directory is shared by design (D3).** Per-agent isolation is possible for Cursor, Windsurf/Devin, and Claude Code through vendor directories, and impossible for Codex, whose only documented project location is the neutral one.

### 10.2 Adapter scope by phase

**Phase 1 [Decided]:** `claude-code` to `.claude/skills/`, and `agents-neutral` to `.agents/skills/`. Hermes is reached through `agents-neutral` plus the documented `hermes skills trust` follow-up **[Verified]**.

**Phase 2:** isolated `cursor` and `windsurf-devin` adapters, available when a user opts out of the shared default. Both documented well enough to build; neither documents refresh behavior, so output hedges accordingly.

**Phase 3:** hook support per target. Which targets offer a pre-model, per-message hook, where it is installed, and what it may return are to be verified against each vendor's hook documentation before the phase is scoped **[Assumption]** that at least Claude Code offers one.

### 10.3 Adapter interface

```
Adapter
  id
  consumers()                   -> agents known to read this destination
  detect(project_root)          -> present | absent | unknown
  destination(project_root)     -> project-relative path
  scan_locations(project_root)  -> ranked, from documented evidence
  strategies()                  -> [symlink, copy]
  refresh(ctx)                  -> none | command(<cmd>) | restart | unknown
  followups(project_root)       -> actionable steps
  hook_support()                -> none | { kind, install_location, output_contract }   (Phase 3)
  evidence()                    -> (claim, url) pairs
```

An adapter must carry evidence for `destination`, `scan_locations`, and `hook_support`. `refresh` returns `unknown` rather than a guess.

### 10.4 Stated compatibility limits

- Verification proves a file is readable at a documented scan path. It cannot prove the agent read it.
- **Symlink following inside skill directories is not documented by any target** **[Assumption]**. Gated by the M0 spike in the technical spec; copy fallback exists for this reason.
- Codex's upward scan means a nested project inherits an ancestor's neutral selection. Reported, not solvable.
- Cursor and Windsurf/Devin compatibility reads of `.claude/skills/` mean both Phase 1 adapters active in one project can present a skill twice to those agents. Detected and reported.
- Cloud-hosted or sandboxed agents cannot see a user-scope library **[Verified]**. Local agents only.
- Phase 3 is limited to targets that expose a per-message hook. Targets without one get Phase 1 behavior only.

---

## 11. Activation mechanism, ownership, updates, recovery

### 11.1 Mechanism comparison

| Approach | Strengths | Weaknesses |
|---|---|---|
| **Directory symlink** per skill | No duplication; source canonical; supporting files and relative paths intact for free; live edits; one-unlink cleanup; ownership self-evident | Agent symlink following undocumented; Git-tracked destinations commit machine-specific links; no pinning; broken when source volume absent |
| **Managed copy** | Works everywhere; survives source absence; enables pinning | Duplicates content; drifts; destination edits lost on refresh; cleanup must be hash-checked |
| **Generated agent config** pointing at source dirs | No materialization | Exposes a whole directory, which is the problem; writes config the tool does not own; few agents support it |
| **Generated stub skill** referencing the source | Tiny; reversible | Breaks relative resolution of `scripts/`, `references/`, `assets/` **[Verified]**; sends the agent outside the project; catalog shows the wrong description |
| **CoW clone / hardlink** | Cheap | Filesystem-specific; hardlinks cannot link directories; still a copy semantically |

### 11.2 Recommendation

**[Recommendation] Symlink-first with copy fallback, per target, overridable per project.** Symlinks are the only mechanism satisfying VSN-1 and G10 without compromise. Copy is selected automatically when symlinks fail or are unavailable, when a pin is requested, or when an adapter is known not to follow links; always labeled as a managed copy with a hash. The M0 spike gates the default (Q1).

Destination naming uses the skill's `name`, because agents key on it. Same-name conflicts follow EDG-2.

### 11.3 Source ownership

The source is canonical and read-only. The technical spec enforces this at the type level: write operations do not accept a source path. `status` always shows each entry's absolute source path.

### 11.4 Update behavior

**Follow** is the default and is correct for a library under active development. **Pin** (Phase 2) is opt-in, materializes a copy, and is the only reproducible mode. The tradeoff is stated at the moment of choice.

### 11.5 Recovery

Desired, applied, observed. Journal before mutation, cleared after. `reconcile` reports and repairs toward desired; `rollback` restores the previous applied state; bounded history. Every repair path previews first and never deletes unreceipted content.

---

## 12. Phases

### Phase 1: selection and activation (CLI)

Maps to technical spec milestones M0 through M5.

**In:** M0 symlink spike; source registration and discovery (SRC-1 to SRC-6, DIS-1 to DIS-10); Hermes-shaped bundles (BND-1 to BND-10); project and target scoped selections with replace, add, single-skill delta, preview, idempotency (ACT-1 to ACT-13); two adapters; symlink-first materialization; receipt-based ownership (OWN-1 to OWN-5, OWN-7); three-state model, journal, resume, rollback, reconcile (STA-1 to STA-9); verification with six states (VER-1 to VER-6); session-behavior reporting (SES-1 to SES-4); P0 and P1 edge cases; CLI with `--json` (UX-1, UX-4, UX-5, UX-6).

**Done when:** every P0 and P1 acceptance criterion in section 13 passes, and the author uses it daily instead of the manual workflow.

### Phase 2: surface, reproducibility, release

Maps to spec milestones M6 and M7 plus the spec's v2 themes.

**In:** interactive picker (UX-2, UX-3); pinning and drift (VSN-3 to VSN-6); isolated `cursor` and `windsurf-devin` adapters; `adopt` (OWN-6); bundle composition and export (BND-11, BND-12); remote sources (SRC-7); OSS release mechanics: README, evidence docs, CHANGELOG, license, `cargo-dist` builds, install story.

**Done when:** the primary scenario is one interactive step, and a stranger can install and use the tool from the README.

### Phase 3: intent-driven exposure

**Entry gate (D8).** Phase 3 starts only after Phase 1 is in daily use and one week of normal work has been recorded: tool calls per turn, with wasted calls tagged by cause (wrong skill chosen, missing skill, overused skill, orientation, retry loop, redundant verification). Phase 3 proceeds only if a meaningful share falls in the first three causes, since those are the only ones routing can fix.

**The boundary this phase changes.** Phases 1 and 2 hold that the tool is not a conversation participant. Phase 3 installs a per-message hook, which runs inside the agent's turn and can shape what the model sees. That is a deliberate step across the earlier line, and it is bounded:

- The hook decides **discoverability for one turn**. It does not write skill bodies into the prompt, does not summarize them, and does not instruct the model to use them.
- It emits a single visible line stating what it exposed and why.
- It never claims to have removed anything from the model's context, because it cannot (SES-5).
- The project selection from Phase 1 is the floor. Routing only adds for a turn; it never deactivates.

**The four tiers (G15).** Author-declared in `metadata.unambiguous.tier`:

| Tier | Meaning | Router behavior |
|---|---|---|
| `global` | Always loaded. In practice a small fixed set, on the order of six. | Always exposed. |
| `excluded` | Would have made the global cut, but the agent overuses it when present. | Exposed only when the prompt explicitly names it or matches a declared topic with high specificity. |
| `domain` | Domain-specific, not tied to one project. | Exposed when the prompt or referenced paths match `topics` or `paths`. |
| `project` | Project-specific, one-time, or niche. Default for unannotated skills (EDG-15). | Exposed only through the Phase 1 selection; never routed in from the registry. |

**The atlas (RTE-2).** Built per project from the repository tree intersected with every registered skill's `paths` globs and `topics`. It answers "which skills claim relevance to this file or subject." Cached, rebuilt on source or tree change, inspectable with `unam atlas show`.

**The router (RTE-3, RTE-5).** Deterministic by default: tokenize the prompt, extract path-like tokens, match against the atlas and against `topics`, apply tier rules, rank by `importance`, cap by a configurable exposure budget expressed in `size_tokens`. A model-backed classifier is opt-in and off by default; the deterministic path has to be good enough to be useful on its own, or the phase is not worth building. The router favors precision (D9, RTE-11): below the confidence bar it exposes the global tier and nothing else.

**Measurement (RTE-10, G16).** Exposure is logged locally per turn. Where the target makes skill loads observable, wasted exposures (exposed, never loaded) and missed exposures (loaded by explicit invocation after not being exposed) become computable. This is the phase's success metric; without it the phase is a guess.

**In:** DIS-11, DIS-12, RTE-1 to RTE-10, SES-5, EDG-14, EDG-15; hook adapters for targets that document a per-message hook; metadata backfill tooling for the author's own library.

**Done when:** on the author's own library and projects, measured wasted-exposure rate is below a threshold set after a baseline week, and no Phase 1 or 2 acceptance criterion regresses.

### Development approach

This PRD, the technical specification, and ADR-0001 define the work. The toolkit's own unit of work is also called a phase; in toolkit plans, one toolkit phase maps to one spec milestone, and "Phase 1, 2, 3" always means the product phases defined here.

---

## 13. Success criteria and acceptance criteria

### 13.1 Measurable success criteria

| ID | Phase | Metric | Target |
|---|---|---|---|
| S1 | 1 | Time from invoking the tool to a verified-active two-skill selection, cold cache | Under 60 seconds, median under 30 |
| S2 | 1 | Commands to add one skill to an existing selection | One |
| S3 | 1 | Source mutations caused by any operation | Zero, enforced by test |
| S4 | 1 | Foreign destination files modified or deleted | Zero, enforced by test |
| S5 | 1 | Median active selection size after four weeks | Under 10 |
| S6 | 1 | Estimated catalog cost of a typical selection versus the whole library | At least 90 percent lower |
| S7 | 1 | Unreconcilable states after simulated interruption | Zero |
| S8 | 1 | Projects diverged from desired without being reported | Zero |
| S9 | 1 | Adapter claims lacking an evidence URL | Zero |
| S10 | 2 | Keystrokes in the picker to add one skill | Four or fewer |
| S11 | 2 | Install to first activation for a new user following the README | Under 10 minutes |
| S12 | 3 | Wasted-exposure rate on the author's library | Below the post-baseline threshold |
| S13 | 3 | Missed-exposure rate | Below the post-baseline threshold |
| S14 | 3 | Per-turn router latency, deterministic path | Under 50 ms |
| S15 | 3 | Tool calls per turn on the author's own work, against the Phase 1 baseline (D8) | Lower, by a margin set after the baseline week |

### 13.2 Acceptance criteria

Observable behavior. Phase 1 unless marked.

**Source and discovery**
- **AC-1** (SRC-1, SRC-3) Registering a non-Git directory succeeds and its skills are discoverable.
- **AC-2** (SRC-5, S3) A hash of the entire source tree is identical before and after a full activate, modify, deactivate cycle.
- **AC-3** (DIS-2) Three-level nested categories expose every skill with real paths, no renaming or flattening.
- **AC-4** (DIS-1, DIS-5) A source containing `.git/` and `node_modules/` yields no skills from either.
- **AC-5** (DIS-3) Name/dir mismatch loads with a named warning; unparseable YAML is skipped with a named diagnostic.
- **AC-6** (DIS-6) A search matching three categories returns all three in one result set with no filter.

**Bundles**
- **AC-7** (BND-2) A Hermes-only bundle with bare names loads unchanged; absent `description` yields the Hermes default form.
- **AC-8** (BND-3, EDG-2) A bare name matching two categories errors naming both IDs; nothing activates.
- **AC-9** (BND-4) One bundle spanning two categories and two sources activates all of them.
- **AC-10** (BND-5) A saved bundle applied to an empty project yields the original selection.
- **AC-11** (BND-7) Removing one bundle skill leaves every other entry's text and order unchanged.
- **AC-12** (BND-8) Bundle plus one add plus one remove resolves correctly; `status` shows provenance.

**The primary scenario**
- **AC-13** (ACT-4, G3) Two skills from two categories both reach `active` with no category-related step.
- **AC-14** (VER-1) `status` shows source path, destination, strategy, provenance, state per entry.
- **AC-15** (ACT-9) `--yes` still prints the plan; the plan matches the filesystem afterward.
- **AC-16** (ACT-10) Second identical activation makes zero changes and says so.
- **AC-17** (ACT-6) Remove one, add one: exactly two destination entries change; a third retains its receipt record verbatim.
- **AC-18** (ACT-5) `--replace` and `--add` from the same start produce different, correct end states.
- **AC-43** (ACT-13, D3) Every plan against `agents-neutral` names codex, cursor, windsurf-devin, and hermes as consumers.

**Project independence**
- **AC-19** (ACT-3, ACT-8, G9) Operations in project A leave every file and state artifact under project B unchanged, by hash.
- **AC-20** (ACT-3) Two targets in one project hold and report different selections.
- **AC-21** Returning to a project shows its prior selection unchanged.

**Ownership**
- **AC-22** (OWN-3) Collision with a foreign skill refuses, names both paths, changes nothing.
- **AC-23** (OWN-2, OWN-4) `deactivate --all` with two owned and one foreign removes exactly two, leaves the foreign byte-identical, leaves the directory.
- **AC-24** (OWN-5) No agent config file is created or modified; required changes print as manual steps.
- **AC-25** (OWN-1) Every created entry appears in the receipt with source ID and strategy.

**Verification**
- **AC-26** (VER-2, VER-4) A broken destination symlink reports `orphaned`, never `active`.
- **AC-27** (VER-3) A shadowed skill reports `shadowed` with both paths.
- **AC-28** (VER-5) No output asserts an agent read, loaded, or applied a skill.

**Session behavior**
- **AC-29** (SES-1) Against `claude-code`, output states no restart required with the evidence; against an undocumented target, output says it is undocumented.
- **AC-30** (SES-2) Every removal prints the loaded-instructions caveat.

**State and recovery**
- **AC-31** (STA-2, STA-3) Kill mid-materialization: journal remains; next invocation detects it and offers resume or rollback without acting.
- **AC-32** (STA-4) One of three entries fails: the other two remain applied; output names the failure and reason.
- **AC-33** (STA-5) `rollback` restores exact prior destination and receipt.
- **AC-34** (STA-6) Hand-deleting an entry: `reconcile` reports; `--repair` restores.
- **AC-35** (STA-1) Hand-edited selection without apply shows `selected`, not `active`.
- **AC-53** (STA-9) A mutating command started while another holds the project refuses with a named reason and changes nothing.

**Edge cases**
- **AC-36** (EDG-1) Renamed source skill: `orphaned` with candidates; blocked by default.
- **AC-37** (EDG-4) `source relocate` repairs every affected project; each verifies clean.
- **AC-38** (EDG-5) Unmounted source: reported unreachable; no mutation; nothing marked inactive on that basis.
- **AC-39** (EDG-6) Source edit under follow is visible at destination with no re-activation and no drift warning.
- **AC-40** (EDG-7) A skill in two bundles appears once with two provenance records; removing one bundle keeps it.
- **AC-41** (EDG-9, EDG-10) Contradictory skills activate without a conflict error; no semantic conflict claim appears anywhere.
- **AC-42** (EDG-11) Nested project under an ancestor-scanning target warns and names the ancestor.

**Phase 2**
- **AC-44** (UX-2, UX-3, S10) From bare `unam`, typing a query, pressing space on a result, and pressing enter twice yields an active skill.
- **AC-45** (VSN-3, VSN-4) A pinned entry reports drift after the source changes; a followed entry does not.

**Phase 3**
- **AC-46** (RTE-3, Journey H) A prompt mentioning `migrations/` exposes the skills whose `paths` match it plus all global-tier skills and nothing else; the hook line names them.
- **AC-47** (EDG-14) A prompt matching nothing exposes only global-tier skills and the hook line says so.
- **AC-48** (RTE-3, tier `excluded`) An excluded-tier skill is not exposed for a prompt matching only its general area, and is exposed for a prompt naming it.
- **AC-49** (RTE-4) Routing never removes a Phase 1 selected skill from discovery.
- **AC-50** (SES-5) Hook output never contains language implying unloading, by snapshot test.
- **AC-51** (RTE-9) `unam route --dry-run` outside any agent produces the same exposure set the hook would.
- **AC-52** (RTE-10) After a session, the exposure log contains one record per turn with exposed IDs and reasons.
- **AC-54** (RTE-11, D9) A prompt whose only match falls below the confidence bar exposes only global-tier skills.
- **AC-55** (RTE-12) A follow-up of "yes, do it" exposes the same set as the previous turn and does not announce it again.
- **AC-56** (RTE-3) A skill whose `not_when` matches the prompt is not exposed even when its `topics` match.

---

## 14. Risks and assumptions

### Risks

| ID | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | Symlink following inside skill directories is undocumented for every target. | High; core mechanism. | M0 spike gates the default; copy fallback; `doctor` probes; verification reports resolution, never assumes it. |
| R2 | Agents change discovery paths and hook behavior frequently. | High over time. | Evidence URL per adapter claim; `unknown` as a first-class answer; documented re-verification cadence. |
| R3 | Shared destination means Codex cannot be isolated and other agents see selections not meant for them. | Medium; accepted in D3. | ACT-13 names consumers on every plan; Phase 2 isolated adapters for those that support it. |
| R4 | Destinations may be Git-tracked; symlinks are machine-specific. | Medium. | EDG-12 warning; `.gitignore` suggestion; copy strategy where committing is intended. |
| R5 | Users hand-edit destinations and are surprised by reconcile. | Medium. | Receipt ownership; refuse-on-collision; `adopt` in Phase 2; reconcile is report-first. |
| R6 | The loaded-instructions caveat is missed. | Medium; corrosive. | SES-2 mandatory; AC-30 tests it. Phase 3 adds SES-5 and AC-50. |
| R7 | The tool becomes another thing to maintain. | Medium. | Single binary, no daemon; selection file hand-editable. |
| R8 | Scope creep, especially toward Phase 3 before Phase 1 is in daily use, or toward context offloading. | High for a build this size. | D5 sequencing; D6 exclusion; Phase 1 "done when" requires daily use before Phase 2 starts. |
| R9 | **Phase 3 reintroduces the conversation-layer problem the product otherwise avoids.** A hook that shapes what the model sees is a participant. | High for trust in the product's honesty. | Section 12 bounds exactly what the hook may do; SES-5; the exposure line is always visible. |
| R10 | Phase 3 routing without measurement is indistinguishable from guessing. | High for the phase's value. | RTE-10 and S12, S13 are hard requirements of the phase, not nice-to-haves. |
| R11 | Phase 3 depends on authors annotating skills; unannotated libraries get no benefit. | Medium. | EDG-15 safe default; DIS-12 reporting; the author's own library is the first to be annotated. |
| R12 | Rust slows the first milestone versus a scripting language. | Low; accepted in D1. | M1 is a read-only path; the type-level source protection pays back immediately. |

### Assumptions

| ID | Assumption | Verification |
|---|---|---|
| A1 | Target agents follow directory symlinks in skill discovery paths. | M0 spike per adapter. |
| A2 | Agents tolerate a skill directory name differing from frontmatter `name`, needed for destination aliasing. | Per-adapter test; ecosystem guidance says lenient implementations warn and load **[Verified]**. |
| A3 | 50 to 100 tokens per skill of catalog cost is a usable estimate. | Published figure **[Verified]**; labeled estimate. |
| A4 | Project is the right grain for Phases 1 and 2, and per-message is the right grain for Phase 3. | Phase 3 measurement. |
| A5 | A project-local `.unam/` directory is acceptable and the receipt gets gitignored. | Review. |
| A6 | At least Claude Code offers a per-message hook usable for Phase 3; which others do is unknown. | Vendor hook documentation, verified before Phase 3 scoping. |
| A7 | Skill loads are observable in at least one target, so missed-exposure rate can be computed. | Vendor hook and log documentation. |

---

## 15. Open questions

Only decisions that materially change the product. Q2 and Q4 from the previous version are closed (D3, D2) and removed.

**Q1. Symlink-first or copy-first default?** Blocked on the M0 spike. If symlinks fail for either Phase 1 adapter, the default may become copy, which weakens VSN-1 and changes update behavior from live to refresh-based.

**Q3. Is the project selection file a committed artifact?** Committing enables the team-lead journey but puts source aliases into a shared repository where another developer's aliases differ. Needs alias indirection, which is design work. Decide before Phase 2 release.

**Q5. How is the Hermes `instruction` honored?** Current answer: generated brief skill, discoverable but not guaranteed read. Phase 3 opens a different answer: the hook could expose the brief alongside the exposed skills for the turn. Still would not inject it. Decide in Phase 3 scoping.

**Q6. Native Hermes adapter emitting bundle manifests into `$HERMES_BUNDLES_DIR`?** Small once `_slugify` parity is confirmed. Phase 2 candidate; not committed.

**Q7. Where does the atlas get its topic vocabulary?** From author `topics` only (strict, deterministic, under-recalls), or also from a derived index of the repository (README headings, directory names, manifest keywords), which improves recall and introduces inference. Decide before RTE-2 is built.

**Q8. Is a model-backed intent classifier ever in scope?** RTE-5 makes it opt-in and off by default. If the deterministic router proves insufficient on the author's own library, the choice is between a local small model, a call to the host agent's own model through the hook, or accepting the limit. Each has a different cost, latency, and honesty profile. Decide after the deterministic baseline is measured.

**Q9. What does "expose for one turn" mean mechanically, per target?** Options: write and remove symlinks around the turn (filesystem-visible, works with any directory-scanning target, racy with file watchers); emit a catalog delta through the hook's output channel where the target supports additional context (cleaner, target-specific, and closer to injection); or pre-materialize all `domain`-tier skills and use per-skill visibility controls where the target has them. This is the central technical question of Phase 3 and determines which targets can support it at all. ADR-0001 proposes the second option; it stays open until each target's hook contract is verified.

**Q10. How does `unam hook install` square with OWN-5?** Installing a hook means writing to an agent configuration file the tool does not own, which OWN-5 forbids in Phases 1 and 2. Options: print the exact change as a manual step, consistent with OWN-5; or write it with explicit per-target consent, receipted and removable by `unam hook uninstall`. Decide in Phase 3 scoping.

**Q11. Crate name and license.** `unam`, or `unambiguous` with a `unam` binary; license not chosen. Decide before the Phase 2 release.

---

## 16. Sources

**Hermes reference model**
- [Hermes skills documentation](https://github.com/NousResearch/hermes-agent/blob/main/website/docs/user-guide/features/skills.md): bundle location and invocation, YAML fields, bundle precedence, `skills.external_dirs`, precedence rules, project-local directories and `hermes skills trust`, five-skill chained invocation cap, conditional activation fields under `metadata.hermes`, progressive disclosure.
- [Hermes bundle implementation, `agent/skill_bundles.py`](https://github.com/NousResearch/hermes-agent/blob/main/agent/skill_bundles.py): dataclass fields and defaults, `$HERMES_BUNDLES_DIR` and `<HERMES_HOME>/skill-bundles`, accepted extensions, alphabetical ordering, mtime cache, `_slugify`, `ValueError("Bundle name is required")`, skip-and-note handling of unknown skills, absence of a maximum, duplicate-slug resolution, instruction merging.

**Agent Skills specification and guidance**
- [Agent Skills specification](https://agentskills.io/specification): frontmatter fields and constraints, `metadata` as string-to-string map, `allowed-tools`, directory conventions, relative path resolution from the skill root, progressive disclosure, `skills-ref validate`.
- [Adding skills support to your agent](https://agentskills.io/client-implementation/adding-skills-support.md): 50 to 100 tokens per skill catalog cost, the `.agents/skills/` convention, scan bounds, lenient validation, project-over-user precedence, trust gating, compaction protection of loaded skills, cloud and sandboxed constraints.

**Coding agents**
- [Claude Code skills](https://code.claude.com/docs/en/skills): `~/.claude/skills/`, `.claude/skills/`, nested, plugin and enterprise locations; enterprise over personal over project precedence; `skillOverrides`; `disable-model-invocation`; directory watching with in-session pickup except bare mode; `/reload-skills`.
- [Codex skills](https://learn.chatgpt.com/docs/build-skills.md): `.agents/skills` from cwd to repo root, `$HOME/.agents/skills`, `/etc/codex/skills`, required `name` and `description`, optional `agents/openai.yaml`, automatic detection with restart fallback.
- [Cursor agent skills](https://cursor.com/docs/skills): `.agents/skills/`, `.cursor/skills/`, compatibility reads of `.claude/skills/` and `.codex/skills/`, recursive walk, frontmatter fields.
- [Windsurf / Devin skills](https://docs.devin.ai/desktop/cascade/skills): `.devin/skills/`, legacy `.windsurf/skills/`, `~/.codeium/windsurf/skills/`, `~/.config/devin/skills/`, enterprise paths, `.agents/skills/` and `~/.agents/skills/`, `.claude/skills/` when Claude config reading is enabled.
- [Windsurf / Devin workflows](https://docs.devin.ai/desktop/cascade/workflows): workflow paths and migration to skills.