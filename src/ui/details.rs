use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::{App, InputMode, StatusTab, ViewMode};
use crate::ui::util::{format_size, symbol};

pub fn draw_details_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let details_lines = if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        build_details_lines(app, app.selected_package_result())
    } else if app.status_tab == StatusTab::Services {
        build_service_details_lines(app)
    } else {
        match app.view_mode {
            ViewMode::Details => build_details_lines(app, app.selected_package_name()),
            ViewMode::PackageResults => build_package_results(app),
        }
    };

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

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(
            " Details",
            Style::default()
                .fg(theme.accent)
                .add_modifier(title_modifier),
        ));

    let visible_lines: Vec<Line> = details_lines
        .into_iter()
        .skip(app.details_scroll_offset)
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn build_details_lines(app: &App, pkg: Option<&str>) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let Some(pkg) = pkg else {
        if matches!(
            app.input_mode,
            InputMode::PackageSearch | InputMode::PackageResults
        ) {
            return vec![
                Line::from(""),
                Line::from(Span::styled(
                    "  No results yet".to_string(),
                    Style::default().fg(theme.text_muted),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "  Press Enter to search".to_string(),
                    Style::default().fg(theme.text_muted),
                )),
            ];
        }

        return vec![
            Line::from(""),
            Line::from(Span::styled(
                "  No package selected".to_string(),
                Style::default().fg(theme.text_muted),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  Select a package from the list".to_string(),
                Style::default().fg(theme.text_muted),
            )),
        ];
    };

    let is_pending = app
        .pending_details
        .as_deref()
        .map(|pending| pending == pkg)
        .unwrap_or(false);

    let mut lines = Vec::new();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", pkg),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));

    if let Some(details) = app.details_cache.peek(pkg) {
        if let Some(desc) = details.desc.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  {}", desc),
                Style::default().fg(theme.text_primary),
            )));
        }

        if let Some(homepage) = details.homepage.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Homepage".to_string(),
                Style::default().fg(theme.text_secondary),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {}", homepage),
                Style::default().fg(theme.accent),
            )));
        }

        lines.push(Line::from(""));

        let size_text = app
            .sizes
            .iter()
            .find(|e| e.name == pkg)
            .map(|e| format!(" ({})", format_size(e.size_kb)))
            .unwrap_or_else(|| {
                if app.pending_sizes {
                    " (size: loading...)".to_string()
                } else {
                    " (size: n/a)".to_string()
                }
            });

        lines.push(Line::from(Span::styled(
            format!(
                "  Installed: {}{}",
                format_list_inline(&details.installed),
                size_text
            ),
            Style::default().fg(theme.green),
        )));

        if let Some(latest) = details.latest.as_ref() {
            lines.push(Line::from(Span::styled(
                format!("  Latest: {latest}"),
                Style::default().fg(theme.text_secondary),
            )));
        }

        if let Some(artifacts) = details.artifacts.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Artifacts ({})", artifacts.len()),
                Style::default().fg(theme.orange),
            )));
            lines.extend(format_list_multiline(app, artifacts, theme, "    "));
        }

        let is_cask = details.artifacts.is_some();

        if is_cask {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Dependencies: not available for casks".to_string(),
                Style::default().fg(theme.text_muted),
            )));
        } else if let Some(deps) = details.deps.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Dependencies ({})", deps.len()),
                Style::default().fg(theme.yellow),
            )));
            lines.extend(format_list_multiline(app, deps, theme, "    "));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if is_pending {
                    "  Dependencies: loading...".to_string()
                } else {
                    "  Dependencies: press 'd' to load".to_string()
                },
                Style::default().fg(theme.text_muted),
            )));
        }

        if is_cask {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Used by: not available for casks".to_string(),
                Style::default().fg(theme.text_muted),
            )));
        } else if let Some(uses) = details.uses.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  Used by ({})", uses.len()),
                Style::default().fg(theme.orange),
            )));
            lines.extend(format_list_multiline(app, uses, theme, "    "));
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                if is_pending {
                    "  Used by: loading...".to_string()
                } else {
                    "  Used by: press 'd' to load".to_string()
                },
                Style::default().fg(theme.text_muted),
            )));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            if is_pending {
                "  Loading details...".to_string()
            } else {
                "  Press Enter to load details".to_string()
            },
            Style::default().fg(theme.text_muted),
        )));
    }

    lines
}

