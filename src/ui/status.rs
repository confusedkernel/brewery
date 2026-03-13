use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, StatusTab, ToastLevel};
use crate::brew::{CommandKind, StatusSnapshot};
use crate::ui::util::symbol;

type StatusLine = (String, Color);

const STATUS_TABS: [(&str, StatusTab); 5] = [
    ("Activity", StatusTab::Activity),
    ("Issues", StatusTab::Issues),
    ("Outdated", StatusTab::Outdated),
    ("Services", StatusTab::Services),
    ("History", StatusTab::History),
];

pub fn tab_at_column(app: &App, area: Rect, column: u16) -> Option<StatusTab> {
    if area.width <= 2 {
        return None;
    }

    let inner_left = area.x.saturating_add(1);
    let inner_right = area.x.saturating_add(area.width.saturating_sub(2));
    if column < inner_left || column > inner_right {
        return None;
    }

    let separator = symbol(app, "·", "|");
    let separator_width = text_width(separator);
    let mut cursor = inner_left;

    for (index, (name, tab)) in STATUS_TABS.iter().enumerate() {
        let label = format!(" {} ", name);
        let tab_width = text_width(&label);
        let tab_end = cursor.saturating_add(tab_width.saturating_sub(1));

        if column >= cursor && column <= tab_end {
            return Some(*tab);
        }

        cursor = cursor.saturating_add(tab_width);

        if index + 1 < STATUS_TABS.len() {
            let separator_end = cursor.saturating_add(separator_width.saturating_sub(1));
            if column >= cursor && column <= separator_end {
                return None;
            }
            cursor = cursor.saturating_add(separator_width);
        }

        if cursor > inner_right {
            break;
        }
    }

    None
}

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
    let mut title_spans: Vec<Span> = Vec::new();
    for (i, (name, tab)) in STATUS_TABS.iter().enumerate() {
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
        if i + 1 < STATUS_TABS.len() {
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
        StatusTab::Services => build_services_items(app, system_status),
        StatusTab::History => build_history_items(app),
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

fn build_services_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let theme = &app.theme;
    if system_status.services.is_empty() {
        return vec![(
            format!("{} No Homebrew services found", symbol(app, "✓", "ok")),
            theme.text_muted,
        )];
    }

    let selected = app.services_selected_index.unwrap_or(0);
    system_status
        .services
        .iter()
        .enumerate()
        .map(|(index, service)| {
            let marker = if index == selected {
                symbol(app, "▸", ">")
            } else {
                " "
            };
            let status_color = match service.status.as_str() {
                "started" => theme.green,
                "stopped" | "none" => theme.text_muted,
                "error" => theme.red,
                _ => theme.yellow,
            };
            (
                format!("{marker} {} ({})", service.name, service.status),
                status_color,
            )
        })
        .collect()
}

fn build_history_items(app: &App) -> Vec<StatusLine> {
    let theme = &app.theme;
    if app.command_history.is_empty() {
        return vec![(
            format!("{} No commands yet", symbol(app, "ℹ", "i")),
            theme.text_muted,
        )];
    }

    app.command_history
        .iter()
        .map(|entry| {
            let prefix = if entry.success {
                symbol(app, "✓", "ok")
            } else {
                symbol(app, "✗", "x")
            };
            let color = if entry.success {
                theme.green
            } else {
                theme.red
            };
            let exit_label = entry
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "n/a".to_string());
            (
                format!(
                    "{prefix} [{}] {} (exit {exit_label}, {}s, {}s ago)",
                    entry.kind,
                    entry.command,
                    entry.duration_secs,
                    entry.finished_at.elapsed().as_secs()
                ),
                color,
            )
        })
        .collect()
}

fn build_activity_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let mut items = Vec::new();

    if let Some(command_items) = build_pending_command_items(app) {
        items.extend(command_items);
    }

    items.extend(build_pending_request_items(app));

    if items.is_empty() {
        items = build_recent_completion_items(app).unwrap_or_default();
    }

    if items.is_empty() {
        items = build_status_snapshot_items(app, system_status);
    }

    if !app.pending_command {
        prepend_toast_item(app, &mut items);
        append_last_command_error(app, &mut items);
    }

    items
}

