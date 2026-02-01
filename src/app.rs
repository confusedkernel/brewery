use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::brew::{
    fetch_details_basic, fetch_details_full, fetch_health, fetch_leaves, fetch_sizes,
    run_brew_command, CommandMessage, Details, DetailsLoad, DetailsMessage, HealthMessage,
    HealthStatus, SizeEntry, SizesMessage,
};
use crate::theme::{detect_system_theme, Theme, ThemeMode};

pub struct App {
    pub started_at: Instant,
    pub last_refresh: Instant,
    pub status: String,
    pub theme_mode: ThemeMode,
    pub theme: Theme,
    pub input_mode: InputMode,
    pub leaves_query: String,
    pub package_query: String,
    pub leaves: Vec<String>,
    pub filtered_leaves: Vec<usize>,
    pub package_results_selected: Option<usize>,
    pub last_package_search: Option<String>,
    pub last_result_details_pkg: Option<String>,
    pub selected_index: Option<usize>,
    pub details_cache: HashMap<String, Details>,
    pub pending_details: Option<String>,
    pub package_results: Vec<String>,
    pub view_mode: ViewMode,
    pub sizes: Vec<SizeEntry>,
    pub pending_sizes: bool,
    pub icon_mode: IconMode,
    pub icons_ascii: bool,
    pub pending_command: bool,
    pub last_command: Option<String>,
    pub last_command_output: Vec<String>,
    pub last_command_error: Option<String>,
    pub last_error: Option<String>,
    pub last_leaves_refresh: Option<Instant>,
    pub last_sizes_refresh: Option<Instant>,
    pub focus_panel: FocusedPanel,
    pub sizes_scroll_offset: usize,
    pub details_scroll_offset: usize,
    pub health_scroll_offset: usize,
    pub health: Option<HealthStatus>,
    pub pending_health: bool,
    pub last_health_check: Option<Instant>,
    pub health_tab: HealthTab,
    pub show_help_popup: bool,
    pub help_scroll_offset: usize,
}

impl App {
    pub fn new() -> Self {
        let theme = detect_system_theme();
        Self {
            started_at: Instant::now(),
            last_refresh: Instant::now(),
            status: "Ready".to_string(),
            theme_mode: ThemeMode::Auto,
            theme,
            input_mode: InputMode::Normal,
            leaves_query: String::new(),
            package_query: String::new(),
            leaves: Vec::new(),
            filtered_leaves: Vec::new(),
            package_results_selected: None,
            last_package_search: None,
            last_result_details_pkg: None,
            selected_index: Some(0),
            details_cache: HashMap::new(),
            pending_details: None,
            package_results: Vec::new(),
            view_mode: ViewMode::Details,
            sizes: Vec::new(),
            pending_sizes: false,
            icon_mode: IconMode::Auto,
            icons_ascii: detect_icon_ascii(),
            pending_command: false,
            last_command: None,
            last_command_output: Vec::new(),
            last_command_error: None,
            last_error: None,
            last_leaves_refresh: None,
            last_sizes_refresh: None,
            focus_panel: FocusedPanel::Leaves,
            sizes_scroll_offset: 0,
            details_scroll_offset: 0,
            health_scroll_offset: 0,
            health: None,
            pending_health: false,
            last_health_check: None,
            health_tab: HealthTab::default(),
            show_help_popup: false,
            help_scroll_offset: 0,
        }
    }

