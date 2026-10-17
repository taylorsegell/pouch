//! Picker tests (milestone M6). `unam-cli` is a bin-only crate, so these
//! are in-crate: synthetic `Key` values drive `App::handle_key`, the screen
//! renders to `TestBackend`, and the fixtures are temp directories. Nothing
//! here reads the real HOME, runs `unam` against this repository, or calls
//! an agent CLI.

use super::actions::{Action, Session};
use super::app::{App, Effect, MIXED_MESSAGE, Mode, Opts, Pane};
use super::term;
use super::views;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::termion::event::Key;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use unam_core::catalog::{Catalog, LoadOptions};
use unam_core::change::{ChangeRequest, Decision, Edit};
use unam_core::config::{Config, SourceEntry};
use unam_core::error::{Error, Result as CoreResult};
use unam_core::fsops::{
    DestPath, FsOps, OwnedExpectation, RealFs, RemoveOutcome, RestoreOutcome, StatePath,
};
use unam_core::hash::{Hash, tree_hash};
use unam_core::paths::{Dirs, Env};
use unam_core::plan::Plan;
use unam_core::project::TargetId;
use unam_core::source::SourcePath;
use unam_core::state::SystemClock;
use unam_core::verify::EntryState;

const ENTER: Key = Key::Char('\n');
const SPACE: Key = Key::Char(' ');
const TAB: Key = Key::Char('\t');

fn write(p: &Path, text: &str) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, text).unwrap();
}

fn skill(lib: &Path, rel: &str, name: &str, description: &str) {
    write(
        &lib.join(rel).join("SKILL.md"),
        &format!("---\nname: {name}\ndescription: {description}\n---\n\n# {name}\n\nBody.\n"),
    );
}

/// A temp HOME, config, cache, library, and project.
struct Fx {
    tmp: tempfile::TempDir,
    lib: PathBuf,
    project: PathBuf,
    dirs: Dirs,
    config: Config,
}

fn env_for(base: &Path) -> Dirs {
    let mut env: HashMap<String, OsString> = HashMap::new();
    env.insert("HOME".into(), base.join("home").into());
    env.insert("UNAM_CONFIG_DIR".into(), base.join("config").into());
    env.insert("UNAM_CACHE_DIR".into(), base.join("cache").into());
    env.insert("XDG_DATA_HOME".into(), base.join("data").into());
    Dirs::resolve(Env::from_map(env)).unwrap()
}

impl Fx {
    fn new() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let base = fs::canonicalize(tmp.path()).unwrap();
        let lib = base.join("lib");
        skill(&lib, "sdlc/code-review", "code-review", "Review a diff.");
        skill(&lib, "sdlc/refactor", "refactor", "Refactor with tests.");
        skill(
            &lib,
            "testing/pytest-fixtures",
            "pytest-fixtures",
            "Pytest fixtures.",
        );
        skill(
            &lib,
            "testing/pytest-markers",
            "pytest-markers",
            "Pytest markers.",
        );
        let project = base.join("proj");
        fs::create_dir_all(project.join(".git")).unwrap();
        let dirs = env_for(&base);
        let mut config = Config::default();
        config.sources.push(SourceEntry {
            alias: "fx".into(),
            path: lib.to_string_lossy().into_owned(),
            precedence: 10,
        });
        Fx {
            tmp,
            lib,
            project,
            dirs,
            config,
        }
    }

    fn opts(&self) -> Opts {
        Opts {
            use_cache: false,
            project: Some(self.project.clone()),
            target: None,
            cwd: self.project.clone(),
        }
    }

    fn app(&self) -> App {
        let mut app = App::loading(self.dirs.clone(), self.config.clone(), self.opts());
        app.load().unwrap();
        app
    }

    fn app_for(&self, target: TargetId) -> App {
        let mut o = self.opts();
        o.target = Some(target);
        let mut app = App::loading(self.dirs.clone(), self.config.clone(), o);
        app.load().unwrap();
        app
    }

    fn dest(&self, name: &str) -> PathBuf {
        self.project.join(".claude/skills").join(name)
    }

    fn bundle(&self, file: &str, text: &str) {
        write(&self.dirs.bundles.join(file), text);
    }
}

fn press(app: &mut App, keys: &[Key]) {
    for k in keys {
        let mut effect = app.handle_key(*k);
        while effect == Effect::Run {
            effect = app.run_pending();
        }
    }
}

fn typed(s: &str) -> Vec<Key> {
    s.chars().map(Key::Char).collect()
}

fn render_size(app: &mut App, w: u16, h: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal.draw(|f| views::draw(f, app)).unwrap();
    terminal.backend().to_string()
}

fn render(app: &mut App) -> String {
    render_size(app, 100, 30)
}

fn id_of(app: &App, name: &str) -> String {
    app.s
        .catalog
        .skills
        .iter()
        .find(|s| s.name == name)
        .map(|s| s.id.to_string())
        .unwrap()
}

fn state_of(app: &App, target: TargetId, name: &str) -> Option<EntryState> {
    app.s
        .verify(target)
        .unwrap()
        .records
        .iter()
        .find(|r| r.name == name)
        .map(|r| r.state)
}

fn plant_foreign(project: &Path) -> (PathBuf, Vec<u8>) {
    let p = project.join(".claude/skills/house-style/SKILL.md");
    write(&p, "---\nname: house-style\ndescription: Foreign.\n---\n");
    let bytes = fs::read(&p).unwrap();
    (p, bytes)
}

// ---- AC-44 ----

