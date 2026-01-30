use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

const TICK_RATE: Duration = Duration::from_millis(250);

// Theme struct to hold all colors
#[derive(Clone, Copy)]
struct Theme {
    // Primary accents
    amber: Color,
    copper: Color,
    dark_amber: Color,

    // Text colors
    hop_green: Color,
    text_primary: Color,
    text_secondary: Color,

    // Backgrounds
    bg_main: Color,
    bg_panel: Color,
    bg_header: Color,

    // Accents
    text_on_accent: Color,
    border: Color,
}

impl Theme {
    fn light() -> Self {
        Self {
            // Primary accents - same for both
            amber: Color::Rgb(212, 145, 40),
            copper: Color::Rgb(166, 100, 50),
            dark_amber: Color::Rgb(140, 90, 45),

            // Text - dark on light
            hop_green: Color::Rgb(76, 132, 60),
            text_primary: Color::Rgb(70, 50, 35),
            text_secondary: Color::Rgb(120, 90, 60),

            // Light backgrounds
            bg_main: Color::Rgb(255, 250, 240),
            bg_panel: Color::Rgb(250, 240, 220),
            bg_header: Color::Rgb(255, 248, 230),

            // Accents
            text_on_accent: Color::Rgb(255, 255, 255),
            border: Color::Rgb(180, 150, 120),
        }
    }

    fn dark() -> Self {
        Self {
            // Primary accents - slightly brighter for dark mode
            amber: Color::Rgb(255, 191, 0),
            copper: Color::Rgb(205, 133, 63),
            dark_amber: Color::Rgb(184, 134, 11),

            // Text - light on dark
            hop_green: Color::Rgb(124, 179, 66),
            text_primary: Color::Rgb(245, 235, 220),
            text_secondary: Color::Rgb(200, 180, 160),

            // Dark backgrounds
            bg_main: Color::Rgb(30, 22, 16),
            bg_panel: Color::Rgb(45, 32, 22),
            bg_header: Color::Rgb(38, 28, 20),

            // Accents
            text_on_accent: Color::Rgb(25, 18, 12),
            border: Color::Rgb(100, 75, 55),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ThemeMode {
    Light,
    Dark,
    Auto,
}

fn detect_system_theme() -> Theme {
    // Check COLORFGBG env var (set by many terminals)
    // Format: "fg;bg" where bg > 7 typically means dark background
    if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
        if let Some(bg) = colorfgbg.split(';').last() {
            if let Ok(bg_num) = bg.parse::<u8>() {
                if bg_num == 0 || (bg_num >= 8 && bg_num <= 15) {
                    return Theme::dark();
                }
                return Theme::light();
            }
        }
    }

    // Check for common dark mode indicators
    if let Ok(term) = std::env::var("TERM_PROGRAM") {
        let term_lower = term.to_lowercase();
        // Many modern terminals default to dark
        if term_lower.contains("iterm") || term_lower.contains("alacritty") || term_lower.contains("kitty") {
            // Check if there's a theme preference
            if let Ok(appearance) = std::env::var("TERM_PROGRAM_VERSION") {
                if appearance.contains("light") {
                    return Theme::light();
                }
            }
        }
    }

    // macOS: check AppleInterfaceStyle
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleInterfaceStyle"])
            .output()
        {
            if output.status.success() {
                let style = String::from_utf8_lossy(&output.stdout);
                if style.trim().eq_ignore_ascii_case("dark") {
                    return Theme::dark();
                }
            }
            // If command succeeds but no "Dark", it's light mode
            // If command fails, key doesn't exist = light mode
            return Theme::light();
        }
    }

    // Default to dark (more common in terminals)
    Theme::dark()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;

    let result = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    result
}

struct App {
    started_at: Instant,
    last_refresh: Instant,
    status: String,
    theme_mode: ThemeMode,
    theme: Theme,
    input_mode: InputMode,
    search_query: String,
    leaves: Vec<String>,
    selected_index: Option<usize>,
    details_cache: HashMap<String, Details>,
    last_error: Option<String>,
    last_leaves_refresh: Option<Instant>,
}

impl App {
    fn new() -> Self {
        let theme = detect_system_theme();
        Self {
            started_at: Instant::now(),
            last_refresh: Instant::now(),
            status: "Ready".to_string(),
            theme_mode: ThemeMode::Auto,
            theme,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            leaves: Vec::new(),
            selected_index: Some(0),
            details_cache: HashMap::new(),
            last_error: None,
            last_leaves_refresh: None,
        }
    }

