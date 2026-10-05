use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::app::{App, Confirmation, InputMode, PackageAction, StatusTab, ViewMode};
use crate::brew::Origin;
use crate::format::{format_age, format_date, format_size};
use crate::ui::util::{panel_block, panel_title, styled_line, symbol};

pub fn draw_details_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let details_lines = if app.pending_confirmation == Some(Confirmation::Autoremove) {
        build_autoremove_preview_lines(app)
    } else if is_searching(app) {
        build_details_lines(app, app.selected_package_result())
    } else if app.status_tab == StatusTab::Services {
        build_service_details_lines(app)
    } else {
        match app.view_mode {
            ViewMode::Details => build_details_lines(app, app.selected_package_name()),
            ViewMode::PackageResults => build_package_results(app),
        }
    };

    let visible_lines: Vec<Line> = details_lines
        .into_iter()
        .skip(app.details_scroll_offset)
        .collect();

    let block = panel_block(
        app,
        panel_title(" Details", theme.accent, is_focused),
        is_focused,
    );
    let paragraph = Paragraph::new(visible_lines)
        .block(block)
        .style(Style::default().bg(theme.bg_panel))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn is_searching(app: &App) -> bool {
    matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    )
}

fn heading(text: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(Span::styled(
        text.into(),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ))
}

fn build_details_lines(app: &App, pkg: Option<&str>) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let Some(pkg) = pkg else {
        let (message, hint) = if is_searching(app) {
            ("  No results yet", "  Press Enter to search")
        } else {
            ("  No package selected", "  Select a package from the list")
        };
        return vec![
            Line::from(""),
            styled_line(message, theme.text_muted),
            Line::from(""),
            styled_line(hint, theme.text_muted),
        ];
    };

    let is_pending = app.pending_details.as_deref() == Some(pkg);

    let mut lines = vec![Line::from(""), heading(format!("  {pkg}"), theme.accent)];
    lines.extend(build_origin_lines(app, pkg));
    lines.extend(build_pinned_lines(app, pkg));
    lines.extend(build_removal_impact_lines(app, pkg));

    let Some(details) = app.details_cache.peek(pkg) else {
        lines.push(Line::from(""));
        lines.push(styled_line(
            if is_pending {
                "  Loading details..."
            } else {
                "  Press Enter to load details"
            },
            theme.text_muted,
        ));
        return lines;
    };

    if let Some(desc) = details.desc.as_ref() {
        lines.push(Line::from(""));
        lines.push(styled_line(format!("  {desc}"), theme.text_primary));
    }

    if let Some(homepage) = details.homepage.as_ref() {
        lines.push(Line::from(""));
        lines.push(styled_line("  Homepage", theme.text_secondary));
        lines.push(styled_line(format!("  {homepage}"), theme.accent));
    }

    lines.push(Line::from(""));

    let size_text = match app.sizes.iter().find(|entry| entry.name == pkg) {
        Some(entry) => format!(" ({})", format_size(entry.size_kb)),
        None if app.sizes_job.is_running() => " (size: loading...)".to_string(),
        None => " (size: n/a)".to_string(),
    };
    lines.push(styled_line(
        format!(
            "  Installed: {}{size_text}",
            format_list_inline(&details.installed)
        ),
        theme.green,
    ));

    // Only casks carry artifacts.
    let is_cask = details.artifacts.is_some();

    if let Some(latest) = details.latest.as_ref() {
        let is_outdated = if is_cask {
            app.is_outdated_cask(pkg)
        } else {
            app.is_outdated_leaf(pkg)
        };
        lines.push(if is_outdated {
            styled_line(
                format!(
                    "  Latest: {latest} {} upgrade available (U)",
                    symbol(app, "↑", "^")
                ),
                theme.orange,
            )
        } else {
            styled_line(format!("  Latest: {latest}"), theme.text_secondary)
        });
    }

    if let Some(artifacts) = details.artifacts.as_ref() {
        lines.push(Line::from(""));
        lines.push(styled_line(
            format!("  Artifacts ({})", artifacts.len()),
            theme.orange,
        ));
        lines.extend(format_list_multiline(app, artifacts));
    }

    for (label, items, color) in [
        ("Dependencies", &details.deps, theme.yellow),
        ("Used by", &details.uses, theme.orange),
    ] {
        lines.push(Line::from(""));
        match items {
            _ if is_cask => lines.push(styled_line(
                format!("  {label}: not available for casks"),
                theme.text_muted,
            )),
            Some(items) => {
                lines.push(styled_line(format!("  {label} ({})", items.len()), color));
                lines.extend(format_list_multiline(app, items));
            }
            None if is_pending => {
                lines.push(styled_line(
                    format!("  {label}: loading..."),
                    theme.text_muted,
                ));
            }
            None => lines.push(styled_line(
                format!("  {label}: press 'd' to load"),
                theme.text_muted,
            )),
        }
    }

    lines
}

