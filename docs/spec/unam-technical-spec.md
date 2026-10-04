# Unambiguous (`unam`): Technical Specification and Implementation Plan

Companion to `PRD-unambiguous.md`. The PRD defines what and why; this document defines how, in enough detail that an agent can begin implementation without further product decisions. Requirement and acceptance-criteria IDs (`ACT-6`, `AC-17`) refer to the PRD.

This document covers PRD Phases 1 and 2. Phase 1 is milestones M0 to M5. Phase 2 is milestones M6 and M7 plus the items under "What to revisit" in section 11. Phase 3, intent-driven exposure, is decided in `ADR-0001-dynamic-skill-routing.md` and builds on the registry, catalog, and adapters defined here. Where this document says "v1" it means milestones M0 to M7; "v2" means the remaining Phase 2 scope.

| | |
|---|---|
| Language | Rust, stable toolchain, edition 2024 |
| Platforms | macOS and Linux. Windows is not a build target in v1; nothing in the design precludes it. |
| Scope | Personal tool built to open-source standards: tests, docs, reproducible builds, no telemetry, no network. |
| Destination policy | Shared by default for the neutral `.agents/skills/` adapter. Reported, not hidden. |
| Name | Project: Unambiguous. Command: `unam`. |
| Binary | One: `unam`. CLI and interactive picker share one core library. |

---

## 1. Architecture

### 1.1 Shape

```
                 ┌──────────────────────────────────────────────┐
                 │                unam (binary)                 │
                 │  ┌─────────────────┐  ┌───────────────────┐  │
                 │  │  cli  (clap)    │  │  tui  (ratatui)   │  │
                 │  └────────┬────────┘  └─────────┬─────────┘  │
                 │           └──────────┬──────────┘            │
                 └──────────────────────┼───────────────────────┘
                                        │  public API only
                 ┌──────────────────────▼───────────────────────┐
                 │             unam-core (library)              │
                 │                                              │
                 │ config ─ source ─ catalog ─ bundle ─ project │
                 │       resolve ─ adapter ─ plan ─ apply       │
                 │      verify ─ reconcile ─ state ─ fsops      │
                 └──────────────────────┬───────────────────────┘
                                        │
         ┌──────────────┬───────────────┼───────────────┬──────────────┐
   ~/.config/unam     bundles/     <project>/.unam   <source roots>  <destinations>
   config.toml        *.yaml       selection.yaml    read-only       .claude/skills
                                   applied.json                      .agents/skills
                                   journal.json
                                   history/
```

Three invariants the layering enforces:

1. **The UI layers hold no logic.** `cli` and `tui` call `unam-core` and render results. Every action available in the picker is a core function the CLI also exposes (UX-1, UX-6).
2. **Source roots are opened read-only.** `unam-core` has no code path that writes under a registered source. The `fsops` module exposes write operations only against destination and state paths, and the type system makes a source path unusable as a write target (section 4.9).
3. **Three state stores, never merged.** Desired state (`selection.yaml`), applied state (`applied.json`), observed state (a filesystem scan at runtime). Every command that reports status reads all three and reports divergence (STA-1).

### 1.2 Data flow for the primary scenario

```
unam add skillzone:sdlc/code-review skillzone:testing/pytest-fixtures
   │
   ├─ project::detect_root(cwd)                 -> ProjectRoot
   ├─ adapter::detect_all(root)                 -> [claude-code] (one found, used as default)
   ├─ project::load_selection(root)             -> Selection (desired)
   ├─ catalog::load_or_scan(sources)            -> Catalog
   ├─ resolve::resolve(refs, catalog)           -> [SkillId]  (errors on ambiguity)
   ├─ Selection.add(entries)                    -> Selection'
   ├─ state::load_receipt(root, target)         -> Receipt (applied)
   ├─ verify::observe(dest)                     -> Observed
   ├─ plan::diff(desired', applied, observed)   -> Plan {adds, removes, unchanged, conflicts}
   ├─ render plan; confirm (or --yes)
   ├─ apply::execute(plan)                      -> journal → steps → receipt' → history → clear journal
   ├─ verify::classify(receipt', observed')     -> [EntryState]
   └─ render result + adapter.refresh() + adapter.followups() + SES-2 caveat on removals
```

### 1.3 Crate layout

```
unambiguous/
├── Cargo.toml                 # workspace
├── crates/
│   ├── unam-core/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── config.rs      # user config, paths
│   │       ├── source.rs      # registry CRUD, relocate
│   │       ├── catalog/
│   │       │   ├── mod.rs     # Catalog, scan orchestration, cache
│   │       │   ├── scan.rs    # directory walk
│   │       │   ├── frontmatter.rs
│   │       │   └── search.rs  # fuzzy + field search
│   │       ├── bundle.rs      # manifest load/save, slugify
│   │       ├── project.rs     # root detection, selection load/save
│   │       ├── resolve.rs     # SkillRef -> SkillId
│   │       ├── adapter/
│   │       │   ├── mod.rs     # trait, registry, detection
│   │       │   ├── claude_code.rs
│   │       │   └── agents_neutral.rs
│   │       ├── plan.rs        # diff -> Plan
│   │       ├── apply.rs       # executor, journal, rollback, resume
│   │       ├── verify.rs      # observe, classify, shadow analysis
│   │       ├── reconcile.rs
│   │       ├── state.rs       # receipt, journal, history IO, project lock
│   │       ├── hash.rs        # blake3 tree hashing
│   │       └── fsops.rs       # atomic write, symlink, copy, path typing
│   └── unam-cli/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── main.rs
│       │   ├── cli/           # one file per command
│       │   ├── output.rs      # human + --json renderers
│       │   └── tui/
│       │       ├── mod.rs
│       │       ├── app.rs     # state machine
│       │       ├── picker.rs  # three-pane view
│       │       └── plan_view.rs
│       └── tests/             # integration tests (run against the built binary)
│           ├── fixtures/      # fake source trees, fake projects
│           └── *.rs
├── spike/
│   └── symlink-discovery.md   # procedure and results for the gating spike
└── docs/
    └── adapters.md            # evidence table, one row per adapter claim
```