    fn on_tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            self.status = "Idle".to_string();
        }
    }

    fn cycle_theme(&mut self) {
        self.theme_mode = match self.theme_mode {
            ThemeMode::Auto => ThemeMode::Light,
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Auto,
        };
        self.theme = match self.theme_mode {
            ThemeMode::Light => Theme::light(),
            ThemeMode::Dark => Theme::dark(),
            ThemeMode::Auto => detect_system_theme(),
        };
        self.status = format!("Theme: {:?}", self.theme_mode);
        self.last_refresh = Instant::now();
    }

    fn refresh_leaves(&mut self) {
        match fetch_leaves() {
            Ok(mut leaves) => {
                leaves.sort();
                self.leaves = leaves;
                if self.leaves.is_empty() {
                    self.selected_index = None;
                } else if self
                    .selected_index
                    .map(|idx| idx >= self.leaves.len())
                    .unwrap_or(true)
                {
                    self.selected_index = Some(0);
                }
                self.last_leaves_refresh = Some(Instant::now());
                self.last_error = None;
                self.status = "Leaves updated".to_string();
                self.load_selected_details();
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Failed to refresh".to_string();
            }
        }
        self.last_refresh = Instant::now();
    }

    fn filtered_leaves(&self) -> Vec<(usize, &str)> {
        if self.search_query.is_empty() {
            return self
                .leaves
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.as_str()))
                .collect();
        }
        let needle = self.search_query.to_lowercase();
        self.leaves
            .iter()
            .enumerate()
            .filter(|(_, item)| item.to_lowercase().contains(&needle))
            .map(|(idx, item)| (idx, item.as_str()))
            .collect()
    }

    fn selected_leaf(&self) -> Option<&str> {
        let selected = self.selected_index?;
        self.leaves.get(selected).map(String::as_str)
    }

    fn select_next(&mut self) {
        if self.leaves.is_empty() {
            self.selected_index = None;
            return;
        }
        let next = match self.selected_index {
            Some(idx) => (idx + 1).min(self.leaves.len() - 1),
            None => 0,
        };
        self.selected_index = Some(next);
    }

    fn select_prev(&mut self) {
        if self.leaves.is_empty() {
            self.selected_index = None;
            return;
        }
        let prev = match self.selected_index {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.selected_index = Some(prev);
    }

    fn load_selected_details(&mut self) {
        let Some(pkg) = self.selected_leaf() else {
            return;
        };
        if self.details_cache.contains_key(pkg) {
            return;
        }
        match fetch_details(pkg) {
            Ok(details) => {
                self.details_cache.insert(pkg.to_string(), details);
                self.last_error = None;
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
            }
        }
    }
}

#[derive(Clone, Copy)]
enum InputMode {
    Normal,
    Search,
}

#[derive(Clone)]
struct Details {
    desc: Option<String>,
    homepage: Option<String>,
    installed: Vec<String>,
    deps: Vec<String>,
    uses: Vec<String>,
}

#[derive(serde::Deserialize)]
struct BrewInfo {
    #[serde(default)]
    formulae: Vec<FormulaInfo>,
}

#[derive(serde::Deserialize)]
struct FormulaInfo {
    desc: Option<String>,
    homepage: Option<String>,
    #[serde(default)]
    installed: Vec<InstalledInfo>,
}

#[derive(serde::Deserialize)]
struct InstalledInfo {
    version: String,
}

fn fetch_leaves() -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("brew")
        .arg("leaves")
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew leaves failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let leaves = stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect();

    Ok(leaves)
}

fn fetch_details(pkg: &str) -> anyhow::Result<Details> {
    let output = std::process::Command::new("brew")
        .args(["info", "--json=v2", pkg])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("brew info failed for {pkg}")
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let info: BrewInfo = serde_json::from_slice(&output.stdout)?;
    let formula = info
        .formulae
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("No formula info for {pkg}"))?;

    let installed = formula
        .installed
        .iter()
        .map(|item| item.version.clone())
        .collect();

    let deps = run_brew_lines(["deps", "--installed", pkg])?;
    let uses = run_brew_lines(["uses", "--installed", pkg])?;

    Ok(Details {
        desc: formula.desc.clone(),
        homepage: formula.homepage.clone(),
        installed,
        deps,
        uses,
    })
}

fn run_brew_lines<const N: usize>(args: [&str; N]) -> anyhow::Result<Vec<String>> {
    let output = std::process::Command::new("brew").args(args).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew command failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}

async fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    let mut app = App::new();
    app.refresh_leaves();

    loop {
        terminal.draw(|frame| draw(frame, &app))?;

        if event::poll(TICK_RATE)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('r') => app.refresh_leaves(),
                            KeyCode::Char('t') => app.cycle_theme(),
                            KeyCode::Char('/') => {
                                app.input_mode = InputMode::Search;
                                app.search_query.clear();
                                app.status = "Search".to_string();
                                app.last_refresh = Instant::now();
                            }
                            KeyCode::Enter => {
                                app.load_selected_details();
                                app.status = "Details loaded".to_string();
                                app.last_refresh = Instant::now();
                            }
                            KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                            KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                            _ => {}
                        },
                        InputMode::Search => match key.code {
                            KeyCode::Esc | KeyCode::Enter => {
                                app.input_mode = InputMode::Normal;
                                app.status = "Ready".to_string();
                                app.last_refresh = Instant::now();
                            }
                            KeyCode::Backspace => {
                                app.search_query.pop();
                            }
                            KeyCode::Char(ch) => {
                                app.search_query.push(ch);
                            }
                            _ => {}
                        },
                    }
                }
            }
        }

        app.on_tick();
    }
}

fn draw(frame: &mut ratatui::Frame, app: &App) {
    let theme = &app.theme;

    // Fill background
    let bg_block = Block::default().style(Style::default().bg(theme.bg_main));
    frame.render_widget(bg_block, frame.area());

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(frame.area());

    draw_header(frame, layout[0], app);
    draw_body(frame, layout[1], app);
    draw_footer(frame, layout[2], app);
}

fn draw_header(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let uptime = app.started_at.elapsed().as_secs();

    let theme_indicator = match app.theme_mode {
        ThemeMode::Auto => "auto",
        ThemeMode::Light => "light",
        ThemeMode::Dark => "dark",
    };

    let title = Line::from(vec![
        Span::styled(
            " brewery ",
            Style::default()
                .fg(theme.text_on_accent)
                .bg(theme.amber)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("Homebrew console", Style::default().fg(theme.text_secondary)),
    ]);

    let status = Line::from(vec![
        Span::styled("status ", Style::default().fg(theme.text_secondary)),
        Span::styled(&app.status, Style::default().fg(theme.hop_green)),
        Span::styled("  |  ", Style::default().fg(theme.border)),
        Span::styled(format!("{} ", theme_indicator), Style::default().fg(theme.text_secondary)),
        Span::styled(format!("{}s", uptime), Style::default().fg(theme.copper)),
    ]);

    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(36)])
        .split(area);

    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(title);
    let status_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(status);

    frame.render_widget(header_block, layout[0]);
    frame.render_widget(status_block, layout[1]);
}

fn draw_body(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(45),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
        ])
        .split(area);

    let left_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(columns[0]);

    let middle_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(columns[1]);

    let search_label = match app.input_mode {
        InputMode::Search => "Search (type, Enter to apply)",
        InputMode::Normal => "Search (/ to focus)",
    };
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_header))
        .title(Span::styled(search_label, Style::default().fg(theme.amber)));

    let search_text = if app.search_query.is_empty() {
        Span::styled("type to filter leaves", Style::default().fg(theme.text_secondary))
    } else {
        Span::styled(&app.search_query, Style::default().fg(theme.text_primary))
    };

    let search = Paragraph::new(Line::from(vec![search_text]))
        .block(search_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(search, left_rows[0]);

    let leaves = app.filtered_leaves();
    let leaves_title = format!("Leaves ({})", leaves.len());
    let list_items: Vec<ListItem> = if leaves.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "No leaves found",
            Style::default().fg(theme.text_secondary),
        )))]
    } else {
        leaves
            .iter()
            .map(|(_, item)| {
                ListItem::new(Line::from(Span::styled(
                    *item,
                    Style::default().fg(theme.text_primary),
                )))
            })
            .collect()
    };

    let leaves_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled(leaves_title, Style::default().fg(theme.amber)));
    let leaves_list = List::new(list_items)
        .block(leaves_block)
        .highlight_style(Style::default().bg(theme.amber).fg(theme.text_on_accent));
    let mut list_state = ListState::default();
    let selected_pos = app
        .selected_index
        .and_then(|selected| leaves.iter().position(|(idx, _)| *idx == selected));
    list_state.select(selected_pos);
    frame.render_stateful_widget(leaves_list, left_rows[1], &mut list_state);

    // System panel
    let system = Paragraph::new(vec![
        Line::from(Span::styled("brew --version", Style::default().fg(theme.dark_amber))),
        Line::from(Span::styled("brew --prefix", Style::default().fg(theme.dark_amber))),
        Line::from(Span::styled("brew doctor", Style::default().fg(theme.dark_amber))),
        Line::from(""),
        Line::from(Span::styled("Press r to refresh", Style::default().fg(theme.text_secondary))),
        Line::from(Span::styled("/ to search leaves", Style::default().fg(theme.text_secondary))),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.copper))
            .style(Style::default().bg(theme.bg_panel))
            .title(Span::styled("System", Style::default().fg(theme.amber))),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(system, middle_rows[0]);

    // Activity panel
    let last_refresh = app
        .last_leaves_refresh
        .map(|instant| format!("{}s ago", instant.elapsed().as_secs()))
        .unwrap_or_else(|| "never".to_string());
    let activity = Paragraph::new(vec![
        Line::from(Span::styled(
            format!("Leaves refresh: {}", last_refresh),
            Style::default().fg(theme.text_primary),
        )),
        Line::from(Span::styled("Queue: empty", Style::default().fg(theme.text_primary))),
        Line::from(""),
        Line::from(Span::styled("Errors", Style::default().fg(theme.hop_green))),
        Line::from(Span::styled(
            app.last_error
                .as_deref()
                .unwrap_or("none"),
            Style::default().fg(theme.text_secondary),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.copper))
            .style(Style::default().bg(theme.bg_panel))
            .title(Span::styled("Activity", Style::default().fg(theme.amber))),
    )
    .wrap(Wrap { trim: true });
    frame.render_widget(activity, middle_rows[1]);

    // Details panel
    let details_lines = build_details_lines(app);
    let details_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.copper))
        .style(Style::default().bg(theme.bg_panel))
        .title(Span::styled("Details", Style::default().fg(theme.amber)));
    let details = Paragraph::new(details_lines)
        .block(details_block)
        .wrap(Wrap { trim: true });
    frame.render_widget(details, columns[2]);
}