/// Answers "why do I have this?" — something `brew` cannot be asked directly.
fn build_origin_lines(app: &App, pkg: &str) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let Some(origin) = app.origin_of(pkg) else {
        return Vec::new();
    };

    let mut lines = vec![Line::from("")];

    match origin {
        Origin::OnRequest => {
            lines.push(styled_line("  Installed on request", theme.green));
        }
        Origin::RequiredBy {
            path,
            direct_dependents,
        } => {
            lines.push(styled_line("  Required by", theme.text_secondary));

            // e.g. `zlib ← libpng ← imagemagick`
            let arrow = symbol(app, " ← ", " <- ");
            let chain = std::iter::once(pkg)
                .chain(path.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(arrow);
            lines.push(styled_line(format!("    {chain}"), theme.text_primary));

            if direct_dependents > 1 {
                lines.push(styled_line(
                    format!("    {direct_dependents} formulae depend on it directly"),
                    theme.text_muted,
                ));
            }
        }
        Origin::Orphaned => {
            lines.push(styled_line("  Orphaned", theme.orange));
            lines.push(styled_line(
                "    Nothing requested needs this; brew autoremove would remove it",
                theme.text_muted,
            ));
        }
        Origin::Unknown => {}
    }

    if let Some(installed_at) = app.installed_at(pkg) {
        lines.push(styled_line(
            format!("    installed {}", describe_install_time(installed_at)),
            theme.text_muted,
        ));
    }

    lines
}

/// `2026-06-11 (3 months ago)`, or just the date if the clock is somehow
/// behind the receipt.
fn describe_install_time(installed_at: u64) -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let date = format_date(installed_at);
    match now.checked_sub(installed_at) {
        Some(age) => format!("{date} ({})", format_age(age)),
        None => date,
    }
}

/// A pinned formula is skipped by `brew upgrade`, which is easy to forget
/// when the Outdated tab keeps listing it.
fn build_pinned_lines(app: &App, pkg: &str) -> Vec<Line<'static>> {
    let theme = &app.theme;
    if !app.is_pinned(pkg) {
        return Vec::new();
    }

    vec![
        Line::from(""),
        styled_line(format!("  {} Pinned", symbol(app, "", "*")), theme.yellow),
        styled_line(
            "    brew upgrade skips it; press p to unpin",
            theme.text_muted,
        ),
    ]
}

/// Everything `brew autoremove` is about to delete, shown while its
/// confirmation is armed. The prompt has the count; this has the names.
fn build_autoremove_preview_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = vec![
        Line::from(""),
        heading("  Autoremove preview", theme.accent_secondary),
        Line::from(""),
    ];

    let Some(impact) = app.autoremove_impact() else {
        lines.push(styled_line(
            "  Dependency graph still loading; confirm to run anyway",
            theme.text_muted,
        ));
        return lines;
    };

    let count = impact.orphaned.len();
    let noun = if count == 1 { "formula" } else { "formulae" };
    lines.push(styled_line(
        format!(
            "  Removes {count} {noun} nothing requested needs{}",
            freed_suffix(impact.freed_kb)
        ),
        theme.orange,
    ));
    lines.extend(format_list_multiline(app, &impact.orphaned));
    lines.push(Line::from(""));
    lines.push(styled_line(
        "  [a] confirm, [Esc] cancel",
        theme.text_secondary,
    ));

    lines
}

/// Previews the orphan cascade of an uninstall, shown while its confirmation
/// is armed so the count in the status line can be inspected before confirming.
fn build_removal_impact_lines(app: &App, pkg: &str) -> Vec<Line<'static>> {
    let theme = &app.theme;

    let is_awaiting_uninstall = matches!(
        &app.pending_confirmation,
        Some(Confirmation::Package(pending))
            if pending.action == PackageAction::Uninstall && pending.pkg == pkg
    );
    if !is_awaiting_uninstall {
        return Vec::new();
    }

    let Some(impact) = app.removal_impact_of(pkg) else {
        return Vec::new();
    };

    let mut lines = vec![Line::from("")];

    if impact.is_empty() {
        lines.push(styled_line("  Nothing else depends on this", theme.green));
        return lines;
    }

    lines.push(styled_line(
        format!(
            "  Uninstalling also orphans {}{}",
            impact.orphaned.len(),
            freed_suffix(impact.freed_kb)
        ),
        theme.orange,
    ));
    lines.extend(format_list_multiline(app, &impact.orphaned));

    lines
}