#[test]
fn ac_44_query_space_enter_enter_yields_active_skill() {
    let fx = Fx::new();
    let (foreign, foreign_bytes) = plant_foreign(&fx.project);
    let source_before = tree_hash(&fx.lib, &[]).unwrap();
    let mut app = fx.app();
    assert_eq!(app.mode, Mode::Picker);
    // Bare `unam` is open: type the query, then count the keys after it.
    press(&mut app, &typed("code-review"));
    assert_eq!(app.hits.len(), 1, "{:?}", app.hits);
    let after_query = [SPACE, ENTER, ENTER];
    assert!(after_query.len() <= 4, "S10: at most four keystrokes");
    press(&mut app, &after_query[..1]);
    assert_eq!(app.adds.len(), 1);
    press(&mut app, &after_query[1..2]);
    assert_eq!(app.mode, Mode::Plan, "the first enter opens the plan view");
    press(&mut app, &after_query[2..]);
    assert_eq!(app.mode, Mode::Result, "the second enter applies");
    // The skill is an active symlink at the destination.
    let dest = fx.dest("code-review");
    assert!(
        fs::symlink_metadata(&dest)
            .unwrap()
            .file_type()
            .is_symlink(),
        "{}",
        dest.display()
    );
    assert_eq!(
        state_of(&app, TargetId::ClaudeCode, "code-review"),
        Some(EntryState::Active)
    );
    // Both invariants: the source is untouched and the foreign file is
    // byte-identical; the destination matches the selection.
    assert_eq!(tree_hash(&fx.lib, &[]).unwrap(), source_before);
    assert_eq!(fs::read(&foreign).unwrap(), foreign_bytes);
    let req = unam_core::verify::VerifyRequest {
        dirs: &app.s.dirs,
        config: &app.s.config,
        catalog: &app.s.catalog,
        bundles: &app.s.bundles,
        root: &app.s.root,
        target: TargetId::ClaudeCode,
    };
    let report = unam_core::reconcile::report(&req, &SystemClock).unwrap();
    assert!(report.divergences.is_empty(), "{:?}", report.divergences);
    assert!(!report.verify.journal_present);
    // The result view lists the six-state table.
    let screen = render(&mut app);
    assert!(screen.contains("Result"), "{screen}");
    assert!(screen.contains("STATE"), "{screen}");
    // enter returns to the picker with the pending set cleared.
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Picker);
    assert!(app.adds.is_empty());
    assert!(render(&mut app).contains("[\u{25cf}] code-review"));
}

#[test]
fn ac_44_second_target_and_ancestor_rows_render() {
    let fx = Fx::new();
    // An ancestor `.agents/skills` that contributes a skill (the project has
    // no repository marker, so the walk reaches the parent that has one).
    let outer = fx.tmp.path().canonicalize().unwrap().join("outer");
    fs::create_dir_all(outer.join(".git")).unwrap();
    skill(
        &outer.join(".agents/skills"),
        "anc-skill",
        "anc-skill",
        "Ancestor skill.",
    );
    let project = outer.join("proj");
    fs::create_dir_all(&project).unwrap();
    // A bundle with an instruction: its brief is a generated row.
    fx.bundle(
        "starter.yaml",
        "name: starter\ndescription: Starter.\nskills:\n  - fx:sdlc/code-review\ninstruction: |\n  Start with the review flow.\n",
    );
    let mut o = fx.opts();
    o.project = Some(project.clone());
    o.cwd = project;
    let mut app = App::loading(fx.dirs.clone(), fx.config.clone(), o);
    app.load().unwrap();
    assert_eq!(app.target, TargetId::ClaudeCode);
    // `t` (results pane) switches to the second target.
    press(&mut app, &[TAB, TAB, Key::Char('t')]);
    assert_eq!(app.target, TargetId::AgentsNeutral);
    assert_eq!(app.message.as_deref(), Some("target: agents-neutral"));
    // Bundles overlay: enter uses the bundle (replace) and opens the plan.
    press(&mut app, &[Key::Char('b')]);
    assert_eq!(app.mode, Mode::Bundles);
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    let text = app.plan.as_ref().unwrap().text.clone();
    assert!(text.contains("ancestor-skills"), "{text}");
    assert!(text.contains("generated  starter-brief"), "{text}");
    assert!(text.contains("Start with the review flow."), "{text}");
    // Informational lines are text in the plan view, never selectable skills.
    assert!(app.hits.iter().all(|h| h.id.source.to_string() == "fx"));
    let screen = render(&mut app);
    assert!(screen.contains("ancestor-skills"), "{screen}");
}

// ---- state machine ----

#[test]
fn tui_state_loading_to_picker() {
    let fx = Fx::new();
    let mut app = App::loading(fx.dirs.clone(), fx.config.clone(), fx.opts());
    assert_eq!(app.mode, Mode::Loading);
    let frame = render(&mut app);
    assert!(frame.contains("Loading catalog..."), "{frame}");
    // Keys do nothing while loading.
    press(&mut app, &[Key::Char('x'), ENTER]);
    assert_eq!(app.mode, Mode::Loading);
    app.load().unwrap();
    assert_eq!(app.mode, Mode::Picker);
    assert_eq!(app.hits.len(), 4);
}

#[test]
fn tui_state_plan_n_and_esc_return_to_picker_and_y_applies() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    press(&mut app, &[Key::Char('n')]);
    assert_eq!(app.mode, Mode::Picker);
    assert_eq!(app.adds.len(), 1, "the pending set survives n");
    press(&mut app, &[ENTER, Key::Esc]);
    assert_eq!(app.mode, Mode::Picker);
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_err());
    press(&mut app, &[ENTER, Key::Char('y')]);
    assert_eq!(app.mode, Mode::Result);
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_ok());
}

#[test]
fn tui_state_result_enter_returns_to_picker_and_q_quits() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER, ENTER]);
    assert_eq!(app.mode, Mode::Result);
    assert_eq!(app.handle_key(Key::Char('q')), Effect::Quit);
    assert_eq!(app.handle_key(ENTER), Effect::Continue);
    assert_eq!(app.mode, Mode::Picker);
}

#[test]
fn tui_state_esc_clears_filter_then_quits() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("pytest"));
    assert_eq!(app.hits.len(), 2);
    assert_eq!(app.handle_key(Key::Esc), Effect::Continue);
    assert!(app.query.is_empty());
    assert_eq!(app.hits.len(), 4);
    assert_eq!(app.handle_key(Key::Esc), Effect::Quit);
}