### 1.4 Dependencies

Chosen for maturity and for being the conventional answer in the Rust ecosystem. Versions are left to `cargo add` at init; pin in `Cargo.lock`, commit the lockfile.

| Concern | Crate | Notes |
|---|---|---|
| CLI parsing | `clap` (derive) | Subcommands, `--json`, `--yes`, `--target` |
| TUI | `ratatui` + `crossterm` | Three-pane picker |
| Serialization | `serde`, `serde_norway`, `serde_json`, `toml` | YAML for bundles and selection (Hermes compatibility), TOML for user config, JSON for machine state. `serde_yaml` is deprecated upstream (final release `0.9.34+deprecated`); `serde_norway` is a maintained fork with the same API, including `Mapping`. `serde-saphyr` is the more actively developed alternative if the fork stalls. |
| Directory walk | `ignore` | Respects `.gitignore`, skips hidden and VCS dirs by default, parallel walker available |
| Hashing | `blake3` | Content and tree hashes |
| Paths | `directories` | XDG on Linux, Library on macOS, overridable by env |
| Errors | `thiserror` (core), `anyhow` (binary) | Typed errors in the library, context at the edge |
| Fuzzy search | `nucleo` | Same matcher used by Helix; fast enough for thousands of entries per keystroke |
| Atomic writes | `tempfile` | Write to temp in the same directory, fsync, rename |
| Time | `jiff` | Timestamps in receipt and history |
| File lock | `fs4` | Advisory lock on `<root>/.unam/lock` (§4.10) |
| Logging | `tracing` + `tracing-subscriber` | `UNAM_LOG=debug` |
| Testing | `assert_cmd`, `predicates`, `insta`, `tempfile`, `proptest` | CLI tests, snapshot tests, property tests on diff |

Deliberately absent in v1: any Git library. Root detection walks for a `.git` entry. Commit SHA capture (pin mode, v2) shells out to `git rev-parse HEAD` when `git` is on `PATH` and records `null` otherwise. Adding `gix` is a v2 decision.

---

## 2. Storage, formats, and paths

All paths below are defaults. Every one is overridable in `config.toml` or by environment variable.

| Store | Default path | Format | Owner | In VCS |
|---|---|---|---|---|
| User config | `$XDG_CONFIG_HOME/unam/config.toml` (`~/.config/unam/config.toml`) | TOML | user | n/a |
| Bundles | `~/.config/unam/bundles/<slug>.yaml` | YAML | user | user's choice |
| Project selection (desired) | `<root>/.unam/selection.yaml` | YAML | user, via tool | recommended |
| Receipt (applied) | `<root>/.unam/applied.json` | JSON | tool | gitignore |
| Journal | `<root>/.unam/journal.json` | JSON | tool, transient | gitignore |
| History | `<root>/.unam/history/<timestamp>-<target>.json` | JSON | tool | gitignore |
| Project index | `$XDG_DATA_HOME/unam/projects.json` | JSON | tool, cache | n/a |
| Catalog cache | `$XDG_CACHE_HOME/unam/catalog/<alias>.json` | JSON | tool, cache | n/a |

Environment overrides: `UNAM_CONFIG_DIR`, `UNAM_BUNDLES_DIR`, `UNAM_CACHE_DIR`. The first two mirror the Hermes pattern of `HERMES_BUNDLES_DIR`.

### 2.1 `config.toml`

```toml
# ~/.config/unam/config.toml
version = 1

[defaults]
strategy = "symlink"            # symlink | copy
destination_policy = "shared"   # shared | isolated   (applies to agents-neutral)
catalog_budget_tokens = 1500    # warn above this estimate
allow_missing = false

[[sources]]
alias = "skillzone"
path = "~/Developer/01-AI/skillzone"   # placeholder; user-supplied
precedence = 10                        # orders display and candidate lists; never disambiguates

[[sources]]
alias = "house"
path = "~/work/team-skills"
precedence = 20
```

### 2.2 Bundle manifest

Hermes-compatible. A file containing only `name`, `description`, `skills`, and `instruction` with bare skill names is valid input.

```yaml
# ~/.config/unam/bundles/backend-dev.yaml
name: backend-dev
description: Backend feature work: review, test, PR workflow.
skills:
  - skillzone:sdlc/github-code-review          # qualified: source alias + relative path
  - skillzone:sdlc/github-pr-workflow
  - skillzone:testing/test-driven-development
  - error-handling                              # bare name: resolved only if unambiguous
instruction: |
  Always start by writing failing tests, then implement.
metadata:
  unambiguous:
    version: 1
    materialize_instruction: true
```

Parsing rules (mirroring verified Hermes behavior): `name` falls back to the file stem; a blank `name` after trim is an error; `skills` must be a non-empty list; `description` defaults to `Load {n} skills as a bundle`; `instruction` defaults to empty; files are loaded in alphabetical order; duplicate slugs keep the first and log a warning.

### 2.3 Selection file

```yaml
# <root>/.unam/selection.yaml
version: 1
targets:
  claude-code:
    strategy: symlink
    bundles: [backend-dev]
    add:
      - skillzone:data/schema-migration
    remove:
      - skillzone:sdlc/github-pr-workflow
  agents-neutral:
    strategy: symlink
    bundles: []
    add:
      - skillzone:sdlc/code-review
    remove: []
```

Resolution per target: expand `bundles` in order, append `add`, subtract `remove`, deduplicate by `SkillId` keeping all provenance records. `destination` is not stored; the adapter owns it. `strategy` may be omitted and inherits `defaults.strategy`.

### 2.4 Receipt

```json
{
  "version": 1,
  "project_root": "/abs/path/to/project",
  "target": "claude-code",
  "applied_at": "2026-10-02T21:40:00Z",
  "selection_hash": "blake3:…",
  "entries": [
    {
      "kind": "skill",
      "skill_id": "skillzone:sdlc/code-review",
      "name": "code-review",
      "dest": ".claude/skills/code-review",
      "strategy": "symlink",
      "source_path": "/abs/source/sdlc/code-review",
      "tree_hash": "blake3:…",
      "git_commit": null,
      "pin": "follow",
      "provenance": [{"bundle": "backend-dev"}],
      "applied_at": "2026-10-02T21:40:00Z"
    },
    {
      "kind": "generated",
      "generated": "bundle-brief",
      "bundle": "backend-dev",
      "name": "backend-dev-brief",
      "dest": ".claude/skills/backend-dev-brief",
      "content_hash": "blake3:…",
      "applied_at": "2026-10-02T21:40:00Z"
    }
  ]
}
```

