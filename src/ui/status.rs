use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, StatusTab, ToastLevel};
use crate::brew::StatusSnapshot;
use crate::ui::util::symbol;

type StatusLine = (String, Color);

pub fn draw_status_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;
    let mut lines = Vec::new();

    if app.pending_status {
        lines.push(Line::from(Span::styled(
            "  Checking status...",
            Style::default().fg(theme.text_muted),
        )));
    } else if let Some(system_status) = &app.system_status {
        let scroll_items = build_tab_items(app, system_status);
        append_scrolled_lines(app, &mut lines, &scroll_items);
    } else {
        lines.push(Line::from(Span::styled(
            "  Press 'h' for status check",
            Style::default().fg(theme.text_muted),
        )));
    }

    append_last_error_line(app, &mut lines);

    let border_color = if is_focused {
        theme.border_active
    } else {
        theme.border
    };
    let tabs = [
        ("Activity", StatusTab::Activity),
        ("Issues", StatusTab::Issues),
        ("Outdated", StatusTab::Outdated),
    ];

    let mut title_spans: Vec<Span> = Vec::new();
    for (i, (name, tab)) in tabs.iter().enumerate() {
        let style = if *tab == app.status_tab {
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

fn build_tab_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    match app.status_tab {
        StatusTab::Activity => build_activity_items(app, system_status),
        StatusTab::Issues => build_issues_items(app, system_status),
        StatusTab::Outdated => build_outdated_items(app, system_status),
    }
}

fn build_outdated_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let theme = &app.theme;
    if system_status.outdated_packages.is_empty() {
        return vec![(
            format!("{} All packages up to date", symbol(app, "✓", "ok")),
            theme.green,
        )];
    }

    system_status
        .outdated_packages
        .iter()
        .map(|pkg| (format!("{} {}", symbol(app, "↑", "^"), pkg), theme.orange))
        .collect()
}

fn build_issues_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let theme = &app.theme;
    if system_status.doctor_issues.is_empty() {
        return vec![(
            format!("{} No issues found", symbol(app, "✓", "ok")),
            theme.green,
        )];
    }

    system_status
        .doctor_issues
        .iter()
        .map(|issue| (issue.clone(), theme.yellow))
        .collect()
}

fn build_activity_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let mut items = build_pending_command_items(app)
        .or_else(|| build_recent_completion_items(app))
        .unwrap_or_default();

    if items.is_empty() {
        items = build_status_snapshot_items(app, system_status);
    }

    if !app.pending_command {
        prepend_toast_item(app, &mut items);
        append_last_command_error(app, &mut items);
    }

    items
}

fn build_pending_command_items(app: &App) -> Option<Vec<StatusLine>> {
    if !(app.pending_command
        && matches!(
            app.last_command.as_deref(),
            Some("install")
                | Some("uninstall")
                | Some("upgrade")
                | Some("upgrade-all")
                | Some("self-update")
        ))
    {
        return None;
    }

    let theme = &app.theme;
    let spinner = spinner_frame(app);
    let action = match app.last_command.as_deref() {
        Some("install") => "Installing",
        Some("uninstall") => "Uninstalling",
        Some("upgrade") => "Upgrading",
        Some("upgrade-all") => "Upgrading outdated packages",
        Some("self-update") => "Updating Brewery",
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
    } else if app.last_command.as_deref() == Some("upgrade-all") {
        items.push(("Command: brew upgrade".to_string(), theme.text_muted));
    } else if app.last_command.as_deref() == Some("self-update") {
        items.push((
            "Command: cargo install brewery --locked --force".to_string(),
            theme.text_muted,
        ));
    }
    items.extend(
        app.last_command_output
            .iter()
            .map(|line| (format!("> {line}"), theme.text_muted)),
    );
    Some(items)
}

