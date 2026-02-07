use super::*;

impl App {
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

    pub fn apply_status_message(&mut self, message: StatusMessage) {
        match message.result {
            Ok(status_snapshot) => {
                self.system_status = Some(status_snapshot);
                self.update_filtered_leaves();
                let max_scroll = self.max_status_scroll();
                self.status_scroll_offset = self.status_scroll_offset.min(max_scroll);
                self.last_error = None;
                self.status = "Status check complete".to_string();
                self.last_status_check = Some(Instant::now());
            }
            Err(err) => {
                self.last_error = Some(err.to_string());
                self.status = "Status check failed".to_string();
            }
        }

        self.pending_status = false;
        self.last_refresh = Instant::now();
        self.needs_redraw = true;
    }

    pub fn apply_command_message(&mut self, message: CommandMessage) {
        let mut toast: Option<(ToastLevel, String)> = None;

        match message.result {
            Ok(result) => {
                let lines: Vec<String> = if result.success {
                    if result.stdout.trim().is_empty() {
                        result.stderr.lines().map(str::to_string).collect()
                    } else {
                        result.stdout.lines().map(str::to_string).collect()
                    }
                } else if result.stderr.trim().is_empty() {
                    result.stdout.lines().map(str::to_string).collect()
                } else {
                    result.stderr.lines().map(str::to_string).collect()
                };
                self.last_command_output = lines.into_iter().take(8).collect();
                if result.success {
                    self.status = format!("{label} complete", label = message.label);
                    if let Some(pkg) =
                        package_action_target(&message.label, self.last_command_target.as_deref())
                    {
                        toast = Some((
                            ToastLevel::Success,
                            format!("{} succeeded for {pkg}", action_title(&message.label)),
                        ));
                    } else if message.label == "upgrade-all" {
                        toast = Some((
                            ToastLevel::Success,
                            "Upgrade succeeded for outdated packages".to_string(),
                        ));
                    } else if message.label == "self-update" {
                        toast = Some((
                            ToastLevel::Success,
                            "Brewery updated. Restart to use the new version".to_string(),
                        ));
                    }
                } else {
                    self.status = format!("{label} failed", label = message.label);
                    if !result.stderr.trim().is_empty() {
                        self.last_command_error = Some(result.stderr.trim().to_string());
                    }
                    if let Some(pkg) =
                        package_action_target(&message.label, self.last_command_target.as_deref())
                    {
                        let reason = first_nonempty_line(&result.stderr)
                            .or_else(|| first_nonempty_line(&result.stdout))
                            .unwrap_or("Unknown error");
                        toast = Some((
                            ToastLevel::Error,
                            format!(
                                "{} failed for {pkg}: {reason}",
                                action_title(&message.label)
                            ),
                        ));
                    } else if message.label == "upgrade-all" {
                        let reason = first_nonempty_line(&result.stderr)
                            .or_else(|| first_nonempty_line(&result.stdout))
                            .unwrap_or("Unknown error");
                        toast = Some((
                            ToastLevel::Error,
                            format!("Upgrade failed for outdated packages: {reason}"),
                        ));
                    } else if message.label == "self-update" {
                        let reason = first_nonempty_line(&result.stderr)
                            .or_else(|| first_nonempty_line(&result.stdout))
                            .unwrap_or("Unknown error");
                        toast = Some((
                            ToastLevel::Error,
                            format!("Brewery update failed: {reason}"),
                        ));
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

                if result.success
                    && matches!(message.label.as_str(), "install" | "uninstall" | "upgrade")
                {
                    if let Some(pkg) = self.last_command_target.clone() {
                        self.last_command_completed =
                            Some((message.label.clone(), pkg, Instant::now()));
                    }
                }
            }
            Err(err) => {
                self.last_command_error = Some(err.to_string());
                self.status = format!("{label} failed", label = message.label);
                if let Some(pkg) =
                    package_action_target(&message.label, self.last_command_target.as_deref())
                {
                    toast = Some((
                        ToastLevel::Error,
                        format!("{} failed for {pkg}: {}", action_title(&message.label), err),
                    ));
                } else if message.label == "upgrade-all" {
                    toast = Some((
                        ToastLevel::Error,
                        format!("Upgrade failed for outdated packages: {err}"),
                    ));
                } else if message.label == "self-update" {
                    toast = Some((ToastLevel::Error, format!("Brewery update failed: {err}")));
                }
            }
        }

        if let Some((level, message)) = toast {
            self.show_toast(level, message);
        }

        self.pending_command = false;
        self.command_started_at = None;
        self.last_refresh = Instant::now();
        self.needs_redraw = true;
    }

    fn show_toast(&mut self, level: ToastLevel, message: String) {
        self.toast = Some(Toast {
            level,
            message,
            created_at: Instant::now(),
        });
    }
}

fn merge_details(existing: &mut Details, incoming: &Details) {
    existing.desc = incoming.desc.clone();
    existing.homepage = incoming.homepage.clone();
    existing.latest = incoming.latest.clone();
    existing.installed = incoming.installed.clone();
    if incoming.deps.is_some() {
        existing.deps = incoming.deps.clone();
    }
    if incoming.uses.is_some() {
        existing.uses = incoming.uses.clone();
    }
}

fn package_action_target<'a>(label: &str, target: Option<&'a str>) -> Option<&'a str> {
    if matches!(label, "install" | "uninstall" | "upgrade") {
        return target;
    }
    None
}

fn action_title(label: &str) -> &'static str {
    match label {
        "install" => "Install",
        "uninstall" => "Uninstall",
        "upgrade" => "Upgrade",
        _ => "Action",
    }
}

fn first_nonempty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}
