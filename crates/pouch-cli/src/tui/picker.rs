//! The three-pane picker (spec 6): categories 22 percent, search input and
//! results 48 percent, preview 30 percent, and the footer. Every string
//! that came from a skill is neutralized before it reaches a widget.

use super::app::{App, Pane};
use crate::output::{head, neutralize};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

pub const HINTS: &str = "type: filter  space: select  enter: plan  tab: pane  b: bundles  t: target  ?: help  esc: clear/quit";

fn pane_block(title: &str, focused: bool) -> Block<'_> {
    let style = if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_style(style)
        .title(title.to_string())
}

/// Footer: counts, the message line, and the key hints (three rows).
pub fn footer(f: &mut Frame, app: &App, area: Rect) {
    let lines = vec![
        Line::from(app.footer_counts()),
        Line::from(
            app.footer_message()
                .map(|m| neutralize(&m).into_owned())
                .unwrap_or_default(),
        ),
        Line::styled(HINTS, Style::default().add_modifier(Modifier::DIM)),
    ];
    f.render_widget(Paragraph::new(lines), area);
}

/// The picker body: everything above the footer.
pub fn draw(f: &mut Frame, app: &mut App, area: Rect) {
    let cols = Layout::horizontal([
        Constraint::Percentage(22),
        Constraint::Percentage(48),
        Constraint::Percentage(30),
    ])
    .split(area);
    categories(f, app, cols[0]);
    let center = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).split(cols[1]);
    search_box(f, app, center[0]);
    results(f, app, center[1]);
    preview(f, app, cols[2]);
}

fn categories(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .categories
        .iter()
        .map(|c| {
            let indent = "  ".repeat(c.depth.saturating_sub(1));
            ListItem::new(format!(
                "{indent}{} {}/{}",
                head(&c.label, 24),
                c.marked,
                c.total
            ))
        })
        .collect();
    let mut state = ListState::default().with_selected(Some(app.cat_ix));
    let list = List::new(items)
        .block(pane_block("Categories", app.focus == Pane::Category))
        .highlight_symbol("> ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, area, &mut state);
}

fn search_box(f: &mut Frame, app: &App, area: Rect) {
    let cursor = if app.focus == Pane::Search { "_" } else { "" };
    let text = format!("> {}{cursor}", neutralize(&app.query));
    f.render_widget(
        Paragraph::new(text).block(pane_block("Search", app.focus == Pane::Search)),
        area,
    );
}

fn results(f: &mut Frame, app: &mut App, area: Rect) {
    let items: Vec<ListItem> = app
        .hits
        .iter()
        .filter_map(|h| app.s.catalog.skill(&h.id))
        .map(|s| {
            let id = s.id.to_string();
            ListItem::new(format!(
                "{} {}  {}",
                app.marker(&id, &s.name),
                head(&s.name, 40),
                head(&s.id.source.to_string(), 12)
            ))
        })
        .collect();
    let title = format!("Results ({})", app.hits.len());
    let focused = app.focus == Pane::Results;
    app.list_state.select(if items.is_empty() {
        None
    } else {
        Some(app.cursor)
    });
    let list = List::new(items)
        .block(pane_block(&title, focused))
        .highlight_symbol("> ")
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn preview(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();
    if let Some(s) = app.focused_skill() {
        let id = s.id.to_string();
        lines.push(Line::styled(
            neutralize(&s.name).into_owned(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        lines.push(Line::from(neutralize(&id).into_owned()));
        lines.push(Line::from(format!(
            "category: {}",
            neutralize(if s.category.is_empty() {
                "-"
            } else {
                &s.category
            })
        )));
        lines.push(Line::from(format!("state: {}", app.state_text(&id))));
        lines.push(Line::from(""));
        for l in neutralize(&s.description).lines() {
            lines.push(Line::from(l.to_string()));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(format!(
            "files: SKILL.md + {} supporting",
            s.supporting.len()
        )));
        for file in s.supporting.iter().take(6) {
            lines.push(Line::from(format!("  {}", head(file, 40))));
        }
    } else {
        lines.push(Line::from("no skill to preview"));
    }
    f.render_widget(
        Paragraph::new(lines)
            .block(pane_block("Preview", false))
            .wrap(Wrap { trim: false }),
        area,
    );
}
