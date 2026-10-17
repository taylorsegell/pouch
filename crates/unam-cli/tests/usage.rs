//! Bare `unam`, help, version, usage errors, verbosity, and logging.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;

#[test]
fn bare_unam_prints_usage_without_config() {
    let env = TestEnv::new();
    let (code, out, _) = env.run(&[]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"), "{out}");
    assert!(!out.contains("later milestone"), "{out}");
    assert!(out.contains("  bundle "), "{out}");
    for cmd in [
        "add",
        "rm",
        "use",
        "deactivate",
        "resume",
        "rollback",
        "status",
        "reconcile",
        "doctor",
    ] {
        assert!(out.contains(&format!("  {cmd} ")), "{cmd}: {out}");
    }
    for flag in ["--project <PATH>", "--target <ID>", "--yes"] {
        assert!(out.contains(flag), "{flag}: {out}");
    }
    assert!(!env.config_file().exists());
}

#[test]
fn bare_unam_prints_status_for_a_project() {
    let fx = Fx::new();
    let p = fx.project("p");
    // No `.unam/` yet: usage only.
    let (code, out, _) = fx.run(&p, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:") && !out.contains("STATE"), "{out}");
    let (code, v) = fx.json_raw(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{v}");
    let (code, out, _) = fx.run(&p, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"), "{out}");
    assert!(out.contains("STATE") && out.contains("active"), "{out}");
    // Even with an orphaned entry, bare `unam` exits 0.
    std::fs::remove_file(p.join(".claude/skills/code-review")).unwrap();
    let (code, out, _) = fx.run(&p, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("orphaned"), "{out}");
    fx.done();
}

#[test]
fn bare_unam_non_tty_prints_usage_and_status_exit_0() {
    // assert_cmd gives the child no terminal, so bare `unam` must not open
    // the picker (milestone M6): usage, then the status table, exit 0.
    let fx = Fx::new();
    let p = fx.project("non-tty-p");
    let (code, v) = fx.json_raw(&p, &["add", "fx:sdlc/code-review", "--yes"]);
    assert_eq!(code, 0, "{v}");
    let (code, out, _) = fx.run(&p, &[]);
    assert_eq!(code, 0);
    assert!(out.contains("Usage:"), "{out}");
    assert!(out.contains("STATE") && out.contains("active"), "{out}");
    assert!(!out.contains("\u{1b}[?1049h"), "alternate screen entered");
    fx.done();
    // No project and no config: still usage only, no config file written.
    let env = TestEnv::new();
    let (code, out, _) = env.run(&[]);
    assert_eq!(code, 0);
    assert!(
        out.contains("Usage:") && !out.contains("\u{1b}[?1049h"),
        "{out}"
    );
    assert!(!env.config_file().exists());
}

#[test]
fn help_and_version_exit_0() {
    let env = TestEnv::new();
    let (code, out, _) = env.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("source"));
    assert!(out.contains("bundle"), "{out}");
    let (code, out, _) = env.run(&["bundle", "--help"]);
    assert_eq!(code, 0);
    for sub in ["save", "ls", "show", "edit", "duplicate", "rm", "add"] {
        assert!(out.contains(sub), "{sub}: {out}");
    }
    let (code, out, _) = env.run(&["use", "--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("--replace") && out.contains("--add"), "{out}");
    let (code, out, _) = env.run(&["--version"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("unam "), "{out}");
}

#[test]
fn unknown_subcommand_exits_2() {
    let env = TestEnv::new();
    let (code, _, err) = env.run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(!err.is_empty());
    let (code, out, _) = env.run(&["--json", "frobnicate"]);
    assert_eq!(code, 2);
    assert!(out.is_empty());
}

#[test]
fn quiet_hides_notes() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let (code, _, err) = env.run(&["source", "add", "fx", lib.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(err.contains("note["), "{err}");
    let env2 = TestEnv::new();
    let lib2 = build_library(&env2);
    let (code, _, err) = env2.run(&["-q", "source", "add", "fx", lib2.to_str().unwrap()]);
    assert_eq!(code, 0);
    assert!(!err.contains("note["), "{err}");
    assert!(err.contains("warning["), "{err}");
}

#[test]
fn unam_log_debug_keeps_json_stdout_clean() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let out = env
        .cmd()
        .env("UNAM_LOG", "debug")
        .args(["--json", "source", "add", "fx", lib.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("DEBUG"), "{err}");
}