`dest` is project-relative so the receipt survives a project directory move. `source_path` is absolute at apply time and is informational; the live source path is re-derived from the registry.

### 2.5 Journal

```json
{
  "version": 1,
  "op_id": "01J…",
  "target": "claude-code",
  "started_at": "…",
  "previous_receipt": { "...": "full copy, for rollback" },
  "plan": [ { "step": 1, "op": "remove", "dest": "...", "..." : "..." } ],
  "completed": [1, 2]
}
```

Written and fsynced before the first filesystem mutation; after each step the journal is rewritten atomically with the step added to `completed`; deleted only after the new receipt has been written atomically and the previous receipt copied to `history/`.

### 2.6 Reserved for Phase 3: skill metadata

Phase 3 reads per-skill metadata (PRD DIS-11, RTE-8) from two places, the overlay winning per field: frontmatter under `metadata.unambiguous`, and a user-scope overlay file `~/.config/unam/skills.yaml` keyed by `SkillId`. The schema is in the PRD, section 9.

Phases 1 and 2 do not parse or act on it. They only guarantee it survives: `Skill.frontmatter` keeps the full frontmatter mapping, unknown keys included, and `show` prints it. That is enough for skills to be annotated at any time before Phase 3 exists.

The path `~/.config/unam/skills.yaml` and the directory `<root>/.unam/` are reserved; Phase 3 adds `atlas.json` to the latter.

---

## 3. Domain model

Illustrative Rust. Field names are the contract; exact derives and visibility are implementation detail.

```rust
// ---- identity -------------------------------------------------------------

/// Stable identifier: "<alias>:<relative/path>" with forward slashes.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SkillId { pub source: SourceAlias, pub rel_path: RelPath }

/// What a user may type or write in a manifest. Resolved to SkillId.
pub enum SkillRef {
    Qualified { source: SourceAlias, rel_path: RelPath }, // "skillzone:sdlc/code-review"
    SourceName { source: SourceAlias, name: String },     // "skillzone:code-review"
    Bare(String),                                         // "code-review"
}

// ---- sources and catalog --------------------------------------------------

pub struct Source {
    pub alias: SourceAlias,
    pub path: SourcePath,          // read-only newtype, see §4.9
    pub precedence: u32,
    pub kind: SourceKind,          // Directory | GitRepo (detected)
}

pub struct Skill {
    pub id: SkillId,
    pub name: String,              // frontmatter `name`, else dir name
    pub description: String,
    pub category: RelPath,         // parent of rel_path; may be empty
    pub frontmatter: serde_norway::Mapping,
    pub supporting: Vec<RelPath>,  // everything except SKILL.md
    pub tree_hash: Hash,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct Catalog {
    pub skills: Vec<Skill>,
    by_id: HashMap<SkillId, usize>,
    by_name: HashMap<String, Vec<usize>>,   // collisions are expected
    pub scanned_at: Timestamp,
}

// ---- bundles --------------------------------------------------------------

pub struct Bundle {
    pub slug: Slug,
    pub name: String,
    pub description: String,
    pub skills: Vec<SkillRef>,
    pub instruction: Option<String>,
    pub metadata: BundleMetadata,   // typed view over metadata.unambiguous.*
    pub path: PathBuf,
}

// ---- project and desired state -------------------------------------------

pub struct ProjectRoot { pub path: PathBuf, pub how: RootDetection } // Git | Explicit

pub struct Selection {
    pub version: u32,
    pub targets: BTreeMap<TargetId, TargetSelection>,
}

pub struct TargetSelection {
    pub strategy: Option<Strategy>,
    pub bundles: Vec<Slug>,
    pub add: Vec<SkillRef>,
    pub remove: Vec<SkillRef>,
}

/// Fully resolved desired entry for one target.
pub struct DesiredEntry {
    pub id: SkillId,
    pub name: String,
    pub strategy: Strategy,
    pub pin: Pin,                   // Follow | Commit(String) | Hash(Hash)
    pub provenance: Vec<Provenance>, // Bundle(slug) | Adhoc
}

// ---- applied and observed state ------------------------------------------

pub struct Receipt { /* §2.4 */ }
pub enum ReceiptEntry { Skill(SkillReceipt), Generated(GeneratedReceipt) }

pub struct Observed {
    pub entries: Vec<ObservedEntry>,
}
pub struct ObservedEntry {
    pub dest: RelPath,
    pub kind: ObservedKind,         // Symlink{target, resolves} | Dir | Other
    pub skill_md: Option<ParsedHeader>,  // name + description if parseable
    pub ownership: Ownership,       // Owned(skill_id) | Foreign | Orphaned
}

// ---- plan ----------------------------------------------------------------

pub struct Plan {
    pub target: TargetId,
    pub steps: Vec<Step>,
    pub unchanged: Vec<SkillId>,
    pub conflicts: Vec<Conflict>,   // blocking unless overridden
    pub warnings: Vec<Warning>,
    pub estimate: CatalogEstimate,
}
pub enum Step {
    Remove { dest: RelPath, expect: Ownership },
    Link   { id: SkillId, dest: RelPath, source: PathBuf },
    Copy   { id: SkillId, dest: RelPath, source: PathBuf },
    WriteGenerated { dest: RelPath, content: String, kind: GeneratedKind },
}
pub enum Conflict {
    ForeignAtDest { dest: RelPath },
    DuplicateName { name: String, ids: Vec<SkillId> },
    UnresolvedRef { reference: String, candidates: Vec<SkillId> },
    AmbiguousRef  { reference: String, candidates: Vec<SkillId> },
    SourceUnreachable { alias: SourceAlias },
}

// ---- verification ---------------------------------------------------------

pub enum EntryState { Selected, Applied, Active, Shadowed { by: PathBuf }, Foreign, Orphaned }
```

