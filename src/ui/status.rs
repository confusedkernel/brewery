use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, StatusTab, ToastLevel};
use crate::brew::{CommandKind, StatusSnapshot};
use crate::format::format_size;
use crate::ui::util::{panel_block, styled_line, symbol};

type StatusLine = (String, Color);

/// How many scrollable lines the active tab renders, which bounds how far it
/// can scroll.
pub fn item_count(app: &App) -> usize {
    app.system_status
        .as_ref()
        .map_or(0, |status| build_tab_items(app, status).len())
}

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

    for (index, tab) in StatusTab::ALL.into_iter().enumerate() {
        let label = format!(" {} ", tab.label());
        let tab_width = text_width(&label);
        let tab_end = cursor.saturating_add(tab_width.saturating_sub(1));

        if column >= cursor && column <= tab_end {
            return Some(tab);
        }

        cursor = cursor.saturating_add(tab_width);

        if index + 1 < StatusTab::ALL.len() {
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

    if app.status_job.is_running() {
        lines.push(styled_line("  Checking status...", theme.text_muted));
    } else if let Some(system_status) = &app.system_status {
        let scroll_items = build_tab_items(app, system_status);
        append_scrolled_lines(app, &mut lines, &scroll_items);
    } else {
        lines.push(styled_line(
            "  Press 'h' for status check",
            theme.text_muted,
        ));
    }

    append_last_error_line(app, &mut lines);

    let mut title_spans: Vec<Span> = Vec::new();
    for (i, tab) in StatusTab::ALL.into_iter().enumerate() {
        let style = if tab == app.status_tab {
            let modifier = if is_focused {
                Modifier::BOLD
            } else {
                Modifier::empty()
            };
            Style::default().fg(theme.accent).add_modifier(modifier)
        } else {
            Style::default().fg(theme.text_muted)
        };
        title_spans.push(Span::styled(format!(" {} ", tab.label()), style));
        if i + 1 < StatusTab::ALL.len() {
            title_spans.push(Span::styled(
                symbol(app, "·", "|"),
                Style::default().fg(theme.border),
            ));
        }
    }

    let block = panel_block(app, Line::from(title_spans), is_focused);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel));
    frame.render_widget(paragraph, area);
}

fn build_tab_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    match app.status_tab {
        StatusTab::Activity => build_activity_items(app, system_status),
        StatusTab::Issues => build_issues_items(app),
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

    let arrow = symbol(app, "→", "->");
    system_status
        .outdated_packages
        .iter()
        .map(|pkg| {
            let entry = system_status
                .outdated
                .iter()
                .find(|entry| &entry.name == pkg);
            let Some(entry) = entry else {
                return (format!("{} {pkg}", symbol(app, "↑", "^")), theme.orange);
            };

            let mut tags = String::new();
            if entry.is_cask {
                tags.push_str(" (cask)");
            }
            if entry.pinned {
                tags.push_str(" [pinned]");
            }
            let color = if entry.pinned {
                theme.text_muted
            } else {
                theme.orange
            };
            (
                format!(
                    "{} {pkg}  {} {arrow} {}{tags}",
                    symbol(app, "↑", "^"),
                    entry.installed_label(),
                    entry.current_version
                ),
                color,
            )
        })
        .collect()
}

fn build_issues_items(app: &App) -> Vec<StatusLine> {
    let theme = &app.theme;

    let Some(report) = app.doctor.as_ref() else {
        // The doctor run outlives the rest of the status check, so this tab is
        // the one place where the wait is visible.
        let message = if app.doctor_job.is_running() {
            "Running brew doctor..."
        } else {
            "brew doctor could not be run"
        };
        return vec![(
            format!("{} {message}", symbol(app, "·", "-")),
            theme.text_muted,
        )];
    };

    if report.issues.is_empty() {
        return vec![(
            format!("{} No issues found", symbol(app, "✓", "ok")),
            theme.green,
        )];
    }

    report
        .issues
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

    let filtered_indices = app.filtered_service_indices();
    if filtered_indices.is_empty() {
        return vec![(
            format!(
                "{} No services match filters ({})",
                symbol(app, "i", "i"),
                app.services_filter_summary()
            ),
            theme.text_muted,
        )];
    }

    filtered_indices
        .iter()
        .map(|service_index| {
            let service = &system_status.services[*service_index];
            let marker = if app.services_selected_index == Some(*service_index) {
                symbol(app, "▸", ">")
            } else {
                " "
            };
            let status_color = if service.has_failed() {
                theme.red
            } else if service.is_running() {
                theme.green
            } else {
                theme.text_muted
            };
            let backend = app.service_backend_label(&service.name);
            let exit_label = service
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "-".to_string());
            (
                format!(
                    "{marker} {} ({}, exit {exit_label}, {backend})",
                    service.name,
                    service.state_label(),
                ),
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

    if !app.command_job.is_running() {
        prepend_toast_item(app, &mut items);
        append_last_command_error(app, &mut items);
    }

    items
}

fn build_pending_request_items(app: &App) -> Vec<StatusLine> {
    let spinner = spinner_frame(app);
    [
        (app.leaves_job, "leaves"),
        (app.casks_job, "casks"),
        (app.sizes_job, "sizes"),
        (app.status_job, "status/outdated/services"),
    ]
    .into_iter()
    .filter(|(job, _)| job.is_running())
    .map(|(job, what)| {
        (
            format!("{spinner} Refreshing {what} ({}s)", job.elapsed_secs()),
            app.theme.accent_secondary,
        )
    })
    .collect()
}

fn build_pending_command_items(app: &App) -> Option<Vec<StatusLine>> {
    if !(app.command_job.is_running()
        && app
            .last_command
            .is_some_and(CommandKind::is_activity_command))
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
        Some(CommandKind::Pin) => "Pinning",
        Some(CommandKind::Unpin) => "Unpinning",
        Some(CommandKind::Update) => "Updating Homebrew",
        Some(CommandKind::ServiceStart) => "Starting service",
        Some(CommandKind::ServiceStop) => "Stopping service",
        Some(CommandKind::ServiceRestart) => "Restarting service",
        Some(CommandKind::ServiceInfo) => "Loading service info",
        Some(CommandKind::SelfUpdate) => "Updating Brewery",
        _ => "Running",
    };
    let label = app
        .last_command_target
        .as_ref()
        .map(|pkg| format!("{spinner} {action} {pkg}"))
        .unwrap_or_else(|| format!("{spinner} {action}"));
    let elapsed = app.command_job.elapsed_secs();

    let mut items = vec![
        (format!("{label} ({elapsed}s)"), theme.accent),
        (
            format!("Command: {}", app.last_command_line()),
            theme.text_muted,
        ),
    ];
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
        CommandKind::Pin => "Pin",
        CommandKind::Unpin => "Unpin",
        CommandKind::ServiceStart => "Service start",
        CommandKind::ServiceStop => "Service stop",
        CommandKind::ServiceRestart => "Service restart",
        _ => "Command",
    };
    Some(vec![(format!("{verb} completed: {pkg}"), theme.green)])
}