fn build_pending_request_items(app: &App) -> Vec<StatusLine> {
    let theme = &app.theme;
    let spinner = spinner_frame(app);
    let mut items = Vec::new();

    if app.pending_leaves {
        let elapsed = app
            .pending_leaves_started_at
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0);
        items.push((
            format!("{spinner} Refreshing leaves ({elapsed}s)"),
            theme.accent_secondary,
        ));
    }

    if app.pending_casks {
        let elapsed = app
            .pending_casks_started_at
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0);
        items.push((
            format!("{spinner} Refreshing casks ({elapsed}s)"),
            theme.accent_secondary,
        ));
    }

    if app.pending_sizes {
        let elapsed = app
            .pending_sizes_started_at
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0);
        items.push((
            format!("{spinner} Refreshing sizes ({elapsed}s)"),
            theme.accent_secondary,
        ));
    }

    if app.pending_status {
        let elapsed = app
            .pending_status_started_at
            .map(|started| started.elapsed().as_secs())
            .unwrap_or(0);
        items.push((
            format!("{spinner} Refreshing status/outdated/services ({elapsed}s)"),
            theme.accent_secondary,
        ));
    }

    items
}

fn build_pending_command_items(app: &App) -> Option<Vec<StatusLine>> {
    if !(app.pending_command
        && app
            .last_command
            .map(CommandKind::is_activity_command)
            .unwrap_or(false))
    {
        return None;
    }

    let theme = &app.theme;
    let spinner = spinner_frame(app);
    let action = match app.last_command {
        Some(CommandKind::Install) => "Installing",
        Some(CommandKind::Uninstall) => "Uninstalling",
        Some(CommandKind::Upgrade) => "Upgrading",
        Some(CommandKind::UpgradeAll) => "Upgrading outdated packages",
        Some(CommandKind::ServiceStart) => "Starting service",
        Some(CommandKind::ServiceStop) => "Stopping service",
        Some(CommandKind::ServiceRestart) => "Restarting service",
        Some(CommandKind::SelfUpdate) => "Updating Brewery",
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
    if let Some(kind) = app.last_command {
        let binary = if kind == CommandKind::SelfUpdate {
            "cargo"
        } else {
            "brew"
        };
        let args = app.last_command_args.join(" ");
        let command_text = if args.is_empty() {
            binary.to_string()
        } else {
            format!("{binary} {args}")
        };
        items.push((format!("Command: {command_text}"), theme.text_muted));
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
    let (kind, pkg, completed_at) = app.last_command_completed.as_ref()?;
    if completed_at.elapsed().as_secs() >= 3 {
        return None;
    }

    let verb = match kind {
        CommandKind::Install => "Install",
        CommandKind::Uninstall => "Uninstall",
        CommandKind::Upgrade => "Upgrade",
        CommandKind::UpgradeAll => "Upgrade all outdated",
        CommandKind::ServiceStart => "Service start",
        CommandKind::ServiceStop => "Service stop",
        CommandKind::ServiceRestart => "Service restart",
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

    if !system_status.services.is_empty() {
        let running = system_status
            .services
            .iter()
            .filter(|service| service.status == "started")
            .count();
        let color = if running > 0 {
            theme.green
        } else {
            theme.text_muted
        };
        items.push((
            format!(
                "Services: {running}/{} running",
                system_status.services.len()
            ),
            color,
        ));
    }

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
    if let Some(t) = app.last_casks_refresh {
        items.push((
            format!("Casks refresh: {}s ago", t.elapsed().as_secs()),
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
        let label = app
            .last_command
            .map(|kind| kind.label())
            .unwrap_or("command");
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

fn text_width(value: &str) -> u16 {
    value.chars().count() as u16
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::tab_at_column;
    use crate::app::{App, StatusTab};

    #[test]
    fn maps_clicks_to_expected_tabs() {
        let app = App::new();
        let area = Rect::new(0, 0, 70, 6);

        assert_eq!(tab_at_column(&app, area, 2), Some(StatusTab::Activity));
        assert_eq!(tab_at_column(&app, area, 13), Some(StatusTab::Issues));
        assert_eq!(tab_at_column(&app, area, 22), Some(StatusTab::Outdated));
        assert_eq!(tab_at_column(&app, area, 33), Some(StatusTab::Services));
        assert_eq!(tab_at_column(&app, area, 44), Some(StatusTab::History));
    }

    #[test]
    fn ignores_separator_clicks() {
        let app = App::new();
        let area = Rect::new(0, 0, 70, 6);

        assert_eq!(tab_at_column(&app, area, 11), None);
        assert_eq!(tab_at_column(&app, area, 20), None);
    }
}
