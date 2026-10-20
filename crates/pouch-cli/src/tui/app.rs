//! The picker state machine: `Loading, Picker, Plan, Applying, Result,
//! Picker` plus the `Interrupted` modal, the bundle overlay, and help
//! (spec 6). Pure with respect to the terminal: keys go in through
//! `handle_key`, the screen is drawn from the public fields by `picker` and
//! `views`. Core calls go through `actions`.

use super::actions::{Session, error_text};
use crate::output::{
    ResultView, catalog_warning_set, diag_line, render_plan, render_records, render_result,
};
use pouch_core::apply::ApplyStatus;
use pouch_core::catalog::search::Hit;
use pouch_core::catalog::{FlatDiagnostic, LoadOptions};
use pouch_core::change::{Edit, Outcome, RecoverOutcome};
use pouch_core::config::Config;
use pouch_core::paths::Dirs;
use pouch_core::project::{ProjectRoot, RootDetection, TargetId};
use pouch_core::verify::EntryState;
use ratatui::termion::event::Key;
use ratatui::widgets::ListState;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The footer message when the pending set mixes additions and removals
/// (M6-5): `change::Edit` carries one kind per apply.
pub const MIXED_MESSAGE: &str = "apply additions and removals one at a time; esc clears";

/// How the picker starts.
#[derive(Debug, Clone)]
pub struct Opts {
    pub use_cache: bool,
    pub project: Option<PathBuf>,
    pub target: Option<TargetId>,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Loading,
    Picker,
    Plan,
    Applying,
    Result,
    Interrupted,
    Bundles,
    Naming,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Search,
    Category,
    Results,
}

/// What the terminal loop does after a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    Continue,
    Quit,
    /// Draw the `Applying` frame, then call `run_pending`.
    Run,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunKind {
    Apply,
    Resume,
    Rollback,
}

/// One row of the category tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRow {
    /// Path filter (`""` is "All").
    pub path: String,
    pub label: String,
    pub depth: usize,
    /// Selected over total (the `3/12` of spec 6).
    pub marked: usize,
    pub total: usize,
}

/// The plan view's content.
pub struct PlanState {
    pub edit: Edit,
    pub command: &'static str,
    pub text: String,
    pub scroll: u16,
    pub blocked: bool,
}

pub struct App {
    opts: Opts,
    pub s: Session,
    pub mode: Mode,
    return_mode: Mode,
    pub targets: Vec<TargetId>,
    pub target: TargetId,
    pub query: String,
    pub focus: Pane,
    pub categories: Vec<CategoryRow>,
    pub cat_ix: usize,
    pub hits: Vec<Hit>,
    pub cursor: usize,
    pub list_state: ListState,
    /// Skill IDs to add (pending).
    pub adds: BTreeSet<String>,
    /// Skill IDs to remove (pending).
    pub drops: BTreeSet<String>,
    /// Skill ID to state, from `verify` for the current target.
    states: BTreeMap<String, EntryState>,
    /// Names of foreign entries in the destination.
    foreign: BTreeSet<String>,
    pub message: Option<String>,
    /// Config-level diagnostics from startup (reported with the catalog's).
    config_diags: Vec<FlatDiagnostic>,
    /// The footer summary of catalog and config warnings; it stays until
    /// the picker closes.
    warning: Option<String>,
    /// Set when the starting target was ambiguous; cleared by `t`.
    ambiguity: Option<String>,
    pub plan: Option<PlanState>,
    run: Option<RunKind>,
    pub result_text: String,
    pub result_scroll: u16,
    pub interrupted: Option<TargetId>,
    pub bundle_ix: usize,
    pub name_input: String,
}

/// `N warnings: <first warning>` (the CLI's `warning[code] where: message`
/// line, without its hint), or `None` when there are none.
fn warning_summary(warnings: &[FlatDiagnostic]) -> Option<String> {
    let first = warnings.first()?;
    let line = diag_line(first);
    let head = line.lines().next().unwrap_or_default();
    let n = warnings.len();
    Some(format!(
        "{n} warning{}: {head}",
        if n == 1 { "" } else { "s" }
    ))
}

fn printable(c: char) -> bool {
    !c.is_control()
}