/// The install summary that used to come from `brew info` — a 1.2s subprocess
/// for a single line of text. Every number in it is already on hand from the
/// Cellar scan and the size pass, so it costs nothing to build here.
fn installed_summary(app: &App) -> String {
    let formulae = app.all_formulae.len();
    let casks = app.casks.len();
    let total_kb: u64 = app.sizes.iter().map(|entry| entry.size_kb).sum();

    if total_kb == 0 {
        return format!("{formulae} formulae, {casks} casks");
    }

    format!(
        "{formulae} formulae, {casks} casks, {}",
        format_size(total_kb)
    )
}

fn build_status_snapshot_items(app: &App, system_status: &StatusSnapshot) -> Vec<StatusLine> {
    let theme = &app.theme;
    let mut items = Vec::new();

    if let Some(ver) = &system_status.brew_version {
        let sep = symbol(app, "·", "|");
        items.push((
            format!("Version: {ver} {sep} {}", installed_summary(app)),
            theme.text_primary,
        ));
    }

    if system_status.brewery_update_available
        && let Some(latest) = system_status.brewery_latest_version.as_ref()
    {
        items.push((format!("Brewery update: v{latest} available"), theme.orange));
    }

    let doctor_status = match app.doctor.as_ref() {
        Some(report) if report.ok => (symbol(app, "✓ Healthy", "ok Healthy"), theme.green),
        Some(_) => (
            symbol(app, "⚠ Issues found", "! Issues found"),
            theme.yellow,
        ),
        None if app.doctor_job.is_running() => ("Checking...", theme.text_muted),
        None => ("? Unknown", theme.text_muted),
    };
    items.push((format!("Doctor: {}", doctor_status.0), doctor_status.1));

    let pinned_suffix = match system_status.pinned.len() {
        0 => String::new(),
        n => format!(", {n} pinned"),
    };
    let outdated_status = match system_status.outdated_count {
        Some(0) => (
            format!("{} All up to date{pinned_suffix}", symbol(app, "✓", "ok")),
            theme.green,
        ),
        Some(n) => (
            format!("{} {} outdated{pinned_suffix}", symbol(app, "↑", "^"), n),
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
            .filter(|service| service.is_running())
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
        let (color, hint) = match update_status.as_str() {
            "Up to date" => (theme.green, ""),
            "Update recommended" => (theme.orange, " (e to run)"),
            _ => (theme.text_muted, ""),
        };
        items.push((format!("Brew update: {update_status}{hint}"), color));
    }
    if let Some(secs) = system_status.last_brew_update_secs_ago {
        items.push((
            format!("Last brew update: {} ago", format_elapsed(secs)),
            theme.text_muted,
        ));
    }
    if let Some(hint) = app.autoremove_hint() {
        items.push((format!("Orphans: {hint}, a to autoremove"), theme.orange));
    }

    for (job, label) in [
        (app.status_job, "Last check"),
        (app.leaves_job, "Leaves refresh"),
        (app.casks_job, "Casks refresh"),
        (app.sizes_job, "Sizes refresh"),
    ] {
        if let Some(finished_at) = job.finished_at {
            items.push((
                format!("{label}: {}s ago", finished_at.elapsed().as_secs()),
                theme.text_muted,
            ));
        }
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
        lines.push(styled_line(
            format!(
                "  {} {} more above",
                symbol(app, "↑", "^"),
                app.status_scroll_offset
            ),
            theme.text_muted,
        ));
    }

    for (text, color) in scroll_items.iter().skip(app.status_scroll_offset) {
        lines.push(styled_line(format!("  {text}"), *color));
    }
}

fn append_last_error_line(app: &App, lines: &mut Vec<Line<'_>>) {
    let theme = &app.theme;
    if let Some(error) = app.last_error.as_deref() {
        lines.push(styled_line(
            format!("  {} {error}", symbol(app, "✗", "x")),
            theme.red,
        ));
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