---

## 4. Module specifications

### 4.1 `config`

- Load `config.toml`; create with defaults and an empty `sources` list on first run.
- Expand `~` and environment variables in source paths at load time; store the user's spelling, use the expanded form.
- Validate: unique aliases matching `^[a-z0-9][a-z0-9-]{0,31}$`; precedence unique or tie-broken alphabetically with a warning.
- All writes atomic (`fsops::write_atomic`).

### 4.2 `source`

- `add(alias, path)`: canonicalize, check readable, run a scan, warn (not fail) on zero skills (SRC-6).
- `relocate(alias, new_path)`: update config, invalidate cache, and, for every project in the project index whose receipt references the alias, mark entries for re-verification. Selections reference by alias so they need no rewrite (SRC-4, EDG-4).
- `remove(alias)`: refuse if any indexed project selection references it, unless `--force`, which leaves those selections with unresolvable refs and says so.

### 4.3 `catalog`

**Scan** (`scan.rs`): walk each source with `ignore::WalkBuilder`, hidden entries skipped, `.gitignore` respected, `max_depth(8)`, hard cap of 5000 directories per source with a diagnostic when hit (DIS-5). A directory is a skill iff it contains a file named exactly `SKILL.md`. Do not descend into a skill directory looking for nested skills; its contents are supporting files (DIS-8).

**Frontmatter** (`frontmatter.rs`): a leading UTF-8 BOM is stripped and CRLF line endings are normalized to LF; the file must then begin with `---\n`; take text up to the next line that is exactly `---`; parse with `serde_norway`. On parse failure, retry once after quoting unquoted values containing `: ` (the common cross-client defect). Lenient validation per DIS-3: name/dir mismatch warns and loads; over-length name warns and loads; missing or empty description skips with a diagnostic; unparseable YAML after retry skips with a diagnostic.

**Hashing**: `tree_hash` is blake3 over a canonical listing: for each file in sorted relative-path order, `path\0size\0blake3(content)\n`. Symlinks inside a source skill are hashed by their target path string, not followed.

**Cache**: one JSON file per source keyed by alias. For each skill it stores the parsed metadata, the tree hash, and a stat fingerprint: the sorted list of `(relative path, size, mtime in nanoseconds)` for every file in the skill directory. Every load walks the source and stats files; that walk is cheap and is the only reliable way to notice a new skill in a nested category or a file edited in place, neither of which changes the source root's mtime. A skill whose fingerprint is unchanged reuses its cached metadata and hash; a changed or new fingerprint triggers a re-parse and re-hash of that skill only; a cached skill no longer on disk is dropped (DIS-9). `--no-cache` ignores the cache and re-parses everything.

**Search** (`search.rs`): `nucleo` fuzzy match over a composite string `name  category  alias  description`, with filters `--category <prefix>` (path prefix match) and `--source <alias>`. Results carry match indices for highlighting in the TUI.

### 4.4 `bundle`

- Load all `*.yaml` and `*.yml` from the bundle directory, sorted by filename, applying the §2.2 rules.
- `slugify(name)`: lowercase, replace any run of non-`[a-z0-9]` with `-`, trim leading and trailing `-`. This approximates Hermes `_slugify`, which lives in `skill_commands` and has not been read directly; before claiming parity, read it and align. Record the outcome in `docs/adapters.md`.
- `save_from_selection(root, target, name)`: writes a manifest whose `skills` are fully qualified refs, so the bundle is unambiguous regardless of future library changes (BND-5).
- `add_skill`, `remove_skill`: edit the `skills` sequence in place preserving comments and key order. `serde_norway` does not preserve comments, so a line-oriented edit of the `skills:` block, and fall back to a full rewrite only with `--rewrite` (BND-7, AC-11).

### 4.5 `project`

- `detect_root(cwd)`: walk up for a directory containing `.git` (file or dir); if none, the given path with `RootDetection::Explicit`. `--project <path>` overrides.
- `load_selection`, `save_selection`: atomic; create `.unam/` on first write and offer the `.gitignore` lines for `applied.json`, `journal.json`, `history/` (OWN-7).
- Register the root in the project index on every write.

### 4.6 `resolve`

```
resolve(ref, catalog, sources) -> Result<SkillId, ResolveError>

Qualified{source, rel_path}  -> exact lookup; NotFound(candidates by path similarity)
SourceName{source, name}     -> by_name within source; 0 => NotFound; >1 => Ambiguous(ids)
Bare(name)                   -> by_name across sources;
                                0 => NotFound(candidates by edit distance ≤ 2)
                                1 => Ok
                                >1 => Ambiguous(ids), always. Source precedence never
                                      disambiguates two skills with the same name; it only
                                      orders the candidate list.
```

Ambiguity is always an error with candidates listed (EDG-2, AC-8). There is no "pick the first" path.

### 4.7 `adapter`

```rust
pub trait Adapter: Send + Sync {
    fn id(&self) -> TargetId;
    fn consumers(&self) -> &'static [&'static str];        // agents known to read this destination
    fn detect(&self, root: &Path) -> Detection;             // Present | Absent | Unknown, with reasons
    fn destination(&self, root: &Path) -> RelPath;          // project-relative
    fn scan_locations(&self, root: &Path) -> Vec<ScanLocation>; // for shadow analysis
    fn strategies(&self) -> &'static [Strategy];
    fn refresh(&self, ctx: &ApplyContext) -> Refresh;       // None | Command(String) | Restart | Unknown
    fn followups(&self, root: &Path) -> Vec<Followup>;
    fn evidence(&self) -> &'static [Evidence];              // (claim, url)
}

pub struct ScanLocation { pub path: PathBuf, pub scope: Scope, pub rank: u8 } // lower rank wins
```

