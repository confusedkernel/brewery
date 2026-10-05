use super::*;

impl App {
    pub fn new() -> Self {
        let theme = detect_system_theme();
        Self {
            started_at: Instant::now(),
            last_refresh: Instant::now(),
            status: "Ready".to_string(),
            toast: None,
            theme_mode: ThemeMode::Auto,
            theme,
            input_mode: InputMode::Normal,
            leaves_query: String::new(),
            package_query: String::new(),
            active_package_kind: PackageKind::Formula,
            leaves: Vec::new(),
            leaf_formulae: Vec::new(),
            all_formulae: Vec::new(),
            leaves_job: Job::default(),
            leaves_only: true,
            sort_mode: SortMode::default(),
            casks: Vec::new(),
            casks_job: Job::default(),
            filtered_leaves: Vec::new(),
            filtered_casks: Vec::new(),
            outdated_leaves: HashSet::new(),
            outdated_casks: HashSet::new(),
            pinned: HashSet::new(),
            package_results_selected: None,
            last_result_details_pkg: None,
            selected_index: Some(0),
            selected_cask_index: Some(0),
            details_cache: LruCache::new(NonZeroUsize::new(DETAILS_CACHE_CAPACITY).unwrap()),
            pending_details: None,
            package_results: Vec::new(),
            view_mode: ViewMode::Details,
            sizes: Vec::new(),
            sizes_job: Job::default(),
            icons_ascii: detect_icon_ascii(),
            mouse_enabled: detect_mouse_enabled(),
            last_command: None,
            command_job: Job::default(),
            last_command_target: None,
            last_command_completed: None,
            last_command_output: Vec::new(),
            last_command_error: None,
            last_error: None,
            pending_confirmation: None,
            command_history: VecDeque::with_capacity(COMMAND_HISTORY_CAPACITY),
            last_command_args: Vec::new(),
            focus_panel: FocusedPanel::Leaves,
            sizes_scroll_offset: 0,
            details_scroll_offset: 0,
            status_scroll_offset: 0,
            dependency_graph: None,
            graph_job: Job::default(),
            system_status: None,
            status_job: Job::default(),
            doctor: None,
            doctor_job: Job::default(),
            status_tab: StatusTab::default(),
            services_selected_index: None,
            services_failed_only: false,
            services_autostart_only: false,
            services_kind_filter: ServiceKindFilter::default(),
            leaves_outdated_only: false,
            show_help_popup: false,
            help_scroll_offset: 0,
            help_selected_command: 0,
            needs_redraw: true,
            last_selection_change: None,
            recent_selection_count: 0,
        }
    }

