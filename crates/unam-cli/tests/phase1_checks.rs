//! Product Phase 1 exit checks that need no real library (milestone M5):
//! network none, background none, and the AC coverage check (PRD 12, 13.1,
//! spec 9). `phase1_exit.rs` holds the checks that need Taylor's library.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn files_under(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd {
        let p = e.unwrap().path();
        if p.is_dir() {
            files_under(&p, ext, out);
        } else if p.extension().is_some_and(|x| x == ext) {
            out.push(p);
        }
    }
}

fn crate_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = fs::read_dir(repo_root().join("crates"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir())
        .collect();
    v.sort();
    v
}

/// The package names of `Cargo.lock`.
fn locked_packages() -> Vec<String> {
    let text = fs::read_to_string(repo_root().join("Cargo.lock")).unwrap();
    text.lines()
        .filter_map(|l| {
            let name = l.strip_prefix("name = \"")?;
            name.strip_suffix('"').map(str::to_string)
        })
        .collect()
}

#[test]
fn no_network_dependencies() {
    let pkgs = locked_packages();
    // Not vacuous: the lockfile is read and holds the workspace crates.
    assert!(pkgs.len() > 30, "{} packages parsed", pkgs.len());
    assert!(pkgs.iter().any(|p| p == "unambiguous") && pkgs.iter().any(|p| p == "unam-core"));
    // Any HTTP, TLS, async-runtime, or socket client crate.
    let denied = [
        "reqwest",
        "hyper",
        "hyper-util",
        "hyper-tls",
        "h2",
        "http",
        "ureq",
        "curl",
        "curl-sys",
        "isahc",
        "surf",
        "attohttpc",
        "minreq",
        "rustls",
        "native-tls",
        "openssl",
        "openssl-sys",
        "tokio",
        "async-std",
        "mio",
        "socket2",
        "tonic",
        "tungstenite",
        "tokio-tungstenite",
        "git2",
        "gix",
    ];
    let found: Vec<&&str> = denied
        .iter()
        .filter(|d| pkgs.iter().any(|p| p == **d))
        .collect();
    assert!(
        found.is_empty(),
        "network-capable crates in Cargo.lock: {found:?}"
    );
}

#[test]
fn no_network_symbols() {
    // Needles are assembled so this file never matches itself.
    let needles = [
        format!("std::{}", "net"),
        format!("Tcp{}", "Stream"),
        format!("Udp{}", "Socket"),
        format!("Tcp{}", "Listener"),
        format!(".{}(", "spawn"),
    ];
    let mut scanned = 0;
    let mut hits: Vec<String> = Vec::new();
    for c in crate_dirs() {
        let mut files = Vec::new();
        files_under(&c.join("src"), "rs", &mut files);
        for f in files {
            scanned += 1;
            let text = fs::read_to_string(&f).unwrap();
            for n in &needles {
                if text.contains(n.as_str()) {
                    hits.push(format!("{}: {n}", f.display()));
                }
            }
        }
    }
    assert!(scanned > 30, "{scanned} source files scanned");
    assert!(hits.is_empty(), "{hits:?}");
}

/// True when `text` sets panic to abort: `panic = "abort"` with any spacing
/// or quotes, or `-C panic=abort` (comments after `#` are ignored).
fn sets_panic_abort(text: &str) -> bool {
    text.lines().any(|line| {
        let code = line.split('#').next().unwrap_or("");
        let compact: String = code
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
            .collect();
        compact.contains("panic=abort")
    })
}

