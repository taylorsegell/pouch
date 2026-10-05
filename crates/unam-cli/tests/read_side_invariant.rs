//! read-side invariant; milestone M3 reuses the helper in every mutating test.
//!
//! First test of the project (decision D12): register, scan, search, show, and
//! list leave the source fixture's tree hash and stat snapshot unchanged.

// Integration test file: unwrap is acceptable here.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;

#[test]
fn source_tree_unchanged_read_side() {
    let env = TestEnv::new();
    let lib = build_library(&env);
    let before = snapshot_source(&lib);
    let lib_s = lib.to_string_lossy().into_owned();

    for args in [
        vec!["source", "add", "fx", lib_s.as_str()],
        vec!["search", "review"],
        vec!["show", "fx:sdlc/code-review"],
        vec!["source", "list"],
        vec!["--json", "search", "review"],
        vec!["--no-cache", "show", "fx:sdlc/code-review"],
    ] {
        let (code, stdout, stderr) = env.run(&args);
        assert_eq!(
            code, 0,
            "{args:?} failed\nstdout: {stdout}\nstderr: {stderr}"
        );
    }
    let (code, v) = run_json(&env, &["search", "review"]);
    assert_eq!(code, 0);
    assert!(!result_ids(&v).is_empty(), "search found nothing: {v}");
    assert_source_unchanged(&before, &lib);
}