#[test]
fn tui_state_ctrl_c_quits_from_every_mode() {
    let fx = Fx::new();
    let mut app = fx.app();
    // Picker.
    assert_eq!(app.mode, Mode::Picker);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    // Help (opened from the results pane).
    press(&mut app, &[Key::Right, Key::Char('?')]);
    assert_eq!(app.mode, Mode::Help);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    // Bundle overlay and the name prompt.
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, Key::Right, Key::Char('b')]);
    assert_eq!(app.mode, Mode::Bundles);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    press(&mut app, &[Key::Char('s')]);
    assert_eq!(app.mode, Mode::Naming);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    // Plan view and result view.
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Result);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    // Loading (before the first load) and the Interrupted modal.
    let mut app = App::loading(fx.dirs.clone(), fx.config.clone(), fx.opts());
    assert_eq!(app.mode, Mode::Loading);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
    let fx2 = Fx::new();
    interrupt(&fx2);
    let mut app = fx2.app();
    assert_eq!(app.mode, Mode::Interrupted);
    assert_eq!(app.handle_key(Key::Ctrl('c')), Effect::Quit);
}

#[test]
fn tui_state_key_routing_letters_are_typed_in_search_and_act_elsewhere() {
    let fx = Fx::new();
    let mut app = fx.app();
    assert_eq!(app.focus, Pane::Search);
    // b, t, and ? are typed into the query while the search box has focus.
    press(&mut app, &typed("bt?"));
    assert_eq!(app.query, "bt?");
    assert_eq!(app.mode, Mode::Picker);
    assert_eq!(app.target, TargetId::ClaudeCode);
    press(&mut app, &[Key::Backspace, Key::Backspace, Key::Backspace]);
    assert!(app.query.is_empty());
    // On the results pane the same keys act.
    press(&mut app, &[TAB, TAB]);
    assert_eq!(app.focus, Pane::Results);
    press(&mut app, &[Key::Char('?')]);
    assert_eq!(app.mode, Mode::Help);
    assert!(render(&mut app).contains("Help"));
    press(&mut app, &[Key::Char('x')]);
    assert_eq!(app.mode, Mode::Picker);
    assert!(
        app.query.is_empty(),
        "the key that closes help is not typed"
    );
    // Space toggles from any pane.
    press(&mut app, &[SPACE]);
    assert_eq!(app.adds.len(), 1);
    press(&mut app, &[TAB, SPACE]);
    assert_eq!(
        app.adds.len(),
        0,
        "space on the focused result from the search pane"
    );
}

#[test]
fn tui_state_category_pane_filters_and_counts() {
    let fx = Fx::new();
    let mut app = fx.app();
    // Rows: All, sdlc, testing.
    let labels: Vec<&str> = app.categories.iter().map(|c| c.label.as_str()).collect();
    assert_eq!(labels, ["All", "sdlc", "testing"]);
    press(&mut app, &[TAB, Key::Down]);
    assert_eq!(app.focus, Pane::Category);
    assert_eq!(app.hits.len(), 2);
    assert!(
        app.hits
            .iter()
            .all(|h| h.id.rel_path.as_str().starts_with("sdlc/"))
    );
    press(&mut app, &[TAB, SPACE]);
    let sdlc = &app.categories[1];
    assert_eq!((sdlc.marked, sdlc.total), (1, 2));
    assert_eq!((app.categories[0].marked, app.categories[0].total), (1, 4));
    press(&mut app, &[Key::Esc]);
    assert_eq!(app.hits.len(), 4);
    assert!(app.adds.is_empty());
}

#[test]
fn tui_state_mixed_pending_shows_message_and_opens_no_plan() {
    let fx = Fx::new();
    let mut app = fx.app();
    // refactor becomes active, then is marked for removal while another
    // skill is a pending add.
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER, ENTER, ENTER, Key::Esc]);
    assert_eq!(app.mode, Mode::Picker);
    assert!(app.query.is_empty());
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE]);
    assert_eq!(app.drops.len(), 1);
    press(&mut app, &[Key::Esc]);
    assert!(app.drops.is_empty(), "esc clears the pending sets");
    app.drops.insert(id_of(&app, "refactor"));
    app.adds.insert(id_of(&app, "code-review"));
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Picker);
    assert!(app.plan.is_none());
    assert_eq!(app.message.as_deref(), Some(MIXED_MESSAGE));
    assert!(render(&mut app).contains("one at a time"));
    // esc clears both sets.
    press(&mut app, &[Key::Esc]);
    assert!(app.adds.is_empty() && app.drops.is_empty());
}

#[test]
fn tui_state_removal_flow_marks_and_removes() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER, ENTER, ENTER, Key::Esc]);
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_ok());
    press(&mut app, &typed("refactor"));
    assert_eq!(app.hits.len(), 1);
    assert!(render(&mut app).contains("[\u{25cf}] refactor"));
    press(&mut app, &[SPACE]);
    assert!(render(&mut app).contains("[\u{25cb}] refactor"));
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    assert!(
        app.plan
            .as_ref()
            .unwrap()
            .text
            .contains("remove, tool-owned")
    );
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Result);
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_err());
    assert!(app.result_text.contains("Removed from future discovery"));
}

#[test]
fn tui_state_locked_error_is_shown_in_the_footer() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    // Another holder of the project lock.
    let _lock =
        unam_core::state::ProjectLock::acquire(&app.s.root.path, "test", &SystemClock).unwrap();
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Picker);
    let message = app.message.clone().unwrap();
    assert!(message.starts_with("error:"), "{message}");
    assert!(message.to_lowercase().contains("lock"), "{message}");
    assert!(render(&mut app).contains("error:"));
    assert_eq!(app.adds.len(), 1, "the pending set is kept for a retry");
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_err());
}