fn build_recent_completion_items(app: &App) -> Option<Vec<StatusLine>> {
    let theme = &app.theme;
    let (label, pkg, completed_at) = app.last_command_completed.as_ref()?;
    if completed_at.elapsed().as_secs() >= 3 {
        return None;
    }

    let verb = match label.as_str() {
        "install" => "Install",
        "uninstall" => "Uninstall",
        "upgrade" => "Upgrade",
        "upgrade-all" => "Upgrade all outdated",
        _ => "Command",
    };
    Some(vec![(format!("{verb} completed: {pkg}"), theme.green)])
}

fn build_status_snapshot_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let theme = &app.theme;
    let mut items = Vec::new();

    if let Some(ver) = &system_status.brew_version {
        let sep = symbol(app, "·", "|");
        let info = system_status
            .brew_info
            .as_ref()
            .map(|value| format!(" {sep} {value}"))
            .unwrap_or_default();
        items.push((format!("Version: {ver}{info}"), theme.text_primary));
    }

    if system_status.brewery_update_available
        && let Some(latest) = system_status.brewery_latest_version.as_ref()
    {
        items.push((format!("Brewery update: v{latest} available"), theme.orange));
    }

    let doctor_status = match system_status.doctor_ok {
        Some(true) => (symbol(app, "✓ Healthy", "ok Healthy"), theme.green),
        Some(false) => (
            symbol(app, "⚠ Issues found", "! Issues found"),
            theme.yellow,
        ),
        None => ("? Unknown", theme.text_muted),
    };
    items.push((format!("Doctor: {}", doctor_status.0), doctor_status.1));

    let outdated_status = match system_status.outdated_count {
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

    if let Some(update_status) = system_status.brew_update_status.as_ref() {
        let color = match update_status.as_str() {
            "Up to date" => theme.green,
            "Update recommended" => theme.orange,
            _ => theme.text_muted,
        };
        items.push((format!("Brew update: {update_status}"), color));
    }
    if let Some(secs) = system_status.last_brew_update_secs_ago {
        items.push((
            format!("Last brew update: {} ago", format_elapsed(secs)),
            theme.text_muted,
        ));
    }

    if let Some(t) = app.last_status_check {
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

fn prepend_toast_item(app: &App, items: &mut Vec<StatusLine>) {
    let theme = &app.theme;
    if let Some(toast) = app.toast.as_ref() {
        let (label, color) = match toast.level {
            ToastLevel::Success => (
                format!("{} {}", symbol(app, "✓", "ok"), toast.message),
                theme.green,
            ),
            ToastLevel::Error => (
                format!("{} {}", symbol(app, "✗", "x"), toast.message),
                theme.red,
            ),
        };
        items.insert(0, (label, color));
    }
}

fn append_last_command_error(app: &App, items: &mut Vec<StatusLine>) {
    let theme = &app.theme;
    if let Some(error) = app.last_command_error.as_ref() {
        let label = app.last_command.as_deref().unwrap_or("command");
        items.push((format!("Last cmd failed: {label}"), theme.red));
        for line in error.lines().take(6) {
            items.push((format!("> {line}"), theme.red));
        }
    }
}

fn append_scrolled_lines(app: &App, lines: &mut Vec<Line<'_>>, scroll_items: &[StatusLine]) {
    let theme = &app.theme;
    if app.status_scroll_offset > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "  {} {} more above",
                symbol(app, "↑", "^"),
                app.status_scroll_offset
            ),
            Style::default().fg(theme.text_muted),
        )));
    }

    for (text, color) in scroll_items.iter().skip(app.status_scroll_offset) {
        lines.push(Line::from(Span::styled(
            format!("  {}", text),
            Style::default().fg(*color),
        )));
    }
}

fn append_last_error_line(app: &App, lines: &mut Vec<Line<'_>>) {
    let theme = &app.theme;
    if let Some(error) = app.last_error.as_deref() {
        lines.push(Line::from(vec![
            Span::styled(
                format!("  {} ", symbol(app, "✗", "x")),
                Style::default().fg(theme.red),
            ),
            Span::styled(error.to_string(), Style::default().fg(theme.red)),
        ]));
    }
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
