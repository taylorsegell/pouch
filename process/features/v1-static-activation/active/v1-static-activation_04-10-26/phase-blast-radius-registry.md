# Phase Blast Radius Registry: v1-static-activation

Append-only. One section per toolkit phase (one milestone each). Phases run sequentially, so overlaps are ownership hand-offs, not conflicts.

## Phase 1: milestone M0
- `spike/symlink-discovery.md` (results and Outcome; filled in by Taylor)
- status: DONE

## Phase 2: milestone M1
- `Cargo.toml`, `Cargo.lock`, `crates/unam-core/**`, `crates/unam-cli/**` (created), CI workflow, `cargo-dist` config, clippy config
- status: DONE

## Phase 3: milestone M2
- `docs/adapters.md` (created if absent; Hermes slugify outcome; extended by M4 and M7); `crates/unam-core/src/{bundle,project,resolve}.rs`; `crates/unam-cli/src/cli/` bundle commands; `crates/unam-cli/tests/`
- Additions at PLAN-SUPPLEMENT 2026-10-05 (the modules are directories): created `crates/unam-core/src/bundle/{mod,slug,word_table,edit}.rs`, `crates/unam-core/src/project/{mod,selection,index}.rs`, `crates/unam-core/src/resolve.rs`, `crates/unam-cli/src/cli/bundle.rs`, `crates/unam-cli/tests/{ac_bundles,ac_selection,bundle_cli}.rs`, `crates/unam-cli/tests/fixtures/{bundles,library2}/**`, nine new `crates/unam-cli/tests/snapshots/snapshots__bundle_*.snap`, `docs/adapters.md`, and the Pre-Step record `phase-03-m2-pre-step_REF_<dd-mm-yy>.md` in the program task folder
- Additions at PVL cycle 1 (2026-10-05): created `crates/unam-core/tools/gen-word-table.py` (committed word-table generator and checker) and `crates/unam-core/tools/rust-word-ranges.txt` (committed Rust-side data file); neither is a cargo source
- Additions (modified M1 files): `crates/unam-core/src/{lib,error,paths,source}.rs`, `crates/unam-core/src/catalog/{mod,lookup}.rs` (candidate ordering, `canonical_ref_text`, `diag_class`), `crates/unam-cli/src/cli/{mod,show}.rs`, `crates/unam-cli/tests/{common/mod,snapshots,show_cli,usage}.rs`, and the regenerated snapshots `snapshots__show_found.snap` and `snapshots__usage_bare.snap`
- Read-only verification (must stay green unmodified): `crates/unam-cli/tests/{source_cli,ac_read_path,read_side_invariant,search_cli,cache_cli}.rs`
- Not touched: `fsops.rs`, `hash.rs`, `config.rs`, `catalog/{scan,cache,frontmatter,search}.rs`, `output.rs`, `main.rs`, the three `Cargo.toml` files, `Cargo.lock`
- Overlap note: M2 takes over the provisional `projects.json` reader from M1 (`source.rs`) into `project/index.rs` and supersedes the M1 temporary `lookup` role with `resolve.rs` (the code stays as the primitive); M3 extends `project/`, `resolve.rs`, and `docs/adapters.md` is extended by M4
- status: DONE (2026-10-05; also created `crates/unam-core/src/bundle/crud.rs`; modified M1 test `crates/unam-cli/tests/show_cli.rs`)

