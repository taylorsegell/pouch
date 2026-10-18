//! Fault injection over the `FsOps` seam (M3 plan P19, G4): `FailingFs`
//! counts EVERY mutating `FsOps` call with one shared counter and, at call
//! `n`, fails (an IO error) or crashes (a panic with a `CrashPayload`)
//! before or after performing it. Crashes are caught with `catch_unwind`
//! so the on-disk state of a killed process is kept exactly.

#![allow(clippy::unwrap_used)]

use super::{TestEnv, dirs_for};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use unam_core::bundle::BundleSet;
use unam_core::catalog::{Catalog, LoadOptions};
use unam_core::change::{ChangeRequest, Edit, RecoverRequest};
use unam_core::config::Config;
use unam_core::error::{Error, Result};
use unam_core::fsops::{
    DestPath, FsOps, OwnedExpectation, RealFs, RemoveOutcome, RestoreOutcome, StatePath,
};
use unam_core::hash::Hash;
use unam_core::paths::Dirs;
use unam_core::project::{ProjectRoot, RootDetection, TargetId};
use unam_core::source::SourcePath;
use unam_core::state::Clock;

/// When to fail or crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailMode {
    None,
    FailBefore(u32),
    FailAfter(u32),
    CrashBefore(u32),
    CrashAfter(u32),
}

impl FailMode {
    pub fn all_at(n: u32) -> [FailMode; 4] {
        [
            FailMode::FailBefore(n),
            FailMode::FailAfter(n),
            FailMode::CrashBefore(n),
            FailMode::CrashAfter(n),
        ]
    }
}

/// The panic payload of a simulated crash.
#[derive(Debug)]
pub struct CrashPayload;

/// Silence the default panic message for simulated crashes only.
pub fn quiet_crashes() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let default = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if info.payload().downcast_ref::<CrashPayload>().is_none() {
                default(info);
            }
        }));
    });
}

/// True when `result` is a caught simulated crash.
pub fn crashed<T>(result: &std::result::Result<T, Box<dyn Any + Send>>) -> bool {
    matches!(result, Err(p) if p.downcast_ref::<CrashPayload>().is_some())
}

/// `RealFs` with one shared call counter over every mutating method.
pub struct FailingFs {
    mode: FailMode,
    count: Cell<u32>,
    calls: RefCell<Vec<(u32, &'static str, PathBuf)>>,
}

impl FailingFs {
    pub fn new(mode: FailMode) -> Self {
        FailingFs {
            mode,
            count: Cell::new(0),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// The number of mutating calls so far.
    pub fn total(&self) -> u32 {
        self.count.get()
    }

    /// `(index, method, path)` of every mutating call.
    pub fn calls(&self) -> Vec<(u32, &'static str, PathBuf)> {
        self.calls.borrow().clone()
    }

    fn call<T>(
        &self,
        method: &'static str,
        path: &Path,
        f: impl FnOnce() -> Result<T>,
    ) -> Result<T> {
        let n = self.count.get() + 1;
        self.count.set(n);
        self.calls
            .borrow_mut()
            .push((n, method, path.to_path_buf()));
        let injected = || Error::io(format!("injected at call {n}"), std::io::Error::other("x"));
        match self.mode {
            FailMode::FailBefore(k) if k == n => Err(injected()),
            FailMode::CrashBefore(k) if k == n => std::panic::panic_any(CrashPayload),
            FailMode::FailAfter(k) if k == n => {
                f()?;
                Err(injected())
            }
            FailMode::CrashAfter(k) if k == n => {
                let _ = f();
                std::panic::panic_any(CrashPayload)
            }
            _ => f(),
        }
    }
}

impl FsOps for FailingFs {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.call("write_state", p.path(), || RealFs.write_state(p, b))
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> Result<()> {
        self.call("append_state", p.path(), || RealFs.append_state(p, b))
    }
    fn remove_state(&self, p: &StatePath) -> Result<()> {
        self.call("remove_state", p.path(), || RealFs.remove_state(p))
    }
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> Result<bool> {
        self.call("remove_state_dir_if_empty", p.path(), || {
            RealFs.remove_state_dir_if_empty(p)
        })
    }
    fn create_dir(&self, d: &DestPath) -> Result<bool> {
        self.call("create_dir", d.path(), || RealFs.create_dir(d))
    }
    fn rmdir_if_empty(&self, d: &DestPath) -> Result<bool> {
        self.call("rmdir_if_empty", d.path(), || RealFs.rmdir_if_empty(d))
    }
    fn symlink_dir(&self, s: &SourcePath, d: &DestPath) -> Result<()> {
        self.call("symlink_dir", d.path(), || RealFs.symlink_dir(s, d))
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, h: &Hash) -> Result<Hash> {
        self.call("copy_tree", d.path(), || RealFs.copy_tree(s, d, tag, h))
    }
    fn remove_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
    ) -> Result<RemoveOutcome> {
        self.call("remove_owned", d.path(), || RealFs.remove_owned(d, e, t))
    }
    fn restore_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
        s: Option<&SourcePath>,
        tag: &str,
    ) -> Result<RestoreOutcome> {
        self.call("restore_owned", d.path(), || {
            RealFs.restore_owned(d, e, t, s, tag)
        })
    }
    fn write_generated(&self, d: &DestPath, c: &[u8], tag: &str) -> Result<()> {
        self.call("write_generated", d.path(), || {
            RealFs.write_generated(d, c, tag)
        })
    }
    fn remove_generated(&self, d: &DestPath, h: &Hash, t: &StatePath) -> Result<RemoveOutcome> {
        self.call("remove_generated", d.path(), || {
            RealFs.remove_generated(d, h, t)
        })
    }
}

/// A clock with a fixed start that steps one millisecond per call.
pub struct StepClock {
    millis: Cell<u64>,
}

impl StepClock {
    pub fn new() -> Self {
        Self::starting_at(1_759_622_400_000)
    }

