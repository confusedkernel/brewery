use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};

use crate::app::App;
use crate::ui::util::symbol;

struct HelpCommand {
    key_label: String,
    description: &'static str,
    key_event: KeyEvent,
}

struct HelpRenderData {
    lines: Vec<Line<'static>>,
    command_line_indices: Vec<usize>,
    command_keys: Vec<KeyEvent>,
}

fn plain_key_event(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn build_help_render_data(app: &App) -> HelpRenderData {
    let theme = &app.theme;
    let keymaps = vec![
        (
            "Navigation",
            vec![
                HelpCommand {
                    key_label: if app.icons_ascii {
                        "j / down".to_string()
                    } else {
                        "j / ↓".to_string()
                    },
                    description: "Move down",
                    key_event: plain_key_event(KeyCode::Char('j')),
                },
                HelpCommand {
                    key_label: if app.icons_ascii {
                        "k / up".to_string()
                    } else {
                        "k / ↑".to_string()
                    },
                    description: "Move up",
                    key_event: plain_key_event(KeyCode::Char('k')),
                },
                HelpCommand {
                    key_label: "Tab".to_string(),
                    description: "Next panel",
                    key_event: plain_key_event(KeyCode::Tab),
                },
                HelpCommand {
                    key_label: "S-Tab".to_string(),
                    description: "Previous panel",
                    key_event: plain_key_event(KeyCode::BackTab),
                },
                HelpCommand {
                    key_label: if app.icons_ascii {
                        "l / left".to_string()
                    } else {
                        "l / ←".to_string()
                    },
                    description: "Status tab prev",
                    key_event: plain_key_event(KeyCode::Char('l')),
                },
                HelpCommand {
                    key_label: if app.icons_ascii {
                        "; / right".to_string()
                    } else {
                        "; / →".to_string()
                    },
                    description: "Status tab next",
                    key_event: plain_key_event(KeyCode::Char(';')),
                },
            ],
        ),
        (
            "Search",
            vec![
                HelpCommand {
                    key_label: "/".to_string(),
                    description: "Search installed list",
                    key_event: plain_key_event(KeyCode::Char('/')),
                },
                HelpCommand {
                    key_label: "f".to_string(),
                    description: "Find packages",
                    key_event: plain_key_event(KeyCode::Char('f')),
                },
                HelpCommand {
                    key_label: "C".to_string(),
                    description: "Toggle formula/cask list",
                    key_event: plain_key_event(KeyCode::Char('C')),
                },
            ],
        ),
        (
            "Actions",
            vec![
                HelpCommand {
                    key_label: "Enter".to_string(),
                    description: "Load details",
                    key_event: plain_key_event(KeyCode::Enter),
                },
                HelpCommand {
                    key_label: "d".to_string(),
                    description: "Load deps/uses",
                    key_event: plain_key_event(KeyCode::Char('d')),
                },
                HelpCommand {
                    key_label: "i".to_string(),
                    description: "Install selected (confirm)",
                    key_event: plain_key_event(KeyCode::Char('i')),
                },
                HelpCommand {
                    key_label: "u".to_string(),
                    description: "Uninstall selected (confirm)",
                    key_event: plain_key_event(KeyCode::Char('u')),
                },
                HelpCommand {
                    key_label: "U".to_string(),
                    description: "Upgrade selected or all outdated (confirm)",
                    key_event: plain_key_event(KeyCode::Char('U')),
                },
                HelpCommand {
                    key_label: "P".to_string(),
                    description: "Update Brewery via cargo (confirm)",
                    key_event: plain_key_event(KeyCode::Char('P')),
                },
                HelpCommand {
                    key_label: "o".to_string(),
                    description: "Toggle outdated-only formula filter",
                    key_event: plain_key_event(KeyCode::Char('o')),
                },
            ],
        ),
        (
            "Data",
            vec![
                HelpCommand {
                    key_label: "r".to_string(),
                    description: "Refresh formulae + casks",
                    key_event: plain_key_event(KeyCode::Char('r')),
                },
                HelpCommand {
                    key_label: "s".to_string(),
                    description: "Load sizes",
                    key_event: plain_key_event(KeyCode::Char('s')),
                },
                HelpCommand {
                    key_label: "h".to_string(),
                    description: "Status check",
                    key_event: plain_key_event(KeyCode::Char('h')),
                },
            ],
        ),
        (
            "Other",
            vec![
                HelpCommand {
                    key_label: "t".to_string(),
                    description: "Cycle theme",
                    key_event: plain_key_event(KeyCode::Char('t')),
                },
                HelpCommand {
                    key_label: "Alt+i".to_string(),
                    description: "Toggle icons",
                    key_event: KeyEvent::new(KeyCode::Char('i'), KeyModifiers::ALT),
                },
                HelpCommand {
                    key_label: "c".to_string(),
                    description: "Cleanup",
                    key_event: plain_key_event(KeyCode::Char('c')),
                },
                HelpCommand {
                    key_label: "a".to_string(),
                    description: "Autoremove",
                    key_event: plain_key_event(KeyCode::Char('a')),
                },
                HelpCommand {
                    key_label: "b".to_string(),
                    description: "Bundle dump",
                    key_event: plain_key_event(KeyCode::Char('b')),
                },
                HelpCommand {
                    key_label: "v".to_string(),
                    description: "Toggle view",
                    key_event: plain_key_event(KeyCode::Char('v')),
                },
                HelpCommand {
                    key_label: "q".to_string(),
                    description: "Quit",
                    key_event: plain_key_event(KeyCode::Char('q')),
                },
                HelpCommand {
                    key_label: "Esc".to_string(),
                    description: "Cancel action",
                    key_event: plain_key_event(KeyCode::Esc),
                },
            ],
        ),
    ];

    let mut lines: Vec<Line> = Vec::new();
    let mut command_line_indices = Vec::new();
    let mut command_keys = Vec::new();

    let section_count = keymaps.len();

    for (index, (section, keys)) in keymaps.into_iter().enumerate() {
        lines.push(Line::from(Span::styled(
            format!(" {}", section),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));

        for command in keys {
            command_line_indices.push(lines.len());
            command_keys.push(command.key_event);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("   {:12}", command.key_label),
                    Style::default().fg(theme.yellow),
                ),
                Span::styled(command.description, Style::default().fg(theme.text_primary)),
            ]));
        }

        if index + 1 < section_count {
            lines.push(Line::from(""));
        }
    }

    HelpRenderData {
        lines,
        command_line_indices,
        command_keys,
    }
}

pub fn help_line_count(app: &App) -> usize {
    build_help_render_data(app).lines.len()
}

pub fn help_command_count(app: &App) -> usize {
    build_help_render_data(app).command_keys.len()
}

pub fn help_command_line(app: &App, command_index: usize) -> Option<usize> {
    build_help_render_data(app)
        .command_line_indices
        .get(command_index)
        .copied()
}

pub fn help_selected_command_key(app: &App) -> Option<KeyEvent> {
    build_help_render_data(app)
        .command_keys
        .get(app.help_selected_command)
        .copied()
}

pub fn draw_help_popup(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;
    let area = frame.area();

    let body_area = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let dim_overlay = Block::default().style(Style::default().bg(theme.bg_dim));
    frame.render_widget(dim_overlay, body_area[1]);

    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 22.min(area.height.saturating_sub(4));
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

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

    let render_data = build_help_render_data(app);
    let selected_line = render_data
        .command_line_indices
        .get(app.help_selected_command)
        .copied();
    let lines = render_data.lines;
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
