use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
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
        let items = if results.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No results yet",
                Style::default().fg(theme.text_muted),
            )))]
        } else {
            results
                .iter()
                .map(|item| {
                    ListItem::new(Line::from(Span::styled(
                        format!(" {}", item),
                        Style::default().fg(theme.text_primary),
                    )))
                })
                .collect()
        };
        (title, items, app.package_results_selected)
    } else {
        let leaves = app.filtered_leaves();
        let title = format!(" Leaves ({})", leaves.len());
        let items = if leaves.is_empty() {
            vec![ListItem::new(Line::from(Span::styled(
                "  No leaves found",
                Style::default().fg(theme.text_muted),
            )))]
        } else {
            leaves
                .iter()
                .map(|(_, item)| {
                    ListItem::new(Line::from(Span::styled(
                        format!(" {}", item),
                        Style::default().fg(theme.text_primary),
                    )))
                })
                .collect()
        };
        let selected = app
            .selected_index
            .and_then(|selected| leaves.iter().position(|(idx, _)| *idx == selected));
        (title, items, selected)
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