fn build_package_results(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = Vec::new();

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "  Search Results".to_string(),
        Style::default()
            .fg(theme.accent_secondary)
            .add_modifier(Modifier::BOLD),
    )));

    if app.package_results.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No results yet".to_string(),
            Style::default().fg(theme.text_muted),
        )));
        lines.push(Line::from(Span::styled(
            "  Press 'f' to search packages".to_string(),
            Style::default().fg(theme.text_muted),
        )));
        return lines;
    }

    lines.push(Line::from(""));
    for item in app.package_results.iter().take(16) {
        lines.push(Line::from(Span::styled(
            format!("  {} {}", symbol(app, "•", "*"), item),
            Style::default().fg(theme.text_primary),
        )));
    }

    lines
}

fn build_service_details_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Service Inspector".to_string(),
            Style::default()
                .fg(theme.accent_secondary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  Filters: {}", app.services_filter_summary()),
            Style::default().fg(theme.text_muted),
        )),
        Line::from(Span::styled(
            "  Actions: [S] start  [X] stop  [R] restart  [I] info".to_string(),
            Style::default().fg(theme.text_secondary),
        )),
        Line::from(Span::styled(
            "  Filter keys: [F] failed-only  [A] auto-start-only  [K] kind".to_string(),
            Style::default().fg(theme.text_secondary),
        )),
    ];

    let Some(service) = app.selected_service_entry() else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  No service selected".to_string(),
            Style::default().fg(theme.text_muted),
        )));
        return lines;
    };

    let state_color = if service.has_failed() {
        theme.red
    } else if service.is_running() {
        theme.green
    } else {
        theme.text_muted
    };
    let exit_code = service
        .exit_code
        .map(|value| value.to_string())
        .unwrap_or_else(|| "n/a".to_string());
    let exit_color = if service.exit_code.is_some_and(|value| value != 0) {
        theme.red
    } else {
        theme.text_secondary
    };

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!("  {}", service.name),
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        format!("  State: {}", service.state_label()),
        Style::default().fg(state_color),
    )));
    lines.push(Line::from(Span::styled(
        format!("  Raw status: {}", service.status),
        Style::default().fg(theme.text_secondary),
    )));
    lines.push(Line::from(Span::styled(
        format!("  Last exit code: {exit_code}"),
        Style::default().fg(exit_color),
    )));

    let user_label = service.user.as_deref().unwrap_or("n/a");
    lines.push(Line::from(Span::styled(
        format!("  User: {user_label}"),
        Style::default().fg(theme.text_secondary),
    )));
    lines.push(Line::from(Span::styled(
        format!("  Backend: {}", app.service_backend_label(&service.name)),
        Style::default().fg(theme.text_secondary),
    )));

    let autostart = if service.auto_start_enabled() {
        "yes"
    } else {
        "no"
    };
    lines.push(Line::from(Span::styled(
        format!("  Auto-start: {autostart}"),
        Style::default().fg(theme.text_secondary),
    )));

    if let Some(file) = service.file.as_deref() {
        lines.push(Line::from(Span::styled(
            format!("  Unit file: {file}"),
            Style::default().fg(theme.text_muted),
        )));
    }

    if service.has_failed() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("  {} Why is this red?", symbol(app, "⚠", "!")),
            Style::default().fg(theme.red).add_modifier(Modifier::BOLD),
        )));
        lines.extend(platform_service_hints(app, &service.name));
    }

    lines
}

fn platform_service_hints(app: &App, service: &str) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = Vec::new();

    if cfg!(target_os = "macos") {
        lines.push(Line::from(Span::styled(
            format!("    {} brew services info {service}", symbol(app, "•", "*")),
            Style::default().fg(theme.text_primary),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "    {} launchctl print gui/$UID/homebrew.mxcl.{service}",
                symbol(app, "•", "*")
            ),
            Style::default().fg(theme.text_primary),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "    {} log show --style compact --predicate 'process CONTAINS \"{service}\"' --last 10m",
                symbol(app, "•", "*")
            ),
            Style::default().fg(theme.text_primary),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            format!(
                "    {} systemctl --user status {service}.service",
                symbol(app, "•", "*")
            ),
            Style::default().fg(theme.text_primary),
        )));
        lines.push(Line::from(Span::styled(
            format!(
                "    {} journalctl --user-unit {service}.service -n 50 --no-pager",
                symbol(app, "•", "*")
            ),
            Style::default().fg(theme.text_primary),
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

fn format_list_multiline(
    app: &App,
    items: &[String],
    theme: &crate::theme::Theme,
    prefix: &str,
) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled(
            format!("{}none", prefix),
            Style::default().fg(theme.text_muted),
        ))];
    }

    items
        .iter()
        .map(|item| {
            Line::from(Span::styled(
                format!("{}{} {}", prefix, symbol(app, "•", "*"), item),
                Style::default().fg(theme.text_primary),
            ))
        })
        .collect()
}