impl App {
    /// A picker in the `Loading` mode: nothing is read until `load`.
    pub fn loading(dirs: Dirs, config: Config, opts: Opts) -> App {
        let placeholder = ProjectRoot {
            path: opts.cwd.clone(),
            how: RootDetection::Explicit,
        };
        let session = Session {
            dirs,
            config,
            root: placeholder,
            catalog: pouch_core::catalog::Catalog::default(),
            bundles: pouch_core::bundle::BundleSet::default(),
            options: LoadOptions::product(opts.use_cache),
        };
        App {
            opts,
            s: session,
            mode: Mode::Loading,
            return_mode: Mode::Picker,
            targets: Session::targets(),
            target: TargetId::ClaudeCode,
            query: String::new(),
            focus: Pane::Search,
            categories: Vec::new(),
            cat_ix: 0,
            hits: Vec::new(),
            cursor: 0,
            list_state: ListState::default(),
            adds: BTreeSet::new(),
            drops: BTreeSet::new(),
            states: BTreeMap::new(),
            foreign: BTreeSet::new(),
            message: None,
            config_diags: Vec::new(),
            warning: None,
            ambiguity: None,
            plan: None,
            run: None,
            result_text: String::new(),
            result_scroll: 0,
            interrupted: None,
            bundle_ix: 0,
            name_input: String::new(),
        }
    }

    /// Record the config-level diagnostics found at startup.
    pub fn set_config_diags(&mut self, diags: Vec<pouch_core::error::Diagnostic>) {
        self.config_diags = diags.into_iter().map(FlatDiagnostic::global).collect();
    }

    /// Load the catalog (synchronously; a cold scan shows the static
    /// `Loading` frame, M6-8) and move to `Picker`, or to `Interrupted` when
    /// a journal exists (M6-7).
    pub fn load(&mut self) -> anyhow::Result<()> {
        self.s = Session::load(
            self.s.dirs.clone(),
            self.s.config.clone(),
            self.s.options,
            self.opts.project.as_deref(),
            &self.opts.cwd,
        )?;
        self.targets = Session::targets();
        let (target, ambiguity) = self.s.initial_target(self.opts.target);
        self.target = target;
        self.ambiguity = ambiguity;
        self.warning = warning_summary(&catalog_warning_set(&self.config_diags, &self.s.catalog));
        self.reload_records();
        self.rebuild_categories();
        self.refilter();
        if let Some(t) = self.s.journal_target() {
            self.interrupted = Some(t);
            self.mode = Mode::Interrupted;
        } else {
            self.mode = Mode::Picker;
        }
        if self.s.config.sources.is_empty() && self.message.is_none() {
            self.message =
                Some("no skill sources yet: run `pouch source add <alias> <path>`".to_string());
        }
        Ok(())
    }

    // ---- derived data ----

