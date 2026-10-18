# The `--json` output

Every `unam` command prints one JSON object on stdout when you pass `--json`, and nothing else on stdout. Human text, logs, and prompts go to stderr or are suppressed. This page documents the shape of each command's output. It was checked against the real output: `crates/unam-cli/tests/snapshots/` holds one recorded output per case, and the test `docs_json_every_snapshot_is_documented` fails when a recorded output has a `data` key that this page does not mention. The prose (types and meanings) is hand-written; the test checks coverage of keys, not the wording.

Paths in the recorded outputs are replaced by placeholders (`[TMP]`, `[OP_ID]`); real output has real paths and operation ids. Each section below begins with a marker comment naming the recorded case it covers. Several cases can share one section when they share a shape.

## Envelope

Every `--json` output has these five top-level keys, in this order.

| Key | Type | Meaning |
|---|---|---|
| `ok` | boolean | `true` when the command succeeded. A command that finds a problem it reports (for example `status` finding an orphaned entry) can print `ok: false` with a full `data` object. |
| `command` | string | The command that ran, for example `source add`, `bundle show`, `add`. |
| `data` | object | The command's result. Its keys are documented per command below. An error with no result has an empty object. |
| `warnings` | array | Diagnostics of severity warning or error that did not stop the command. |
| `errors` | array | Errors that made the command fail. Empty on success. |

A diagnostic in `warnings` (and in `diagnostics` arrays inside `data`) is an object: `source` (alias), `skill` (the skill id, when it concerns one skill), `code` (a stable kebab-case code), `severity` (`info`, `warning`, or `error`), `message`, `path` (a file path or null), and `remedy` (a suggested fix or null).

An entry in `errors` is an object: `code` (stable), `message`, `remedy` (what to do about it, or null), and, for an ambiguous skill reference, `candidates` (the full ids that matched).

Exit codes: 0 ok, 1 generic failure, 2 bad argument, 3 source referenced by a project, 4 not found, 5 ambiguous reference, 6 blocked by conflicts, 7 apply failed, 8 destination diverges from the selection, 9 doctor found a failure, 10 an earlier apply was interrupted, 11 another `unam` process holds the project lock. The exit code and the `errors[0].code` always agree.

## `unam source add --json`

<!-- snap: add_success -->

Registers a source directory and scans it.

| Key | Type | Meaning |
|---|---|---|
| `alias` | string | The source alias. |
| `path` | string | The path as given (it may contain `~` or `$VAR`). |
| `expanded_path` | string | The path after expansion. |
| `kind` | string | `directory`, or `git-repo` when the source root directly contains a `.git` entry. |
| `precedence` | number | Lower numbers win when two sources define the same skill name. |
| `skill_count` | number | Skills found by the scan. |
| `diagnostics` | array | Scan diagnostics for this source, including notes. Warnings among them are repeated in the envelope `warnings`. |

<!-- snap: add_bad_path -->

A path that does not exist or is not readable fails with exit 2, `ok: false`, an empty `data`, and one entry in `errors` with code `source-path-invalid`.

## `unam source list --json`

<!-- snap: list -->

| Key | Type | Meaning |
|---|---|---|
| `sources` | array | One object per registered source: `alias`, `path`, `expanded_path`, `kind`, `precedence`, `reachable` (boolean), `skill_count`, `last_scan` (when the catalog was last scanned), and `diagnostics`. |

## `unam source rm --json`

<!-- snap: rm_success -->

| Key | Type | Meaning |
|---|---|---|
| `alias` | string | The source that was unregistered. |
| `removed` | boolean | `true` when it was removed. |
| `forced` | boolean | `true` when `--force` was needed because projects still reference it. |
| `dangling_projects` | array | Project roots whose selections now reference a source that no longer exists. |

<!-- snap: rm_referenced -->

Without `--force`, a source that a project still references is not removed: exit 3, empty `data`, `errors[0].code` is `source-referenced`.

## `unam source relocate --json`

<!-- snap: source_relocate_json -->

Points a source at a new path and repairs the projects that use it.