fn draw_footer(frame: &mut ratatui::Frame, area: Rect, app: &App) {
    let theme = &app.theme;

    let text = Line::from(vec![
        Span::styled(" q ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" quit  ", Style::default().fg(theme.text_primary)),
        Span::styled(" r ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" refresh  ", Style::default().fg(theme.text_primary)),
        Span::styled(" Enter ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" details  ", Style::default().fg(theme.text_primary)),
        Span::styled(" / ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" search  ", Style::default().fg(theme.text_primary)),
        Span::styled(" t ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" theme  ", Style::default().fg(theme.text_primary)),
        Span::styled(" ? ", Style::default().bg(theme.amber).fg(theme.text_on_accent)),
        Span::styled(" help", Style::default().fg(theme.text_primary)),
    ]);

    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_header));
    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, area);
}

fn build_details_lines(app: &App) -> Vec<Line<'static>> {
    let theme = &app.theme;
    let Some(pkg) = app.selected_leaf() else {
        return vec![Line::from(Span::styled(
            "No leaves installed".to_string(),
            Style::default().fg(theme.text_secondary),
        ))];
    };

    let mut lines = Vec::new();
    lines.push(Line::from(Span::styled(
        pkg.to_string(),
        Style::default().fg(theme.amber).add_modifier(Modifier::BOLD),
    )));

    if let Some(details) = app.details_cache.get(pkg) {
        if let Some(desc) = details.desc.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                desc.to_string(),
                Style::default().fg(theme.text_primary),
            )));
        }

        if let Some(homepage) = details.homepage.as_ref() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Homepage".to_string(),
                Style::default().fg(theme.text_secondary),
            )));
            lines.push(Line::from(Span::styled(
                homepage.to_string(),
                Style::default().fg(theme.dark_amber),
            )));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Installed: {}", format_list_inline(&details.installed)),
            Style::default().fg(theme.text_primary),
        )));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Deps ({})", details.deps.len()),
            Style::default().fg(theme.text_secondary),
        )));
        lines.extend(format_list_multiline(&details.deps, theme));

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("Used by ({})", details.uses.len()),
            Style::default().fg(theme.text_secondary),
        )));
        lines.extend(format_list_multiline(&details.uses, theme));
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Press Enter to load details".to_string(),
            Style::default().fg(theme.text_secondary),
        )));
    }

    lines
}

fn format_list_inline(items: &[String]) -> String {
    if items.is_empty() {
        return "none".to_string();
    }
    items.join(", ")
}

fn format_list_multiline(items: &[String], theme: &Theme) -> Vec<Line<'static>> {
    if items.is_empty() {
        return vec![Line::from(Span::styled(
            "- none".to_string(),
            Style::default().fg(theme.text_secondary),
        ))];
    }

    items
        .iter()
        .take(8)
        .map(|item| {
            Line::from(Span::styled(
                format!("- {item}"),
                Style::default().fg(theme.text_primary),
            ))
        })
        .collect()
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}