fn freed_suffix(freed_kb: Option<u64>) -> String {
    freed_kb
        .map(|kb| format!(" (~{} freed)", format_size(kb)))
        .unwrap_or_default()
}

fn build_package_results(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = vec![
        Line::from(""),
        heading("  Search Results", theme.accent_secondary),
        Line::from(""),
    ];

    if app.package_results.is_empty() {
        lines.push(styled_line("  No results yet", theme.text_muted));
        lines.push(styled_line(
            "  Press 'f' to search packages",
            theme.text_muted,
        ));
        return lines;
    }

    let bullet = symbol(app, "•", "*");
    lines.extend(
        app.package_results
            .iter()
            .take(16)
            .map(|item| styled_line(format!("  {bullet} {item}"), theme.text_primary)),
    );
    lines
}

fn build_service_details_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines = vec![
        Line::from(""),
        heading("  Service Inspector", theme.accent_secondary),
        Line::from(""),
        styled_line(
            format!("  Filters: {}", app.services_filter_summary()),
            theme.text_muted,
        ),
        styled_line(
            "  Actions: [S] start  [X] stop  [R] restart  [I] info",
            theme.text_secondary,
        ),
        styled_line(
            "  Filter keys: [F] failed-only  [A] auto-start-only  [K] kind",
            theme.text_secondary,
        ),
    ];

    let Some(service) = app.selected_service_entry() else {
        lines.push(Line::from(""));
        lines.push(styled_line("  No service selected", theme.text_muted));
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
        .map_or_else(|| "n/a".to_string(), |code| code.to_string());
    let exit_color = if service.exit_code.is_some_and(|code| code != 0) {
        theme.red
    } else {
        theme.text_secondary
    };
    let autostart = if service.auto_start_enabled() {
        "yes"
    } else {
        "no"
    };

    lines.extend([
        Line::from(""),
        heading(format!("  {}", service.name), theme.accent),
        styled_line(format!("  State: {}", service.state_label()), state_color),
        styled_line(
            format!("  Raw status: {}", service.status),
            theme.text_secondary,
        ),
        styled_line(format!("  Last exit code: {exit_code}"), exit_color),
        styled_line(
            format!("  User: {}", service.user.as_deref().unwrap_or("n/a")),
            theme.text_secondary,
        ),
        styled_line(
            format!("  Backend: {}", app.service_backend_label(&service.name)),
            theme.text_secondary,
        ),
        styled_line(format!("  Auto-start: {autostart}"), theme.text_secondary),
    ]);

    if let Some(file) = service.file.as_deref() {
        lines.push(styled_line(
            format!("  Unit file: {file}"),
            theme.text_muted,
        ));
    }

    if service.has_failed() {
        lines.push(Line::from(""));
        lines.push(heading(
            format!("  {} Why is this red?", symbol(app, "⚠", "!")),
            theme.red,
        ));
        lines.extend(platform_service_hints(app, &service.name));
    }

    lines
}

fn platform_service_hints(app: &App, service: &str) -> Vec<Line<'static>> {
    let commands = if cfg!(target_os = "macos") {
        vec![
            format!("brew services info {service}"),
            format!("launchctl print gui/$UID/homebrew.mxcl.{service}"),
            format!(
                "log show --style compact --predicate 'process CONTAINS \"{service}\"' --last 10m"
            ),
        ]
    } else {
        vec![
            format!("systemctl --user status {service}.service"),
            format!("journalctl --user-unit {service}.service -n 50 --no-pager"),
        ]
    };

    let bullet = symbol(app, "•", "*");
    commands
        .into_iter()
        .map(|command| styled_line(format!("    {bullet} {command}"), app.theme.text_primary))
        .collect()
}

fn format_list_inline(items: &[String]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    items.join(", ")
}