    pub fn on_tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            self.status = "Idle".to_string();
        }
    }

    pub fn cycle_theme(&mut self) {
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

    pub fn toggle_icons(&mut self) {
        self.icon_mode = match self.icon_mode {
            IconMode::Auto => IconMode::Ascii,
            IconMode::Ascii => IconMode::Nerd,
            IconMode::Nerd => IconMode::Ascii,
        };
        self.icons_ascii = match self.icon_mode {
            IconMode::Ascii => true,
            IconMode::Nerd => false,
            IconMode::Auto => detect_icon_ascii(),
        };
        self.status = format!(
            "Icons: {}",
            if self.icons_ascii { "ASCII" } else { "Nerd" }
        );
        self.last_refresh = Instant::now();
    }

    pub fn cycle_focus(&mut self) {
        self.focus_panel = match self.focus_panel {
            FocusedPanel::Leaves => FocusedPanel::Sizes,
            FocusedPanel::Sizes => FocusedPanel::Health,
            FocusedPanel::Health => FocusedPanel::Details,
            FocusedPanel::Details => FocusedPanel::Leaves,
        };
        self.status = format!("Focus: {:?}", self.focus_panel);
        self.last_refresh = Instant::now();
    }

    pub fn health_tab_next(&mut self) {
        self.health_tab = match self.health_tab {
            HealthTab::Activity => HealthTab::Issues,
            HealthTab::Issues => HealthTab::Outdated,
            HealthTab::Outdated => HealthTab::Activity,
        };
        self.health_scroll_offset = 0; // Reset scroll when switching tabs
    }

    pub fn health_tab_prev(&mut self) {
        self.health_tab = match self.health_tab {
            HealthTab::Activity => HealthTab::Outdated,
            HealthTab::Issues => HealthTab::Activity,
            HealthTab::Outdated => HealthTab::Issues,
        };
        self.health_scroll_offset = 0;
    }

    pub fn toggle_help(&mut self) {
        self.show_help_popup = !self.show_help_popup;
        self.help_scroll_offset = 0;
    }

    pub fn scroll_focused_up(&mut self) {
        match self.focus_panel {
            FocusedPanel::Leaves => self.select_prev(),
            FocusedPanel::Sizes => {
                self.sizes_scroll_offset = self.sizes_scroll_offset.saturating_sub(1);
            }
            FocusedPanel::Health => {
                self.health_scroll_offset = self.health_scroll_offset.saturating_sub(1);
            }
            FocusedPanel::Details => {
                self.details_scroll_offset = self.details_scroll_offset.saturating_sub(1);
            }
        }
    }

    pub fn scroll_focused_down(&mut self) {
        match self.focus_panel {
            FocusedPanel::Leaves => self.select_next(),
            FocusedPanel::Sizes => {
                let max_scroll = self.sizes.len().saturating_sub(1);
                self.sizes_scroll_offset = (self.sizes_scroll_offset + 1).min(max_scroll);
            }
            FocusedPanel::Health => {
                let max_scroll = self.max_health_scroll();
                self.health_scroll_offset = (self.health_scroll_offset + 1).min(max_scroll);
            }
            FocusedPanel::Details => {
                self.details_scroll_offset += 1;
            }
        }
    }

    fn max_health_scroll(&self) -> usize {
        self.health
            .as_ref()
            .map(|h| {
                let count = match self.health_tab {
                    HealthTab::Outdated => h.outdated_packages.len(),
                    HealthTab::Issues => h.doctor_issues.len(),
                    HealthTab::Activity => 7, // Fixed number of activity items
                };
                count.saturating_sub(2)
            })
            .unwrap_or(0)
    }

    pub fn refresh_leaves(&mut self) {
        match fetch_leaves() {
            Ok(mut leaves) => {
                leaves.sort();
                self.leaves = leaves;
                self.update_filtered_leaves();
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
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Failed to refresh".to_string();
            }
        }
        self.last_refresh = Instant::now();
    }

    pub fn filtered_leaves(&self) -> Vec<(usize, &str)> {
        self.filtered_leaves
            .iter()
            .filter_map(|idx| self.leaves.get(*idx).map(|item| (*idx, item.as_str())))
            .collect()
    }

    pub fn selected_package_result(&self) -> Option<&str> {
        let selected = self.package_results_selected?;
        self.package_results.get(selected).map(String::as_str)
    }

    pub fn selected_package_name(&self) -> Option<&str> {
        if self.input_mode == InputMode::PackageSearch {
            self.selected_package_result()
        } else {
            self.selected_leaf()
        }
    }

    pub fn select_next_result(&mut self) {
        if self.package_results.is_empty() {
            self.package_results_selected = None;
            return;
        }
        let next = match self.package_results_selected {
            Some(idx) => (idx + 1).min(self.package_results.len() - 1),
            None => 0,
        };
        self.package_results_selected = Some(next);
        self.last_result_details_pkg = None;
    }

    pub fn select_prev_result(&mut self) {
        if self.package_results.is_empty() {
            self.package_results_selected = None;
            return;
        }
        let prev = match self.package_results_selected {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.package_results_selected = Some(prev);
        self.last_result_details_pkg = None;
    }

    pub fn clear_package_results(&mut self) {
        self.package_results.clear();
        self.package_results_selected = None;
        self.last_package_search = None;
        self.last_result_details_pkg = None;
    }

    pub fn update_filtered_leaves(&mut self) {
        if self.leaves_query.is_empty() {
            self.filtered_leaves = (0..self.leaves.len()).collect();
            return;
        }

        let needle = self.leaves_query.to_lowercase();
        self.filtered_leaves = self
            .leaves
            .iter()
            .enumerate()
            .filter(|(_, item)| item.to_lowercase().contains(&needle))
            .map(|(idx, _)| idx)
            .collect();
    }

    pub fn selected_leaf(&self) -> Option<&str> {
        let selected = self.selected_index?;
        self.leaves.get(selected).map(String::as_str)
    }

    pub fn select_next(&mut self) {
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

    pub fn select_prev(&mut self) {
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

    pub fn request_details(
        &mut self,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        let Some(pkg) = self.selected_leaf().map(str::to_string) else {
            return;
        };

        self.request_details_for(&pkg, load, tx);
    }

    pub fn request_details_for(
        &mut self,
        pkg: &str,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        let pkg = pkg.to_string();

        if let Some(pending) = self.pending_details.as_ref() {
            if pending == &pkg {
                return;
            }
        }

        if let Some(existing) = self.details_cache.get(&pkg) {
            match load {
                DetailsLoad::Basic => return,
                DetailsLoad::Full => {
                    if existing.deps.is_some() && existing.uses.is_some() {
                        return;
                    }
                }
            }
        }

        self.pending_details = Some(pkg.clone());
        self.status = match load {
            DetailsLoad::Basic => "Loading details...".to_string(),
            DetailsLoad::Full => "Loading deps/uses...".to_string(),
        };
        self.last_refresh = Instant::now();

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = match load {
                DetailsLoad::Basic => fetch_details_basic(&pkg).await,
                DetailsLoad::Full => fetch_details_full(&pkg).await,
            };
            let _ = tx.send(DetailsMessage { pkg, load, result });
        });
    }

    pub fn apply_details_message(&mut self, message: DetailsMessage) {
        match message.result {
            Ok(details) => {
                self.details_cache
                    .entry(message.pkg.clone())
                    .and_modify(|existing| merge_details(existing, &details))
                    .or_insert(details);
                self.last_error = None;
                self.status = match message.load {
                    DetailsLoad::Basic => "Details loaded".to_string(),
                    DetailsLoad::Full => "Deps/uses loaded".to_string(),
                };
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Details failed".to_string();
            }
        }

        self.pending_details = None;
        self.last_refresh = Instant::now();
    }

    pub fn request_sizes(&mut self, tx: &mpsc::UnboundedSender<SizesMessage>) {
        if self.pending_sizes {
            return;
        }

        self.pending_sizes = true;
        self.status = "Loading sizes...".to_string();
        self.last_refresh = Instant::now();

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_sizes().await;
            let _ = tx.send(SizesMessage { result });
        });
    }

    pub fn apply_sizes_message(&mut self, message: SizesMessage) {
        match message.result {
            Ok(sizes) => {
                self.sizes = sizes;
                if self.sizes.is_empty() {
                    self.sizes_scroll_offset = 0;
                } else {
                    let max_scroll = self.sizes.len().saturating_sub(1);
                    self.sizes_scroll_offset = self.sizes_scroll_offset.min(max_scroll);
                }
                self.last_error = None;
                self.status = "Sizes updated".to_string();
                self.last_sizes_refresh = Some(Instant::now());
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Sizes failed".to_string();
            }
        }

        self.pending_sizes = false;
        self.last_refresh = Instant::now();
    }

    pub fn request_health(&mut self, tx: &mpsc::UnboundedSender<HealthMessage>) {
        if self.pending_health {
            return;
        }

        self.pending_health = true;
        self.status = "Checking health...".to_string();
        self.last_refresh = Instant::now();

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_health().await;
            let _ = tx.send(HealthMessage { result });
        });
    }

    pub fn apply_health_message(&mut self, message: HealthMessage) {
        match message.result {
            Ok(health) => {
                self.health = Some(health);
                let max_scroll = self.max_health_scroll();
                self.health_scroll_offset = self.health_scroll_offset.min(max_scroll);
                self.last_error = None;
                self.status = "Health check complete".to_string();
                self.last_health_check = Some(Instant::now());
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Health check failed".to_string();
            }
        }

        self.pending_health = false;
        self.last_refresh = Instant::now();
    }

    pub fn request_command(&mut self, label: &str, args: &[&str], tx: &mpsc::UnboundedSender<CommandMessage>) {
        if self.pending_command {
            return;
        }

        self.pending_command = true;
        self.last_command = Some(label.to_string());
        self.last_command_output.clear();
        self.last_command_error = None;
        self.status = format!("Running {label}...");
        self.last_refresh = Instant::now();

        let tx = tx.clone();
        let label = label.to_string();
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        tokio::spawn(async move {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let result = run_brew_command(&arg_refs).await;
            let _ = tx.send(CommandMessage { label, result });
        });
    }

    pub fn apply_command_message(&mut self, message: CommandMessage) {
        match message.result {
            Ok(result) => {
                let lines: Vec<String> = if result.stdout.trim().is_empty() {
                    result.stderr.lines().map(str::to_string).collect()
                } else {
                    result.stdout.lines().map(str::to_string).collect()
                };
                self.last_command_output = lines.into_iter().take(8).collect();
                if result.success {
                    self.status = format!("{label} complete", label = message.label);
                } else {
                    self.status = format!("{label} failed", label = message.label);
                    if !result.stderr.trim().is_empty() {
                        self.last_command_error = Some(result.stderr.trim().to_string());
                    }
                }

                if message.label == "search" {
                    self.package_results = result
                        .stdout
                        .lines()
                        .map(str::trim)
                        .filter(|line| !line.is_empty())
                        .map(|line| line.to_string())
                        .collect();
                    if self.package_results.is_empty() {
                        self.package_results_selected = None;
                    } else {
                        self.package_results_selected = Some(0);
                    }
                    self.last_result_details_pkg = None;
                    self.status = format!("Search results: {}", self.package_results.len());
                }
            }
            Err(err) => {
                self.last_command_error = Some(err.to_string());
                self.status = format!("{label} failed", label = message.label);
            }
        }

        self.pending_command = false;
        self.last_refresh = Instant::now();
    }
}

fn merge_details(existing: &mut Details, incoming: &Details) {
    existing.desc = incoming.desc.clone();
    existing.homepage = incoming.homepage.clone();
    existing.installed = incoming.installed.clone();
    if incoming.deps.is_some() {
        existing.deps = incoming.deps.clone();
    }
    if incoming.uses.is_some() {
        existing.uses = incoming.uses.clone();
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum InputMode {
    Normal,
    SearchLeaves,
    PackageSearch,
    PackageInstall,
    PackageUninstall,
}

#[derive(Clone, Copy, PartialEq)]
pub enum IconMode {
    Auto,
    Nerd,
    Ascii,
}

fn detect_icon_ascii() -> bool {
    if let Ok(value) = std::env::var("BREWERY_ASCII") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    false
}

#[derive(Clone, Copy, PartialEq)]
pub enum ViewMode {
    Details,
    PackageResults,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FocusedPanel {
    Leaves,
    Sizes,
    Health,
    Details,
}

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum HealthTab {
    #[default]
    Activity,
    Issues,
    Outdated,
}
