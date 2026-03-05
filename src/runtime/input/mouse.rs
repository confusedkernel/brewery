use std::time::Instant;

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use crossterm::terminal::size;
use ratatui::layout::Rect;

use crate::app::{App, FocusedPanel, InputMode, StatusTab};
use crate::ui::{help, layout};

#[derive(Clone, Copy)]
enum ScrollDirection {
    Up,
    Down,
}

pub fn handle_mouse_event(app: &mut App, mouse: MouseEvent, help_max_offset: usize) {
    if app.show_help_popup {
        handle_help_popup_mouse(app, mouse, help_max_offset);
        return;
    }

    let app_layout = layout::split_app(terminal_area());

    match mouse.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            handle_left_click(app, mouse.column, mouse.row, app_layout);
        }
        MouseEventKind::ScrollUp => {
            handle_scroll(
                app,
                mouse.column,
                mouse.row,
                ScrollDirection::Up,
                app_layout,
            );
        }
        MouseEventKind::ScrollDown => {
            handle_scroll(
                app,
                mouse.column,
                mouse.row,
                ScrollDirection::Down,
                app_layout,
            );
        }
        _ => {}
    }
}

fn handle_help_popup_mouse(app: &mut App, mouse: MouseEvent, help_max_offset: usize) {
    let popup_area = layout::help_popup_area(terminal_area());

    match mouse.kind {
        MouseEventKind::ScrollUp if contains_point(popup_area, mouse.column, mouse.row) => {
            let next = app.help_scroll_offset.saturating_sub(1);
            if next != app.help_scroll_offset {
                app.help_scroll_offset = next;
                app.needs_redraw = true;
            }
        }
        MouseEventKind::ScrollDown if contains_point(popup_area, mouse.column, mouse.row) => {
            let visible_height = layout::help_visible_line_capacity(terminal_area());
            let max_offset = help::help_line_count(app)
                .saturating_sub(visible_height)
                .min(help_max_offset);
            let next = (app.help_scroll_offset + 1).min(max_offset);
            if next != app.help_scroll_offset {
                app.help_scroll_offset = next;
                app.needs_redraw = true;
            }
        }
        MouseEventKind::Down(MouseButton::Left)
            if contains_point(popup_area, mouse.column, mouse.row) =>
        {
            let inner = inner_rect(popup_area);
            if !contains_point(inner, mouse.column, mouse.row) {
                return;
            }

            let line = app.help_scroll_offset + mouse.row.saturating_sub(inner.y) as usize;
            if let Some(command_index) = help::help_command_index_at_line(app, line)
                && command_index != app.help_selected_command
            {
                app.help_selected_command = command_index;
                app.needs_redraw = true;
            }
        }
        _ => {}
    }
}

fn handle_left_click(app: &mut App, column: u16, row: u16, app_layout: layout::AppLayout) {
    if contains_point(app_layout.leaves, column, row) {
        focus_panel(app, FocusedPanel::Leaves);
        select_leaves_row(app, column, row, app_layout.leaves);
        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.sizes, column, row) {
        focus_panel(app, FocusedPanel::Sizes);
        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.status, column, row) {
        focus_panel(app, FocusedPanel::Status);

        if row == app_layout.status.y {
            select_status_tab(app, column, app_layout.status);
        } else {
            select_status_row(app, column, row, app_layout.status);
        }

        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.details, column, row) {
        focus_panel(app, FocusedPanel::Details);
        app.needs_redraw = true;
    }
}

fn handle_scroll(
    app: &mut App,
    column: u16,
    row: u16,
    direction: ScrollDirection,
    app_layout: layout::AppLayout,
) {
    if contains_point(app_layout.leaves, column, row) {
        focus_panel(app, FocusedPanel::Leaves);
        scroll_leaves(app, direction);
        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.sizes, column, row) {
        focus_panel(app, FocusedPanel::Sizes);
        match direction {
            ScrollDirection::Up => app.scroll_focused_up(),
            ScrollDirection::Down => app.scroll_focused_down(),
        }
        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.status, column, row) {
        focus_panel(app, FocusedPanel::Status);
        match direction {
            ScrollDirection::Up => app.scroll_focused_up(),
            ScrollDirection::Down => app.scroll_focused_down(),
        }
        app.needs_redraw = true;
        return;
    }

    if contains_point(app_layout.details, column, row) {
        focus_panel(app, FocusedPanel::Details);
        match direction {
            ScrollDirection::Up => app.scroll_focused_up(),
            ScrollDirection::Down => app.scroll_focused_down(),
        }
        app.needs_redraw = true;
    }
}