    /// A clock starting at another instant (distinct operation ids).
    pub fn starting_at(millis: u64) -> Self {
        StepClock {
            millis: Cell::new(millis),
        }
    }
}

impl Default for StepClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for StepClock {
    fn now_rfc3339(&self) -> String {
        let v = self.millis.get();
        self.millis.set(v + 1);
        format!("2026-10-05T00:00:{:02}.{:03}Z", (v / 1000) % 60, v % 1000)
    }
    fn now_millis(&self) -> u64 {
        let v = self.millis.get();
        self.millis.set(v + 1);
        v
    }
    fn entropy(&self) -> u32 {
        0x00c0_ffee
    }
}

/// Everything the core API needs, loaded in process from the `TestEnv`.
pub struct CoreCtx {
    pub dirs: Dirs,
    pub config: Config,
    pub catalog: Catalog,
    pub bundles: BundleSet,
    pub root: ProjectRoot,
}

/// Load config, catalog, and bundles for `project` through `dirs_for`.
pub fn core_ctx(env: &TestEnv, project: &Path) -> CoreCtx {
    let dirs = dirs_for(env);
    let (config, _) = Config::load_or_create(&dirs).unwrap();
    let catalog = Catalog::load(
        &config.sources,
        &dirs,
        LoadOptions {
            use_cache: false,
            git_global: false,
        },
    )
    .unwrap();
    let bundles = unam_core::bundle::load_dir(&dirs);
    CoreCtx {
        dirs,
        config,
        catalog,
        bundles,
        root: ProjectRoot {
            path: std::fs::canonicalize(project).unwrap(),
            how: RootDetection::Git,
        },
    }
}

impl CoreCtx {
    /// A `ChangeRequest` for the claude-code target.
    pub fn request<'a>(
        &'a self,
        edit: Edit,
        command: &'a str,
        clock: &'a dyn Clock,
    ) -> ChangeRequest<'a> {
        self.request_for(TargetId::ClaudeCode, edit, command, clock)
    }

    /// A `ChangeRequest` for any target.
    pub fn request_for<'a>(
        &'a self,
        target: TargetId,
        edit: Edit,
        command: &'a str,
        clock: &'a dyn Clock,
    ) -> ChangeRequest<'a> {
        ChangeRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            bundles: &self.bundles,
            root: &self.root,
            target,
            edit,
            command,
            clock,
        }
    }

    /// A `RecoverRequest` (target from the journal).
    pub fn recover<'a>(&'a self, clock: &'a dyn Clock) -> RecoverRequest<'a> {
        RecoverRequest {
            dirs: &self.dirs,
            config: &self.config,
            catalog: &self.catalog,
            root: &self.root,
            target: None,
            clock,
        }
    }
}

