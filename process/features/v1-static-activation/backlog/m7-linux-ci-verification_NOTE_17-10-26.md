---
name: note:m7-linux-ci-verification
description: "Backlog note from M7: Linux, continuous integration, the 1.89 minimum toolchain, the first real release run, and the Homebrew tap were never exercised; the Docker daemon was down during M7 EXECUTE"
date: 17-10-26
feature: v1-static-activation
---

# M7 follow-up: Linux and CI verification (owner: Taylor, after pushing)

Priority: medium. Everything below is a named residual of milestone M7. It keeps the M7 closeout at VERIFIED (WITH_GAPS); none of it blocks the local gates.

## What was not exercised

| Residual | Why it is open | How to close it |
|---|---|---|
| `cargo test --workspace` on Linux | The Docker daemon was not running during M7 EXECUTE (`docker image ls` could not connect to `/var/run/docker.sock`). No daemon was started and no container was touched. | After pushing `main`, read the `ubuntu` job of `.github/workflows/ci.yml`. Or, with Docker running and a local `rust` image, copy tracked files (`git ls-files -z`) into a temp directory and run `cargo test --workspace --locked --offline` in a container started with `--network none`. `cargo fetch --locked --offline --target x86_64-unknown-linux-gnu` exited 0 at PVL, so the registry cache holds the Linux-only crates. |
| `.github/workflows/ci.yml` | Written in M1, never run (no push). | Push `main` and watch the first run. Expect to fix small things (toolchain pin, cache keys). |
| `rust-version = "1.89"` | Only the local toolchain (rustc 1.99) ever compiled the code. | Run `cargo +1.89 check --workspace --locked`, or add a CI job on 1.89. |
| Release workflow | `.github/workflows/release.yml` was generated and checked with `dist plan` and `dist generate --check`; it has never run. A real run needs a tag push. | Follow the release handoff in `phase-08-m7-oss-readiness_REPORT_17-10-26.md` ("For Taylor"). |
| Homebrew tap | The tap name `taylorsegell/homebrew-tap` is a guess from the repository owner and the repository does not exist. | Create the repository, then decide whether to add `publish-jobs = ["homebrew"]` and a tap token. |
| macOS universal binary | cargo-dist 0.32.0 rejects `universal2-apple-darwin` and `universal-apple-darwin` ("Unrecognized architecture"). Two separate macOS archives are built instead. | Revisit when a newer cargo-dist supports it, if wanted. |

## Why it matters

The README says "Linux: not yet verified". That sentence stays until one of the first two rows is closed and a Linux run is green.
