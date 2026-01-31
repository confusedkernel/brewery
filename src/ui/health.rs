use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, HealthTab};
use crate::ui::util::symbol;

pub fn draw_health_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let mut lines = Vec::new();

    if app.pending_health {
        lines.push(Line::from(Span::styled(
            "  Checking health...",
            Style::default().fg(theme.text_muted),
        )));
    } else if let Some(health) = &app.health {
        let scroll_items: Vec<(String, ratatui::style::Color)> = match app.health_tab {
            HealthTab::Outdated => {
                if health.outdated_packages.is_empty() {
                    vec![(
                        format!("{} All packages up to date", symbol(app, "✓", "ok")),
                        theme.green,
                    )]
                } else {
                    health
                        .outdated_packages
                        .iter()
                        .map(|pkg| (format!("{} {}", symbol(app, "↑", "^"), pkg), theme.orange))
                        .collect()
                }
            }
            HealthTab::Errors => {
                if health.doctor_issues.is_empty() {
                    vec![(
                        format!("{} No issues found", symbol(app, "✓", "ok")),
                        theme.green,
                    )]
                } else {
                    health
                        .doctor_issues
                        .iter()
                        .map(|issue| (issue.clone(), theme.yellow))
                        .collect()
                }
            }
            HealthTab::Activity => {
                let mut items = Vec::new();

                if let Some(ver) = &health.brew_version {
                    items.push((format!("Version: {}", ver), theme.text_primary));
                }

                let doctor_status = match health.doctor_ok {
                    Some(true) => (symbol(app, "✓ Healthy", "ok Healthy"), theme.green),
                    Some(false) => (
                        symbol(app, "⚠ Issues found", "! Issues found"),
                        theme.yellow,
                    ),
                    None => ("? Unknown", theme.text_muted),
                };
                items.push((format!("Doctor: {}", doctor_status.0), doctor_status.1));

                let outdated_status = match health.outdated_count {
                    Some(0) => (
                        format!("{} All up to date", symbol(app, "✓", "ok")),
                        theme.green,
                    ),
                    Some(n) => (
                        format!("{} {} outdated", symbol(app, "↑", "^"), n),
                        theme.orange,
                    ),
                    None => ("? Unknown".to_string(), theme.text_muted),
                };
                items.push((
                    format!("Packages: {}", outdated_status.0),
                    outdated_status.1,
                ));

                if let Some(t) = app.last_health_check {
                    items.push((
                        format!("Last check: {}s ago", t.elapsed().as_secs()),
                        theme.text_muted,
                    ));
                }
                if let Some(t) = app.last_leaves_refresh {
                    items.push((
                        format!("Leaves refresh: {}s ago", t.elapsed().as_secs()),
                        theme.text_muted,
                    ));
                }
                if let Some(t) = app.last_sizes_refresh {
                    items.push((
                        format!("Sizes refresh: {}s ago", t.elapsed().as_secs()),
                        theme.text_muted,
                    ));
                }
                if let Some(cmd) = &app.last_command {
                    items.push((format!("Last cmd: {}", cmd), theme.text_secondary));
                }

                items
            }
        };

        if app.health_scroll_offset > 0 {
            lines.push(Line::from(Span::styled(
                format!(
                    "  {} {} more above",
                    symbol(app, "↑", "^"),
                    app.health_scroll_offset
                ),
                Style::default().fg(theme.text_muted),
            )));
        }

        for (text, color) in scroll_items.iter().skip(app.health_scroll_offset) {
            lines.push(Line::from(Span::styled(
                format!("  {}", text),
                Style::default().fg(*color),
            )));
        }
    } else {
        lines.push(Line::from(Span::styled(
            "  Press 'h' for health check",
            Style::default().fg(theme.text_muted),
        )));
    }

    if let Some(error) = app.last_error.as_deref() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", symbol(app, "✗", "x")),
                Style::default().fg(theme.red),
            ),
            Span::styled(error, Style::default().fg(theme.red)),
        ]));
    }

    let border_color = if is_focused {
        theme.border_active
    } else {
        theme.border
    };
    let title_modifier = if is_focused {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };

    let tabs = [
        ("Activity", HealthTab::Activity),
        ("Errors", HealthTab::Errors),
        ("Outdated", HealthTab::Outdated),
    ];

    let mut title_spans: Vec<Span> = vec![
        Span::styled(
            " Health ",
            Style::default()
                .fg(theme.green)
                .add_modifier(title_modifier),
        ),
        Span::styled("│", Style::default().fg(theme.border)),
    ];

    for (i, (name, tab)) in tabs.iter().enumerate() {
        let style = if *tab == app.health_tab {
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_muted)
        };
        title_spans.push(Span::styled(format!(" {} ", name), style));
        if i < 2 {
            title_spans.push(Span::styled(
                symbol(app, "·", "|"),
                Style::default().fg(theme.border),
            ));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(Line::from(title_spans));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(paragraph, area);
}
