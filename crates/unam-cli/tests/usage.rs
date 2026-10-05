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
    assert!(out.contains("status arrives in a later milestone"), "{out}");
    assert!(!env.config_file().exists());
}

#[test]
fn help_and_version_exit_0() {
    let env = TestEnv::new();
    let (code, out, _) = env.run(&["--help"]);
    assert_eq!(code, 0);
    assert!(out.contains("source"));
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