#[test]
fn tui_state_blocked_plan_cannot_be_applied() {
    let fx = Fx::new();
    // A foreign skill with the same name as a library skill.
    write(
        &fx.dest("refactor").join("SKILL.md"),
        "---\nname: refactor\ndescription: Foreign.\n---\n",
    );
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    assert!(render(&mut app).contains("[!] refactor"));
    press(&mut app, &[SPACE, ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    assert!(app.plan.as_ref().unwrap().blocked);
    press(&mut app, &[ENTER]);
    assert_eq!(
        app.mode,
        Mode::Plan,
        "a blocked plan stays in the plan view"
    );
    assert!(app.message.as_deref().unwrap().contains("conflicts"));
    assert!(fx.dest("refactor").join("SKILL.md").is_file());
    assert!(fs::symlink_metadata(fx.dest("refactor")).unwrap().is_dir());
}

#[test]
fn tui_state_bundle_save_and_use() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("pytest"));
    press(&mut app, &[SPACE, Key::Down, SPACE]);
    assert_eq!(app.adds.len(), 2);
    // Results pane: b opens the overlay; s asks for a name.
    press(&mut app, &[TAB, TAB, Key::Char('b')]);
    assert_eq!(app.mode, Mode::Bundles);
    press(&mut app, &[Key::Char('s')]);
    assert_eq!(app.mode, Mode::Naming);
    press(&mut app, &typed("Py Tools"));
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Picker);
    assert_eq!(app.message.as_deref(), Some("saved bundle py-tools"));
    assert!(fx.dirs.bundles.join("py-tools.yaml").is_file());
    // Use the saved bundle (replace) from the overlay.
    press(&mut app, &[Key::Esc]);
    press(&mut app, &[TAB, TAB, Key::Char('b'), ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Result);
    assert!(fs::symlink_metadata(fx.dest("pytest-fixtures")).is_ok());
    assert!(fs::symlink_metadata(fx.dest("pytest-markers")).is_ok());
    // Saving with nothing selected is refused with a message.
    press(&mut app, &[ENTER]);
    app.focus = Pane::Results;
    press(&mut app, &[Key::Char('b'), Key::Char('s')]);
    assert_eq!(app.mode, Mode::Bundles);
    assert!(app.message.as_deref().unwrap().contains("nothing to save"));
}

#[test]
fn tui_state_untrusted_text_is_neutralized() {
    let fx = Fx::new();
    write(
        &fx.lib.join("evil/bad/SKILL.md"),
        "---\nname: bad\ndescription: \"red \\x1b[31m text\"\n---\n\n# bad\n",
    );
    let mut app = fx.app();
    press(&mut app, &typed("bad"));
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| views::draw(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();
    assert!(screen.contains("\\x1b[31m"), "{screen}");
    assert!(!screen.contains('\u{1b}'));
    let buffer = terminal.backend().buffer();
    assert!(
        buffer
            .content
            .iter()
            .all(|c| !c.symbol().contains('\u{1b}'))
    );
}

#[test]
fn tui_state_initial_target_prefers_flag() {
    let fx = Fx::new();
    let app = fx.app_for(TargetId::AgentsNeutral);
    assert_eq!(app.target, TargetId::AgentsNeutral);
    assert_eq!(Session::targets().len(), 2);
}

// ---- Interrupted modal ----

/// `RealFs` that fails before the `n`th mutating call (the process "dies").
struct FailAt {
    n: u32,
    count: std::cell::Cell<u32>,
}

impl FailAt {
    fn hit(&self) -> CoreResult<()> {
        let k = self.count.get() + 1;
        self.count.set(k);
        if k == self.n {
            Err(Error::io("injected", std::io::Error::other("x")))
        } else {
            Ok(())
        }
    }
}

impl FsOps for FailAt {
    fn write_state(&self, p: &StatePath, b: &[u8]) -> CoreResult<()> {
        self.hit()?;
        RealFs.write_state(p, b)
    }
    fn append_state(&self, p: &StatePath, b: &[u8]) -> CoreResult<()> {
        self.hit()?;
        RealFs.append_state(p, b)
    }
    fn remove_state(&self, p: &StatePath) -> CoreResult<()> {
        self.hit()?;
        RealFs.remove_state(p)
    }
    fn remove_state_dir_if_empty(&self, p: &StatePath) -> CoreResult<bool> {
        self.hit()?;
        RealFs.remove_state_dir_if_empty(p)
    }
    fn create_dir(&self, d: &DestPath) -> CoreResult<bool> {
        self.hit()?;
        RealFs.create_dir(d)
    }
    fn rmdir_if_empty(&self, d: &DestPath) -> CoreResult<bool> {
        self.hit()?;
        RealFs.rmdir_if_empty(d)
    }
    fn symlink_dir(&self, s: &SourcePath, d: &DestPath) -> CoreResult<()> {
        self.hit()?;
        RealFs.symlink_dir(s, d)
    }
    fn copy_tree(&self, s: &SourcePath, d: &DestPath, tag: &str, e: &Hash) -> CoreResult<Hash> {
        self.hit()?;
        RealFs.copy_tree(s, d, tag, e)
    }
    fn remove_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
    ) -> CoreResult<RemoveOutcome> {
        self.hit()?;
        RealFs.remove_owned(d, e, t)
    }
    fn restore_owned(
        &self,
        d: &DestPath,
        e: &OwnedExpectation,
        t: &StatePath,
        s: Option<&SourcePath>,
        tag: &str,
    ) -> CoreResult<RestoreOutcome> {
        self.hit()?;
        RealFs.restore_owned(d, e, t, s, tag)
    }
    fn write_generated(&self, d: &DestPath, c: &[u8], tag: &str) -> CoreResult<()> {
        self.hit()?;
        RealFs.write_generated(d, c, tag)
    }
    fn remove_generated(&self, d: &DestPath, e: &Hash, t: &StatePath) -> CoreResult<RemoveOutcome> {
        self.hit()?;
        RealFs.remove_generated(d, e, t)
    }
}

