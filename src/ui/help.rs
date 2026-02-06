use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

fn build_help_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let keymaps = vec![
        (
            "Navigation",
            vec![
                (
                    if app.icons_ascii {
                        "j / down"
                    } else {
                        "j / ↓"
                    },
                    "Move down",
                ),
                (if app.icons_ascii { "k / up" } else { "k / ↑" }, "Move up"),
                ("Tab", "Next panel"),
                ("S-Tab", "Previous panel"),
                (
                    if app.icons_ascii {
                        "l / left"
                    } else {
                        "l / ←"
                    },
                    "Status tab prev",
                ),
                (
                    if app.icons_ascii {
                        "; / right"
                    } else {
                        "; / →"
                    },
                    "Status tab next",
                ),
            ],
        ),
        (
            "Search",
            vec![("/", "Search leaves"), ("f", "Find packages")],
        ),
        (
            "Actions",
            vec![
                ("Enter", "Load details"),
                ("d", "Load deps/uses"),
                ("i", "Install selected (confirm)"),
                ("u", "Uninstall selected (confirm)"),
                ("U", "Upgrade selected or all outdated (confirm)"),
                ("o", "Toggle outdated-only leaves filter"),
            ],
        ),
        (
            "Data",
            vec![
                ("r", "Refresh leaves"),
                ("s", "Load sizes"),
                ("h", "Status check"),
            ],
        ),
        (
            "Other",
            vec![
                ("t", "Cycle theme"),
                ("Alt+i", "Toggle icons"),
                ("c", "Cleanup"),
                ("a", "Autoremove"),
                ("b", "Bundle dump"),
                ("v", "Toggle view"),
                ("q", "Quit"),
                ("Esc", "Cancel action"),
            ],
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();

    for (index, (section, keys)) in keymaps.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!(" {}", section),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        for (key, desc) in keys.iter() {
            lines.push(Line::from(vec![
                Span::styled(format!("   {:12}", key), Style::default().fg(theme.yellow)),
                Span::styled(*desc, Style::default().fg(theme.text_primary)),
            ]));
        }
        if index + 1 < keymaps.len() {
            lines.push(Line::from(""));
        }
    }

    lines
}

pub fn help_line_count(app: &App) -> usize {
    build_help_lines(app).len()
}

pub fn draw_help_popup(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;
    let area = frame.area();

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let dim_overlay = Block::default().style(Style::default().bg(theme.bg_dim));
    frame.render_widget(dim_overlay, layout[1]);

    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    frame.render_widget(Clear, popup_area);

    let bg_fill = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_fill, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg_main))
        .title(Span::styled(
            " Keymaps ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " Press ? or Esc to close ",
            Style::default().fg(theme.text_muted),
        )));

    let lines = build_help_lines(app);
    let inner = block.inner(popup_area);
    let visible_height = inner.height as usize;
    let max_offset = lines.len().saturating_sub(visible_height);
    let offset = app.help_scroll_offset.min(max_offset);
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(offset)
        .take(visible_height)
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(block)
        .style(Style::default().bg(theme.bg_main));
    frame.render_widget(paragraph, popup_area);
}