    fn reload_records(&mut self) {
        self.states.clear();
        self.foreign.clear();
        match self.s.verify(self.target) {
            Ok(report) => {
                for r in &report.records {
                    match (&r.id, r.state) {
                        (Some(id), st) => {
                            self.states.insert(id.clone(), st);
                        }
                        (None, EntryState::Foreign) => {
                            self.foreign.insert(r.name.clone());
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => self.message = Some(error_text(&e)),
        }
    }

    /// Skill IDs currently in the selection or the receipt.
    fn in_set(&self, id: &str) -> bool {
        self.states.contains_key(id)
    }

    /// The marker of spec 6 and M6-11.
    pub fn marker(&self, id: &str, name: &str) -> &'static str {
        if self.adds.contains(id) {
            return "[x]";
        }
        match self.states.get(id) {
            Some(EntryState::Orphaned) => "[!]",
            Some(_) if self.drops.contains(id) => "[\u{25cb}]",
            Some(_) => "[\u{25cf}]",
            None if self.foreign.contains(name) => "[!]",
            None => "[ ]",
        }
    }

    /// The state word of a skill for the preview pane.
    pub fn state_text(&self, id: &str) -> &'static str {
        if self.adds.contains(id) {
            return "pending add";
        }
        match self.states.get(id) {
            Some(st) if self.drops.contains(id) => {
                if *st == EntryState::Orphaned {
                    "orphaned, pending removal"
                } else {
                    "active, pending removal"
                }
            }
            Some(st) => st.as_str(),
            None => "not selected",
        }
    }

    fn marked(&self, id: &str) -> bool {
        self.adds.contains(id) || (self.in_set(id) && !self.drops.contains(id))
    }

    /// Rebuild the category tree and its `3/12` counts, keeping the
    /// selected row when it still exists.
    fn rebuild_categories(&mut self) {
        let keep = self
            .categories
            .get(self.cat_ix)
            .map(|c| c.path.clone())
            .unwrap_or_default();
        let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        let mut all = (0usize, 0usize);
        for skill in &self.s.catalog.skills {
            let id = skill.id.to_string();
            let marked = usize::from(self.marked(&id));
            all.0 += marked;
            all.1 += 1;
            let mut prefix = String::new();
            for part in skill.category.split('/').filter(|p| !p.is_empty()) {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(part);
                let e = counts.entry(prefix.clone()).or_insert((0, 0));
                e.0 += marked;
                e.1 += 1;
            }
        }
        let mut rows = vec![CategoryRow {
            path: String::new(),
            label: "All".to_string(),
            depth: 0,
            marked: all.0,
            total: all.1,
        }];
        for (path, (marked, total)) in counts {
            let depth = path.matches('/').count() + 1;
            let label = path.rsplit('/').next().unwrap_or(&path).to_string();
            rows.push(CategoryRow {
                path,
                label,
                depth,
                marked,
                total,
            });
        }
        self.cat_ix = rows.iter().position(|r| r.path == keep).unwrap_or(0);
        self.categories = rows;
    }

    fn refilter(&mut self) {
        let cat = self
            .categories
            .get(self.cat_ix)
            .map(|c| c.path.as_str())
            .filter(|p| !p.is_empty());
        self.hits = self.s.search(&self.query, cat);
        self.cursor = 0;
    }

    /// The skill under the cursor.
    pub fn focused_skill(&self) -> Option<&pouch_core::catalog::Skill> {
        self.hits
            .get(self.cursor)
            .and_then(|h| self.s.catalog.skill(&h.id))
    }

    /// The footer's message line: the transient message, else the standing
    /// notes (target ambiguity, then the catalog warning summary).
    pub fn footer_message(&self) -> Option<String> {
        if let Some(m) = &self.message {
            return Some(m.clone());
        }
        let notes: Vec<&str> = [self.ambiguity.as_deref(), self.warning.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        if notes.is_empty() {
            None
        } else {
            Some(notes.join(" | "))
        }
    }

    /// The footer's counts (spec 6: pending change counts and catalog size).
    pub fn footer_counts(&self) -> String {
        let active = self
            .states
            .values()
            .filter(|s| matches!(s, EntryState::Active | EntryState::Applied))
            .count();
        format!(
            "{} to add, {} to remove | {} active | catalog {} skills | target {}",
            self.adds.len(),
            self.drops.len(),
            active,
            self.s.catalog.skills.len(),
            self.target.as_str()
        )
    }

    // ---- keys ----

    /// Handle one key. `Ctrl-C` quits from every mode.
    pub fn handle_key(&mut self, key: Key) -> Effect {
        if key == Key::Ctrl('c') {
            return Effect::Quit;
        }
        self.message = None;
        match self.mode {
            Mode::Loading | Mode::Applying => Effect::Continue,
            Mode::Picker => self.key_picker(key),
            Mode::Plan => self.key_plan(key),
            Mode::Result => self.key_result(key),
            Mode::Interrupted => self.key_interrupted(key),
            Mode::Bundles => self.key_bundles(key),
            Mode::Naming => self.key_naming(key),
            Mode::Help => {
                self.mode = self.return_mode;
                Effect::Continue
            }
        }
    }

    fn clear_all(&mut self) {
        self.query.clear();
        self.cat_ix = 0;
        self.adds.clear();
        self.drops.clear();
        self.rebuild_categories();
        self.refilter();
    }

    fn key_picker(&mut self, key: Key) -> Effect {
        match key {
            Key::Esc => {
                let dirty = !self.query.is_empty()
                    || self.cat_ix != 0
                    || !self.adds.is_empty()
                    || !self.drops.is_empty();
                if dirty {
                    self.clear_all();
                } else {
                    return Effect::Quit;
                }
            }
            Key::Char('\t') => self.cycle_focus(true),
            Key::BackTab => self.cycle_focus(false),
            Key::Left => self.focus = Pane::Category,
            Key::Right => self.focus = Pane::Results,
            Key::Char('\n') => self.enter(),
            Key::Char(' ') => self.toggle(),
            Key::Up => self.move_by(-1),
            Key::Down => self.move_by(1),
            Key::PageUp => self.move_by(-10),
            Key::PageDown => self.move_by(10),
            Key::Home => self.move_by(isize::MIN / 2),
            Key::End => self.move_by(isize::MAX / 2),
            Key::Backspace => {
                if self.query.pop().is_some() {
                    self.refilter();
                }
            }
            Key::Char(c) if printable(c) => {
                if self.focus == Pane::Search {
                    self.type_char(c);
                } else {
                    match c {
                        'b' => self.open_bundles(),
                        't' => self.cycle_target(),
                        '?' => {
                            self.return_mode = Mode::Picker;
                            self.mode = Mode::Help;
                        }
                        _ => self.type_char(c),
                    }
                }
            }
            _ => {}
        }
        Effect::Continue
    }

    fn type_char(&mut self, c: char) {
        self.query.push(c);
        self.refilter();
    }

    fn cycle_focus(&mut self, forward: bool) {
        self.focus = match (self.focus, forward) {
            (Pane::Search, true) | (Pane::Results, false) => Pane::Category,
            (Pane::Category, true) | (Pane::Search, false) => Pane::Results,
            (Pane::Results, true) | (Pane::Category, false) => Pane::Search,
        };
    }

    fn move_by(&mut self, delta: isize) {
        if self.focus == Pane::Category {
            let last = self.categories.len().saturating_sub(1);
            let next = self.cat_ix.saturating_add_signed(delta).min(last);
            if next != self.cat_ix {
                self.cat_ix = next;
                self.refilter();
            }
        } else {
            let last = self.hits.len().saturating_sub(1);
            self.cursor = self.cursor.saturating_add_signed(delta).min(last);
        }
    }

    fn toggle(&mut self) {
        let Some(hit) = self.hits.get(self.cursor) else {
            self.message = Some("no result to select".to_string());
            return;
        };
        let id = hit.id.to_string();
        let set = if self.in_set(&id) {
            &mut self.drops
        } else {
            &mut self.adds
        };
        if !set.remove(&id) {
            set.insert(id);
        }
        self.rebuild_categories();
    }

    fn enter(&mut self) {
        match (self.adds.is_empty(), self.drops.is_empty()) {
            (true, true) => {
                self.message = Some("nothing selected: press space on a result".to_string());
            }
            (false, false) => self.message = Some(MIXED_MESSAGE.to_string()),
            (false, true) => {
                let refs: Vec<String> = self.adds.iter().cloned().collect();
                self.open_plan(Edit::Add(refs), "add");
            }
            (true, false) => {
                let refs: Vec<String> = self.drops.iter().cloned().collect();
                self.open_plan(Edit::Remove(refs), "rm");
            }
        }
    }

    fn open_plan(&mut self, edit: Edit, command: &'static str) {
        match self.s.preview(self.target, edit.clone(), command) {
            Ok(p) => {
                let note = self.s.refresh_note(self.target, &p.plan);
                self.plan = Some(PlanState {
                    edit,
                    command,
                    text: render_plan(&p.plan, note.as_ref()),
                    scroll: 0,
                    blocked: p.plan.is_blocked(),
                });
                self.mode = Mode::Plan;
            }
            Err(f) => {
                self.message = Some(error_text(&f.error));
                self.mode = Mode::Picker;
            }
        }
    }

    fn key_plan(&mut self, key: Key) -> Effect {
        match key {
            Key::Char('\n') | Key::Char('y') => {
                if self.plan.as_ref().is_some_and(|p| p.blocked) {
                    self.message =
                        Some("the plan has conflicts and cannot be applied; esc goes back".into());
                } else {
                    self.run = Some(RunKind::Apply);
                    self.mode = Mode::Applying;
                    return Effect::Run;
                }
            }
            Key::Char('n') | Key::Esc => {
                self.plan = None;
                self.mode = Mode::Picker;
            }
            Key::Up => self.scroll_plan(-1),
            Key::Down => self.scroll_plan(1),
            Key::PageUp => self.scroll_plan(-10),
            Key::PageDown => self.scroll_plan(10),
            _ => {}
        }
        Effect::Continue
    }

    fn scroll_plan(&mut self, delta: i32) {
        if let Some(p) = self.plan.as_mut() {
            // The upper bound is the wrapped height, which only the drawing
            // code knows; `views::draw` clamps the scroll to it.
            let next = i32::from(p.scroll) + delta;
            p.scroll = u16::try_from(next.max(0)).unwrap_or(u16::MAX);
        }
    }

    fn key_result(&mut self, key: Key) -> Effect {
        match key {
            Key::Char('q') => return Effect::Quit,
            Key::Char('\n') => {
                self.plan = None;
                self.adds.clear();
                self.drops.clear();
                self.reload_records();
                self.rebuild_categories();
                self.refilter();
                self.result_scroll = 0;
                self.mode = Mode::Picker;
            }
            Key::Up => self.result_scroll = self.result_scroll.saturating_sub(1),
            Key::Down => self.result_scroll = self.result_scroll.saturating_add(1),
            Key::PageUp => self.result_scroll = self.result_scroll.saturating_sub(10),
            Key::PageDown => self.result_scroll = self.result_scroll.saturating_add(10),
            _ => {}
        }
        Effect::Continue
    }

    fn key_interrupted(&mut self, key: Key) -> Effect {
        match key {
            Key::Char('r') => {
                self.run = Some(RunKind::Resume);
                self.mode = Mode::Applying;
                Effect::Run
            }
            Key::Char('u') => {
                self.run = Some(RunKind::Rollback);
                self.mode = Mode::Applying;
                Effect::Run
            }
            Key::Esc => Effect::Quit,
            _ => Effect::Continue,
        }
    }

    // ---- bundles ----

    fn open_bundles(&mut self) {
        self.s.reload_bundles();
        self.bundle_ix = 0;
        self.mode = Mode::Bundles;
    }

    fn key_bundles(&mut self, key: Key) -> Effect {
        let count = self.s.bundles.bundles.len();
        match key {
            Key::Esc => self.mode = Mode::Picker,
            Key::Up => self.bundle_ix = self.bundle_ix.saturating_sub(1),
            Key::Down => self.bundle_ix = (self.bundle_ix + 1).min(count.saturating_sub(1)),
            Key::Char('\n') => self.use_bundle(true),
            Key::Char('a') => self.use_bundle(false),
            Key::Char('s') => {
                if self.adds.is_empty() {
                    self.message =
                        Some("nothing to save: press space on results first".to_string());
                } else {
                    self.name_input.clear();
                    self.mode = Mode::Naming;
                }
            }
            _ => {}
        }
        Effect::Continue
    }

    fn use_bundle(&mut self, replace: bool) {
        let Some(slug) = self
            .s
            .bundles
            .bundles
            .get(self.bundle_ix)
            .map(|b| b.slug.clone())
        else {
            self.message = Some("no bundle to use".to_string());
            return;
        };
        let edit = if replace {
            Edit::UseReplace(slug)
        } else {
            Edit::UseAdd(slug)
        };
        self.open_plan(edit, "use");
    }

    fn key_naming(&mut self, key: Key) -> Effect {
        match key {
            Key::Esc => self.mode = Mode::Bundles,
            Key::Backspace => {
                self.name_input.pop();
            }
            Key::Char('\n') => {
                let ids: Vec<String> = self.adds.iter().cloned().collect();
                match self.s.save_bundle(&self.name_input, &ids) {
                    Ok(out) => {
                        self.s.reload_bundles();
                        self.message = Some(format!("saved bundle {}", out.slug));
                        self.mode = Mode::Picker;
                    }
                    Err(e) => self.message = Some(error_text(&e)),
                }
            }
            Key::Char(c) if printable(c) => self.name_input.push(c),
            _ => {}
        }
        Effect::Continue
    }

    fn cycle_target(&mut self) {
        let ix = self
            .targets
            .iter()
            .position(|t| *t == self.target)
            .unwrap_or(0);
        self.target = self.targets[(ix + 1) % self.targets.len()];
        self.ambiguity = None;
        self.adds.clear();
        self.drops.clear();
        self.reload_records();
        self.rebuild_categories();
        self.message = Some(format!("target: {}", self.target.as_str()));
    }

    // ---- pending work ----

    /// Run the work `handle_key` queued (`Effect::Run`): apply, resume, or
    /// rollback. The terminal loop draws the `Applying` frame first.
    pub fn run_pending(&mut self) -> Effect {
        match self.run.take() {
            Some(RunKind::Apply) => self.do_apply(),
            Some(RunKind::Resume) => self.do_recover(true),
            Some(RunKind::Rollback) => self.do_recover(false),
            None => {}
        }
        Effect::Continue
    }

    fn state_list(&self) -> String {
        match self.s.verify(self.target) {
            Ok(r) => render_records(&r.records),
            Err(e) => error_text(&e),
        }
    }

    fn do_apply(&mut self) {
        let Some(plan) = self.plan.as_ref() else {
            self.mode = Mode::Picker;
            return;
        };
        let (edit, command) = (plan.edit.clone(), plan.command);
        let confirmed = plan.text.clone();
        let target = self.target;
        let (result, plan_changed) = self.s.apply(target, edit.clone(), command, &confirmed);
        match result {
            Ok(Outcome::Applied(prepared, report)) => {
                let followups = self.s.followups(target);
                let shared = self.s.shared_consumers(target);
                let mut text = String::new();
                if report.status != ApplyStatus::Applied {
                    text.push_str("Applied with problems: see the lines below.\n\n");
                }
                text.push_str(&render_result(&ResultView {
                    results: &report.results,
                    refresh: Some(&report.refresh),
                    followups: &followups,
                    removal_applied: report.removal_applied,
                    gitignore_lines: &prepared.gitignore_lines,
                    visible_to: &shared,
                }));
                text.push('\n');
                text.push_str(&self.state_list());
                self.finish(text);
            }
            Ok(Outcome::NoChanges(prepared)) => {
                let shared = self.s.shared_consumers(target);
                let mut text =
                    String::from("Nothing to change: the destination already matches.\n");
                text.push_str(&render_result(&ResultView {
                    results: &[],
                    refresh: None,
                    followups: &[],
                    removal_applied: false,
                    gitignore_lines: &prepared.gitignore_lines,
                    visible_to: &shared,
                }));
                text.push('\n');
                text.push_str(&self.state_list());
                self.finish(text);
            }
            Ok(Outcome::Blocked(prepared)) => {
                let note = self.s.refresh_note(target, &prepared.plan);
                if let Some(p) = self.plan.as_mut() {
                    p.text = render_plan(&prepared.plan, note.as_ref());
                    p.blocked = true;
                }
                self.message = Some("the plan has conflicts and cannot be applied".to_string());
                self.mode = Mode::Plan;
            }
            Ok(Outcome::Declined(_)) if plan_changed => {
                // The plan the lock-holder computed is not the one the user
                // confirmed: show the new one and ask again.
                self.open_plan(edit, command);
                self.message = Some("the plan changed; review it again".to_string());
            }
            Ok(Outcome::Declined(_) | Outcome::Refused(_)) => {
                self.message = Some("cancelled: nothing was changed".to_string());
                self.mode = Mode::Picker;
            }
            Ok(Outcome::Interrupted(summary)) => {
                self.interrupted = Some(TargetId::parse(&summary.target).unwrap_or(target));
                self.mode = Mode::Interrupted;
            }
            Err(f) => {
                self.message = Some(error_text(&f.error));
                self.plan = None;
                self.mode = Mode::Picker;
            }
        }
    }

    fn do_recover(&mut self, resume: bool) {
        let Some(target) = self.interrupted else {
            self.mode = Mode::Picker;
            return;
        };
        match self.s.recover(target, resume) {
            Ok(RecoverOutcome::Recovered(report, summary)) => {
                let followups = self.s.followups(target);
                let shared = self.s.shared_consumers(target);
                let mut text = format!(
                    "{} of {} finished.\n",
                    if resume { "Resume" } else { "Rollback" },
                    summary.op_id
                );
                text.push_str(&render_result(&ResultView {
                    results: &report.results,
                    refresh: report.refresh.as_ref(),
                    followups: &followups,
                    removal_applied: report.removal_applied,
                    gitignore_lines: &[],
                    visible_to: &shared,
                }));
                text.push('\n');
                text.push_str(&self.state_list());
                self.interrupted = None;
                self.target = target;
                self.finish(text);
            }
            Ok(RecoverOutcome::Declined(_) | RecoverOutcome::Refused(_)) => {
                self.message = Some("cancelled: nothing was changed".to_string());
                self.mode = Mode::Interrupted;
            }
            Err(f) => {
                self.message = Some(error_text(&f.error));
                self.mode = Mode::Interrupted;
            }
        }
    }

    fn finish(&mut self, text: String) {
        self.result_text = text;
        self.result_scroll = 0;
        self.mode = Mode::Result;
    }
}