/// Leave an interrupted `add` of two skills in the project: the apply fails
/// before its `n`th mutating call. Returns the first `n` that leaves a
/// journal behind.
fn interrupt(fx: &Fx) -> u32 {
    let catalog = Catalog::load(&fx.config.sources, &fx.dirs, LoadOptions::product(false)).unwrap();
    let bundles = unam_core::bundle::BundleSet::default();
    let root = unam_core::change::resolve_root(Some(&fx.project), &fx.project).unwrap();
    for n in 1..40 {
        let fail = FailAt {
            n,
            count: std::cell::Cell::new(0),
        };
        let req = ChangeRequest {
            dirs: &fx.dirs,
            config: &fx.config,
            catalog: &catalog,
            bundles: &bundles,
            root: &root,
            target: TargetId::ClaudeCode,
            edit: Edit::Add(vec![
                "fx:sdlc/code-review".into(),
                "fx:sdlc/refactor".into(),
            ]),
            command: "add",
            clock: &SystemClock,
        };
        let mut decide = |_: &Plan| Decision::Proceed;
        let _ = unam_core::change::run(&req, &fail, &mut decide);
        if unam_core::change::has_journal(&root.path, Some(TargetId::ClaudeCode)) {
            return n;
        }
    }
    panic!("no injected failure left a journal");
}

#[test]
fn tui_state_interrupted_modal_resume_finishes_the_apply() {
    let fx = Fx::new();
    interrupt(&fx);
    let mut app = fx.app();
    assert_eq!(
        app.mode,
        Mode::Interrupted,
        "shown before the first picker draw"
    );
    assert_eq!(app.interrupted, Some(TargetId::ClaudeCode));
    // Other keys change nothing.
    press(&mut app, &[Key::Char('x'), ENTER]);
    assert_eq!(app.mode, Mode::Interrupted);
    press(&mut app, &[Key::Char('r')]);
    assert_eq!(app.mode, Mode::Result);
    assert!(!unam_core::change::has_journal(&app.s.root.path, None));
    assert!(app.result_text.contains("Resume of"), "{}", app.result_text);
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Picker);
}

#[test]
fn tui_state_interrupted_modal_rollback_and_esc() {
    let fx = Fx::new();
    interrupt(&fx);
    let mut app = fx.app();
    assert_eq!(app.handle_key(Key::Esc), Effect::Quit);
    press(&mut app, &[Key::Char('u')]);
    assert_eq!(app.mode, Mode::Result);
    assert!(!unam_core::change::has_journal(&app.s.root.path, None));
    assert!(
        app.result_text.contains("Rollback of"),
        "{}",
        app.result_text
    );
    assert!(fs::symlink_metadata(fx.dest("code-review")).is_err());
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_err());
}

// ---- review fix round ----

#[test]
fn tui_state_unreachable_source_warning_shows_in_footer() {
    let mut fx = Fx::new();
    fx.config.sources.push(SourceEntry {
        alias: "gone".into(),
        path: fx
            .tmp
            .path()
            .join("no-such-library")
            .to_string_lossy()
            .into_owned(),
        precedence: 20,
    });
    let mut app = fx.app();
    let frame = render(&mut app);
    assert!(frame.contains("1 warning"), "{frame}");
    assert!(frame.contains("source-unreachable"), "{frame}");
    assert!(frame.contains("gone"), "{frame}");
    // The warning survives a key press.
    press(&mut app, &[Key::Char('x')]);
    assert!(render(&mut app).contains("1 warning"));
}

#[test]
fn tui_state_no_warning_text_without_diagnostics() {
    let fx = Fx::new();
    let mut app = fx.app();
    assert!(!render(&mut app).contains("warning"));
}

#[test]
fn tui_state_plan_changed_between_preview_and_apply_is_not_applied() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER]);
    assert_eq!(app.mode, Mode::Plan);
    let shown = app.plan.as_ref().unwrap().text.clone();
    // The destination appears after the preview, so the re-computed plan differs.
    fs::create_dir_all(fx.project.join(".claude/skills")).unwrap();
    press(&mut app, &[ENTER]);
    assert!(
        fs::symlink_metadata(fx.dest("refactor")).is_err(),
        "nothing is applied for a plan the user never saw"
    );
    let message = app.message.clone().unwrap();
    assert!(message.contains("the plan changed"), "{message}");
    assert_eq!(app.mode, Mode::Plan, "the plan view is reopened");
    let fresh = app.plan.as_ref().unwrap().text.clone();
    assert_ne!(shown, fresh, "the view shows the new plan");
    // The refreshed plan can now be confirmed.
    press(&mut app, &[ENTER]);
    assert_eq!(app.mode, Mode::Result);
    assert!(fs::symlink_metadata(fx.dest("refactor")).is_ok());
}

fn long_lines(n: usize, width: usize) -> String {
    let mut text = String::new();
    for i in 0..n {
        text.push_str(&format!("L{i:03} {}\n", "word ".repeat(width / 5)));
    }
    text.push_str("THE-LAST-LINE\n");
    text
}

#[test]
fn tui_state_plan_scroll_reaches_the_tail_of_wrapped_text() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER]);
    app.plan.as_mut().unwrap().text = long_lines(20, 300);
    assert!(!render(&mut app).contains("THE-LAST-LINE"));
    for _ in 0..400 {
        app.handle_key(Key::Down);
        render(&mut app);
    }
    let frame = render(&mut app);
    assert!(frame.contains("THE-LAST-LINE"), "{frame}");
    // Scrolling back up works from the clamped position (one row at a time).
    let at_end = app.plan.as_ref().unwrap().scroll;
    assert!(at_end > 0);
    app.handle_key(Key::Up);
    render(&mut app);
    assert_eq!(app.plan.as_ref().unwrap().scroll, at_end - 1);
}

