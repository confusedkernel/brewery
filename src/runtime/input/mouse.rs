use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::size;
use ratatui::layout::{Margin, Position, Rect};

use crate::app::{App, FocusedPanel, InputMode, StatusTab};
use crate::ui::{keymap, layout, status_tab_at_column};

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent, help_max_offset: usize) {
    let point = Position::new(mouse.column, mouse.row);
    if app.show_help_popup {
        handle_help_popup_mouse(app, mouse.kind, point, help_max_offset);
        return;
    }

    let app_layout = layout::split_app(terminal_area());
    let Some((panel, area)) = [
        (FocusedPanel::Leaves, app_layout.leaves),
        (FocusedPanel::Sizes, app_layout.sizes),
        (FocusedPanel::Status, app_layout.status),
        (FocusedPanel::Details, app_layout.details),
    ]
    .into_iter()
    .find(|(_, area)| area.contains(point)) else {
        return;
    };

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            focus_panel(app, panel);
            match panel {
                FocusedPanel::Leaves => select_leaves_row(app, point, area),
                FocusedPanel::Status if point.y == area.y => select_status_tab(app, point.x, area),
                FocusedPanel::Status => select_status_row(app, point, area),
                FocusedPanel::Sizes | FocusedPanel::Details => {}
            }
        }
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            focus_panel(app, panel);
            let up = mouse.kind == MouseEventKind::ScrollUp;
            if panel == FocusedPanel::Leaves {
                scroll_leaves(app, up);
            } else if up {
                app.scroll_focused_up();
            } else {
                app.scroll_focused_down();
            }
        }
        _ => return,
    }
    app.needs_redraw = true;
}

fn handle_help_popup_mouse(
    app: &mut App,
    kind: MouseEventKind,
    point: Position,
    help_max_offset: usize,
) {
    let popup_area = layout::help_popup_area(terminal_area());
    if !popup_area.contains(point) {
        return;
    }

    let before = (app.help_scroll_offset, app.help_selected_command);
    match kind {
        MouseEventKind::ScrollUp => {
            app.help_scroll_offset = app.help_scroll_offset.saturating_sub(1);
        }
        MouseEventKind::ScrollDown => {
            app.help_scroll_offset = (app.help_scroll_offset + 1).min(help_max_offset);
        }
        MouseEventKind::Down(MouseButton::Left) => {
            let inner = inner_rect(popup_area);
            if inner.contains(point) {
                let line = app.help_scroll_offset + (point.y - inner.y) as usize;
                if let Some(command_index) = keymap::command_index_at_line(line) {
                    app.help_selected_command = command_index;
                }
            }
        }
        _ => {}
    }
    if (app.help_scroll_offset, app.help_selected_command) != before {
        app.needs_redraw = true;
    }
}

fn scroll_leaves(app: &mut App, up: bool) {
    let selection = |app: &App| {
        (
            app.package_results_selected,
            app.selected_cask_index,
            app.selected_index,
        )
    };
    let before = selection(app);

    let searching = matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    );
    match (searching, up) {
        (true, true) => app.select_prev_result(),
        (true, false) => app.select_next_result(),
        (false, true) => app.select_prev(),
        (false, false) => app.select_next(),
    }

    if selection(app) != before {
        app.clear_pending_confirmations();
        app.on_selection_change();
    }
}

fn select_leaves_row(app: &mut App, point: Position, area: Rect) {
    let inner = inner_rect(area);
    if !inner.contains(point) {
        return;
    }

    let row_index = (point.y - inner.y) as usize;
    let visible_height = inner.height as usize;

    if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        select_package_result_row(app, row_index, visible_height);
    } else {
        select_installed_row(app, row_index, visible_height);
    }
}

fn select_package_result_row(app: &mut App, row_index: usize, visible_height: usize) {
    if app.package_results.is_empty() || visible_height == 0 {
        return;
    }

    let selected = app.package_results_selected.unwrap_or(0);
    let offset = selected.saturating_add(1).saturating_sub(visible_height);
    let list_index = offset + row_index;

    if list_index >= app.package_results.len() {
        return;
    }

    let next = Some(list_index);
    if app.package_results_selected != next {
        app.package_results_selected = next;
        app.clear_pending_confirmations();
        app.on_selection_change();
    }
}

fn select_installed_row(app: &mut App, row_index: usize, visible_height: usize) {
    let (filtered, selected) = if app.is_cask_mode() {
        (&app.filtered_casks, &mut app.selected_cask_index)
    } else {
        (&app.filtered_leaves, &mut app.selected_index)
    };

    // The list scrolls just far enough to keep the selection on screen.
    let selected_pos = selected
        .and_then(|current| filtered.iter().position(|idx| *idx == current))
        .unwrap_or(0);
    let offset = (selected_pos + 1).saturating_sub(visible_height);
    let Some(&clicked) = filtered.get(offset + row_index) else {
        return;
    };
    if *selected == Some(clicked) {
        return;
    }

    *selected = Some(clicked);
    app.clear_pending_confirmations();
    app.on_selection_change();
}

fn select_status_tab(app: &mut App, column: u16, area: Rect) {
    if let Some(tab) = status_tab_at_column(app, area, column) {
        app.select_status_tab(tab);
    }
}

fn select_status_row(app: &mut App, point: Position, area: Rect) {
    if app.status_tab != StatusTab::Services {
        return;
    }

    let filtered = app.filtered_service_indices();
    if filtered.is_empty() {
        app.services_selected_index = None;
        return;
    }

    let inner = inner_rect(area);
    if !inner.contains(point) {
        return;
    }

    let mut line_index = (point.y - inner.y) as usize;
    // Past the top, the first line is the "N more above" marker.
    if app.status_scroll_offset > 0 {
        if line_index == 0 {
            return;
        }
        line_index -= 1;
    }

    if let Some(&service_index) = filtered.get(app.status_scroll_offset + line_index) {
        app.services_selected_index = Some(service_index);
    }
}

fn focus_panel(app: &mut App, panel: FocusedPanel) {
    if app.focus_panel == panel {
        return;
    }

    app.focus_panel = panel;
    app.set_focus_status();
}

fn terminal_area() -> Rect {
    let (width, height) = size().unwrap_or((0, 0));
    Rect::new(0, 0, width, height)
}

/// The area inside a panel's border.
fn inner_rect(area: Rect) -> Rect {
    area.inner(Margin::new(1, 1))
}
