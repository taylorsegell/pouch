//! A closed stdout (`pouch ... | head`) ends the process quietly with exit 0
//! and never panics (spec 7: never panic on user data).

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use std::io::Read;
use std::process::{Command, Stdio};

fn run_with_early_close(args: &[&str]) {
    let env = TestEnv::new();
    let lib = env.path("work").join("biglib");
    for i in 0..1200 {
        write_file(
            &lib,
            &format!("cat/skill-{i:04}/SKILL.md"),
            skill_md(
                &format!("skill-{i:04}"),
                "A skill used to overflow the pipe.",
            )
            .as_bytes(),
        );
    }
    let (code, v) = run_json(&env, &["source", "add", "big", lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");

    let make_cmd = || {
        let mut cmd = Command::new(assert_cmd::cargo::cargo_bin!("pouch"));
        cmd.env_clear();
        for (k, v) in env.vars() {
            cmd.env(k, v);
        }
        cmd.current_dir(env.path("cwd")).args(args);
        cmd
    };
    // Guard: the full output must exceed a 64 KB pipe buffer, otherwise the
    // child could finish writing before the read end closes (vacuous test).
    let full = make_cmd().output().unwrap();
    assert!(
        full.stdout.len() > 65_536,
        "fixture output too small to overflow a pipe: {} bytes",
        full.stdout.len()
    );

    let mut child = make_cmd()
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Read a few bytes, then close the read end while the child still has
    // far more than a pipe buffer (64 KB) left to write.
    let mut stdout = child.stdout.take().unwrap();
    let mut head = [0u8; 50];
    stdout.read_exact(&mut head).unwrap();
    drop(stdout);
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(!stderr.contains("panicked"), "stderr: {stderr}");
    assert_eq!(status.code(), Some(0), "stderr: {stderr}");
}

#[test]
fn json_output_to_closed_pipe_exits_zero() {
    run_with_early_close(&["--json", "search", "skill"]);
}

#[test]
fn human_output_to_closed_pipe_exits_zero() {
    run_with_early_close(&["search", "skill"]);
}