#[test]
fn tui_state_result_scroll_cannot_pass_the_end() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER, ENTER]);
    assert_eq!(app.mode, Mode::Result);
    app.result_text = long_lines(40, 60);
    for _ in 0..120 {
        app.handle_key(Key::Down);
        render(&mut app);
    }
    let frame = render(&mut app);
    assert!(frame.contains("THE-LAST-LINE"), "{frame}");
    assert!(app.result_scroll < 120, "scroll {}", app.result_scroll);
    let at_end = app.result_scroll;
    app.handle_key(Key::Down);
    render(&mut app);
    assert_eq!(app.result_scroll, at_end);
    app.handle_key(Key::Up);
    render(&mut app);
    assert_eq!(app.result_scroll, at_end - 1);
}

#[test]
fn tui_state_ambiguous_initial_target_names_the_ambiguity() {
    let fx = Fx::new();
    // Apply to both targets so the selection names two.
    let mut app = fx.app_for(TargetId::ClaudeCode);
    press(&mut app, &typed("refactor"));
    press(&mut app, &[SPACE, ENTER, ENTER, ENTER]);
    press(&mut app, &[Key::Right, Key::Char('t')]);
    assert_eq!(app.target, TargetId::AgentsNeutral);
    press(&mut app, &[Key::Left]);
    press(&mut app, &[Key::Right, SPACE, ENTER, ENTER]);
    assert_eq!(app.mode, Mode::Result);
    // A fresh picker without --target cannot choose.
    let mut fresh = fx.app();
    assert_eq!(
        fresh.target,
        TargetId::ClaudeCode,
        "falls back to the first"
    );
    let frame = render(&mut fresh);
    assert!(frame.contains("ambiguous"), "{frame}");
    assert!(frame.contains("press t"), "{frame}");
    // Switching target clears the note.
    press(&mut fresh, &[Key::Right, Key::Char('t')]);
    assert!(!render(&mut fresh).contains("ambiguous"));
}

#[test]
fn tui_parity_comment_only_needle_fails() {
    let actions_src = "// change::only_in_a_comment\nfn f() {}\n";
    let cli = "fn g() { change::real(); }\n";
    let raw_ok = check_parity(
        actions_src,
        cli,
        &[("change::only_in_a_comment", "change::real")],
    );
    assert!(
        raw_ok.is_ok(),
        "the raw substring check is fooled by a comment"
    );
    let stripped = strip_comments(actions_src);
    assert!(
        check_parity(
            &stripped,
            cli,
            &[("change::only_in_a_comment", "change::real")]
        )
        .is_err()
    );
    let cli_comment = "// change::real\nfn g() {}\n";
    assert!(
        check_parity(
            "fn f() { change::real(); }",
            &strip_comments(cli_comment),
            &[("change::real", "change::real")]
        )
        .is_err()
    );
    // Code on a line before a trailing comment stays; `//!` and `///` go.
    let kept =
        strip_comments("//! doc change::a\n/// doc change::b\nlet x = change::c(); // change::d\n");
    assert!(kept.contains("change::c"));
    assert!(
        !kept.contains("change::a") && !kept.contains("change::b") && !kept.contains("change::d")
    );
}

// ---- parity ----

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read_all(dir: &Path, skip: &[&str]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for e in fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().into_owned();
        if name.ends_with(".rs") && !skip.contains(&name.as_str()) {
            out.push((name, fs::read_to_string(&p).unwrap()));
        }
    }
    out
}

/// `text` without `//` comments (line, doc, and inner doc): everything
/// from the first `//` on each line goes. A `//` inside a string literal
/// also cuts the line, which can only make the parity check stricter.
fn strip_comments(text: &str) -> String {
    text.lines()
        .map(|l| l.find("//").map_or(l, |i| &l[..i]))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `actions.rs` without the parity table, which names every needle and so
/// would match itself.
fn actions_without_table() -> String {
    let src = fs::read_to_string(src_dir().join("tui/actions.rs")).unwrap();
    let begin = src.find("// BEGIN parity table").unwrap();
    let end = src.find("// END parity table").unwrap();
    assert!(begin < end);
    strip_comments(&format!("{}{}", &src[..begin], &src[end..]))
}

fn check_parity(
    actions_src: &str,
    cli_src: &str,
    entries: &[(&str, &str)],
) -> std::result::Result<(), String> {
    for (tui, cli) in entries {
        if !actions_src.contains(tui) {
            return Err(format!("tui/actions.rs does not contain `{tui}`"));
        }
        if !cli_src.contains(cli) {
            return Err(format!("the CLI source does not contain `{cli}`"));
        }
    }
    Ok(())
}

fn cli_source() -> String {
    read_all(&src_dir().join("cli"), &[])
        .into_iter()
        .map(|(_, t)| strip_comments(&t))
        .collect()
}

#[test]
fn tui_parity_every_action_maps_to_a_core_function_the_cli_calls() {
    // A hand-written count: a new variant must be added here and in ALL.
    assert_eq!(Action::ALL.len(), 13);
    let actions_src = actions_without_table();
    let cli = cli_source();
    let entries: Vec<(&str, &str)> = Action::ALL.iter().map(|a| a.core_entry()).collect();
    check_parity(&actions_src, &cli, &entries).unwrap();
    // Every action has a distinct entry pair.
    let mut seen = entries.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), entries.len());
}

#[test]
fn tui_parity_checker_rejects_a_bogus_needle() {
    let actions_src = actions_without_table();
    let cli = cli_source();
    let bad_tui = [("change::no_such_function", "change::run")];
    assert!(check_parity(&actions_src, &cli, &bad_tui).is_err());
    let bad_cli = [("change::run", "change::no_such_function")];
    assert!(check_parity(&actions_src, &cli, &bad_cli).is_err());
}

/// Tokens that mean "this file writes". Assembled from parts so this file
/// never matches itself.
fn write_needles() -> Vec<String> {
    let j = |a: &str, b: &str| format!("{a}{b}");
    vec![
        j("std::", "fs"),
        j("fs::", "write"),
        j("fs::", "create_dir"),
        j("fs::", "remove_"),
        j("fs::", "rename"),
        j("fs::", "copy"),
        j("File::", "create"),
        j("Open", "Options"),
        j("hard_", "link"),
        j("set_", "permissions"),
        j("symlink", "("),
    ]
}

