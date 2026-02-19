use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::app::{App, InputMode};
use crate::ui::util::symbol;

pub fn draw_leaves_panel(frame: &mut ratatui::Frame, area: Rect, app: &App, is_focused: bool) {
    let theme = &app.theme;

    let (title, list_items, selected_pos) = if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        let results = &app.package_results;
        let title = format!(" Results ({})", results.len());
        let rows = if results.is_empty() {
            vec![styled_item("  No results yet", theme.text_muted)]
        } else {
            results
                .iter()
                .map(|item| styled_item(format!(" {item}"), theme.text_primary))
                .collect()
        };
        (title, rows, app.package_results_selected)
    } else if app.is_cask_mode() {
        let casks = &app.filtered_casks;
        let title = format!(" Casks ({})", casks.len());
        let rows = if casks.is_empty() {
            vec![styled_item("  No casks found", theme.text_muted)]
        } else {
            casks
                .iter()
                .filter_map(|idx| app.casks.get(*idx))
                .map(|item| styled_item(format!(" {item}"), theme.text_primary))
                .collect()
        };
        let selected = app
            .selected_cask_index
            .and_then(|selected| casks.iter().position(|idx| *idx == selected));
        (title, rows, selected)
    } else {
        let leaves = &app.filtered_leaves;
        let filter_suffix = if app.leaves_outdated_only {
            format!(" {} outdated", symbol(app, "·", "|"))
        } else {
            String::new()
        };
        let title = format!(" Leaves ({}){}", leaves.len(), filter_suffix);
        let rows = if leaves.is_empty() {
            let empty_label = if app.leaves_outdated_only {
                if app.system_status.is_some() {
                    "  No outdated leaves"
                } else {
                    "  No outdated data yet (press h)"
                }
            } else {
                "  No leaves found"
            };
            vec![styled_item(empty_label, theme.text_muted)]
        } else {
            leaves
                .iter()
                .filter_map(|idx| app.leaves.get(*idx))
                .map(|item| {
                    let marker = if app.is_outdated_leaf(item.as_str()) {
                        format!("{} ", symbol(app, "↑", "^"))
                    } else {
                        String::new()
                    };
                    styled_item(format!(" {marker}{item}"), theme.text_primary)
                })
                .collect()
        };
        let selected = app
            .selected_index
            .and_then(|selected| leaves.iter().position(|idx| *idx == selected));
        (title, rows, selected)
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
            title,
            Style::default()
                .fg(theme.accent)
                .add_modifier(title_modifier),
        ));

    let leaves_list = List::new(list_items)
        .block(block)
        .style(Style::default().bg(theme.bg_panel))
        .highlight_style(
            Style::default()
                .bg(theme.bg_selection)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(symbol(app, "▌", "> "));

    let mut list_state = ListState::default();
    list_state.select(selected_pos);
    frame.render_stateful_widget(leaves_list, area, &mut list_state);
}

fn styled_item(text: impl Into<String>, color: Color) -> ListItem<'static> {
    ListItem::new(Line::from(Span::styled(
        text.into(),
        Style::default().fg(color),
    )))
}