| Key | Type | Meaning |
|---|---|---|
| `alias` | string | The source. |
| `old_path` | string | The path before the change. |
| `path` | string | The new path as given. |
| `expanded_path` | string | The new path after expansion. |
| `status` | string | `relocated` when the registry was changed. |
| `projects` | array | One object per project that used the source: `root`, `target`, `status` (for example `repaired`), `plan`, `results`, and `error` (null on success). `plan` and `results` have the shapes described under the apply commands. |

## `unam search --json`

<!-- snap: search_review -->

| Key | Type | Meaning |
|---|---|---|
| `query` | string | The search words. |
| `category` | string or null | The `--category` filter. |
| `source` | string or null | The `--source` filter. |
| `total` | number | Number of matches. |
| `results` | array | One object per match, best first: `id`, `name`, `category`, `source`, `description`, `score` (relevance, higher is better), and `diagnostics`. |

## `unam show --json`

<!-- snap: show_found -->

| Key | Type | Meaning |
|---|---|---|
| `id` | string | The skill id, `alias:relative/path`. |
| `name` | string | The skill name. |
| `description` | string | The description from the front matter. |
| `category` | string | The directory path above the skill, empty at the source root. |
| `source` | string | The source alias. |
| `rel_path` | string | The path inside the source. |
| `path` | string | The absolute path of the skill directory. |
| `frontmatter` | object | The whole front matter, unknown keys kept. |
| `supporting` | array | Files in the skill directory besides `SKILL.md`, relative paths. |
| `tree_hash` | string | A `blake3:` hash of the skill directory. |
| `bundles` | array | Bundles that list this skill. |
| `diagnostics` | array | Diagnostics for this skill. |

<!-- snap: show_ambiguous -->

A bare name that matches more than one skill fails with exit 5, empty `data`, and `errors[0].candidates` listing the full ids. Nothing is picked silently.

<!-- snap: show_not_found -->

An unknown reference fails with exit 4, empty `data`, and `errors[0].code` `not-found`.

## `unam bundle save --json`

<!-- snap: bundle_save -->

Saves the project's current selection as a bundle.

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The file stem of the bundle. |
| `name` | string | The bundle name as written. |
| `path` | string | The bundle file. |
| `project_root` | string | The project the selection came from. |
| `target` | string | The adapter whose selection was saved. |
| `skill_count` | number | Entries saved. |
| `skills` | array | The saved entries, as skill ids. |
| `overwritten` | boolean | `true` when `--force` replaced an existing bundle. |

## `unam bundle ls --json`

<!-- snap: bundle_ls -->

| Key | Type | Meaning |
|---|---|---|
| `bundles` | array | One object per bundle: `slug`, `name`, `description`, `skill_count`, `has_instruction` (boolean), and `path`. |
| `diagnostics` | array | Problems reading bundle files, such as a duplicate slug. |

## `unam bundle show --json`

<!-- snap: bundle_show -->

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The bundle slug. |
| `name` | string | The bundle name. |
| `description` | string | The description. |
| `instruction` | string or null | The instruction text, when set. |
| `path` | string | The bundle file. |
| `skills` | array | One object per entry: `entry` (as written), `status` (`resolved`, `ambiguous`, or `unresolved`), `id` (the resolved skill id or null), and `candidates` (ids, for an ambiguous entry). |
| `problems` | number | Entries that did not resolve. |

<!-- snap: bundle_show_not_found -->

An unknown bundle fails with exit 4, empty `data`, and `errors[0].code` `bundle-not-found`.

## `unam bundle edit --json`

<!-- snap: bundle_edit -->

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The bundle. |
| `path` | string | The bundle file. |
| `changed` | boolean | `true` when the editor changed the file and the result was valid. |
| `problems` | number | Entries that did not resolve after the edit. |

## `unam bundle duplicate --json`

<!-- snap: bundle_duplicate -->

| Key | Type | Meaning |
|---|---|---|
| `source` | string | The slug that was copied. |
| `slug` | string | The new slug. |
| `name` | string | The new name. |
| `path` | string | The new file. |
| `overwritten` | boolean | `true` when `--force` replaced an existing bundle. |