// ---- scenarios A, B, C (G4, Gap 14) ----

/// A start state plus the change the matrix injects into.
pub struct Scenario {
    pub project: PathBuf,
    pub edit: Edit,
    pub command: &'static str,
    /// Run with `defaults.strategy = copy` (scenario C has no selection).
    pub copy_default: bool,
    /// The target the change runs against (scenarios A to C: claude-code).
    pub target: TargetId,
}

impl Scenario {
    /// The core context for this scenario's project.
    pub fn ctx(&self, env: &TestEnv) -> CoreCtx {
        let mut c = core_ctx(env, &self.project);
        if self.copy_default {
            c.config.defaults.strategy = unam_core::config::Strategy::Copy;
        }
        c
    }

    /// Run the change through the core API.
    pub fn run(
        &self,
        env: &TestEnv,
        fs: &dyn FsOps,
        clock: &dyn Clock,
    ) -> std::result::Result<unam_core::change::Outcome, unam_core::change::ChangeFailure> {
        let c = self.ctx(env);
        let req = c.request_for(self.target, self.edit.clone(), self.command, clock);
        super::in_lock_section(|| {
            unam_core::change::run(&req, fs, &mut |_| unam_core::change::Decision::Proceed)
        })
    }
}

fn core_apply(env: &TestEnv, project: &Path, edit: Edit, copy: bool) {
    core_apply_for(env, project, TargetId::ClaudeCode, edit, copy);
}

/// Apply `edit` to `target` through the core API (a scenario's start state).
pub fn core_apply_for(env: &TestEnv, project: &Path, target: TargetId, edit: Edit, copy: bool) {
    let mut c = core_ctx(env, project);
    if copy {
        c.config.defaults.strategy = unam_core::config::Strategy::Copy;
    }
    // An earlier instant than the change under test, so the start state
    // and the change never share an operation id.
    let clock = StepClock::starting_at(1_700_000_000_000);
    let req = c.request_for(target, edit, "add", &clock);
    let out = super::in_lock_section(|| {
        unam_core::change::run(&req, &RealFs, &mut |_| unam_core::change::Decision::Proceed)
    })
    .map_err(|f| f.error)
    .unwrap();
    assert!(matches!(out, unam_core::change::Outcome::Applied(..)));
}

fn refs(r: &[&str]) -> Vec<String> {
    r.iter().map(|s| s.to_string()).collect()
}