**`claude-code`**
- `destination`: `.claude/skills`
- `scan_locations`: `~/.claude/skills` (scope User, rank 1), `<root>/.claude/skills` (Project, rank 2), nested `<subdir>/.claude/skills` (Nested, rank 3, found by walking the project with the `ignore` walker so that gitignored trees such as `node_modules` are skipped, capped at depth 6). Enterprise managed-settings location is platform-specific and is read if present, rank 0.
- Precedence for shadowing: enterprise > personal > project, per documentation. A same-name skill at rank 0 or 1 marks ours `Shadowed`.
- `refresh`: `None` when `.claude/skills` existed before this apply; `Command("/reload-skills")` when the apply created it.
- `detect`: Present if `<root>/.claude/` or `~/.claude/` exists.
- `consumers`: `["claude-code", "cursor (compat read)", "windsurf-devin (when Claude config reading is enabled)"]`

**`agents-neutral`**
- `destination`: `.agents/skills`
- `scan_locations`: `~/.agents/skills` (User, rank 2), every ancestor `.agents/skills` from `<root>` up to the Git root (non-empty only when `--project` names a directory below the Git root, since detected roots are the Git root) (Ancestor, rank 1 for nearer, since Codex walks upward; documented behavior does not state ancestor precedence, so treat equal-rank same-name as a warning, not a shadow), `<root>/.agents/skills` (Project, rank 0).
- `refresh`: `Unknown`, rendered per consumer: Codex "detects changes automatically; restart if the skill does not appear", Cursor "not documented", Windsurf/Devin "not documented", Hermes "run `hermes skills trust` once per repository".
- `followups`: `hermes skills trust` when `~/.hermes/` exists.
- `detect`: Present if `<root>/.agents/`, `~/.codex/`, `~/.cursor/`, `~/.codeium/`, `~/.config/devin/`, or `~/.hermes/` exists.
- `consumers`: `["codex", "cursor", "windsurf-devin", "hermes"]`
- Shared-destination warning emitted on every plan: "Visible to: codex, cursor, windsurf-devin, hermes."

**Cross-adapter duplicate check**: when both adapters are active in one project, any skill present in both destinations is reported once as `warning: duplicate for cursor and windsurf-devin via compat read of .claude/skills`.

Every `Evidence` entry is rendered by `unam doctor --evidence` and in `docs/adapters.md`. An adapter without evidence for its `destination` and `scan_locations` fails a unit test.

### 4.8 `plan`

```
diff(desired: [DesiredEntry], applied: Receipt, observed: Observed, adapter) -> Plan

dest(entry) = adapter.destination(root) / entry.name
```

1. **Resolve destination names.** Two desired entries with the same `name` → `Conflict::DuplicateName` (EDG-2). Blocking.
2. **Classify observed.** For each observed dest: receipted and matches (symlink target equals current source path, or copy hash equals receipt) → Owned; receipted and mismatched → Orphaned; not receipted → Foreign.
3. **Adds** = desired entries whose dest is not Owned. If dest is Foreign → `Conflict::ForeignAtDest` (OWN-3). Blocking unless `--skip-conflicts` (skips that entry) or `--adopt` (v2).
4. **Removes** = Owned entries not in desired. Orphaned receipted entries not in desired are also removed from the receipt; the filesystem entry is removed only if it is a symlink whose target is under a registered source, or a directory whose hash matches the receipt. Otherwise it is left and reported.
5. **Replacements** = desired entries that are Owned but whose strategy or pin differs → one Remove then one Link/Copy.
6. **Unchanged** = Owned, same strategy and pin. Not touched. Receipt entry carried forward verbatim (AC-17).
7. **Generated**: for each bundle in the target selection with `materialize_instruction` true and a non-empty `instruction`, a `WriteGenerated` step for `<dest>/<slug>-brief/SKILL.md`; content-hash compared to the receipt so an unchanged brief is a no-op.
8. **Order**: Removes, then Replacements, then Adds, then Generated.
9. **Warnings**: shadowing per `scan_locations`; ancestor `.agents/skills` presence (EDG-11); destination under VCS tracking, detected by `git ls-files --error-unmatch <dest>` when `git` is available (EDG-12); catalog estimate over budget (ACT-12).
10. **Estimate**: `sum over desired of (ceil((len(name) + len(description)) / 4) + 20)`, where 20 is a per-skill allowance for catalog framing. Always labeled an estimate.

A plan with zero steps and zero conflicts is reported as "no changes" and `apply` returns without writing anything (ACT-10, AC-16).

### 4.9 `fsops`

Type-level separation of read-only and writable paths:

```rust
pub struct SourcePath(PathBuf);   // constructed only by `source::Source`; no write fns accept it
pub struct DestPath(PathBuf);     // constructed only from adapter.destination(root)
pub struct StatePath(PathBuf);    // <root>/.unam/*, config, cache

pub fn write_atomic(p: &StatePath, bytes: &[u8]) -> Result<()>;
pub fn symlink_dir(src: &SourcePath, dst: &DestPath) -> Result<()>;
pub fn copy_tree(src: &SourcePath, dst: &DestPath) -> Result<Hash>;
pub fn remove_owned(dst: &DestPath, expect: &Ownership) -> Result<()>; // re-checks ownership immediately before unlink
pub fn read_dir_shallow(p: &Path) -> Result<Vec<DirEntryInfo>>;       // observation, any path
```

The mutating functions are methods of an `FsOps` trait with one production implementation; `apply` takes `&dyn FsOps`. That seam is what lets the interruption tests (§8) inject a failure at step N.

`remove_owned` re-verifies ownership at call time, not at plan time, to close the window where a user replaces a symlink with real content between plan and apply.

`symlink_dir` creates an absolute symlink. Relative symlinks would survive moving source and project together, which is rare; absolute links survive moving the project alone, which is common.

`copy_tree` copies regular files and directories, preserves permissions, re-creates intra-skill relative symlinks as symlinks, refuses to follow symlinks pointing outside the skill directory, and returns the tree hash.

### 4.10 `apply`

```
execute(plan, ctx) -> Result<ApplyReport>

0. take the project lock (<root>/.unam/lock); if held -> Err(Locked)
1. if journal exists -> Err(Interrupted(journal))   // caller must resume or rollback first
2. write journal {previous_receipt, plan, completed: []}; fsync
3. for step in plan.steps:
       run step (fsops)
       on Ok: journal.completed.push(step.n); rewrite journal atomically
       on Err: record failure, break                 // leave journal in place
4. build receipt' from (unchanged ∪ succeeded adds ∪ generated) minus removed
5. write journal.previous_receipt to history/<ts>-<target>.json; then write receipt' atomically
6. delete journal
7. release the lock; return report {succeeded, failed, skipped}
```

