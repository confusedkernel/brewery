use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::ui::keymap;
use crate::ui::layout;
use crate::ui::util::symbol;

/// Renders the keymap table into popup lines. Only the popup itself needs
/// these; line/selection math lives in [`keymap`] and allocates nothing.
fn build_help_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let mut lines: Vec<Line> = Vec::with_capacity(keymap::line_count());

    for (index, section) in keymap::SECTIONS.iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!(" {}", section.title),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));

        for command in section.commands {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {:12}", command.label(app.icons_ascii)),
                    Style::default().fg(theme.yellow),
                ),
                Span::styled(command.description, Style::default().fg(theme.text_primary)),
            ]));
        }

        if index + 1 < keymap::SECTIONS.len() {
            lines.push(Line::from(""));
        }
    }

    lines
}

pub fn draw_help_popup(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;
    let area = frame.area();
    let app_layout = layout::split_app(area);

    let dim_overlay = Block::default().style(Style::default().bg(theme.bg_dim));
    frame.render_widget(dim_overlay, app_layout.body);

    let popup_area = layout::help_popup_area(area);

    frame.render_widget(Clear, popup_area);

    let bg_fill = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_fill, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_active))
        .style(Style::default().bg(theme.bg_main))
        .title(Span::styled(
            " Keymaps ",
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(Span::styled(
            " Enter runs keymap - ?/Esc close ",
            Style::default().fg(theme.text_muted),
        )));

    let selected_line = keymap::command_line(app.help_selected_command);
    let lines = build_help_lines(app);
    let inner = block.inner(popup_area);
    let visible_height = inner.height as usize;
    let max_offset = lines.len().saturating_sub(visible_height);
    let offset = app.help_scroll_offset.min(max_offset);
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .skip(offset)
        .take(visible_height)
        .collect();
    let visible_items: Vec<ListItem> = visible_lines.into_iter().map(ListItem::new).collect();

    let selected_visible_index = selected_line
        .and_then(|line_index| line_index.checked_sub(offset))
        .filter(|idx| *idx < visible_items.len());

    let list = List::new(visible_items)
        .block(block)
        .style(Style::default().bg(theme.bg_main))
        .highlight_style(
            Style::default()
                .bg(theme.bg_selection)
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(symbol(app, "▌", "> "));

    let mut state = ListState::default();
    state.select(selected_visible_index);
    frame.render_stateful_widget(list, popup_area, &mut state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_one_line_per_keymap_entry() {
        let app = App::new();
        assert_eq!(build_help_lines(&app).len(), keymap::line_count());
    }
}
