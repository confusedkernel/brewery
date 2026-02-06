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
            "  Checking status...",
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
            HealthTab::Issues => {
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
                let mut items = if app.pending_command
                    && matches!(
                        app.last_command.as_deref(),
                        Some("install") | Some("uninstall") | Some("upgrade")
                    ) {
                    let spinner = spinner_frame(app);
                    let action = match app.last_command.as_deref() {
                        Some("install") => "Installing",
                        Some("uninstall") => "Uninstalling",
                        Some("upgrade") => "Upgrading",
                        _ => "Running",
                    };
                    let label = app
                        .last_command_target
                        .as_ref()
                        .map(|pkg| format!("{spinner} {action} {pkg}"))
                        .unwrap_or_else(|| format!("{spinner} {action}"));
                    let elapsed = app
                        .command_started_at
                        .map(|t| format!("{}s", t.elapsed().as_secs()))
                        .unwrap_or_else(|| "0s".to_string());
                    let mut items = vec![(format!("{label} ({elapsed})"), theme.accent)];
                    if let Some(target) = app.last_command_target.as_ref() {
                        let command = app.last_command.as_deref().unwrap_or("command");
                        items.push((
                            format!("Command: brew {command} {target}"),
                            theme.text_muted,
                        ));
                    }
                    items.extend(
                        app.last_command_output
                            .iter()
                            .map(|line| (format!("> {line}"), theme.text_muted)),
                    );
                    items
                } else if let Some((label, pkg, completed_at)) = app.last_command_completed.as_ref()
                {
                    if completed_at.elapsed().as_secs() < 3 {
                        let verb = match label.as_str() {
                            "install" => "Install",
                            "uninstall" => "Uninstall",
                            "upgrade" => "Upgrade",
                            _ => "Command",
                        };
                        vec![(format!("{verb} completed: {pkg}"), theme.green)]
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                if items.is_empty() {
                    if let Some(ver) = &health.brew_version {
                        let sep = symbol(app, "·", "|");
                        let info = health
                            .brew_info
                            .as_ref()
                            .map(|value| format!(" {sep} {value}"))
                            .unwrap_or_default();
                        items.push((format!("Version: {ver}{info}"), theme.text_primary));
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

                    if let Some(update_status) = health.brew_update_status.as_ref() {
                        let color = match update_status.as_str() {
                            "Up to date" => theme.green,
                            "Update recommended" => theme.orange,
                            _ => theme.text_muted,
                        };
                        items.push((format!("Brew update: {update_status}"), color));
                    }
                    if let Some(secs) = health.last_brew_update_secs_ago {
                        items.push((
                            format!("Last brew update: {} ago", format_elapsed(secs)),
                            theme.text_muted,
                        ));
                    }

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
                }

                if !app.pending_command {
                    if let Some(error) = app.last_command_error.as_ref() {
                        let label = app.last_command.as_deref().unwrap_or("command");
                        items.push((format!("Last cmd failed: {label}"), theme.red));
                        for line in error.lines().take(6) {
                            items.push((format!("> {line}"), theme.red));
                        }
                    }
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
            "  Press 'h' for status check",
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
    let tabs = [
        ("Activity", HealthTab::Activity),
        ("Issues", HealthTab::Issues),
        ("Outdated", HealthTab::Outdated),
    ];

    let mut title_spans: Vec<Span> = Vec::new();

    for (i, (name, tab)) in tabs.iter().enumerate() {
        let style = if *tab == app.health_tab {
            let modifier = if is_focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            Style::default().fg(theme.accent).add_modifier(modifier)
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

fn spinner_frame(app: &App) -> &'static str {
    if app.icons_ascii {
        const FRAMES: [&str; 4] = ["|", "/", "-", "\\"];
        let index = app.started_at.elapsed().as_millis() / 120;
        FRAMES[(index as usize) % FRAMES.len()]
    } else {
        const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let index = app.started_at.elapsed().as_millis() / 80;
        FRAMES[(index as usize) % FRAMES.len()]
    }
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        return format!("{secs}s");
    }
    if secs < 3600 {
        return format!("{}m", secs / 60);
    }
    if secs < 86_400 {
        return format!("{}h", secs / 3600);
    }
    format!("{}d", secs / 86_400)
}
