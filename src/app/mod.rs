mod types;

pub use types::{
    FocusedPanel, HealthTab, IconMode, InputMode, PackageAction, PendingPackageAction, ViewMode,
};

use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use lru::LruCache;
use tokio::sync::mpsc;

use crate::brew::{
    CommandMessage, Details, DetailsLoad, DetailsMessage, HealthMessage, HealthStatus,
    LeavesMessage, SizeEntry, SizesMessage, fetch_details_basic, fetch_details_full, fetch_health,
    fetch_leaves, fetch_sizes, run_brew_command,
};
use crate::theme::{Theme, ThemeMode, detect_system_theme};

/// Maximum number of package details to cache
const DETAILS_CACHE_CAPACITY: usize = 64;

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
    pub filtered_leaves_dirty: bool,
    pub package_results_selected: Option<usize>,
    pub last_package_search: Option<String>,
    pub last_result_details_pkg: Option<String>,
    pub selected_index: Option<usize>,
    pub details_cache: LruCache<String, Details>,
    pub pending_details: Option<String>,
    pub package_results: Vec<String>,
    pub view_mode: ViewMode,
    pub sizes: Vec<SizeEntry>,
    pub pending_sizes: bool,
    pub icon_mode: IconMode,
    pub icons_ascii: bool,
    pub pending_command: bool,
    pub last_command: Option<String>,
    pub last_command_target: Option<String>,
    pub command_started_at: Option<Instant>,
    pub last_command_completed: Option<(String, String, Instant)>,
    pub last_command_output: Vec<String>,
    pub last_command_error: Option<String>,
    pub last_error: Option<String>,
    pub pending_package_action: Option<PendingPackageAction>,
    pub pending_leaves: bool,
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
    pub needs_redraw: bool,
    pub last_selection_change: Option<Instant>,
    /// Count of recent selection changes (for detecting rapid scrolling)
    pub recent_selection_count: u8,
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
            filtered_leaves_dirty: true,
            package_results_selected: None,
            last_package_search: None,
            last_result_details_pkg: None,
            selected_index: Some(0),
            details_cache: LruCache::new(NonZeroUsize::new(DETAILS_CACHE_CAPACITY).unwrap()),
            pending_details: None,
            package_results: Vec::new(),
            view_mode: ViewMode::Details,
            sizes: Vec::new(),
            pending_sizes: false,
            icon_mode: IconMode::Auto,
            icons_ascii: detect_icon_ascii(),
            pending_command: false,
            last_command: None,
            last_command_target: None,
            command_started_at: None,
            last_command_completed: None,
            last_command_output: Vec::new(),
            last_command_error: None,
            last_error: None,
            pending_package_action: None,
            pending_leaves: false,
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
            needs_redraw: true,
            last_selection_change: None,
            recent_selection_count: 0,
        }
    }

    pub fn on_tick(&mut self) {
        // Always request redraw on tick for spinner animation and elapsed time updates
        self.needs_redraw = true;

        // Decay the rapid scroll counter over time
        // This allows the counter to reset if the user pauses
        if self
            .last_selection_change
            .map(|t| t.elapsed() >= Duration::from_millis(300))
            .unwrap_or(true)
        {
            self.recent_selection_count = 0;
        }

        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            self.status = "Idle".to_string();
        }
    }

    /// Call this when the selection changes (scrolling through list)
    /// Tracks rapid scrolling to avoid excessive detail fetches
    pub fn on_selection_change(&mut self) {
        self.last_selection_change = Some(Instant::now());
        // Increment counter, saturating at 255
        self.recent_selection_count = self.recent_selection_count.saturating_add(1);
        self.needs_redraw = true;
    }

    /// Returns true if the user appears to be rapidly scrolling
    /// (more than 2 selection changes without a pause)
    pub fn is_rapid_scrolling(&self) -> bool {
        self.recent_selection_count > 2
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
        self.status = format!("Icons: {}", if self.icons_ascii { "ASCII" } else { "Nerd" });
        self.last_refresh = Instant::now();
    }

    pub fn cycle_focus(&mut self) {
        self.focus_panel = match self.focus_panel {
            FocusedPanel::Leaves => FocusedPanel::Sizes,
            FocusedPanel::Sizes => FocusedPanel::Status,
            FocusedPanel::Status => FocusedPanel::Details,
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
            FocusedPanel::Status => {
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
            FocusedPanel::Status => {
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
                    HealthTab::Activity => self.activity_item_count(),
                };
                count.saturating_sub(2)
            })
            .unwrap_or(0)
    }

    fn activity_item_count(&self) -> usize {
        let Some(health) = self.health.as_ref() else {
            return 0;
        };

        let mut count = 0;
        if self.pending_command
            && matches!(
                self.last_command.as_deref(),
                Some("install") | Some("uninstall")
            )
        {
            count += 1 + self.last_command_output.len();
            if self.last_command_target.is_some() {
                count += 1;
            }
        }
        if self
            .last_command_completed
            .as_ref()
            .map(|(_, _, at)| at.elapsed().as_secs() < 3)
            .unwrap_or(false)
        {
            count += 1;
        }
        if health.brew_version.is_some() {
            count += 1;
        }
        count += 2; // doctor + packages
        if self.last_health_check.is_some() {
            count += 1;
        }
        if self.last_leaves_refresh.is_some() {
            count += 1;
        }
        if self.last_sizes_refresh.is_some() {
            count += 1;
        }
        if self.last_command.is_some() {
            count += 1;
        }
        count
    }

    pub fn request_leaves(&mut self, tx: &mpsc::UnboundedSender<LeavesMessage>) {
        if self.pending_leaves {
            return;
        }

        self.pending_leaves = true;
        self.status = "Loading leaves...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_leaves().await;
            let _ = tx.send(LeavesMessage { result });
        });
    }

    pub fn apply_leaves_message(&mut self, message: LeavesMessage) {
        match message.result {
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
        self.pending_leaves = false;
        self.last_refresh = Instant::now();
        self.needs_redraw = true;
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
        if matches!(
            self.input_mode,
            InputMode::PackageSearch | InputMode::PackageResults
        ) {
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
        self.filtered_leaves_dirty = false;

        if self.leaves_query.is_empty() {
            self.filtered_leaves = (0..self.leaves.len()).collect();
            if self.leaves.is_empty() {
                self.selected_index = None;
            } else if self.selected_index.is_none() {
                self.selected_index = Some(0);
            } else if self
                .selected_index
                .map(|idx| idx >= self.leaves.len())
                .unwrap_or(true)
            {
                self.selected_index = Some(0);
            }
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

        if self.filtered_leaves.is_empty() {
            self.selected_index = None;
        } else if self
            .selected_index
            .map(|selected| self.filtered_leaves.contains(&selected))
            .unwrap_or(false)
        {
            // keep current selection
        } else {
            self.selected_index = self.filtered_leaves.first().copied();
        }
    }

    pub fn selected_leaf(&self) -> Option<&str> {
        let selected = self.selected_index?;
        self.leaves.get(selected).map(String::as_str)
    }

    pub fn select_next(&mut self) {
        if self.leaves_query.is_empty() {
            if self.leaves.is_empty() {
                self.selected_index = None;
                return;
            }
            let next = match self.selected_index {
                Some(idx) => (idx + 1).min(self.leaves.len() - 1),
                None => 0,
            };
            self.selected_index = Some(next);
            return;
        }

        if self.filtered_leaves.is_empty() {
            self.selected_index = None;
            return;
        }

        let current_pos = self
            .selected_index
            .and_then(|selected| self.filtered_leaves.iter().position(|idx| *idx == selected));
        let next_pos = match current_pos {
            Some(pos) => (pos + 1).min(self.filtered_leaves.len() - 1),
            None => 0,
        };
        self.selected_index = self.filtered_leaves.get(next_pos).copied();
    }

    pub fn select_prev(&mut self) {
        if self.leaves_query.is_empty() {
            if self.leaves.is_empty() {
                self.selected_index = None;
                return;
            }
            let prev = match self.selected_index {
                Some(idx) => idx.saturating_sub(1),
                None => 0,
            };
            self.selected_index = Some(prev);
            return;
        }

        if self.filtered_leaves.is_empty() {
            self.selected_index = None;
            return;
        }

        let current_pos = self
            .selected_index
            .and_then(|selected| self.filtered_leaves.iter().position(|idx| *idx == selected));
        let prev_pos = match current_pos {
            Some(pos) => pos.saturating_sub(1),
            None => 0,
        };
        self.selected_index = self.filtered_leaves.get(prev_pos).copied();
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
                // LruCache doesn't have entry API, so we handle it manually
                if let Some(existing) = self.details_cache.get_mut(&message.pkg) {
                    merge_details(existing, &details);
                } else {
                    self.details_cache.put(message.pkg.clone(), details);
                }
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
        self.needs_redraw = true;
    }

    pub fn request_sizes(&mut self, tx: &mpsc::UnboundedSender<SizesMessage>) {
        if self.pending_sizes {
            return;
        }

        self.pending_sizes = true;
        self.status = "Loading sizes...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

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
        self.needs_redraw = true;
    }

    pub fn request_health(&mut self, tx: &mpsc::UnboundedSender<HealthMessage>) {
        if self.pending_health {
            return;
        }

        self.pending_health = true;
        self.status = "Checking status...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

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
                self.status = "Status check complete".to_string();
                self.last_health_check = Some(Instant::now());
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Status check failed".to_string();
            }
        }

        self.pending_health = false;
        self.last_refresh = Instant::now();
        self.needs_redraw = true;
    }

    pub fn request_command(
        &mut self,
        label: &str,
        args: &[&str],
        tx: &mpsc::UnboundedSender<CommandMessage>,
    ) {
        if self.pending_command {
            return;
        }

        self.pending_command = true;
        self.last_command = Some(label.to_string());
        self.last_command_target = match label {
            "install" | "uninstall" | "upgrade" => args.last().map(|value| (*value).to_string()),
            _ => None,
        };
        self.command_started_at = Some(Instant::now());
        self.last_command_output.clear();
        self.last_command_error = None;
        self.status = format!("Running {label}...");
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

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
                        self.status = "No results found".to_string();
                    } else {
                        self.package_results_selected = Some(0);
                        // Auto-transition to PackageResults mode
                        self.input_mode = InputMode::PackageResults;
                        self.status = format!("{} results", self.package_results.len());
                    }
                    self.last_result_details_pkg = None;
                }

                if matches!(message.label.as_str(), "install" | "uninstall" | "upgrade") {
                    if let Some(pkg) = self.last_command_target.clone() {
                        self.last_command_completed =
                            Some((message.label.clone(), pkg, Instant::now()));
                    }
                }
            }
            Err(err) => {
                self.last_command_error = Some(err.to_string());
                self.status = format!("{label} failed", label = message.label);
            }
        }

        self.pending_command = false;
        self.command_started_at = None;
        self.last_refresh = Instant::now();
        self.needs_redraw = true;
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

fn detect_icon_ascii() -> bool {
    if let Ok(value) = std::env::var("BREWERY_ASCII") {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            return true;
        }
    }
    false
}