/// One bulleted line per item, indented under a section heading.
fn format_list_multiline(app: &App, items: &[String]) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![styled_line("    none", app.theme.text_muted)];
    }

    let bullet = symbol(app, "•", "*");
    items
        .iter()
        .map(|item| styled_line(format!("    {bullet} {item}"), app.theme.text_primary))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{Confirmation, PackageKind, PendingPackageAction};
    use crate::brew::{DependencyGraph, FormulaReceipt, Receipts};

    fn text(lines: &[Line<'static>]) -> String {
        lines
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// `imagemagick` was requested and pulls in `libpng`, which needs `zlib`.
    fn app_with_graph() -> App {
        let receipts = Receipts::from([
            (
                "imagemagick".to_string(),
                FormulaReceipt {
                    on_request: true,
                    direct: vec!["libpng".to_string()],
                    runtime: vec!["libpng".to_string(), "zlib".to_string()],
                    installed_at: None,
                },
            ),
            (
                "libpng".to_string(),
                FormulaReceipt {
                    on_request: false,
                    direct: vec!["zlib".to_string()],
                    runtime: vec!["zlib".to_string()],
                    installed_at: None,
                },
            ),
            ("zlib".to_string(), FormulaReceipt::default()),
        ]);

        let mut app = App::new();
        app.dependency_graph = Some(DependencyGraph::new(receipts));
        app
    }

    #[test]
    fn marks_an_explicitly_installed_formula() {
        let app = app_with_graph();
        assert!(text(&build_origin_lines(&app, "imagemagick")).contains("Installed on request"));
    }

    #[test]
    fn shows_the_chain_that_explains_a_dependency() {
        let app = app_with_graph();
        let rendered = text(&build_origin_lines(&app, "zlib"));

        assert!(rendered.contains("Required by"));
        assert!(
            rendered.contains("zlib")
                && rendered.contains("libpng")
                && rendered.contains("imagemagick"),
            "chain should trace up to the requester, got:\n{rendered}"
        );
    }

    #[test]
    fn says_nothing_about_packages_outside_the_graph() {
        let app = app_with_graph();
        assert!(build_origin_lines(&app, "not-installed").is_empty());
    }

    #[test]
    fn says_nothing_until_the_graph_loads() {
        let app = App::new();
        assert!(build_origin_lines(&app, "imagemagick").is_empty());
    }

    #[test]
    fn previews_the_cascade_only_while_its_uninstall_is_armed() {
        let mut app = app_with_graph();
        assert!(
            build_removal_impact_lines(&app, "imagemagick").is_empty(),
            "no preview without a pending confirmation"
        );

        app.pending_confirmation = Some(Confirmation::Package(PendingPackageAction {
            action: PackageAction::Uninstall,
            kind: PackageKind::Formula,
            pkg: "imagemagick".to_string(),
        }));

        let rendered = text(&build_removal_impact_lines(&app, "imagemagick"));
        assert!(rendered.contains("orphans 2"), "got:\n{rendered}");
        assert!(rendered.contains("libpng") && rendered.contains("zlib"));
    }

    #[test]
    fn previews_the_autoremove_set_with_its_size() {
        let mut app = app_with_graph();
        app.dependency_graph = Some(DependencyGraph::new(Receipts::from([
            (
                "kept".to_string(),
                FormulaReceipt {
                    on_request: true,
                    ..FormulaReceipt::default()
                },
            ),
            ("stale".to_string(), FormulaReceipt::default()),
        ])));
        app.sizes = vec![crate::brew::SizeEntry {
            name: "stale".to_string(),
            size_kb: 2048,
        }];

        let rendered = text(&build_autoremove_preview_lines(&app));
        assert!(rendered.contains("Removes 1 formula"), "got:\n{rendered}");
        assert!(rendered.contains("stale"));
        assert!(rendered.contains("2.0M"));
    }

    #[test]
    fn autoremove_preview_admits_when_the_graph_is_missing() {
        let app = App::new();
        let rendered = text(&build_autoremove_preview_lines(&app));
        assert!(rendered.contains("still loading"));
    }

    #[test]
    fn mentions_pinning_only_for_pinned_formulae() {
        let mut app = app_with_graph();
        assert!(build_pinned_lines(&app, "imagemagick").is_empty());

        app.pinned.insert("imagemagick".to_string());
        let rendered = text(&build_pinned_lines(&app, "imagemagick"));
        assert!(rendered.contains("Pinned"));
        assert!(rendered.contains("unpin"));
    }

    #[test]
    fn shows_the_install_date_when_the_receipt_has_one() {
        let mut app = app_with_graph();
        assert!(
            !text(&build_origin_lines(&app, "imagemagick")).contains("installed "),
            "no timestamp, no date line"
        );

        app.dependency_graph = Some(DependencyGraph::new(Receipts::from([(
            "imagemagick".to_string(),
            FormulaReceipt {
                on_request: true,
                installed_at: Some(1_770_570_764),
                ..FormulaReceipt::default()
            },
        )])));
        let rendered = text(&build_origin_lines(&app, "imagemagick"));
        assert!(
            rendered.contains("installed 2026-02-08"),
            "got:\n{rendered}"
        );
    }

    #[test]
    fn does_not_preview_a_cascade_for_a_pending_install() {
        let mut app = app_with_graph();
        app.pending_confirmation = Some(Confirmation::Package(PendingPackageAction {
            action: PackageAction::Install,
            kind: PackageKind::Formula,
            pkg: "imagemagick".to_string(),
        }));

        assert!(build_removal_impact_lines(&app, "imagemagick").is_empty());
    }
}