    /// Sets the transient header status and restarts the idle countdown.
    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
        self.last_refresh = Instant::now();
    }

    pub fn has_pending_confirmation(&self) -> bool {
        self.pending_confirmation.is_some()
    }

    pub fn clear_pending_confirmations(&mut self) {
        self.pending_confirmation = None;
    }

    /// Whether anything with a visible spinner is in flight. The graph and
    /// doctor jobs load quietly and do not count.
    pub fn is_busy(&self) -> bool {
        [
            self.command_job,
            self.leaves_job,
            self.casks_job,
            self.sizes_job,
            self.status_job,
        ]
        .iter()
        .any(Job::is_running)
    }

    pub fn on_tick(&mut self) {
        if self.is_busy() {
            self.needs_redraw = true;
        }

        // Decay the rapid scroll counter over time
        // This allows the counter to reset if the user pauses
        if self
            .last_selection_change
            .is_none_or(|t| t.elapsed() >= Duration::from_millis(300))
        {
            self.recent_selection_count = 0;
        }

        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            if self.status != "Idle" {
                self.status = "Idle".to_string();
                self.needs_redraw = true;
            }
        }

        if self
            .toast
            .as_ref()
            .is_some_and(|toast| toast.created_at.elapsed() > TOAST_DURATION)
        {
            self.toast = None;
            self.needs_redraw = true;
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
        self.set_status(format!("Theme: {:?}", self.theme_mode));
    }

    pub fn toggle_icons(&mut self) {
        self.icons_ascii = !self.icons_ascii;
        self.set_status(format!(
            "Icons: {}",
            if self.icons_ascii { "ASCII" } else { "Nerd" }
        ));
    }

    pub fn toggle_mouse(&mut self) {
        self.mouse_enabled = !self.mouse_enabled;
        self.set_status(format!(
            "Mouse: {}",
            if self.mouse_enabled {
                "enabled"
            } else {
                "disabled"
            }
        ));
    }

    pub fn cycle_focus(&mut self) {
        self.focus_panel = match self.focus_panel {
            FocusedPanel::Leaves => FocusedPanel::Sizes,
            FocusedPanel::Sizes => FocusedPanel::Status,
            FocusedPanel::Status => FocusedPanel::Details,
            FocusedPanel::Details => FocusedPanel::Leaves,
        };
        self.set_focus_status();
    }

    pub fn cycle_focus_back(&mut self) {
        self.focus_panel = match self.focus_panel {
            FocusedPanel::Leaves => FocusedPanel::Details,
            FocusedPanel::Sizes => FocusedPanel::Leaves,
            FocusedPanel::Status => FocusedPanel::Sizes,
            FocusedPanel::Details => FocusedPanel::Status,
        };
        self.set_focus_status();
    }

    /// Focus moves from several places (Tab, S-Tab, mouse clicks); they all
    /// report it the same way.
    pub fn set_focus_status(&mut self) {
        self.set_status(format!("Focus: {:?}", self.focus_panel));
    }

    pub fn select_status_tab(&mut self, tab: StatusTab) {
        if self.status_tab != tab {
            self.status_tab = tab;
            self.status_scroll_offset = 0;
        }
    }

    pub fn cycle_sort_mode(&mut self) {
        if self.is_cask_mode() {
            self.set_status("Sorting only applies to formulae");
            return;
        }

        self.sort_mode = self.sort_mode.next();
        self.sync_installed_list();

        let hint = match self.sort_mode {
            SortMode::Size if self.sizes.is_empty() => " (sizes still loading)",
            SortMode::Recent if self.dependency_graph.is_none() => " (receipts still loading)",
            _ => "",
        };
        self.set_status(format!("Sort: {}{hint}", self.sort_mode.label()));
    }

    pub fn toggle_help(&mut self) {
        self.show_help_popup = !self.show_help_popup;
        self.help_scroll_offset = 0;
        self.help_selected_command = 0;
    }

    pub fn toggle_installed_kind(&mut self) {
        self.active_package_kind = match self.active_package_kind {
            PackageKind::Formula => PackageKind::Cask,
            PackageKind::Cask => PackageKind::Formula,
        };

        if self.active_package_kind == PackageKind::Cask && self.leaves_outdated_only {
            self.leaves_outdated_only = false;
        }

        self.set_status(format!("View: {}", self.active_package_kind.plural()));
    }

    pub fn scroll_focused_up(&mut self) {
        match self.focus_panel {
            FocusedPanel::Leaves => self.select_prev(),
            FocusedPanel::Sizes => {
                self.sizes_scroll_offset = self.sizes_scroll_offset.saturating_sub(1);
            }
            FocusedPanel::Status => {
                if self.status_tab == StatusTab::Services {
                    self.select_prev_service();
                } else {
                    self.status_scroll_offset = self.status_scroll_offset.saturating_sub(1);
                }
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
                if self.status_tab == StatusTab::Services {
                    self.select_next_service();
                } else {
                    let max_scroll = self.max_status_scroll();
                    self.status_scroll_offset = (self.status_scroll_offset + 1).min(max_scroll);
                }
            }
            FocusedPanel::Details => {
                self.details_scroll_offset += 1;
            }
        }
    }

    /// The status panel always keeps its last two lines in view.
    pub(super) fn max_status_scroll(&self) -> usize {
        crate::ui::status_item_count(self).saturating_sub(2)
    }
}

fn detect_icon_ascii() -> bool {
    std::env::var("BREWERY_ASCII")
        .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

fn detect_mouse_enabled() -> bool {
    if let Ok(value) = std::env::var("BREWERY_MOUSE") {
        let value = value.trim();
        if value == "0" || value.eq_ignore_ascii_case("false") || value.eq_ignore_ascii_case("off")
        {
            return false;
        }
    }
    true
}
