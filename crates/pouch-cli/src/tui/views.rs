//! Drawing dispatch and the non-picker views: loading frame, plan view,
//! result view, the `Interrupted` modal, the bundle overlay, the name
//! prompt, and help (spec 6). Plan and result text come from the same
//! `output::render_plan` and `output::render_result` the CLI prints.

use super::app::{App, Mode};
use super::picker;
use crate::output::{head, neutralize};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Widget, Wrap};

pub const HELP_LINES: &[&str] = &[
    "type          filter the results (search box has focus at start)",
    "up / down     move; in the category pane this changes the filter",
    "space         select or unselect the focused result (any pane)",
    "tab           cycle panes: search, categories, results",
    "enter         go to the plan for the pending selection",
    "b             bundles: use (enter), add (a), save selection (s)",
    "t             switch target",
    "?             this help",
    "esc           clear the filter and selection, then quit",
    "ctrl-c        quit",
    "",
    "b, t, and ? act when the category or results pane has focus;",
    "in the search box they are typed into the query.",
    "Plan view: enter or y applies; n or esc goes back.",
    "Result view: enter returns to the picker; q quits.",
    "Press any key to close this help.",
];

/// Draw the screen for the current mode.
pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    match app.mode {
        Mode::Loading => loading(f, area),
        Mode::Applying => applying(f, area),
        Mode::Picker => picker_screen(f, app, area),
        Mode::Bundles => {
            picker_screen(f, app, area);
            bundles(f, app, area);
        }
        Mode::Naming => {
            picker_screen(f, app, area);
            naming(f, app, area);
        }
        Mode::Plan => plan(f, app, area),
        Mode::Result => result(f, app, area),
        Mode::Interrupted => interrupted(f, app, area),
        Mode::Help => help(f, area),
    }
}

fn picker_screen(f: &mut Frame, app: &mut App, area: Rect) {
    let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(3)]).split(area);
    picker::draw(f, app, rows[0]);
    picker::footer(f, app, rows[1]);
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    }
}

/// The static loading frame (M6-8): the cold scan is synchronous.
fn loading(f: &mut Frame, area: Rect) {
    let r = centered(area, 40, 3);
    f.render_widget(
        Paragraph::new("Loading catalog...").block(Block::default().borders(Borders::ALL)),
        r,
    );
}

fn applying(f: &mut Frame, area: Rect) {
    let r = centered(area, 40, 3);
    f.render_widget(
        Paragraph::new("Applying...").block(Block::default().borders(Borders::ALL)),
        r,
    );
}

fn message_line(app: &App) -> Line<'static> {
    Line::from(
        app.message
            .as_deref()
            .map(|m| neutralize(m).into_owned())
            .unwrap_or_default(),
    )
}

fn text_view(f: &mut Frame, app: &App, area: Rect, title: &str, text: &str, scroll: u16) {
    let rows = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);
    f.render_widget(
        Paragraph::new(text.to_string())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title.to_string()),
            )
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0)),
        rows[0],
    );
    f.render_widget(Paragraph::new(message_line(app)), rows[1]);
}

/// The rows `text` takes when wrapped to `width` columns: it is drawn into
/// a scratch buffer, so the count is the one the real paragraph produces.
fn wrapped_rows(text: &str, width: u16) -> u16 {
    if width == 0 {
        return 0;
    }
    let w = usize::from(width);
    let upper: usize = text
        .lines()
        .map(|l| 2 * (l.chars().count() / w + 1))
        .sum::<usize>()
        + 1;
    let height = u16::try_from(upper).unwrap_or(u16::MAX);
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    Paragraph::new(text.to_string())
        .wrap(Wrap { trim: false })
        .render(area, &mut buf);
    (0..height)
        .rev()
        .find(|y| (0..width).any(|x| buf[(x, *y)].symbol() != " "))
        .map_or(0, |y| y + 1)
}

/// The largest scroll that still fills the text pane (bordered, one row
/// below it for the message line).
fn max_scroll(text: &str, area: Rect) -> u16 {
    let inner_w = area.width.saturating_sub(2);
    let inner_h = area.height.saturating_sub(1).saturating_sub(2);
    wrapped_rows(text, inner_w).saturating_sub(inner_h)
}

fn plan(f: &mut Frame, app: &mut App, area: Rect) {
    if let Some(p) = app.plan.as_mut() {
        p.scroll = p.scroll.min(max_scroll(&p.text, area));
    }
    let (text, scroll) = app
        .plan
        .as_ref()
        .map_or((String::new(), 0), |p| (p.text.clone(), p.scroll));
    text_view(
        f,
        app,
        area,
        " Plan   enter or y: apply   n or esc: back ",
        &text,
        scroll,
    );
}

fn result(f: &mut Frame, app: &mut App, area: Rect) {
    app.result_scroll = app.result_scroll.min(max_scroll(&app.result_text, area));
    text_view(
        f,
        app,
        area,
        " Result   enter: back to the picker   q: quit ",
        &app.result_text,
        app.result_scroll,
    );
}

fn interrupted(f: &mut Frame, app: &App, area: Rect) {
    let target = app.interrupted.map_or("-", |t| t.as_str());
    let lines = vec![
        Line::from(format!(
            "An earlier apply for target {target} was interrupted."
        )),
        Line::from(""),
        Line::from("  r    resume: finish it"),
        Line::from("  u    rollback: undo it"),
        Line::from("  esc  quit without changing anything"),
        Line::from(""),
        message_line(app),
    ];
    let r = centered(area, 64, 11);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Interrupted "),
            )
            .wrap(Wrap { trim: false }),
        r,
    );
}

fn bundles(f: &mut Frame, app: &App, area: Rect) {
    let r = centered(area, 64, 14);
    f.render_widget(Clear, r);
    let items: Vec<ListItem> = if app.s.bundles.bundles.is_empty() {
        vec![ListItem::new("no bundles yet; s saves the selected skills")]
    } else {
        app.s
            .bundles
            .bundles
            .iter()
            .map(|b| {
                ListItem::new(format!(
                    "{}  ({} skills)",
                    head(&b.slug, 40),
                    b.skills.len()
                ))
            })
            .collect()
    };
    let mut state = ListState::default().with_selected(if app.s.bundles.bundles.is_empty() {
        None
    } else {
        Some(app.bundle_ix)
    });
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Bundles ")
                .title_bottom(" enter: use  a: add  s: save selection  esc: close "),
        )
        .highlight_symbol("> ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, r, &mut state);
}

fn naming(f: &mut Frame, app: &App, area: Rect) {
    let r = centered(area, 56, 4);
    f.render_widget(Clear, r);
    f.render_widget(
        Paragraph::new(format!("> {}_", neutralize(&app.name_input))).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Save selection as bundle   enter: save   esc: back "),
        ),
        r,
    );
}

fn help(f: &mut Frame, area: Rect) {
    let lines: Vec<Line> = HELP_LINES.iter().map(|l| Line::from(*l)).collect();
    f.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(" Help ")),
        area,
    );
}