## `unam bundle rm --json`

<!-- snap: bundle_rm_bundle -->

With no references, the bundle is deleted.

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The bundle. |
| `path` | string | The bundle file. |
| `deleted` | boolean | `true` when the file was removed. |
| `referencing_projects` | array | Project roots whose selections name this bundle. |

<!-- snap: bundle_rm_skill -->

With references, the entries are removed from the bundle.

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The bundle. |
| `path` | string | The bundle file. |
| `removed` | array | The entries that were removed. |
| `rewrite` | boolean | `true` when the whole file was rewritten (which drops comments), `false` when only the affected lines were edited. |
| `skill_count` | number | Entries left. |

## `unam bundle add --json`

<!-- snap: bundle_add -->

| Key | Type | Meaning |
|---|---|---|
| `slug` | string | The bundle. |
| `path` | string | The bundle file. |
| `added` | array | The entries that were added. |
| `skipped` | array | Entries already present. |
| `rewrite` | boolean | `true` when the whole file was rewritten, `false` when lines were edited in place. |
| `skill_count` | number | Entries after the change. |

## `unam add`, `rm`, `use`, `deactivate`, `resume`, `rollback` with `--json`

These six commands plan a change, apply it, and print the same `data` shape. The recorded cases below differ in `status` and in which keys are null or empty.

<!-- snap: add_apply -->
<!-- snap: add_brief_json -->
<!-- snap: add_conflict -->
<!-- snap: add_no_changes -->
<!-- snap: apply_partial_failure -->
<!-- snap: deactivate_apply -->
<!-- snap: plan_agents_neutral_json -->
<!-- snap: refused_without_yes -->
<!-- snap: resume_apply -->
<!-- snap: rm_apply -->
<!-- snap: rollback_apply -->
<!-- snap: rollback_history_json -->
<!-- snap: use_replace -->

| Key | Type | Meaning |
|---|---|---|
| `command` | string | `add`, `rm`, `use`, `deactivate`, `resume`, or `rollback`. |
| `project_root` | string | The project that was changed. |
| `root_detection` | string | `git` when the root is the nearest directory with a `.git` entry, `explicit` when `--project` or the working directory was used. |
| `target` | string | The adapter: `claude-code` or `agents-neutral`. |
| `destination` | string | Where skills are placed, relative to the project, for example `.claude/skills`. |
| `strategy` | string or null | `symlink` or `copy`; null for `resume`, which continues an earlier plan. |
| `detection` | string | Whether the project already uses the target's agent: `present`, `absent`, or `unknown`. |
| `status` | string | `applied` (everything done), `no-changes` (nothing to do), `blocked` (conflicts), `refused` (a confirmation was needed and `--yes` was not given), or `partial` (some steps failed). |
| `op_id` | string or null | The operation id of the apply; null when nothing was applied. |
| `selection` | object or null | The project selection after the command: `written` (boolean), `path`, `bundles`, `add`, `remove`. Null for `resume` and `rollback`. |
| `plan` | object or null | The plan: `steps` (each with `n`, `op` as `add` or `remove`, `id`, `name`, `dest`, `strategy`, and for an add `source` and `provenance`), `unchanged`, `carried`, `conflicts` (each with `code`, `message`, `dest`, `found`, `source`, `id`), `warnings`, and `estimate` (`skills`, `tokens`, and `estimate: true`, since the token count is approximate). |
| `results` | array | One object per executed step: `n`, `op`, `name`, `result` (`done`, `failed`, or `skipped`), `code`, `message`. |
| `receipt` | object or null | The record of what unam placed: `path`, `entries`, `last_op_id`. Null when nothing was applied. |
| `refresh` | object or null | What to do so a running agent notices the change: `kind`, `command`, `text`, `evidence` (the source of the claim). |
| `followups` | array | Extra suggested commands, such as trusting a repository for an agent that needs it. unam never runs them. |
| `removal_caveat` | string or null | Set when a removal was applied: a conversation that already loaded the instructions keeps them until it ends. Null otherwise. |
| `gitignore_lines` | array | Lines unam suggests adding to `.gitignore`. |
| `visible_to` | array | The agents that read the destination. |
| `recovered` | object | Present only on `resume` and `rollback`: `op_id` of the interrupted operation and `action` (`resume` or `rollback`). |

