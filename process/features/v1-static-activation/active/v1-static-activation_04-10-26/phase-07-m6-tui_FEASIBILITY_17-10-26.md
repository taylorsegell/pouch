---
slug: phase-07-m6-tui
date: 2026-10-17
verdict: VIABLE
originating-phase: innovate
---

# Feasibility verdict: milestone M6 TUI without mio or socket2

**Bottom line (read this first):** The hypothesis is VIABLE, but not by the route Taylor chose. "Turn off crossterm events" is NOT achievable through ratatui: the ratatui crossterm backend always switches crossterm's `events` feature back on, and `events` is what pulls `mio`. The route that works is the ratatui **termion backend**: zero `mio`, zero `socket2`, zero `signal-hook`, single-threaded, blocking key reads, and the existing `no_network_dependencies` test passes unchanged against it. This departs from spec section 1.4 (`ratatui` + `crossterm`), so it needs Taylor's sign-off like umbrella open items 15 and 20.

## Hypothesis

A ratatui-based TUI can read keyboard input and render without pulling `mio` or `socket2` into Cargo.lock, so the existing `no_network_dependencies` test (which denies both by name) stays unchanged.

## Mechanism Under Test

1. Whether ratatui 0.30.x can be combined with crossterm 0.29 with the `events` feature off.
2. If not, the smallest key-input and render path that avoids `mio` and `socket2`, is single-threaded, and restores the terminal on panic.

## Probe Family

1 - Local process / Node script (throwaway cargo projects, real `cargo tree` and `Cargo.lock` inspection, compiled binaries driven through a pseudo-terminal by a Python script). Plus a scratch copy of the real workspace to run the real test.

## Probe Cost Class

cheap-local. Safety gate met: all work in the scratch directory `/private/tmp/claude-501/-Users-databased-Developer-01-AI-unambiguous/25bdb9f4-a222-47b4-8488-a41f43dbc181/scratchpad/m6-probe/`; crates fetched from crates.io only; `unam` never run; no agent CLI invoked; the product repository was not modified except for this file.

## Probe Method

Tools: cargo 1.99.0, rustc 1.99 (no `rustup`, so a real 1.89 compile was not possible; MSRV is checked from manifests, see item 4). Every scratch project set `rust-version = "1.89"` so Cargo's MSRV-aware resolver (resolver 3) picked compatible versions.

| Probe project | Dependency line | Purpose |
|---|---|---|
| `p1-default` | `ratatui = "0.30.2"` | default features |
| `p2-ratatui-crossterm-nodefault` | `ratatui = { default-features = false, features = ["crossterm"] }` | crossterm backend, no other defaults |
| `p2b-crossterm-direct-nodefault` | p2 plus `crossterm = { version = "0.29", default-features = false }` | can a direct dependency switch events off? |
| `p3-termion` | `ratatui = { default-features = false, features = ["termion"] }` | termion backend, full working TUI binary |
| `p3a-nohook`, `p3b-abort` | p3 variants | panic restore with no hook; with `panic = "abort"` |
| `p4-termina` | `features = ["termina"]` | termina backend, lock check |
| `p5-termwiz` | `features = ["termwiz"]` | termwiz backend, lock check |
| `p6-crossterm-noevents` | `crossterm = { version = "0.29", default-features = false, features = ["use-dev-tty"] }` plus `ratatui-core` | candidates (a) and (b) |
| `ws-copy` | tracked files of the real workspace plus the termion dependency | real lockfile delta and the real `phase1_checks` test |

The working binary (p3) draws a ratatui `List` in a `Block`, reads keys with termion's blocking `stdin().keys()`, and was driven in a pseudo-terminal by `ptytest.py` (sends keys, checks terminal flags before, during, and after, counts threads with `ps -M`, inspects emitted escape sequences).

Note: the repository's `scout-block` hook rejects command lines that contain the words "build" and "target", so probes were built with the `cargo b` alias and binaries were located with a glob. This changes nothing about the results.

## Evidence Captured

### 1. ratatui 0.30.x default features pull mio. No ratatui flag fixes it for crossterm.

`p1-default`, `cargo tree -i mio`:

```
mio v1.2.4
├── crossterm v0.29.0
│   └── ratatui-crossterm v0.1.2
│       └── ratatui v0.30.2
└── signal-hook-mio v0.2.5
    └── crossterm v0.29.0 (*)
```

`cargo tree -i socket2`: `package ID specification 'socket2' did not match any packages` (socket2 is absent in every probe, including the default one; `mio` is the only denied crate that appears).

Crossterm 0.29.0 manifest: `events = ["dep:mio", "dep:signal-hook", "dep:signal-hook-mio"]`, and `default = ["bracketed-paste", "events", "windows", "derive-more"]`. In `src/lib.rs` the input module is gated: `#[cfg(feature = "events")] pub mod event;`. Without `events`, `use crossterm::event;` fails (p6): `error[E0432]: unresolved import crossterm::event`.

