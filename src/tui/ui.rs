use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use super::app::App;
use crate::models::{format_time, truncate};

pub struct Theme {
    pub fg: Color,
    pub accent: Color,
    pub dim: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
}

pub fn theme_for(name: &str) -> Theme {
    if name.eq_ignore_ascii_case("light") {
        Theme {
            fg: Color::Black,
            accent: Color::Blue,
            dim: Color::DarkGray,
            highlight_bg: Color::Black,
            highlight_fg: Color::White,
        }
    } else {
        Theme {
            fg: Color::White,
            accent: Color::Cyan,
            dim: Color::DarkGray,
            highlight_bg: Color::White,
            highlight_fg: Color::Black,
        }
    }
}

pub fn draw(frame: &mut Frame, app: &mut App) {
    let theme = theme_for(&app.config.theme);
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(3),    // list
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header: title / search prompt + status message.
    let mut spans = vec![];
    if app.search_mode {
        spans.push(Span::styled(
            format!(" search: {}", app.search_query),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    } else {
        spans.push(Span::styled(
            format!(" yoink — {} items", app.filtered.len()),
            Style::default().fg(theme.accent).add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(msg) = &app.message {
        spans.push(Span::raw("   "));
        spans.push(Span::styled(msg.clone(), Style::default().fg(theme.dim)));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), chunks[0]);

    // List of entries.
    let items = build_items(app, &theme);
    let mut state = ListState::default();
    state.select(Some(app.selected));

    let visible = chunks[1].height as usize;
    if app.selected < app.scroll_offset {
        app.scroll_offset = app.selected;
    }
    if app.selected >= app.scroll_offset + visible {
        app.scroll_offset = app.selected + 1 - visible;
    }
    if !app.filtered.is_empty() && app.scroll_offset >= app.filtered.len() {
        app.scroll_offset = app.filtered.len() - 1;
    }
    *state.offset_mut() = app.scroll_offset;

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .bg(theme.highlight_bg)
                .fg(theme.highlight_fg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, chunks[1], &mut state);

    // Footer: key bindings.
    let help = "j/k move · enter copy · y copy&quit · / search · d delete · D clear · p pin · 1-9 select · q quit";
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(help, Style::default().fg(theme.dim)))),
        chunks[2],
    );
}

fn build_items(app: &App, theme: &Theme) -> Vec<ListItem<'static>> {
    let max = app.config.preview_length;
    app.filtered
        .iter()
        .map(|e| {
            let pin = if e.pinned { "◆ " } else { "  " };
            let line = Line::from(vec![
                Span::styled(format!("{:>4} ", e.id), Style::default().fg(theme.dim)),
                Span::styled(pin, Style::default().fg(theme.accent)),
                Span::styled(
                    format!("{}  ", format_time(e.updated_at)),
                    Style::default().fg(theme.dim),
                ),
                Span::styled(
                    format!("{}  ", truncate(&e.source, 16)),
                    Style::default().fg(theme.accent),
                ),
                Span::styled(e.preview(max), Style::default().fg(theme.fg)),
            ]);
            ListItem::new(line)
        })
        .collect()
}