Failures keep the same shape: `blocked` exits 6, `refused` exits 2, `partial` exits 7, and `errors[0].code` names the cause (`blocked-by-conflicts`, `confirmation-required`, `apply-failed`).

## Interrupted and locked failures

<!-- snap: interrupted_exit_10 -->

An apply that was cut off leaves a journal. The next change refuses with exit 10 until you run `unam resume` or `unam rollback`.

| Key | Type | Meaning |
|---|---|---|
| `interrupted` | object | The journal: `op_id`, `command`, `target`, `started_at`, `receipt_written` (boolean), and `steps` (each with `n`, `op`, `name`, and `state` such as `started` or `done`). |
| `next` | array | The commands to run, for example `unam resume`. |

<!-- snap: locked_exit_11 -->

| Key | Type | Meaning |
|---|---|---|
| `locked` | object | The lock holder: `pid`, the process id of the other `unam`. Exit 11. |

## `unam status --json`

<!-- snap: status_json -->
<!-- snap: status_agents_neutral_json -->

Shows the six-state table for the target. `ok` is `false` and the exit code is 8 when an entry is orphaned.

| Key | Type | Meaning |
|---|---|---|
| `command` | string | `status`. |
| `project_root` | string | The project. |
| `root_detection` | string | `git` or `explicit`. |
| `target` | string | The adapter. |
| `destination` | string | The destination relative to the project. |
| `records` | array | One object per entry: `id`, `name`, `category`, `source_path`, `dest`, `strategy`, `provenance` (why it is selected: `bundle`, `adhoc`, and so on), `state`, and `reason`. `state` is one of `selected`, `applied`, `active`, `shadowed`, `foreign`, `orphaned`. `reason` explains an orphaned entry (`missing`, `broken-link`, `modified`, `source-skill-missing`) and is null otherwise. |
| `summary` | object | A count per state: `selected`, `applied`, `active`, `shadowed`, `foreign`, `orphaned`. |
| `footer` | string | A note on what "verified" means: discoverable under the target's documented scan rules, not proven to be loaded by a running agent. |

## `unam reconcile --json`

<!-- snap: reconcile_json -->

Reports where the destination diverges from the selection; `--repair` applies the plan.

| Key | Type | Meaning |
|---|---|---|
| `command` | string | `reconcile`. |
| `project_root` | string | The project. |
| `root_detection` | string | `git` or `explicit`. |
| `target` | string | The adapter. |
| `destination` | string | The destination relative to the project. |
| `records` | array | The same entries as in `status`. |
| `summary` | object | The same counts as in `status`. |
| `footer` | string | The same note as in `status`. |
| `divergences` | array | One object per divergence: `cause` (for example `hand-deleted`), `id`, `name`, `dest`, `detail`. |
| `findings` | array | Things unam notices but never touches: `kind` (for example `foreign`), `name`, `dest`, `detail`. |
| `plan` | object | The repair plan, with the shape described under the apply commands. |
| `repair_blocked` | string or null | The reason a repair cannot run (for example conflicts), or null when the plan can be applied. |

## `unam doctor --json`

<!-- snap: doctor_json -->

| Key | Type | Meaning |
|---|---|---|
| `command` | string | `doctor`. |
| `project_root` | string | The project. |
| `root_detection` | string | `git` or `explicit`. |
| `target` | string | The adapter. |
| `destination` | string | The destination relative to the project. |
| `checks` | array | One object per check: `id`, `status` (`ok`, `info`, or `fail`), `message`, and `remedy` (or null). Any `fail` makes the exit code 9. |
| `evidence` | array or null | With `--evidence`: one object per adapter claim, with `adapter`, `claim_key`, `status` (`verified` or `assumption`), `source` (a URL or a spike file), and `claim`. Null without the flag. |