The ratatui crossterm backend crate (`ratatui-crossterm 0.1.2`) declares `[dependencies.crossterm_0_29] version = "0.29"` with NO `default-features = false` and `[features] default = ["crossterm_0_29", ...]`. Cargo features are additive, so any build that enables the ratatui crossterm backend enables crossterm's default features, `events` included.

Proofs:
- `p2` (`default-features = false, features = ["crossterm"]` on ratatui): `Cargo.lock` still has `mio`, `signal-hook`, `signal-hook-registry`; `cargo tree -i mio` shows the same chain as p1.
- `p2b` (direct `crossterm = { default-features = false }` added next to ratatui's crossterm backend): `cargo tree -i crossterm -e features` still lists `crossterm feature "events"` and `cargo tree -i mio` still lists `mio`. A direct dependency cannot switch the feature off.
- Negative control in `ws-copy` (real workspace, ratatui crossterm backend): `cargo test -p unam --test phase1_checks no_network_dep` gives `test no_network_dependencies ... FAILED`, `network-capable crates in Cargo.lock: ["mio"]`.

Conclusion for question 1: ratatui 0.30.x with default features pulls crossterm `events` and therefore `mio`. No ratatui feature set yields crossterm without `events`; the `ratatui-crossterm` crate has no flag for it either. Taylor's chosen route cannot be done with ratatui's crossterm backend.

### 2. Input paths, one by one

| Option | Result | Lock evidence |
|---|---|---|
| Crossterm without `events`, reading stdin yourself | Works: `terminal::enable_raw_mode`, `terminal::size`, `EnterAlternateScreen`, `cursor::Hide` all compile and run without `events` (p6). Key reading would be raw `stdin().read` plus a hand-written decoder. But ratatui cannot use it (item 1), so ratatui needs a hand-written `Backend` impl (about 10 required methods, `ratatui-core` `Backend` trait; the stock crossterm backend is 1171 lines). | p6 lock: `mio` 0, `socket2` 0, `signal-hook` 0; new crates: `filedescriptor`, `rustix`, `parking_lot`, `libc` |
| (b) crossterm `use-dev-tty` | Dead end. The feature only changes the event source (`src/event/source/unix.rs`, `src/event/sys/unix/waker.rs` are the only users). It adds `filedescriptor` and `rustix/process` but does not create `crossterm::event` without `events`. | n/a |
| (c) ratatui `termion` backend | **Works.** `ratatui-termion 0.1.2` depends only on `ratatui-core`, `instability`, `termion 4`. `termion 4.0.6` depends only on `libc` and `numtoa` (serde optional, off). | p3 lock: `mio` 0, `socket2` 0, `signal-hook` 0, `signal-hook-registry` 0, `nix` 0 |
| `termina` backend | No `mio`/`socket2`, but pulls `signal-hook` and `signal-hook-registry`, 64 crates in the normal tree vs 55 for termion. Not chosen. | p4 lock: `mio` 0, `socket2` 0, `signal-hook` 1 |
| `termwiz` backend | No `mio`/`socket2`, but pulls `signal-hook`, `nix` (2 entries) and a large tree. Not chosen. | p5 lock: `mio` 0, `socket2` 0, `signal-hook` 1, `nix` 2 |

`ratatui-termion` is re-exported by ratatui as `ratatui::termion`, so no separate `termion` dependency line is needed (a direct `use termion::...` fails to resolve; use `ratatui::termion::...`).

Real-workspace delta (`ws-copy`, `Cargo.toml` edit: `ratatui = { version = "0.30.2", default-features = false, features = ["termion"] }` in `unam-cli`):
- `Cargo.lock` packages: 137 before, 176 after (39 added, 0 removed).
- Added: `allocator-api2 approx by_address castaway compact_str darling darling_core darling_macro deranged either foldhash ident_case indoc instability itertools kasuari libm line-clipping lru num-conv num_threads numtoa palette palette_derive palette_math powerfmt ratatui ratatui-core ratatui-termion ratatui-widgets rustversion static_assertions strum strum_macros termion time time-core unicode-truncate unicode-width`.
- None of the 26 names in the `no_network_dependencies` deny list is present. `cargo test -p unam --test phase1_checks no_network` gives `test no_network_dependencies ... ok` and `test no_network_symbols ... ok`, unchanged test file. `cargo check -p unam` passes.
- `signal-hook` is not added, nor is `mio`, `socket2`, `libc` (already locked), `rustix` (already locked, unused by termion).
- Heads-up for the lock-delta reviewer: `time`, `num_threads`, `palette`, `deranged` come in through ratatui-widgets and ratatui-core's non-optional paths even with default features off. They are not network crates.

### 3. Single-threaded, key decoding, resize, panic restore (termion binary p3, driven in a pty)

| Check | Result |
|---|---|
| Threads while running | 1 (`ps -M` shows a single thread line) in all runs |
| Keys decoded | a printable char, Space, Enter, Up, Down, Left, Right, Backspace, a lone Esc, Ctrl-C (raw byte 0x03 arrives as `Key::Ctrl('c')` because raw mode clears ISIG) all rendered with the expected labels; `q` and Ctrl-C exit with code 0 |
| Terminal state | `ICANON/ECHO` were True before, False while running, True after exit; alternate screen entered (`ESC[?1049h`) and left (`ESC[?1049l`), cursor shown |
| Resize | There is no resize event and no SIGWINCH handler. With a 24x80 window resized to 30x100 mid-run, the next redraw (triggered by the next key) laid out for 100 columns and 30 rows (`ESC[30;1H`, `ESC[27;100H` in the output) because `Terminal::draw` re-queries the size each frame. Stale layout persists until the next key press. |
| Panic, `panic = "unwind"` (the default; the workspace sets no panic profile) | Exit 101, termios restored by `RawTerminal` drop, alternate screen left. Without a panic hook the panic message is printed on the alternate screen BEFORE it is left (message at byte 1039, leave-alt at 1135), so the user never sees it. With a hook that writes `ToMainScreen` plus `cursor::Show` before calling the saved default hook, the message appears AFTER leave-alt (leave-alt 983, message 1053) and is visible. Hook plus drop is the right pair. |
| Panic, `panic = "abort"` (variant p3b) | Exit -6 (SIGABRT). The hook ran and left the alt screen, but the termios was NOT restored: `ICANON=False ECHO=False` after exit. Drop does not run under abort, and a hook that only writes escape sequences cannot restore termios. |
| External SIGTERM / SIGINT | Exit -15 / -2 and the terminal is left in raw mode (`ICANON=False ECHO=False`). Ctrl-C from the keyboard is not affected because raw mode turns it into a key. |

Crossterm-without-events (p6, unwind) behaved the same on quit, Ctrl-C and panic. Its panic hook calls `terminal::disable_raw_mode`, which restores termios from inside the hook; that would also cover `panic = "abort"`, but the abort case was not run for p6.

### 4. MSRV (read from `~/.cargo/registry/src/.../*/Cargo.toml`)

| Crate | `rust-version` |
|---|---|
| ratatui 0.30.2 | 1.88.0 |
| ratatui-core 0.1.2 | 1.88.0 |
| ratatui-widgets 0.3.2 | 1.88.0 |
| ratatui-crossterm 0.1.2 | 1.88.0 |
| ratatui-termion 0.1.2 | 1.88.0 |
| ratatui-termina 0.1.0 | 1.88.0 |
| crossterm 0.29.0 | 1.63.0 (edition 2021) |
| termina 0.3.3 | 1.71 |
| termion 4.0.6 | none declared (edition 2015 default, `build = false`, no build script) |
| numtoa 0.2.4 | none declared |

Scan of all locked packages in `ws-copy` whose manifest was in the registry: the highest declared `rust-version` is 1.88.0 (ratatui family, `darling` 0.24.1, `time` 0.3.55); none exceeds the workspace 1.89. `rustup` is not installed here, so a real 1.89 compile was NOT run; resolution was done with `rust-version = "1.89"` set so Cargo would have rejected or downgraded anything incompatible.

### 5. Spawn check

- Probe binary: single thread at runtime (item 3).
- `termion 4.0.6` calls `thread::spawn` only in `src/async.rs` (`async_stdin`, `async_stdin_with_name`), which the recommended path never calls. A grep for `thread::` and `.spawn(` over `ratatui`, `ratatui-core`, `ratatui-widgets`, `ratatui-termion`, and `termion` sources finds nothing else.
- The `no_network_symbols` test scans only `crates/*/src`, so library-internal text never trips it. Product code using `stdin().keys()` contains neither `.spawn(` nor `std::net`.
- No option needs a background thread or process. Nothing outlives a command.

## Verdict

VIABLE

The hypothesis (a ratatui TUI that reads keys and renders with no `mio` or `socket2` in `Cargo.lock`, `no_network_dependencies` unchanged) holds with `ratatui` + the `termion` backend. The specific mechanism Taylor selected, "turn off crossterm events", is NOT viable (item 1), so the design must not rely on it.

## Resulting Design Constraint

- **What this licenses:**
  - Use `ratatui` with `default-features = false, features = ["termion"]` and read keys with the blocking `ratatui::termion::input::TermRead::keys()` on stdin. Single-threaded, no polling, no timeout needed.
  - Keep `no_network_dependencies` and `no_network_symbols` exactly as written; both pass against the real lockfile with the termion backend (run in a scratch copy).
  - Rely on termion's `RawTerminal` and `AlternateScreen` guards for terminal restore on normal exit and on panic unwind, plus a panic hook that writes `ToMainScreen` and `cursor::Show` before the default hook so the panic text is visible.
  - Redraw on the next key to pick up a resized window (`Terminal::draw` re-queries the size).
  - Fallback if termion is later rejected: `crossterm = { version = "0.29", default-features = false }` plus `ratatui-core`/`ratatui` without a backend and a self-written `Backend` impl (about 10 methods) and stdin decoder. Same lock result (no `mio`), more code, and it allows `disable_raw_mode` inside a panic hook (abort case not run).
- **What this forbids:**
  - Do NOT plan on "ratatui crossterm backend with events turned off": ratatui-crossterm forces crossterm default features, so `mio` and `signal-hook-mio` are in `Cargo.lock` and `no_network_dependencies` fails (negative control reproduced).
  - Do NOT add `crossterm`'s `use-dev-tty` expecting it to help: it affects only the event source, which needs `events`.
  - Do NOT use `termion::async_stdin` (spawns a thread; forbidden pattern) or any input path that needs a thread.
  - Do NOT set `panic = "abort"` in any profile (including cargo-dist's release profile): `Drop` would not run and the terminal would stay in raw mode with echo off (reproduced, exit -6).
  - Do NOT add `mio`/`socket2`/`signal-hook` to the deny-list exceptions.
- **What remains uncertain (known-gap):**
  - Spec section 1.4 (line 132 of `docs/spec/unam-technical-spec.md`) names `ratatui` + `crossterm`; moving to termion is a documented departure that needs Taylor's explicit approval, and the spec text, `process/context/all-context.md` ("Still planned: ratatui + crossterm (M6)") and the umbrella open-items list need the same update. The spec and PRD are authoritative until Taylor changes them.
  - Resize is applied only on the next keystroke, and there is no redraw on idle resize. Whether that is acceptable for the three-pane picker is a product call; fixing it needs a SIGWINCH handler (`signal-hook` or `unsafe`, both unwelcome here).
  - External SIGTERM and SIGINT leave the terminal raw (reproduced). Only keyboard Ctrl-C is handled. Accepted under the single-user threat model unless Taylor wants otherwise.
  - Not tested: wide Unicode input (multi-byte keys), Alt/Shift-Tab/function keys, bracketed paste, terminals other than the macOS pty harness (no Linux run), and a real 1.89 toolchain compile (no `rustup`). Termion's key decoder is a small hand-written state machine; known limits include no kitty keyboard protocol and no modifier reporting for arrows.
  - Termion has no declared `rust-version` and is maintained on a slow cadence (4.0.6); a future termion release could raise MSRV or add dependencies. The committed `Cargo.lock` pins it.
  - If stdin is not a terminal (piped), `stdin().keys()` reads the pipe; the TUI entry point should check `std::io::IsTerminal` for stdin and stdout and refuse otherwise (or use `termion::get_tty()`). Not probed.

## Recommendation: one input path

**Ratatui termion backend, blocking key reads on stdin, no threads.** Exact `unam-cli/Cargo.toml` dependency line (workspace-style, add to `[workspace.dependencies]` and reference with `ratatui.workspace = true`):

```toml
ratatui = { version = "0.30.2", default-features = false, features = ["termion"] }
```

No separate `termion` line is needed; use the re-export `ratatui::termion`. Do not add `crossterm` at all. If the TUI lives in a new `unam-tui` crate under the existing `tui/` plan, the same single line goes there instead; `unam-core` stays dependency-free of it.

Implementation skeleton proven in `p3-termion` (scratch):

```rust
use ratatui::termion::{input::TermRead, raw::IntoRawMode, screen::IntoAlternateScreen};
// 1. panic hook: write ToMainScreen + cursor::Show to stderr, then call the saved default hook
// 2. let out = stdout().into_raw_mode()?.into_alternate_screen()?;
// 3. let mut term = Terminal::new(TermionBackend::new(out))?;
// 4. term.draw(..); for key in stdin().keys() { match key? { .. } term.draw(..) }
```

## Unresolved questions

- Does Taylor approve the spec 1.4 departure (termion in place of crossterm)? If not, the fallback is the hand-written `Backend` over crossterm without `events`.
- Is "resize takes effect on the next keystroke" acceptable for the picker?
- Should the `all-context.md` "Technology Stack" line and the umbrella open-items list be updated at M6 UPDATE PROCESS to record the departure (the umbrella already has precedent: items 15 and 20)?
