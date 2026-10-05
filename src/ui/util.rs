use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders};

use crate::app::App;

pub fn symbol<'a>(app: &App, nerd: &'a str, ascii: &'a str) -> &'a str {
    if app.icons_ascii { ascii } else { nerd }
}

/// A single-color line of text.
pub fn styled_line(text: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(Span::styled(text.into(), Style::default().fg(color)))
}

/// The bordered frame every main panel sits in, highlighted when focused.
pub fn panel_block<'a>(app: &App, title: impl Into<Line<'a>>, is_focused: bool) -> Block<'a> {
    let theme = &app.theme;
    let border_color = if is_focused {
        theme.border_active
    } else {
        theme.border
    };

    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(title)
}

/// A panel title in `color`, bold while the panel has focus.
pub fn panel_title(title: impl Into<String>, color: Color, is_focused: bool) -> Span<'static> {
    let style = Style::default().fg(color);
    let style = if is_focused {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    };
    Span::styled(title.into(), style)
}
