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
            leaves: Vec::new(),
            filtered_leaves: Vec::new(),
            outdated_leaves: std::collections::HashSet::new(),
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
            pending_upgrade_all_outdated: false,
            pending_self_update: false,
            pending_leaves: false,
            last_leaves_refresh: None,
            last_sizes_refresh: None,
            focus_panel: FocusedPanel::Leaves,
            sizes_scroll_offset: 0,
            details_scroll_offset: 0,
            status_scroll_offset: 0,
            system_status: None,
            pending_status: false,
            last_status_check: None,
            status_tab: StatusTab::default(),
            leaves_outdated_only: false,
            show_help_popup: false,
            help_scroll_offset: 0,
            needs_redraw: true,
            last_selection_change: None,
            recent_selection_count: 0,
        }
    }

    pub fn on_tick(&mut self) {
        if self.pending_command {
            self.needs_redraw = true;
        }

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
            if self.status != "Idle" {
                self.status = "Idle".to_string();
                self.needs_redraw = true;
            }
        }

        if self
            .toast
            .as_ref()
            .map(|toast| toast.created_at.elapsed() > TOAST_DURATION)
            .unwrap_or(false)
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

    pub fn status_tab_next(&mut self) {
        self.status_tab = match self.status_tab {
            StatusTab::Activity => StatusTab::Issues,
            StatusTab::Issues => StatusTab::Outdated,
            StatusTab::Outdated => StatusTab::Activity,
        };
        self.status_scroll_offset = 0; // Reset scroll when switching tabs
    }

    pub fn status_tab_prev(&mut self) {
        self.status_tab = match self.status_tab {
            StatusTab::Activity => StatusTab::Outdated,
            StatusTab::Issues => StatusTab::Activity,
            StatusTab::Outdated => StatusTab::Issues,
        };
        self.status_scroll_offset = 0;
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
                self.status_scroll_offset = self.status_scroll_offset.saturating_sub(1);
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
                let max_scroll = self.max_status_scroll();
                self.status_scroll_offset = (self.status_scroll_offset + 1).min(max_scroll);
            }
            FocusedPanel::Details => {
                self.details_scroll_offset += 1;
            }
        }
    }

    pub(super) fn max_status_scroll(&self) -> usize {
        self.system_status
            .as_ref()
            .map(|h| {
                let count = match self.status_tab {
                    StatusTab::Outdated => h.outdated_packages.len(),
                    StatusTab::Issues => h.doctor_issues.len(),
                    StatusTab::Activity => self.activity_item_count(),
                };
                count.saturating_sub(2)
            })
            .unwrap_or(0)
    }

    fn activity_item_count(&self) -> usize {
        let Some(system_status) = self.system_status.as_ref() else {
            return 0;
        };

        let mut count = 0;
        if self.pending_command
            && self
                .last_command
                .map(CommandKind::is_activity_command)
                .unwrap_or(false)
        {
            count += 1 + self.last_command_output.len();
            if self.last_command_target.is_some()
                || matches!(
                    self.last_command,
                    Some(CommandKind::UpgradeAll | CommandKind::SelfUpdate)
                )
            {
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
        if system_status.brew_version.is_some() {
            count += 1;
        }
        count += 2; // doctor + packages
        if system_status.brew_update_status.is_some() {
            count += 1;
        }
        if system_status.last_brew_update_secs_ago.is_some() {
            count += 1;
        }
        if self.last_status_check.is_some() {
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
}

fn detect_icon_ascii() -> bool {
    if let Ok(value) = std::env::var("BREWERY_ASCII")
        && (value == "1" || value.eq_ignore_ascii_case("true"))
    {
        return true;
    }
    false
}
