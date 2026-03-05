use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[derive(Clone, Copy)]
pub struct AppLayout {
    pub header: Rect,
    pub body: Rect,
    pub footer: Rect,
    pub search: Rect,
    pub leaves: Rect,
    pub sizes: Rect,
    pub status: Rect,
    pub details: Rect,
}

pub fn split_app(area: Rect) -> AppLayout {
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(root[1]);

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(10),
        ])
        .split(columns[0]);

    let right_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(columns[1]);

    AppLayout {
        header: root[0],
        body: root[1],
        footer: root[2],
        search: left_panels[0],
        leaves: left_panels[1],
        sizes: left_panels[2],
        status: right_panels[0],
        details: right_panels[1],
    }
}

pub fn help_popup_area(area: Rect) -> Rect {
    let popup_width = 50u16.min(area.width.saturating_sub(4));
    let popup_height = 22u16.min(area.height.saturating_sub(4));
    let popup_x = area.x + (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = area.y + (area.height.saturating_sub(popup_height)) / 2;

    Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    }
}

pub fn help_visible_line_capacity(area: Rect) -> usize {
    help_popup_area(area).height.saturating_sub(2) as usize
}