## Phase 4: milestone M3
- `crates/unam-core/src/{adapter/mod,adapter/claude_code,fsops,plan,apply,state,hash}.rs`; `crates/unam-cli/src/cli/` add, rm, use, deactivate; `output.rs`; `crates/unam-cli/tests/`
- Additions at PLAN-SUPPLEMENT 2026-10-05 (HIGH-RISK, destructive writes): created `crates/unam-core/src/{observe,change}.rs` (with the already listed adapter, plan, apply, state); `fsops.rs` is rewritten; `hash.rs` is reused unchanged (not modified); created `crates/unam-cli/src/cli/change.rs` (the six commands add, rm, use, deactivate, resume, rollback); created `crates/unam-cli/tests/{ac_activation,ac_recovery,activation_cli}.rs`, `crates/unam-cli/tests/common/faults.rs`, and twelve new `crates/unam-cli/tests/snapshots/snapshots__*.snap`; created the Pre-Step record `phase-04-m3-pre-step_REF_<dd-mm-yy>.md` and the evidence pack `harness/{risk-gate,context-snippets,verification,adversarial-validation}.json` in the program task folder (`review-decision.json` is left for Taylor); created the backlog NOTE `process/features/v1-static-activation/backlog/m3-d4-first-real-activation_NOTE_<dd-mm-yy>.md`
- Additions (modified M1 and M2 files): `Cargo.toml`, `crates/unam-core/Cargo.toml`, `crates/unam-cli/Cargo.toml` (one `rust-version` line each; `Cargo.lock` unchanged); `crates/unam-core/src/{lib,error,paths,source,resolve}.rs`; `crates/unam-core/src/catalog/mod.rs` (`Catalog::source_path`, `SkillId::parse`, a re-export); `crates/unam-core/src/{config.rs,catalog/cache.rs,project/index.rs,bundle/crud.rs}` (the `write_atomic` caller migration to `ToolPath` only, no behavior change); `crates/unam-core/src/project/selection.rs` (`StatePath`, `GITIGNORE_LINES`, offer rule); `crates/unam-cli/src/{output.rs,cli/mod.rs,cli/bundle.rs}`; tests `crates/unam-cli/tests/{common/mod.rs,snapshots.rs,usage.rs,bundle_cli.rs,ac_bundles.rs,ac_selection.rs}` and the regenerated snapshot `snapshots__usage_bare.snap`
- Read-only verification (must stay green unmodified): `crates/unam-cli/tests/{source_cli,ac_read_path,read_side_invariant,search_cli,cache_cli,show_cli}.rs`, the other nineteen existing snapshots
- Not touched: `docs/**` (including `docs/adapters.md`, left to M4), `spike/`, `Cargo.lock`, `.github/`, `dist-workspace.toml`, `.claude/`, `.codex/`, `.agents/`, `hash.rs`, `catalog/{scan,frontmatter,search,lookup}.rs` (except the one re-export), `bundle/` logic, `main.rs`
- Overlap note: M3 takes over the five M1 and M2 `write_atomic` callers only to change the path type; M3 extends `project/` (offer rule, `GITIGNORE_LINES`) and `resolve.rs` (`Provenance` serde); M4 extends `observe.rs` into `verify`, `apply.rs`, and `adapter/claude_code.rs` and records the claude-code evidence in `docs/adapters.md`; M4 writes its evidence pack in `harness/m4/`
- Additions at PVL cycle 1 supplement (2026-10-05): no new files and no new directories; the same files gain the following content only: `crates/unam-core/src/error.rs` (20 variants: `StateDirInvalid`, `ProjectOverlapsSource`, `DestRootInvalid` added), `fsops.rs` (`ObservedKind` and `observe_one` defined here, lock lstat refusal and identity check, `dest_prefixes`, `.DS_Store` manifest rule), `plan.rs` (`StepOutcome`, `StepResult`, `Step::RemoveDir`), `state.rs` (`JournalStep`, `check_state_layout`, receipt and journal whole-file checks), `apply.rs` and `change.rs` (destination-root re-check, project-versus-source check, recovery details), `project/selection.rs` (selection write through `FsOps`), and the tests `ac_recovery.rs` (13 tests), `activation_cli.rs` (36 tests); the evidence pack gains adversarial scenarios for G1, G2, G7, G9
- status: DONE (2026-10-09; EVL G1 to G19 green; VERIFIED WITH_GAPS)

## Phase 5: milestone M4
- `docs/adapters.md` (create if absent, otherwise extend); `crates/unam-core/src/{verify,reconcile}.rs`; additions to `apply.rs`, `source.rs`, `adapter/claude_code.rs`; CLI status, reconcile, rollback, resume, doctor, source relocate; `crates/unam-cli/tests/`

## Phase 6: milestone M5
- `crates/unam-core/src/adapter/agents_neutral.rs`; additions to `adapter/mod.rs`, `plan.rs`, `apply.rs`, `verify.rs`; CLI output; `crates/unam-cli/tests/`

## Phase 7: milestone M6
- `crates/unam-cli/src/tui/**`, `crates/unam-cli/src/main.rs`; `crates/unam-cli/tests/`

## Phase 8: milestone M7
- `README.md`, `CHANGELOG.md`, license, release workflow, `docs/adapters.md` and `docs/json.md` (permission-gated), package metadata
