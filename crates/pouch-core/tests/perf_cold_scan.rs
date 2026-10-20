// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

//! Spec 9 cold-scan target: 500 skills under 300 ms (release). Cold means no
//! catalog cache. Debug builds assert a generous 5 s bound.

use pouch_core::catalog::{Catalog, LoadOptions};
use pouch_core::config::SourceEntry;
use pouch_core::paths::{Dirs, Env};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Instant;

fn build_fixture(lib: &Path) {
    for c in 0..10 {
        for s in 0..5 {
            for k in 0..10 {
                let dir = lib.join(format!("cat-{c}/sub-{s}/skill-{c}-{s}-{k}"));
                fs::create_dir_all(dir.join("references")).unwrap();
                fs::create_dir_all(dir.join("scripts")).unwrap();
                fs::write(
                    dir.join("SKILL.md"),
                    format!(
                        "---\nname: skill-{c}-{s}-{k}\ndescription: Synthetic skill {c} {s} {k} for the cold scan test.\n---\n\n# Skill\n\nBody text for the synthetic skill.\n"
                    ),
                )
                .unwrap();
                fs::write(dir.join("references/notes.md"), "# Notes\n\nSome notes.\n").unwrap();
                fs::write(dir.join("scripts/run.sh"), "#!/bin/sh\necho run\n").unwrap();
            }
        }
    }
}

#[test]
fn perf_cold_scan() {
    let tmp = tempfile::tempdir().unwrap();
    let lib = tmp.path().join("lib");
    build_fixture(&lib);
    let mut env: HashMap<String, OsString> = HashMap::new();
    env.insert("HOME".into(), tmp.path().join("home").into());
    env.insert("POUCH_CONFIG_DIR".into(), tmp.path().join("config").into());
    env.insert("POUCH_CACHE_DIR".into(), tmp.path().join("cache").into());
    env.insert("XDG_DATA_HOME".into(), tmp.path().join("data").into());
    let dirs = Dirs::resolve(Env::from_map(env)).unwrap();
    let entries = vec![SourceEntry {
        alias: "perf".into(),
        path: lib.to_string_lossy().into_owned(),
        precedence: 10,
    }];
    let options = LoadOptions {
        use_cache: false,
        git_global: false,
    };
    let mut runs = Vec::new();
    for _ in 0..5 {
        let start = Instant::now();
        let catalog = Catalog::load(&entries, &dirs, options).unwrap();
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(catalog.skills.len(), 500);
        runs.push(ms);
    }
    assert!(
        !tmp.path().join("cache").exists(),
        "cold scan must not write a cache"
    );
    let mut sorted = runs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[2];
    let release = !cfg!(debug_assertions);
    let runs_text = runs
        .iter()
        .map(|r| format!("{r:.1}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "\nperf_cold_scan median_ms={median:.1} runs_ms=[{runs_text}] skills=500 profile={}",
        if release { "release" } else { "debug" }
    );
    let bound = if release { 300.0 } else { 5000.0 };
    assert!(median < bound, "median {median:.1} ms over {bound} ms");
}