#[test]
fn no_panic_abort_profile() {
    // The picker restores the terminal through drop guards; abort would skip
    // them (milestone M6, D-TERM).
    let root = repo_root();
    let mut files = vec![root.join("Cargo.toml"), root.join("dist-workspace.toml")];
    for c in crate_dirs() {
        files.push(c.join("Cargo.toml"));
    }
    for name in ["config.toml", "config"] {
        let p = root.join(".cargo").join(name);
        if p.exists() {
            files.push(p);
        }
    }
    // Not vacuous: the root manifest and the dist file are read.
    assert!(root.join("Cargo.toml").is_file() && root.join("dist-workspace.toml").is_file());
    assert!(files.len() >= 4, "{files:?}");
    for f in &files {
        let text = fs::read_to_string(f).unwrap();
        assert!(!text.is_empty(), "{}", f.display());
        assert!(
            !sets_panic_abort(&text),
            "{} sets panic = abort",
            f.display()
        );
    }
    // The matcher sees every spelling it must reject and passes the rest.
    assert!(sets_panic_abort("[profile.release]\npanic = \"abort\"\n"));
    assert!(sets_panic_abort("panic=\"abort\""));
    assert!(sets_panic_abort("rustflags = [\"-C\", \"panic=abort\"]"));
    assert!(sets_panic_abort("  panic   =   'abort'  # why"));
    assert!(!sets_panic_abort("[profile.release]\npanic = \"unwind\"\n"));
    assert!(!sets_panic_abort("# panic = \"abort\" is banned\n"));
}

/// `pgrep -f <needle>`: `Some(true)` when something matches, `None` when
/// `pgrep` is unavailable here.
fn pgrep_matches(needle: &str) -> Option<bool> {
    let out = std::process::Command::new("pgrep")
        .arg("-f")
        .arg(needle)
        .output()
        .ok()?;
    Some(out.status.success() && !out.stdout.is_empty())
}

#[test]
fn no_background_process() {
    let fx = Fx::new();
    let p = fx.project("bg-unique-project-path");
    let canon = fx.canon(&p).to_string_lossy().into_owned();
    // Commands that touch the project, then the process table.
    for args in [
        &["add", "fx:sdlc/code-review", "--yes"][..],
        &["status"],
        &[
            "--target",
            "agents-neutral",
            "add",
            "fx:testing/pytest-fixtures",
            "--yes",
        ],
        &["--target", "agents-neutral", "status"],
        &["--target", "agents-neutral", "reconcile"],
        &["--target", "agents-neutral", "doctor"],
        &["--target", "agents-neutral", "deactivate", "--yes"],
        &["--target", "claude-code", "deactivate", "--yes"],
    ] {
        let (code, _, _) = fx.run(&p, args);
        assert_eq!(code, 0, "{args:?}");
    }
    match pgrep_matches(&canon) {
        Some(found) => assert!(!found, "a process still names {canon}"),
        None => println!("self-skip: pgrep is not available (named residual)"),
    }
    // The control: the check can see a process that does name the path.
    let mut child = std::process::Command::new("sh")
        .arg("-c")
        .arg("sleep 30; true")
        .arg(&canon)
        .spawn()
        .unwrap();
    if let Some(found) = pgrep_matches(&canon) {
        assert!(found, "the probe cannot see a live process");
    }
    child.kill().unwrap();
    child.wait().unwrap();
    fx.done();
}

#[test]
fn phase1_ac_coverage() {
    // Every PRD 13.2 criterion of product Phase 1: AC-1 to AC-43 and AC-53.
    let wanted: Vec<u32> = (1..=43).chain([53]).collect();
    let mut text = String::new();
    for c in crate_dirs() {
        let mut files = Vec::new();
        files_under(&c.join("tests"), "rs", &mut files);
        for f in files {
            if f.file_name().is_some_and(|n| n == "phase1_checks.rs") {
                continue;
            }
            text.push_str(&fs::read_to_string(&f).unwrap());
            text.push('\n');
        }
    }
    assert!(text.len() > 10_000, "test sources not read");
    // Zero-padded: the early criteria are named ac_01_ to ac_09_ (EX4).
    let missing: Vec<u32> = wanted
        .iter()
        .copied()
        .filter(|n| !text.contains(&format!("fn ac_{n:02}_")))
        .collect();
    assert!(
        missing.is_empty(),
        "no test is named for acceptance criteria {missing:?}"
    );
    println!(
        "phase1_ac_coverage: {} criteria have a named test",
        wanted.len()
    );
}