fn scroll_leaves(app: &mut App, direction: ScrollDirection) {
    if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        let before = app.package_results_selected;
        match direction {
            ScrollDirection::Up => app.select_prev_result(),
            ScrollDirection::Down => app.select_next_result(),
        }
        if app.package_results_selected != before {
            clear_pending_confirmations(app);
            app.on_selection_change();
        }
        return;
    }

    if app.is_cask_mode() {
        let before = app.selected_cask_index;
        match direction {
            ScrollDirection::Up => app.select_prev(),
            ScrollDirection::Down => app.select_next(),
        }
        if app.selected_cask_index != before {
            clear_pending_confirmations(app);
            app.on_selection_change();
        }
        return;
    }

    let before = app.selected_index;
    match direction {
        ScrollDirection::Up => app.select_prev(),
        ScrollDirection::Down => app.select_next(),
    }
    if app.selected_index != before {
        clear_pending_confirmations(app);
        app.on_selection_change();
    }
}

fn select_leaves_row(app: &mut App, column: u16, row: u16, area: Rect) {
    let inner = inner_rect(area);
    if !contains_point(inner, column, row) {
        return;
    }

    let row_index = row.saturating_sub(inner.y) as usize;

    if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        select_package_result_row(app, row_index, inner.height as usize);
        return;
    }

    if app.is_cask_mode() {
        select_installed_row(app, row_index, inner.height as usize, true);
    } else {
        select_installed_row(app, row_index, inner.height as usize, false);
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
        clear_pending_confirmations(app);
        app.on_selection_change();
    }
}

fn select_installed_row(app: &mut App, row_index: usize, visible_height: usize, cask_mode: bool) {
    if visible_height == 0 {
        return;
    }

    let (filtered, selected_absolute) = if cask_mode {
        (&app.filtered_casks, app.selected_cask_index)
    } else {
        (&app.filtered_leaves, app.selected_index)
    };

    if filtered.is_empty() {
        return;
    }

    let selected_pos = selected_absolute
        .and_then(|selected| filtered.iter().position(|idx| *idx == selected))
        .unwrap_or(0);
    let offset = selected_pos
        .saturating_add(1)
        .saturating_sub(visible_height);
    let visible_index = offset + row_index;
    let Some(&absolute_index) = filtered.get(visible_index) else {
        return;
    };

    let current = if cask_mode {
        app.selected_cask_index
    } else {
        app.selected_index
    };

    if current == Some(absolute_index) {
        return;
    }

    if cask_mode {
        app.selected_cask_index = Some(absolute_index);
    } else {
        app.selected_index = Some(absolute_index);
    }
    clear_pending_confirmations(app);
    app.on_selection_change();
}

fn select_status_tab(app: &mut App, column: u16, area: Rect) {
    let right_border = area.x.saturating_add(area.width.saturating_sub(1));
    if area.width <= 2 || column <= area.x || column >= right_border {
        return;
    }

    let tabs = [
        StatusTab::Activity,
        StatusTab::Issues,
        StatusTab::Outdated,
        StatusTab::Services,
        StatusTab::History,
    ];

    let relative_x = column.saturating_sub(area.x + 1) as usize;
    let inner_width = area.width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    let tab_index = (relative_x * tabs.len() / inner_width).min(tabs.len().saturating_sub(1));
    let next_tab = tabs[tab_index];
    if app.status_tab != next_tab {
        app.status_tab = next_tab;
        app.status_scroll_offset = 0;
    }
}

fn select_status_row(app: &mut App, column: u16, row: u16, area: Rect) {
    if app.status_tab != StatusTab::Services {
        return;
    }

    let Some(status) = app.system_status.as_ref() else {
        return;
    };
    if status.services.is_empty() {
        return;
    }

    let inner = inner_rect(area);
    if !contains_point(inner, column, row) {
        return;
    }

    let mut line_index = row.saturating_sub(inner.y) as usize;
    if app.status_scroll_offset > 0 {
        if line_index == 0 {
            return;
        }
        line_index = line_index.saturating_sub(1);
    }

    let service_index = app.status_scroll_offset + line_index;
    if service_index < status.services.len() && app.services_selected_index != Some(service_index) {
        app.services_selected_index = Some(service_index);
    }
}

fn focus_panel(app: &mut App, panel: FocusedPanel) {
    if app.focus_panel == panel {
        return;
    }

    app.focus_panel = panel;
    app.status = format!("Focus: {:?}", app.focus_panel);
    app.last_refresh = Instant::now();
}

fn terminal_area() -> Rect {
    let (width, height) = size().unwrap_or((0, 0));
    Rect::new(0, 0, width, height)
}

fn inner_rect(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn contains_point(area: Rect, x: u16, y: u16) -> bool {
    if area.width == 0 || area.height == 0 {
        return false;
    }

    let max_x = area.x.saturating_add(area.width);
    let max_y = area.y.saturating_add(area.height);
    x >= area.x && x < max_x && y >= area.y && y < max_y
}

fn clear_pending_confirmations(app: &mut App) {
    app.pending_package_action = None;
    app.pending_service_action = None;
    app.pending_upgrade_all_outdated = false;
    app.pending_self_update = false;
}