The lock is an advisory file lock held for the whole of `execute`, `resume`, and `rollback`, so a picker left open and a CLI call in another terminal cannot interleave mutations. Read-only commands do not take it.

A failure mid-run leaves the journal and a partially applied destination; the report says exactly which entries succeeded (STA-4, AC-32). The next invocation of any mutating command returns `Interrupted` and the CLI offers `unam resume` or `unam rollback` (STA-3, AC-31).

`resume`: re-execute steps not in `completed`, then finish steps 4 to 7.
`rollback` (from journal): undo `completed` in reverse: Link/Copy → `remove_owned`; Remove → re-create from `previous_receipt`; WriteGenerated → remove. Then restore `previous_receipt` and delete the journal.
`rollback` (no journal, STA-5): load `history/` latest for the target, treat it as desired, plan, apply.

History is capped at 20 entries per target; oldest pruned on write (STA-7).

### 4.11 `verify`

```
observe(adapter, root) -> Observed      // shallow read of destination; readlink; parse SKILL.md header
classify(desired, receipt, observed, adapter) -> Vec<(SkillId | dest, EntryState)>
```

| Condition | State |
|---|---|
| In desired, no receipt entry | `Selected` |
| Receipt entry; dest missing, or link unresolvable, or (copy strategy only) hash differs from the receipt | `Orphaned` |
| Receipt entry; dest resolves; `SKILL.md` parse fails | `Applied` (unverifiable) |
| Receipt entry; resolves and parses; same-name skill at lower rank in `scan_locations` | `Shadowed { by }` |
| Receipt entry; resolves and parses; dest under `adapter.destination`; no shadow | `Active` |
| Observed dest not in receipt | `Foreign` |

`status` prints the table from PRD §8 plus, per entry, the absolute source path re-derived from the registry (VER-1). The footer always carries the sentence: "Verified means discoverable under the target's documented scan rules. It does not mean the agent has read it." (VER-5, AC-28)

### 4.12 `reconcile`

`reconcile()` = `classify` plus a plan computed from desired against the current observed state, printed as divergences grouped by cause (hand-deleted, hand-modified, source moved, source unreachable). `--repair` applies that plan through the normal journaled executor, never deleting Foreign entries (STA-6, AC-34).

---

## 5. CLI specification

Global flags: `--project <path>`, `--target <id>`, `--json`, `--yes`, `--no-cache`, `-v/-q`.

| Command | Behavior | Exit |
|---|---|---|
| `unam` | Open the picker for the current project (from M6; before that, print status and usage). | 0 |
| `source add <alias> <path>` | Register, scan, report count and diagnostics. | 0 / 2 on invalid path |
| `source list` | Alias, path, kind, skill count, last scan. | 0 |
| `source relocate <alias> <path>` | Update path, repair dependent projects, re-verify. | 0 |
| `source rm <alias> [--force]` | Unregister. | 0 / 3 if referenced |
| `search <query> [--category p] [--source a]` | Ranked results with ID, name, category, alias, description head. | 0 |
| `show <ref>` | Full metadata, supporting files, absolute path, bundles referencing it, projects where active. | 0 / 4 not found / 5 ambiguous |
| `add <ref>...` | Resolve, plan, confirm, apply delta. | 0 / 5 / 6 conflicts / 7 apply failed |
| `rm <ref>...` | Plan, confirm, apply delta, print SES-2 caveat. | same |
| `use <bundle> --replace\|--add` | Required flag. Plan against bundle expansion. | same |
| `status` | Six-state table; `--json` emits full records. | 0; 8 if any Orphaned |
| `deactivate [--all]` | Remove everything tool-owned for the target; SES-2 caveat. | 0 |
| `bundle save <name>` | From current target selection, fully qualified refs. | 0 |
| `bundle ls \| show \| edit \| duplicate \| rm` | Manifest CRUD; `edit` opens `$EDITOR`. | 0 |
| `bundle add <bundle> <ref>...` / `bundle rm` | Edit `skills` in place. | 0 |
| `reconcile [--repair]` | Divergence report; repair via executor. | 0; 8 on divergence without repair |
| `rollback` | From journal if present, else from history. | 0 |
| `resume` | Finish an interrupted apply. | 0 |
| `doctor [--evidence]` | Symlink capability probe, destination writability, adapter detection, source reachability, receipt integrity, journal presence; with `--evidence`, print every adapter claim and URL. | 0; 9 if any check fails |

Exit codes: 0 ok, 1 generic, 2 bad argument, 3 referenced, 4 not found, 5 ambiguous, 6 blocked by conflicts, 7 apply failed, 8 divergence, 9 doctor failure, 10 interrupted (journal present), 11 locked (another `unam` process is mutating this project).

`--json` output is one object with `{ "ok": bool, "command": "...", "data": {...}, "warnings": [...], "errors": [...] }`. Schemas are documented in `docs/json.md` and covered by `insta` snapshots.

### Plan rendering (human)

```
Project   ~/code/acme-api  (git root)
Target    claude-code  ->  .claude/skills/        visible to: claude-code, cursor (compat)
Strategy  symlink

  + sdlc/code-review           skillzone   ->  .claude/skills/code-review
  + testing/pytest-fixtures    skillzone   ->  .claude/skills/pytest-fixtures
  = docs/api-reference         skillzone   (unchanged)
  - legacy/old-linter          skillzone   (remove, tool-owned)
  ~ backend-dev-brief          generated   (bundle instruction, unchanged)

  ! foreign   .claude/skills/house-style   (not managed, untouched)
  ! shadowed  code-review  by ~/.claude/skills/code-review  (personal scope wins)

Catalog estimate   4 skills, ~310 tokens (estimate)
After apply        no reload required  (directory watched; evidence: code.claude.com/docs/en/skills)
```

---

## 6. TUI specification

