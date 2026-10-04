# Phase Blast Radius Registry: v1-static-activation

Append-only. One section per toolkit phase (one milestone each). Phases run sequentially, so overlaps are ownership hand-offs, not conflicts.

## Phase 1: milestone M0
- `spike/symlink-discovery.md` (results and Outcome; filled in by Taylor)

## Phase 2: milestone M1
- `Cargo.toml`, `Cargo.lock`, `crates/unam-core/**`, `crates/unam-cli/**` (created), CI workflow, `cargo-dist` config, clippy config

## Phase 3: milestone M2
- `crates/unam-core/src/{bundle,project,resolve}.rs`; `crates/unam-cli/src/cli/` bundle commands; `crates/unam-cli/tests/`

## Phase 4: milestone M3
- `crates/unam-core/src/{adapter/mod,adapter/claude_code,fsops,plan,apply,state,hash}.rs`; `crates/unam-cli/src/cli/` add, rm, use, deactivate; `output.rs`; `crates/unam-cli/tests/`

## Phase 5: milestone M4
- `crates/unam-core/src/{verify,reconcile}.rs`; additions to `apply.rs`, `source.rs`, `adapter/claude_code.rs`; CLI status, reconcile, rollback, resume, doctor, source relocate; `crates/unam-cli/tests/`

## Phase 6: milestone M5
- `crates/unam-core/src/adapter/agents_neutral.rs`; additions to `adapter/mod.rs`, `plan.rs`, `apply.rs`, `verify.rs`; CLI output; `crates/unam-cli/tests/`

## Phase 7: milestone M6
- `crates/unam-cli/src/tui/**`, `crates/unam-cli/src/main.rs`; `crates/unam-cli/tests/`

## Phase 8: milestone M7
- `README.md`, `CHANGELOG.md`, license, release workflow, `docs/adapters.md` and `docs/json.md` (permission-gated), package metadata