/// Hits in `text`: any write needle, or an `fsops` path token other than
/// `RealFs`.
fn write_hits(text: &str) -> Vec<String> {
    let mut hits: Vec<String> = write_needles()
        .into_iter()
        .filter(|n| text.contains(n.as_str()))
        .collect();
    let fsops = format!("{}::", "fsops");
    for part in text.split(&fsops).skip(1) {
        if !part.starts_with("RealFs") {
            hits.push(format!(
                "{fsops}{}",
                part.chars().take(12).collect::<String>()
            ));
        }
    }
    hits
}

#[test]
fn tui_parity_tui_never_writes_files() {
    let files = read_all(&src_dir().join("tui"), &["tests.rs"]);
    assert!(files.len() >= 5, "{} files scanned", files.len());
    for (name, text) in &files {
        let hits = write_hits(text);
        assert!(hits.is_empty(), "{name}: {hits:?}");
    }
    // `actions.rs` does name `RealFs`, the one allowed way.
    let actions = files.iter().find(|(n, _)| n == "actions.rs").unwrap();
    assert!(actions.1.contains("RealFs"));
}

#[test]
fn tui_parity_write_scan_control_case() {
    let j = |a: &str, b: &str| format!("{a}{b}");
    assert!(!write_hits(&format!("let x = {}(p);", j("std::", "fs::read"))).is_empty());
    assert!(!write_hits(&format!("{}(a, b)", j("fs::", "write"))).is_empty());
    assert!(!write_hits(&format!("use unam_core::{}::DestPath;", j("fs", "ops"))).is_empty());
    assert!(write_hits(&format!("use unam_core::{}::RealFs;", j("fs", "ops"))).is_empty());
    assert!(write_hits("let a = 1;").is_empty());
}

// ---- terminal restore ----

#[test]
fn tui_term_panic_hook_restore_bytes() {
    let bytes = String::from_utf8(term::restore_bytes()).unwrap();
    assert!(bytes.contains("\u{1b}[?1049l"), "{bytes:?}");
    assert!(bytes.contains("\u{1b}[?25h"), "{bytes:?}");
}

#[test]
fn tui_term_panic_hook_child() {
    // Self-skips unless the parent test re-runs this binary with the variable.
    if std::env::var_os("UNAM_TUI_PANIC_CHILD").is_none() {
        return;
    }
    term::install_panic_hook();
    panic!("deliberate panic from the child");
}

#[test]
fn tui_term_panic_hook_writes_restore_before_message() {
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .args([
            "tui::tests::tui_term_panic_hook_child",
            "--exact",
            "--nocapture",
        ])
        .env("UNAM_TUI_PANIC_CHILD", "1")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert_eq!(out.status.code(), Some(101), "{stderr}");
    let leave = stderr
        .find("\u{1b}[?1049l")
        .expect("leave-alternate-screen");
    let message = stderr
        .find("deliberate panic from the child")
        .expect("panic message");
    assert!(leave < message, "restore bytes come first: {stderr:?}");
}

// ---- snapshots ----

#[test]
fn tui_snapshot_picker_layout_22_48_30() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("pytest"));
    let screen = render(&mut app);
    // The pane borders sit at the 22, 48, and 30 column boundaries.
    let first = screen.lines().next().unwrap();
    let cols: Vec<usize> = first
        .chars()
        .enumerate()
        .filter(|(_, c)| *c == '\u{250c}')
        .map(|(i, _)| i - 1)
        .collect();
    assert_eq!(cols, [0, 22, 70], "{first}");
    insta::assert_snapshot!("tui_snapshot_picker_layout_22_48_30", screen);
}

#[test]
fn tui_snapshot_markers_and_category_counts() {
    let fx = Fx::new();
    let mut app = fx.app();
    // refactor and pytest-markers become active.
    for name in ["refactor", "pytest-markers"] {
        press(&mut app, &typed(name));
        press(&mut app, &[SPACE, ENTER, ENTER, ENTER, Key::Esc]);
    }
    // pytest-markers: pending removal; code-review: pending add. The query
    // is then cleared so one frame shows every marker kind.
    press(&mut app, &typed("pytest-markers"));
    press(&mut app, &[SPACE]);
    press(&mut app, &[Key::Backspace; 14]);
    press(&mut app, &typed("code-review"));
    press(&mut app, &[SPACE]);
    press(&mut app, &[Key::Backspace; 11]);
    let screen = render(&mut app);
    assert!(screen.contains("[x] code-review"), "{screen}");
    assert!(screen.contains("[\u{25cf}] refactor"), "{screen}");
    assert!(screen.contains("[\u{25cb}] pytest-markers"), "{screen}");
    insta::assert_snapshot!("tui_snapshot_markers_and_category_counts", screen);
}

#[test]
fn tui_snapshot_plan_view() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("code-review"));
    press(&mut app, &[SPACE, ENTER]);
    let root = app.s.root.path.to_string_lossy().into_owned();
    if let Some(p) = app.plan.as_mut() {
        p.text = p.text.replace(&root, "<project>");
    }
    insta::assert_snapshot!("tui_snapshot_plan_view", render(&mut app));
}

#[test]
fn tui_snapshot_result_view() {
    let fx = Fx::new();
    let mut app = fx.app();
    press(&mut app, &typed("code-review"));
    press(&mut app, &[SPACE, ENTER, ENTER]);
    let lib = fx.lib.to_string_lossy().into_owned();
    app.result_text = app.result_text.replace(&lib, "<library>");
    insta::assert_snapshot!("tui_snapshot_result_view", render(&mut app));
}

#[test]
fn tui_snapshot_interrupted_modal() {
    let fx = Fx::new();
    interrupt(&fx);
    let mut app = fx.app();
    assert_eq!(app.mode, Mode::Interrupted);
    insta::assert_snapshot!("tui_snapshot_interrupted_modal", render(&mut app));
}