Entered by bare `unam`. `ratatui` immediate-mode, single-threaded, catalog loaded before first draw (warm cache target under 50 ms, cold scan runs with a spinner).

**Layout**: left pane 22 percent, category tree (collapsible, path-derived, acts as a filter); center 48 percent, search input on top, results list below; right 30 percent, preview of the focused skill; footer, pending change counts and catalog estimate.

**Keys**: type to filter; `↑↓` move; `space` toggle select; `tab` cycle panes; `enter` go to plan; `b` bundle actions (use, save); `t` switch target; `?` help; `esc` clear filter or quit.

**Markers**: `[x]` selected pending; `[●]` currently active; `[○]` active but pending removal; `[!]` conflict. Category rows show `3/12` meaning selected over total.

**Plan view**: identical content to the CLI plan rendering; `y` applies, `n` returns to picker, `e` toggles strategy for the target.

**Result view**: six-state list, refresh guidance, followups, SES-2 caveat when anything was removed; `q` quits, `enter` returns to picker.

**State machine**: `Loading → Picker → Plan → Applying → Result → Picker`. `Interrupted` is a modal shown before `Picker` when a journal exists, offering resume or rollback. The TUI never calls `fsops` directly; it calls `core::plan` and `core::apply`.

---

## 7. Error model

`unam-core` exposes one `enum Error` with `thiserror`, variants grouped by module. Every variant that a user can fix carries a `remedy: &'static str` surfaced by the CLI. Examples:

| Variant | Remedy text |
|---|---|
| `Resolve::Ambiguous { reference, candidates }` | "Qualify the reference, for example `skillzone:sdlc/code-review`." |
| `Plan::ForeignAtDest { dest }` | "Remove or rename the existing entry, or re-run with `--skip-conflicts`." |
| `Apply::Interrupted { journal }` | "Run `unam resume` to finish, or `unam rollback` to undo." |
| `Source::Unreachable { alias, path }` | "Mount the volume or run `unam source relocate <alias> <path>`." |
| `Apply::Locked { pid }` | "Another `unam` process is changing this project. Wait for it to finish, or close the open picker." |
| `Fs::SymlinkUnsupported` | "Use `--strategy copy` or set `defaults.strategy = \"copy\"`." |

Never panic on user data. `unwrap` is denied by clippy config outside tests.

---

## 8. Testing strategy

**Unit** (`unam-core`):
- frontmatter: valid, missing description, unparseable, colon-in-value retry, name/dir mismatch warning.
- resolve: each `SkillRef` form against a catalog with deliberate duplicate names across categories and sources.
- slugify: table-driven; a case marked `#[ignore]` until Hermes parity is confirmed.
- plan::diff: property test with `proptest`: for random desired and applied sets, `diff(d, apply(diff(d, a)))` yields zero steps (idempotency, AC-16); unchanged entries are carried forward byte-identical (AC-17).
- adapters: every adapter has evidence for `destination` and `scan_locations`; `refresh` never returns a guess for an adapter marked undocumented.

**Integration** (`crates/unam-cli/tests/`; the workspace root is virtual and has no package, so root-level `tests/` would not compile): fixtures build a fake source tree in a `tempfile` dir with nested categories, duplicate names, a broken-YAML skill, supporting files, and a nested `.git` to be skipped. A fake project with a pre-existing foreign skill and a pre-existing `~`-style user scan dir (pointed to by env override).

Each PRD acceptance criterion maps to one test, named `ac_NN_*`. Non-negotiable invariants asserted in every mutating test:
- `blake3` tree hash of the source fixture is identical before and after (AC-2, S3).
- Foreign fixture file is byte-identical after (AC-23, S4).

**Interruption** (`AC-31`, `AC-32`): a test implementation of `FsOps` errors on step N. Assert journal contents, `Interrupted` on the next command, and that both `resume` and `rollback` reach a consistent state verified by `reconcile` reporting zero divergence.

**CLI** (`assert_cmd`): every command's `--json` output snapshotted with `insta`; SES-2 caveat asserted present on `rm` and `deactivate` output (AC-30); exit codes per §5.

**Spike** (`spike/symlink-discovery.md`): not a test; a documented manual procedure with a results table. It gates the strategy default (§10, M0).

---

## 9. Non-functional targets

| Property | Target |
|---|---|
| Warm start to picker (500-skill catalog) | under 50 ms |
| Cold scan, 500 skills, local SSD | under 300 ms |
| Memory | under 50 MB resident |
| Network | none, ever |
| Background processes | none |
| Dependencies at runtime | none beyond libc; `git` optional |
| Binary | single static-ish binary via `cargo-dist`, macOS universal and Linux x86_64 and aarch64 |

---

## 10. Implementation plan

Milestones are sequenced so each ends in something usable. Each lists the acceptance criteria it closes. M0 to M5 are PRD Phase 1; M6 and M7 open PRD Phase 2, whose remaining scope (pinning, isolated adapters, `adopt`, bundle composition and export, remote sources) is not yet broken into milestones.

### M0: Gating spike (half day)

Procedure in `spike/symlink-discovery.md`:
1. Create a throwaway Git repo. Create two probe skills in a scratch source dir.
2. Symlink probe A into `.claude/skills/probe-a` and `.agents/skills/probe-a`; copy probe B into both.
3. Start Claude Code in the repo; ask it to list available skills. Record whether A and B appear and whether `/reload-skills` was needed.
4. Start Codex in the repo; same. Record whether a restart was needed.
5. If Cursor or Windsurf is installed, repeat; otherwise mark "not tested".
6. Fill the table: adapter × strategy → discovered, refresh needed.

Outcome: `defaults.strategy` per adapter is set from evidence. If symlinks fail for an adapter, that adapter's `strategies()` drops `Symlink` and the doc records why.

### M1: Core read path (2 to 3 days)

- Workspace, crates, CI (fmt, clippy, test), `cargo-dist` config.
- `config`, `source add|list|rm`, `catalog` scan with cache, `frontmatter`, `search`, `show`.
- Closes: AC-1, AC-3, AC-4, AC-5, AC-6, AC-38 (read-side).

### M2: Bundles and selection (2 days)

