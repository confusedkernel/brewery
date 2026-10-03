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
            leaves_only: true,
            sort_mode: SortMode::default(),
            casks: Vec::new(),
            filtered_leaves: Vec::new(),
            filtered_casks: Vec::new(),
            outdated_leaves: HashSet::new(),
            outdated_casks: HashSet::new(),
            pinned: HashSet::new(),
            filtered_leaves_dirty: true,
            package_results_selected: None,
            last_package_search: None,
            last_result_details_pkg: None,
            selected_index: Some(0),
            selected_cask_index: Some(0),
            details_cache: LruCache::new(NonZeroUsize::new(DETAILS_CACHE_CAPACITY).unwrap()),
            pending_details: None,
            package_results: Vec::new(),
            view_mode: ViewMode::Details,
            sizes: Vec::new(),
            pending_sizes: false,
            icon_mode: IconMode::Auto,
            icons_ascii: detect_icon_ascii(),
            mouse_enabled: detect_mouse_enabled(),
            pending_command: false,
            last_command: None,
            last_command_target: None,
            last_command_target_is_cask: false,
            command_started_at: None,
            last_command_completed: None,
            last_command_output: Vec::new(),
            last_command_error: None,
            last_error: None,
            pending_package_action: None,
            pending_service_action: None,
            pending_upgrade_all_outdated: false,
            pending_autoremove: false,
            pending_self_update: false,
            command_history: VecDeque::with_capacity(COMMAND_HISTORY_CAPACITY),
            last_command_args: Vec::new(),
            pending_leaves: false,
            pending_casks: false,
            pending_leaves_started_at: None,
            pending_casks_started_at: None,
            pending_sizes_started_at: None,
            pending_status_started_at: None,
            last_leaves_refresh: None,
            last_casks_refresh: None,
            last_sizes_refresh: None,
            focus_panel: FocusedPanel::Leaves,
            sizes_scroll_offset: 0,
            details_scroll_offset: 0,
            status_scroll_offset: 0,
            dependency_graph: None,
            pending_graph: false,
            system_status: None,
            pending_status: false,
            doctor: None,
            pending_doctor: false,
            last_status_check: None,
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
        self.pending_package_action.is_some()
            || self.pending_service_action.is_some()
            || self.pending_upgrade_all_outdated
            || self.pending_autoremove
            || self.pending_self_update
    }

    pub fn clear_pending_confirmations(&mut self) {
        self.pending_package_action = None;
        self.pending_service_action = None;
        self.pending_upgrade_all_outdated = false;
        self.pending_autoremove = false;
        self.pending_self_update = false;
    }

    pub fn on_tick(&mut self) {
        if self.pending_command
            || self.pending_leaves
            || self.pending_casks
            || self.pending_sizes
            || self.pending_status
        {
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
        self.icon_mode = match self.icon_mode {
            IconMode::Ascii => IconMode::Nerd,
            IconMode::Auto | IconMode::Nerd => IconMode::Ascii,
        };
        self.icons_ascii = match self.icon_mode {
            IconMode::Ascii => true,
            IconMode::Nerd => false,
            IconMode::Auto => detect_icon_ascii(),
        };
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

    /// Focus moves from several places (Tab, S-Tab, mouse clicks); they all
    /// report it the same way.
    pub fn set_focus_status(&mut self) {
        self.set_status(format!("Focus: {:?}", self.focus_panel));
    }

    pub fn status_tab_next(&mut self) {
        self.status_tab = match self.status_tab {
            StatusTab::Activity => StatusTab::Issues,
            StatusTab::Issues => StatusTab::Outdated,
            StatusTab::Outdated => StatusTab::Services,
            StatusTab::Services => StatusTab::History,
            StatusTab::History => StatusTab::Activity,
        };
        self.status_scroll_offset = 0; // Reset scroll when switching tabs
    }

    pub fn status_tab_prev(&mut self) {
        self.status_tab = match self.status_tab {
            StatusTab::Activity => StatusTab::History,
            StatusTab::Issues => StatusTab::Activity,
            StatusTab::Outdated => StatusTab::Issues,
            StatusTab::Services => StatusTab::Outdated,
            StatusTab::History => StatusTab::Services,
        };
        self.status_scroll_offset = 0;
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

        self.set_status(format!("View: {}", self.active_kind_label_plural()));
    }

    pub fn active_kind_label_singular(&self) -> &'static str {
        match self.active_package_kind {
            PackageKind::Formula => "formula",
            PackageKind::Cask => "cask",
        }
    }

    pub fn active_kind_label_plural(&self) -> &'static str {
        match self.active_package_kind {
            PackageKind::Formula => "formulae",
            PackageKind::Cask => "casks",
        }
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

    pub(super) fn max_status_scroll(&self) -> usize {
        self.system_status.as_ref().map_or(0, |h| {
            let count = match self.status_tab {
                StatusTab::Outdated => h.outdated_packages.len(),
                StatusTab::Issues => self.doctor.as_ref().map_or(0, |d| d.issues.len()),
                StatusTab::Services => self.filtered_service_count(),
                StatusTab::History => self.command_history.len(),
                StatusTab::Activity => self.activity_item_count(),
            };
            count.saturating_sub(2)
        })
    }

    fn activity_item_count(&self) -> usize {
        let Some(system_status) = self.system_status.as_ref() else {
            return 0;
        };

        let mut count = 0;
        if self.pending_command
            && self
                .last_command
                .is_some_and(CommandKind::is_activity_command)
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
            .is_some_and(|(_, _, at)| at.elapsed().as_secs() < 3)
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
        if self
            .autoremove_impact()
            .is_some_and(|impact| !impact.is_empty())
        {
            count += 1;
        }
        if self.last_status_check.is_some() {
            count += 1;
        }
        if self.last_leaves_refresh.is_some() {
            count += 1;
        }
        if self.last_casks_refresh.is_some() {
            count += 1;
        }
        if self.last_sizes_refresh.is_some() {
            count += 1;
        }
        if self.last_command.is_some() {
            count += 1;
        }
        if self.pending_leaves {
            count += 1;
        }
        if self.pending_casks {
            count += 1;
        }
        if self.pending_sizes {
            count += 1;
        }
        if self.pending_status {
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
