# M4 deletion-safety evidence (wave 4 step 1)

Date: 12-10-26. Scope: does `reconcile --repair`, or any other M4 path, delete anything that is not a receipted entry (foreign file, or anything under the source library)?

## Finding

`reconcile.rs`, `verify.rs`, and `doctor.rs` contain no production deletion. Reporting is read-only. `--repair` builds a plan via `change::prepare` and applies it through the M3 apply engine, so every delete goes through the same guarded functions.

## Deletion paths

| Path | Guard (file:line) | Covering test |
|---|---|---|
| Repair Remove steps (repoint, strategy change, deselect) | `plan.rs:589` and `:696` emit Remove only when `is_owned` (`plan.rs:425`: symlink target equals receipt source, or copy tree hash equals receipt hash); else Conflict (`plan.rs:653`) or DropOrphan (receipt-only, `plan.rs:705`) | `evidence_repair_never_touches_foreign`, `evidence_repair_never_overwrites_modified_entries`, `evidence_deselected_modified_entry_is_dropped_from_receipt_only` (ac_reconcile) |
| Apply `remove` | `apply.rs:477` takes a `DestPath` from `join_child` (`fsops.rs:111`: single validated component under a destination root) and calls `remove_owned` with the receipt expectation | `remove_owned_*` (fsops tests), `ac_recovery` fault matrix |
| `remove_owned_at` symlink | `fsops.rs:998-1013`: foreign kind skipped, target mismatch skipped, re-observed, only then `remove_file` on the link itself | `remove_owned_symlink_rows` |
| `remove_owned_at` copy | `fsops.rs:1014-1053`: type, hash, re-hash after rename-aside; mismatch moves back and skips; deletes only manifest content (`delete_manifest`, `fsops.rs:1060`), leaves `.git` and similar as residue | `remove_owned_copy_*` (7 tests) |
| `RemoveDir` (created dirs) | `plan.rs:735-750` only for receipt-claimed prefixes and only when no entries remain; `apply.rs:506` resolves via `ancestor`; `fsops.rs:603` `rmdir_if_empty` uses non-recursive `remove_dir` | `remove_dir_all_is_confined`, activation tests |
| State removals (lock, journal, trash) | `fsops.rs:549`, `:559` take `StatePath`, non-recursive | ac_recovery |
| `remove_own_temp_dir` | `fsops.rs:937`: name must start with `TEMP_PREFIX`, lstat must show a real dir; the only `remove_dir_all` in production | `remove_dir_all_is_confined` |
| `source rm` / relocate cache delete | `source.rs:164` and `:400`: single file under `<cache dir>/catalog/<validated alias>`; the library is never opened for write | `source_rm_counts_bundle_members`, `evidence_relocate_to_incomplete_library_reports_and_deletes_nothing` |
| Source library | No write function accepts `SourcePath`; `SourcePath` is only passed to `symlink_dir` and `copy_tree` as the read side (`apply.rs:460-465`) | `read_side_invariant`, `evidence_relocate_*` |

## Commands run

All from the repo root, with `source ~/.cargo/env &&`:

| Command | Result |
|---|---|
| `cargo test --workspace --locked --test ac_reconcile` | 12 passed, 0 failed |
| `cargo test --workspace --locked --test ac_verify` | 7 passed, 0 failed |
| `cargo test --workspace --locked --test ac_recovery` | 14 passed, 0 failed |
| `cargo test --workspace --locked --test ac_activation` | 17 passed, 0 failed |
| `cargo test -p unam-core --locked --lib -- remove_owned remove_dir_all_is_confined reconcile:: relocate source::tests` | 25 passed, 0 failed |

## Verdict

PASS. Every M4 deletion is gated by a receipt ownership check (hash or link target) on a typed `DestPath`; foreign and modified entries are skipped or dropped from the receipt only; the source library has no write path. No gap blocks committing M4.

Notes (non-blocking): line numbers are as of the uncommitted working tree. Tests were not re-read line by line for assertions; the names and pass counts above are the evidence, alongside the guard code read directly.