- `bundle` load/save/CRUD with Hermes compatibility and in-place edits.
- `project` root detection, `selection.yaml` IO, resolution to flat desired set with provenance.
- `resolve` with ambiguity errors.
- Closes: AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-35 (desired side).

### M3: Plan and apply for `claude-code` (3 to 4 days)

- `adapter` trait and `claude_code`; `fsops` with typed paths; `plan::diff`; `apply` with journal, receipt, history; `add`, `rm`, `use`, `deactivate`.
- SES-1 and SES-2 output.
- Closes: AC-13, AC-15, AC-16, AC-17, AC-18, AC-19, AC-21, AC-22, AC-23, AC-24, AC-25, AC-29, AC-30, AC-31, AC-32, AC-33, AC-40, AC-53.

### M4: Verify, status, reconcile, rollback (2 days)

- `verify` six-state classification and shadow analysis; `status`; `reconcile [--repair]`; `rollback` from history; `doctor`.
- Closes: AC-14, AC-26, AC-27, AC-28, AC-34, AC-36, AC-37, AC-39, AC-41.

### M5: `agents-neutral` adapter and generated briefs (1 to 2 days)

- Second adapter with shared-destination reporting, ancestor warnings, Hermes followup, cross-adapter duplicate check.
- `WriteGenerated` bundle brief with opt-out.
- Closes: AC-20, AC-42, AC-43, AC-24 (second adapter).

M5 completes PRD Phase 1. Phase 1 is done when every P0 and P1 acceptance criterion passes and the tool is in daily use; Phase 2 does not start before that.

### M6: TUI (3 days)

- Picker, plan view, result view, interrupted modal. Parity test: every TUI action calls a core function also reachable from the CLI.
- Closes: AC-44.

### M7: OSS readiness (1 to 2 days)

- README with the primary scenario as the first example; `docs/adapters.md` generated from `doctor --evidence`; `docs/json.md`; CHANGELOG; license; release workflow via `cargo-dist`; `cargo install` and Homebrew tap.

Total: roughly three working weeks of focused effort for one engineer, with the TUI as the largest discretionary item.

---

## 11. Decisions and trade-offs

| Decision | Chosen | Alternative considered | Why |
|---|---|---|---|
| Language | Rust | Python (Hermes parity, faster iteration) | Single static binary, no runtime on the machine, strong typing for the state model; the user chose it. Cost: slower first milestone and a TUI that is more work than Textual. |
| Bundle and selection format | YAML | TOML, JSON | Hermes bundle files must load unchanged; a project selection should look like a bundle. Cost: Rust YAML serializers do not preserve comments; mitigated by line-oriented edits of the `skills:` block. |
| Machine state format | JSON | YAML, SQLite | Receipt and journal are written by the tool, read by the tool, and occasionally inspected by a human. JSON is unambiguous and `serde_json` is exact. SQLite adds a dependency for a file that rarely exceeds a few kilobytes. |
| Default strategy | Symlink, pending M0 | Copy | Only mechanism that satisfies "one canonical copy" with supporting files intact and live edits. Cost: undocumented agent behavior, no pinning; copy exists as fallback. |
| Desired state location | In project, `.unam/selection.yaml` | Central store keyed by path | Survives project moves, is diffable and shareable. Cost: a directory in the project; mitigated by gitignore suggestions. |
| Skill identity | `alias:rel/path` | Frontmatter `name` | Names collide across categories by design in a real library. Cost: longer identifiers; mitigated by bare-name sugar when unambiguous. |
| Ambiguity handling | Error with candidates | Precedence-based pick | A wrong silent pick mis-provisions a project for days. Cost: one extra keystroke in rare cases. |
| Bundle instruction | Generated brief skill, opt-out | Display only; write into agent context file | Only option that is discoverable without writing files the tool does not own. Cost: discoverable is not the same as read; stated in output. |
| Git integration | Shell out, optional | `gix` or `git2` | No native dependency for v1; commit SHA is only needed for pin mode (v2). |
| Project index | Cache, not source of truth | Authoritative registry | A stale index can only make `list --all` incomplete, never make a project wrong. |

### What to revisit as the system grows

- **Phase 3, intent-driven exposure,** reads the skill metadata reserved in §2.6 and adds the atlas, the router, and hook commands to the same binary. Its technical decisions are in ADR-0001, and it is gated by PRD decision D8.
- **Pin mode** (v2) turns the receipt into a reproducibility record and needs `gix` or `git2` for `git show` snapshots without a worktree.
- **Isolated destination policy** needs `cursor` and `windsurf-devin` adapters writing to vendor directories; the trait already supports it, the evidence table does not yet.
- **Bundle composition** (bundles referencing bundles) needs cycle detection in `bundle` and provenance depth in `DesiredEntry`.
- **Team use** of committed selections needs alias indirection: a committed `selection.yaml` referencing `skillzone:` must map to the cloner's own alias. A `[aliases]` section in `config.toml` mapping foreign aliases to local ones is the likely shape.
- **Native Hermes adapter** emitting bundle manifests into `$HERMES_BUNDLES_DIR` would close the loop with the reference model and is small once `slugify` parity is confirmed.
- **Windows**: `fsops::symlink_dir` becomes `symlink_dir` on Windows with a capability probe in `doctor`; `copy` becomes the default there.

---

## 12. Kickoff

Start with M0; do not write application code before the spike table exists. Then:

```
cargo new --lib crates/unam-core
cargo new --bin crates/unam-cli --name unam
# workspace Cargo.toml listing both; resolver = "3" (the edition 2024 default, which a virtual workspace must set explicitly)
cargo add -p unam-core serde --features derive
cargo add -p unam-core serde_norway serde_json toml thiserror blake3 ignore directories tempfile tracing nucleo jiff fs4
cargo add -p unam clap --features derive
cargo add -p unam ratatui crossterm anyhow tracing-subscriber
cargo add -p unam --path crates/unam-core
cargo add -p unam-core --dev proptest insta
cargo add -p unam --dev assert_cmd predicates tempfile insta
```

First test to write: `ac_02_source_tree_unchanged`, because it is the invariant everything else must preserve.