/// Scenario A (copy): two applied skills and one prior history entry; the
/// change `use swap --replace` removes one skill and adds another. The
/// caller writes bundle `swap` once.
pub fn scenario_a(fx: &super::Fx, name: &str) -> Scenario {
    let project = fx.project(name);
    fx.write_selection(&project, "targets:\n  claude-code:\n    strategy: copy\n");
    core_apply(
        &fx.env,
        &project,
        Edit::Add(refs(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"])),
        false,
    );
    Scenario {
        project,
        edit: Edit::UseReplace("swap".into()),
        command: "use",
        copy_default: false,
        target: TargetId::ClaudeCode,
    }
}

/// Scenario B: `deactivate` of an applied project whose receipt records
/// `created_dirs` (`.claude`, `.claude/skills`).
pub fn scenario_b(fx: &super::Fx, name: &str) -> Scenario {
    let project = fx.project(name);
    core_apply(
        &fx.env,
        &project,
        Edit::Add(refs(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"])),
        false,
    );
    Scenario {
        project,
        edit: Edit::Deactivate,
        command: "deactivate",
        copy_default: false,
        target: TargetId::ClaudeCode,
    }
}

/// Scenario C (copy): an empty project (only the `.git` marker); a first
/// `add` of two skills creates `.claude` and `.claude/skills`.
pub fn scenario_c(fx: &super::Fx, name: &str) -> Scenario {
    let project = fx.project(name);
    Scenario {
        project,
        edit: Edit::Add(refs(&["fx:sdlc/code-review", "fx:testing/pytest-fixtures"])),
        command: "add",
        copy_default: true,
        target: TargetId::ClaudeCode,
    }
}

/// Assert `reconcile` reports zero divergence and no leftover journal for
/// the project of `c` (spec 8, PRD S7).
pub fn assert_zero_divergence(c: &CoreCtx, context: &str) {
    assert_zero_divergence_for(c, TargetId::ClaudeCode, context);
}

/// `assert_zero_divergence` for any target (EX8).
pub fn assert_zero_divergence_for(c: &CoreCtx, target: TargetId, context: &str) {
    let req = unam_core::verify::VerifyRequest {
        dirs: &c.dirs,
        config: &c.config,
        catalog: &c.catalog,
        bundles: &c.bundles,
        root: &c.root,
        target,
    };
    let report = unam_core::reconcile::report(&req, &StepClock::new()).unwrap();
    assert!(
        report.divergences.is_empty(),
        "{context}: divergences remain: {:?}",
        report.divergences
    );
    assert!(
        !report.verify.journal_present,
        "{context}: a journal is still present"
    );
}

/// `assert_zero_divergence_for` on a project of `env`, loading the core
/// context itself (the check after an apply in a CLI-level test).
pub fn assert_zero_divergence_in(env: &TestEnv, project: &Path, target: TargetId, context: &str) {
    let c = core_ctx(env, project);
    assert_zero_divergence_for(&c, target, context);
}

/// The three-part ordering invariant of the AC-31 row, checked after an
/// injection and before any recovery.
pub fn assert_ordering_invariant(project: &Path, context: &str) {
    assert_ordering_invariant_for(project, TargetId::ClaudeCode, context);
}

/// `assert_ordering_invariant` for any target (EX8). Generated bundle
/// briefs count like skills: a receipt `generated` entry or a journal
/// `write_generated` step accounts for a directory on disk.
pub fn assert_ordering_invariant_for(project: &Path, target: TargetId, context: &str) {
    let state = project.join(".unam").join(target.as_str());
    let journal = state.join("journal.jsonl");
    let journal_present = journal.exists();
    let receipt: Vec<String> = match std::fs::read(state.join("applied.json")) {
        Ok(b) => {
            let v: serde_json::Value = serde_json::from_slice(&b).unwrap();
            v["entries"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| e["kind"] == "skill" || e["kind"] == "generated")
                .map(|e| e["name"].as_str().unwrap().to_string())
                .collect()
        }
        Err(_) => Vec::new(),
    };
    let mut journal_adds: Vec<String> = Vec::new();
    if journal_present {
        let text = std::fs::read_to_string(&journal).unwrap();
        if let Some(first) = text.lines().next()
            && let Ok(v) = serde_json::from_str::<serde_json::Value>(first)
            && let Some(plan) = v["plan"].as_array()
        {
            for s in plan {
                if s["op"] == "add" || s["op"] == "write_generated" {
                    journal_adds.push(s["name"].as_str().unwrap().to_string());
                }
            }
        }
    }
    let dest = project.join(match target {
        TargetId::ClaudeCode => ".claude/skills",
        TargetId::AgentsNeutral => ".agents/skills",
    });
    let on_disk: Vec<String> = match std::fs::read_dir(&dest) {
        Ok(rd) => rd
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|n| !n.starts_with(".unam-tmp-"))
            .collect(),
        Err(_) => Vec::new(),
    };
    // (i) no tool-owned entry without a receipt entry or a journal Add.
    for n in &on_disk {
        assert!(
            receipt.contains(n) || journal_adds.contains(n),
            "{context}: {n} on disk without a receipt entry or a journal Add record"
        );
    }
    // (ii) no receipt entry names a missing destination unless a journal
    // exists.
    if !journal_present {
        for n in &receipt {
            assert!(
                on_disk.contains(n),
                "{context}: receipt names {n}, missing on disk, and no journal exists"
            );
        }
        // (iii) no journal: the receipt equals the disk.
        let mut a = receipt.clone();
        a.sort();
        let mut b = on_disk.clone();
        b.sort();
        assert_eq!(
            a, b,
            "{context}: receipt differs from the disk without a journal"
        );
    }
}