#[test]
fn tui_snapshot_bundles_overlay() {
    let fx = Fx::new();
    fx.bundle(
        "starter.yaml",
        "name: starter\ndescription: Starter.\nskills:\n  - fx:sdlc/code-review\n",
    );
    let mut app = fx.app();
    press(&mut app, &[TAB, TAB, Key::Char('b')]);
    insta::assert_snapshot!("tui_snapshot_bundles_overlay", render(&mut app));
}

// ---- performance (spec 9) ----

fn perf_fixture(lib: &Path) {
    for c in 0..10 {
        for s in 0..5 {
            for k in 0..10 {
                let dir = lib.join(format!("cat-{c}/sub-{s}/skill-{c}-{s}-{k}"));
                write(
                    &dir.join("SKILL.md"),
                    &format!(
                        "---\nname: skill-{c}-{s}-{k}\ndescription: Synthetic skill {c} {s} {k} for the picker test.\n---\n\n# Skill\n\nBody text.\n"
                    ),
                );
                write(&dir.join("references/notes.md"), "# Notes\n");
            }
        }
    }
}

fn perf_world(base: &Path) -> (Dirs, Config, Opts) {
    let lib = base.join("lib");
    let project = base.join("proj");
    fs::create_dir_all(project.join(".git")).unwrap();
    let dirs = env_for(base);
    let mut config = Config::default();
    config.sources.push(SourceEntry {
        alias: "perf".into(),
        path: lib.to_string_lossy().into_owned(),
        precedence: 10,
    });
    let opts = Opts {
        use_cache: true,
        project: Some(project.clone()),
        target: None,
        cwd: project,
    };
    (dirs, config, opts)
}

#[test]
fn perf_warm_start_picker() {
    let tmp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(tmp.path()).unwrap();
    perf_fixture(&base.join("lib"));
    let (dirs, config, opts) = perf_world(&base);
    // Priming load: writes the catalog cache.
    let _ = Catalog::load(&config.sources, &dirs, LoadOptions::product(true)).unwrap();
    let mut runs = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        let mut app = App::loading(dirs.clone(), config.clone(), opts.clone());
        app.load().unwrap();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|f| views::draw(f, &mut app)).unwrap();
        runs.push(start.elapsed().as_secs_f64() * 1000.0);
        assert_eq!(app.s.catalog.skills.len(), 500);
    }
    let mut sorted = runs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[2];
    let release = !cfg!(debug_assertions);
    let text = runs
        .iter()
        .map(|r| format!("{r:.1}"))
        .collect::<Vec<_>>()
        .join(", ");
    println!(
        "\nperf_warm_start_picker median_ms={median:.1} runs_ms=[{text}] skills=500 profile={}",
        if release { "release" } else { "debug" }
    );
    let bound = if release { 50.0 } else { 5000.0 };
    assert!(median < bound, "median {median:.1} ms over {bound} ms");
}

#[test]
fn perf_memory_picker_child() {
    // Self-skips unless the parent test sets the fixture path.
    let Some(path) = std::env::var_os("UNAM_PERF_CHILD") else {
        return;
    };
    let base = PathBuf::from(path);
    let (dirs, config, opts) = perf_world(&base);
    let mut app = App::loading(dirs, config, opts);
    app.load().unwrap();
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal.draw(|f| views::draw(f, &mut app)).unwrap();
    println!(
        "perf_memory_picker_child_done skills={}",
        app.s.catalog.skills.len()
    );
}

/// Peak resident set size in bytes from `/usr/bin/time` output.
fn parse_peak_rss(stderr: &str) -> Option<u64> {
    for line in stderr.lines() {
        let line = line.trim();
        // macOS `-l`: "12345678  maximum resident set size" (bytes).
        if line.ends_with("maximum resident set size") {
            return line.split_whitespace().next()?.parse().ok();
        }
        // Linux `-v`: "Maximum resident set size (kbytes): 12345".
        if let Some(rest) = line.strip_prefix("Maximum resident set size (kbytes):") {
            return rest.trim().parse::<u64>().ok().map(|k| k * 1024);
        }
    }
    None
}

#[test]
fn perf_memory_picker_parser() {
    assert_eq!(
        parse_peak_rss("  3604480  maximum resident set size\n"),
        Some(3_604_480)
    );
    assert_eq!(
        parse_peak_rss("\tMaximum resident set size (kbytes): 2048\n"),
        Some(2_097_152)
    );
    assert_eq!(parse_peak_rss("nothing here"), None);
}

#[test]
fn perf_memory_picker() {
    let time = Path::new("/usr/bin/time");
    if !time.exists() {
        println!("perf_memory_picker self-skip: /usr/bin/time is missing (named residual)");
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let base = fs::canonicalize(tmp.path()).unwrap();
    perf_fixture(&base.join("lib"));
    let (dirs, config, _) = perf_world(&base);
    let _ = Catalog::load(&config.sources, &dirs, LoadOptions::product(true)).unwrap();
    let flag = if cfg!(target_os = "macos") {
        "-l"
    } else {
        "-v"
    };
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(time)
        .arg(flag)
        .arg(exe)
        .args([
            "tui::tests::perf_memory_picker_child",
            "--exact",
            "--nocapture",
        ])
        .env("UNAM_PERF_CHILD", &base)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(out.status.success(), "{stdout}\n{stderr}");
    assert!(
        stdout.contains("perf_memory_picker_child_done skills=500"),
        "the child did not run: {stdout}"
    );
    let rss = parse_peak_rss(&stderr).expect("peak RSS in the time output");
    assert!(rss > 1_000_000, "implausible RSS {rss}");
    let release = !cfg!(debug_assertions);
    let mb = rss as f64 / 1_048_576.0;
    println!(
        "\nperf_memory_picker peak_rss_mb={mb:.1} skills=500 profile={}",
        if release { "release" } else { "debug" }
    );
    let bound = if release { 50.0 } else { 200.0 };
    assert!(mb < bound, "peak RSS {mb:.1} MB over {bound} MB");
}
