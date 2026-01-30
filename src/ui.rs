use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};

use crate::app::{App, InputMode};
use crate::theme::{Theme, ThemeMode};

pub fn draw(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_block, frame.area());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_header(frame, layout[0], app);
    draw_body(frame, layout[1], app);
    draw_footer(frame, layout[2], app);
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let uptime = app.started_at.elapsed().as_secs();

    let theme_indicator = match app.theme_mode {
        ThemeMode::Auto => "auto",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };

    let title = Line::from(vec![
        Span::styled(
            " brewery ",
            Style::default()
                .fg(theme.text_on_accent)
                .bg(theme.amber)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            "Homebrew console",
            Style::default().fg(theme.text_secondary),
        ),
    ]);

    let status = Line::from(vec![
        Span::styled("status ", Style::default().fg(theme.text_secondary)),
        Span::styled(&app.status, Style::default().fg(theme.hop_green)),
        Span::styled("  |  ", Style::default().fg(theme.border)),
        Span::styled(
            format!("{} ", theme_indicator),
            Style::default().fg(theme.text_secondary),
        ),
        Span::styled(format!("{}s", uptime), Style::default().fg(theme.copper)),
    ]);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(36)])
        .split(area);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(title);
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(status);

    frame.render_widget(header_block, layout[0]);
    frame.render_widget(status_block, layout[1]);
}

fn draw_body(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ])
        .split(area);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(columns[0]);

    let middle_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(columns[1]);

    let search_label = match app.input_mode {
        InputMode::Search => "Search (type, Enter to apply)",
        InputMode::Normal => "Search (/ to focus)",
    };
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(Span::styled(search_label, Style::default().fg(theme.amber)));

    let search_text = if app.search_query.is_empty() {
        Span::styled(
            "type to filter leaves",
            Style::default().fg(theme.text_secondary),
        )
    } else {
        Span::styled(&app.search_query, Style::default().fg(theme.text_primary))
    };

    let search = Paragraph::new(Line::from(vec![search_text]))
        .block(search_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(search, left_rows[0]);

    let leaves = app.filtered_leaves();
    let leaves_title = format!("Leaves ({})", leaves.len());
    let list_items: Vec<ListItem> = if leaves.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No leaves found",
            Style::default().fg(theme.text_secondary),
        )))]
    } else {
        leaves
            .iter()
            .map(|(_, item)| {
                ListItem::new(Line::from(Span::styled(
                    *item,
                    Style::default().fg(theme.text_primary),
                )))
            })
            .collect()
    };

    let leaves_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(leaves_title, Style::default().fg(theme.amber)));
    let leaves_list = List::new(list_items)
        .block(leaves_block)
        .highlight_style(Style::default().bg(theme.amber).fg(theme.text_on_accent));
    let mut list_state = ListState::default();
    let selected_pos = app
        .selected_index
        .and_then(|selected| leaves.iter().position(|(idx, _)| *idx == selected));
    list_state.select(selected_pos);
    frame.render_stateful_widget(leaves_list, left_rows[1], &mut list_state);

    // System panel
    let system = Paragraph::new(vec![
        Line::from(Span::styled(
            "brew --version",
            Style::default().fg(theme.dark_amber),
        )),
        Line::from(Span::styled(
            "brew --prefix",
            Style::default().fg(theme.dark_amber),
        )),
        Line::from(Span::styled(
            "brew doctor",
            Style::default().fg(theme.dark_amber),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press r to refresh",
            Style::default().fg(theme.text_secondary),
        )),
        Line::from(Span::styled(
            "/ to search leaves",
            Style::default().fg(theme.text_secondary),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.copper))
            .style(Style::default().bg(theme.bg_panel))
            .title(Span::styled("System", Style::default().fg(theme.amber))),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(system, middle_rows[0]);

    // Activity panel
    let last_refresh = app
        .last_leaves_refresh
        .map(|instant| format!("{}s ago", instant.elapsed().as_secs()))
        .unwrap_or_else(|| "never".to_string());
    let activity = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("Leaves refresh: {}", last_refresh),
            Style::default().fg(theme.text_primary),
        )),
        Line::from(Span::styled(
            "Queue: empty",
            Style::default().fg(theme.text_primary),
        )),
        Line::from(""),
        Line::from(Span::styled("Errors", Style::default().fg(theme.hop_green))),
        Line::from(Span::styled(
            app.last_error.as_deref().unwrap_or("none"),
            Style::default().fg(theme.text_secondary),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.copper))
            .style(Style::default().bg(theme.bg_panel))
            .title(Span::styled("Activity", Style::default().fg(theme.amber))),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(activity, middle_rows[1]);

    // Details panel
    let details_lines = build_details_lines(app);
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled("Details", Style::default().fg(theme.amber)));
    let details = Paragraph::new(details_lines)
        .block(details_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(details, columns[2]);
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let text = Line::from(vec![
        Span::styled(
            " q ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" quit  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " r ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" refresh  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " Enter ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" details  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " d ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" deps  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " / ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" search  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " t ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" theme  ", Style::default().fg(theme.text_primary)),
        Span::styled(
            " ? ",
            Style::default().bg(theme.amber).fg(theme.text_on_accent),
        ),
        Span::styled(" help", Style::default().fg(theme.text_primary)),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_header));
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn build_details_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let Some(pkg) = app.selected_leaf() else {
        return vec![Line::from(Span::styled(
            "No leaves installed".to_string(),
            Style::default().fg(theme.text_secondary),
        ))];
    };

    let is_pending = app
        .pending_details
        .as_deref()
        .map(|pending| pending == pkg)
        .unwrap_or(false);

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        pkg.to_string(),
        Style::default()
            .fg(theme.amber)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(details) = app.details_cache.get(pkg) {
        if let Some(desc) = details.desc.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                desc.to_string(),
                Style::default().fg(theme.text_primary),
            )));
        }

        if let Some(homepage) = details.homepage.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Homepage".to_string(),
                Style::default().fg(theme.text_secondary),
            )));
            lines.push(Line::from(Span::styled(
                homepage.to_string(),
                Style::default().fg(theme.dark_amber),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Installed: {}", format_list_inline(&details.installed)),
            Style::default().fg(theme.text_primary),
        )));

        if let Some(deps) = details.deps.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Deps ({})", deps.len()),
                Style::default().fg(theme.text_secondary),
            )));
            lines.extend(format_list_multiline(deps, theme));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if is_pending {
                    "Deps: loading..."
                } else {
                    "Deps: press d to load"
                },
                Style::default().fg(theme.text_secondary),
            )));
        }

        if let Some(uses) = details.uses.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("Used by ({})", uses.len()),
                Style::default().fg(theme.text_secondary),
            )));
            lines.extend(format_list_multiline(uses, theme));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if is_pending {
                    "Used by: loading..."
                } else {
                    "Used by: press d to load"
                },
                Style::default().fg(theme.text_secondary),
            )));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            if is_pending {
                "Loading details...".to_string()
            } else {
                "Press Enter to load details".to_string()
            },
            Style::default().fg(theme.text_secondary),
        )));
    }

    lines
}

fn format_list_inline(items: &[String]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    items.join(", ")
}

fn format_list_multiline(items: &[String], theme: &Theme) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled(
            "- none".to_string(),
            Style::default().fg(theme.text_secondary),
        ))];
    }

    items
        .iter()
        .take(8)
        .map(|item| {
            Line::from(Span::styled(
                format!("- {item}"),
                Style::default().fg(theme.text_primary),
            ))
        })
        .collect()
}
