//! Product Phase 1 exit checks that need Taylor's real skill library
//! (milestone M5: S1 and S6; PRD 13.1). Both read the library path only
//! from `UNAM_REAL_LIBRARY` and self-skip when it is unset, so the default
//! test run never touches a real library. They use the isolated `TestEnv`
//! (temp HOME, config, cache), a temporary project outside this repository,
//! and snapshot the library before and after to prove it is read-only. They
//! run once at the Phase 1 exit:
//!
//! `UNAM_REAL_LIBRARY=<path> cargo test --release -p unambiguous --test phase1_exit -- --nocapture`

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use serde_json::Value;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const ALIAS: &str = "real";

/// The library named by `UNAM_REAL_LIBRARY`, if it is set and exists.
fn real_library() -> Option<PathBuf> {
    let p = PathBuf::from(std::env::var_os("UNAM_REAL_LIBRARY")?);
    p.is_dir().then_some(p)
}

/// `(id, name)` of every cataloged skill, from a scratch environment.
fn catalog_ids(lib: &std::path::Path) -> Vec<(String, String)> {
    let env = TestEnv::new();
    let (code, v) = run_json(&env, &["source", "add", ALIAS, lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    let (code, v) = run_json(&env, &["search", ""]);
    assert_eq!(code, 0, "{v}");
    v["data"]["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["id"].as_str().unwrap().to_string(),
                r["name"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

/// The first `n` ids whose name appears once in the whole catalog (so a
/// bare duplicate name cannot make the sequence fail).
fn unique_ids(all: &[(String, String)], n: usize) -> Vec<String> {
    all.iter()
        .filter(|(_, name)| all.iter().filter(|(_, other)| other == name).count() == 1)
        .take(n)
        .map(|(id, _)| id.clone())
        .collect()
}

fn median(times: &mut [Duration]) -> Duration {
    times.sort();
    times[times.len() / 2]
}

#[test]
fn s1_cold_to_verified_active_on_real_library() {
    let Some(lib) = real_library() else {
        println!("self-skip: UNAM_REAL_LIBRARY is not set (S1 runs once at the Phase 1 exit)");
        return;
    };
    let before = snapshot_source(&lib);
    let all = catalog_ids(&lib);
    assert!(all.len() >= 2, "the library catalogs {} skills", all.len());
    let pick = unique_ids(&all, 2);
    assert_eq!(
        pick.len(),
        2,
        "no two uniquely named skills in {} skills",
        all.len()
    );
    let mut times: Vec<Duration> = Vec::new();
    for run in 1..=5 {
        // A fresh environment per run: its cache directory is empty (cold).
        let env = TestEnv::new();
        let project = env.path("work").join("s1-project");
        std::fs::create_dir_all(&project).unwrap();
        git_marker(&project);
        let t0 = Instant::now();
        let (code, v) = run_json(&env, &["source", "add", ALIAS, lib.to_str().unwrap()]);
        assert_eq!(code, 0, "run {run}: {v}");
        let mut add = vec!["--json", "add"];
        add.extend(pick.iter().map(String::as_str));
        add.push("--yes");
        let (code, stdout, stderr) = run_in(&env, &project, &add);
        assert_eq!(code, 0, "run {run}: {stdout}\n{stderr}");
        let (code, stdout, stderr) = run_in(&env, &project, &["--json", "status"]);
        assert_eq!(code, 0, "run {run}: {stdout}\n{stderr}");
        let elapsed = t0.elapsed();
        let status: Value = serde_json::from_str(&stdout).unwrap();
        let states = record_states(&status);
        assert_eq!(states.len(), 2, "run {run}: {states:?}");
        assert!(
            states.iter().all(|(_, s)| s == "active"),
            "run {run}: {states:?}"
        );
        println!("S1 run {run}: {:.3} s", elapsed.as_secs_f64());
        assert!(
            elapsed < Duration::from_secs(60),
            "run {run} took {elapsed:?}"
        );
        times.push(elapsed);
    }
    let med = median(&mut times.clone());
    println!(
        "S1: library {} ({} skills), five runs {:?} s, median {:.3} s (limits: each < 60 s, median < 30 s)",
        lib.display(),
        all.len(),
        times.iter().map(|d| d.as_secs_f64()).collect::<Vec<_>>(),
        med.as_secs_f64()
    );
    assert!(med < Duration::from_secs(30), "median {med:?}");
    // Read-only proof: the library is byte- and stat-identical.
    assert_source_unchanged(&before, &lib);
}

/// The `plan.estimate` of a refused or blocked `add` (nothing is applied).
fn estimate_of(env: &TestEnv, project: &std::path::Path, ids: &[String]) -> (u64, u64) {
    let mut args = vec!["--json", "add"];
    args.extend(ids.iter().map(String::as_str));
    // No `--yes`: the plan is computed and printed, and nothing is applied.
    let (code, stdout, stderr) = run_in(env, project, &args);
    assert!(code == 2 || code == 6, "exit {code}: {stdout}\n{stderr}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    let est = &v["data"]["plan"]["estimate"];
    assert_eq!(est["estimate"], true, "{v}");
    (
        est["skills"].as_u64().unwrap(),
        est["tokens"].as_u64().unwrap(),
    )
}

#[test]
fn s6_catalog_cost_reduction_on_real_library() {
    let Some(lib) = real_library() else {
        println!("self-skip: UNAM_REAL_LIBRARY is not set (S6 runs once at the Phase 1 exit)");
        return;
    };
    let before = snapshot_source(&lib);
    let all = catalog_ids(&lib);
    let four = unique_ids(&all, 4);
    assert_eq!(
        four.len(),
        4,
        "no four uniquely named skills in {} skills",
        all.len()
    );
    let env = TestEnv::new();
    let project = env.path("work").join("s6-project");
    std::fs::create_dir_all(&project).unwrap();
    git_marker(&project);
    let (code, v) = run_json(&env, &["source", "add", ALIAS, lib.to_str().unwrap()]);
    assert_eq!(code, 0, "{v}");
    let (n4, tokens4) = estimate_of(&env, &project, &four);
    let everything: Vec<String> = all.iter().map(|(id, _)| id.clone()).collect();
    let (n_all, tokens_all) = estimate_of(&env, &project, &everything);
    assert_eq!(n4, 4);
    assert_eq!(n_all as usize, all.len());
    let ratio = tokens4 as f64 / tokens_all as f64;
    println!(
        "S6: four skills ~{tokens4} tokens; whole library ({n_all} skills) ~{tokens_all} tokens; ratio {:.1} percent (limit 10 percent)",
        ratio * 100.0
    );
    if all.len() >= 40 {
        assert!(
            tokens4 * 10 <= tokens_all,
            "four skills {tokens4} > 10 percent of {tokens_all}"
        );
    } else {
        println!(
            "S6 library-size limit: {} skills is below 40, so a four-skill selection cannot be 10 percent of it; the mechanism is not at fault",
            all.len()
        );
    }
    // Nothing was applied, and the library is untouched.
    assert!(!project.join(".claude").exists());
    assert!(!project.join(".agents").exists());
    assert_source_unchanged(&before, &lib);
}
