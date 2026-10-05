use super::*;
use crate::brew::{CommandResult, InstalledFormulae};
use crate::format::first_nonempty_line;

impl App {
    pub fn apply_message(&mut self, message: Message) {
        match message {
            Message::Leaves(result) => self.apply_leaves(result),
            Message::Casks(result) => self.apply_casks(result),
            Message::Details { pkg, load, result } => self.apply_details(pkg, load, result),
            Message::Sizes(result) => self.apply_sizes(result),
            Message::Command { kind, result } => self.apply_command(kind, result),
            Message::Status(result) => self.apply_status(result),
            Message::Graph(result) => self.apply_graph(result),
            Message::Doctor(result) => self.apply_doctor(result),
        }
        self.needs_redraw = true;
    }

    /// Records a failed request on the status line and the error row.
    fn report_failure(&mut self, err: anyhow::Error, status: &str) {
        self.last_error = Some(err.to_string());
        self.status = status.to_string();
    }

    fn report_success(&mut self, status: &str) {
        self.last_error = None;
        self.status = status.to_string();
    }

    fn apply_leaves(&mut self, result: anyhow::Result<InstalledFormulae>) {
        self.leaves_job.finish(result.is_ok());
        match result {
            Ok(mut installed) => {
                installed.leaves.sort();
                installed.all.sort();
                self.leaf_formulae = installed.leaves;
                self.all_formulae = installed.all;

                self.sync_installed_list();
                clamp_selection(&mut self.selected_index, self.leaves.len());
                self.report_success("Leaves updated");
            }
            Err(err) => self.report_failure(err, "Failed to refresh"),
        }
        self.last_refresh = Instant::now();
    }

    fn apply_details(&mut self, pkg: String, load: DetailsLoad, result: anyhow::Result<Details>) {
        match result {
            Ok(details) => {
                // LruCache doesn't have entry API, so we handle it manually
                if let Some(existing) = self.details_cache.get_mut(&pkg) {
                    merge_details(existing, details);
                } else {
                    self.details_cache.put(pkg, details);
                }
                self.report_success(match load {
                    DetailsLoad::Basic => "Details loaded",
                    DetailsLoad::Full => "Deps/uses loaded",
                });
            }
            Err(err) => self.report_failure(err, "Details failed"),
        }

        self.pending_details = None;
        self.last_refresh = Instant::now();
    }

    fn apply_sizes(&mut self, result: anyhow::Result<Vec<SizeEntry>>) {
        self.sizes_job.finish(result.is_ok());
        match result {
            Ok(sizes) => {
                self.sizes = sizes;
                let max_scroll = self.sizes.len().saturating_sub(1);
                self.sizes_scroll_offset = self.sizes_scroll_offset.min(max_scroll);
                self.report_success("Sizes updated");
                self.resync_sorted_list();
            }
            Err(err) => self.report_failure(err, "Sizes failed"),
        }
        self.last_refresh = Instant::now();
    }

    fn apply_casks(&mut self, result: anyhow::Result<Vec<String>>) {
        self.casks_job.finish(result.is_ok());
        match result {
            Ok(mut casks) => {
                casks.sort();
                self.casks = casks;
                self.update_filtered_casks();
                clamp_selection(&mut self.selected_cask_index, self.casks.len());
                self.reconcile_service_selection();
                self.report_success("Casks updated");
            }
            Err(err) => self.report_failure(err, "Casks refresh failed"),
        }
        self.last_refresh = Instant::now();
    }

    /// The graph loads quietly in the background — it supports other views
    /// rather than being something the user asked for, so it neither claims the
    /// status line on success nor reports a failure as a command error.
    fn apply_graph(&mut self, result: anyhow::Result<DependencyGraph>) {
        self.graph_job.finish(result.is_ok());
        self.dependency_graph = result.ok();
        self.resync_sorted_list();
    }

    fn apply_status(&mut self, result: anyhow::Result<StatusSnapshot>) {
        // A failed check still counts as a check for "Last check: Ns ago".
        self.status_job.finish(true);
        match result {
            Ok(snapshot) => {
                let outdated = |is_cask: bool| {
                    snapshot
                        .outdated
                        .iter()
                        .filter(|entry| entry.is_cask == is_cask)
                        .map(|entry| entry.name.clone())
                        .collect()
                };
                self.outdated_leaves = outdated(false);
                self.outdated_casks = outdated(true);
                self.pinned = snapshot.pinned.iter().cloned().collect();
                self.system_status = Some(snapshot);
                self.reconcile_service_selection();
                self.update_filtered_leaves();
                self.clamp_status_scroll();
                self.report_success("Status check complete");
            }
            Err(err) => self.report_failure(err, "Status check failed"),
        }
        self.last_refresh = Instant::now();
    }

    fn apply_doctor(&mut self, result: anyhow::Result<DoctorReport>) {
        self.doctor_job.finish(result.is_ok());
        // A failed doctor run is not worth taking over the status line the
        // way a failed status check is; the tab says so instead.
        self.doctor = result.ok();
        self.clamp_status_scroll();
    }

    fn clamp_status_scroll(&mut self) {
        self.status_scroll_offset = self.status_scroll_offset.min(self.max_status_scroll());
    }

    fn apply_command(&mut self, kind: CommandKind, result: anyhow::Result<CommandResult>) {
        let duration_secs = self.command_job.elapsed_secs();
        self.command_job.finish(result.is_ok());

        let (success, exit_code, failure) = match &result {
            Ok(output) => {
                // Prefer the stream that carries the outcome, falling back to
                // the other when it is empty.
                let (primary, fallback) = if output.success {
                    (&output.stdout, &output.stderr)
                } else {
                    (&output.stderr, &output.stdout)
                };
                let shown = if primary.trim().is_empty() {
                    fallback
                } else {
                    primary
                };
                self.last_command_output = shown.lines().take(8).map(str::to_string).collect();

                let failure = (!output.success).then(|| {
                    first_nonempty_line(&output.stderr)
                        .or_else(|| first_nonempty_line(&output.stdout))
                        .unwrap_or("Unknown error")
                        .to_string()
                });
                if !output.success && !output.stderr.trim().is_empty() {
                    self.last_command_error = Some(output.stderr.trim().to_string());
                }
                (output.success, output.exit_code, failure)
            }
            Err(err) => {
                self.last_command_error = Some(err.to_string());
                (false, None, Some(err.to_string()))
            }
        };

        self.status = if success {
            format!("{kind} complete")
        } else {
            format!("{kind} failed")
        };

        if let Some((level, message)) = command_toast(
            kind,
            self.last_command_target.as_deref(),
            failure.as_deref(),
            &self.last_command_output,
        ) {
            self.toast = Some(Toast {
                level,
                message,
                created_at: Instant::now(),
            });
        }

        if kind == CommandKind::Search
            && let Ok(output) = &result
        {
            self.apply_search_results(&output.stdout);
        }

        if success
            && kind.has_named_target()
            && let Some(target) = self.last_command_target.clone()
        {
            self.last_command_completed = Some((kind, target, Instant::now()));
        }

        self.push_command_history(kind, success, exit_code, duration_secs);
        self.last_command_args.clear();
        self.last_refresh = Instant::now();
    }

    fn apply_search_results(&mut self, stdout: &str) {
        self.package_results = stdout
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_string)
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

    /// `brew install foo`, or `cargo install ...` for the self-update: the
    /// command as the user would type it.
    pub fn last_command_line(&self) -> String {
        let binary = self.last_command.map_or("brew", CommandKind::binary);
        if self.last_command_args.is_empty() {
            binary.to_string()
        } else {
            format!("{binary} {}", self.last_command_args.join(" "))
        }
    }

    fn push_command_history(
        &mut self,
        kind: CommandKind,
        success: bool,
        exit_code: Option<i32>,
        duration_secs: u64,
    ) {
        self.command_history.push_front(CommandHistoryEntry {
            kind: kind.label().to_string(),
            command: self.last_command_line(),
            success,
            exit_code,
            finished_at: Instant::now(),
            duration_secs,
        });
        self.command_history.truncate(COMMAND_HISTORY_CAPACITY);
    }
}

/// Keeps a selection in range after its list is replaced, falling back to the
/// first entry.
fn clamp_selection(selected: &mut Option<usize>, len: usize) {
    if len == 0 {
        *selected = None;
    } else if selected.is_none_or(|idx| idx >= len) {
        *selected = Some(0);
    }
}

/// The toast for a finished command the user is waiting on. `failure` carries
/// the reason when it did not succeed.
fn command_toast(
    kind: CommandKind,
    target: Option<&str>,
    failure: Option<&str>,
    output: &[String],
) -> Option<(ToastLevel, String)> {
    let target = target.filter(|_| kind.has_named_target());

    let Some(reason) = failure else {
        let message = match (target, kind) {
            (Some(target), _) => format!("{} succeeded for {target}", kind.action_title()),
            (None, CommandKind::UpgradeAll) => {
                "Upgrade succeeded for outdated packages".to_string()
            }
            (None, CommandKind::SelfUpdate) => {
                "Brewery updated. Restart to use the new version".to_string()
            }
            (None, CommandKind::Update) => update_summary(output),
            _ => return None,
        };
        return Some((ToastLevel::Success, message));
    };

    let message = match (target, kind) {
        (Some(target), _) => format!("{} failed for {target}: {reason}", kind.action_title()),
        (None, CommandKind::UpgradeAll) => {
            format!("Upgrade failed for outdated packages: {reason}")
        }
        (None, CommandKind::SelfUpdate) => format!("Brewery update failed: {reason}"),
        (None, CommandKind::Update) => format!("brew update failed: {reason}"),
        _ => return None,
    };
    Some((ToastLevel::Error, message))
}

/// A basic load carries no deps/uses, so it must not wipe ones a full load
/// already cached.
fn merge_details(existing: &mut Details, incoming: Details) {
    existing.desc = incoming.desc;
    existing.homepage = incoming.homepage;
    existing.latest = incoming.latest;
    existing.installed = incoming.installed;
    existing.deps = incoming.deps.or(existing.deps.take());
    existing.uses = incoming.uses.or(existing.uses.take());
    existing.artifacts = incoming.artifacts.or(existing.artifacts.take());
}

/// `brew update` says "Already up-to-date." when nothing changed and prints
/// an `==> Updated Homebrew from ...` banner otherwise; either is a better
/// toast than a bare "complete".
fn update_summary(output: &[String]) -> String {
    let already_current = output
        .iter()
        .any(|line| line.trim_start().starts_with("Already up-to-date"));
    if already_current {
        return "Homebrew already up to date".to_string();
    }

    let banner = output
        .iter()
        .map(|line| line.trim())
        .find(|line| line.starts_with("==> Updated Homebrew"))
        .map(|line| line.trim_start_matches("==> ").to_string());

    banner.unwrap_or_else(|| "Homebrew updated; status re-checked".to_string())
}

#[cfg(test)]
mod tests {
    use super::{CommandKind, ToastLevel, command_toast, update_summary};

    #[test]
    fn names_the_target_of_a_targeted_command() {
        let toast = command_toast(CommandKind::Install, Some("wget"), None, &[]);
        assert_eq!(
            toast.map(|(_, message)| message).as_deref(),
            Some("Install succeeded for wget")
        );

        let toast = command_toast(CommandKind::Install, Some("wget"), Some("no bottle"), &[]);
        assert!(
            matches!(toast, Some((ToastLevel::Error, message)) if message == "Install failed for wget: no bottle")
        );
    }

    #[test]
    fn reports_untargeted_commands_without_a_target() {
        let toast = command_toast(CommandKind::UpgradeAll, None, Some("boom"), &[]);
        assert_eq!(
            toast.map(|(_, message)| message).as_deref(),
            Some("Upgrade failed for outdated packages: boom")
        );
    }

    #[test]
    fn stays_quiet_for_background_commands() {
        assert!(command_toast(CommandKind::Search, None, None, &[]).is_none());
        assert!(command_toast(CommandKind::Cleanup, None, Some("boom"), &[]).is_none());
    }

    #[test]
    fn summarizes_a_no_op_brew_update() {
        let output = ["Already up-to-date.".to_string()];
        assert_eq!(update_summary(&output), "Homebrew already up to date");
    }

    #[test]
    fn summarizes_a_brew_update_that_changed_something() {
        let output = [
            "==> Updating Homebrew...".to_string(),
            "==> Updated Homebrew from abc123 to def456.".to_string(),
            "==> New Formulae".to_string(),
        ];
        assert_eq!(
            update_summary(&output),
            "Updated Homebrew from abc123 to def456."
        );
    }

    #[test]
    fn falls_back_when_brew_update_output_is_unrecognized() {
        assert_eq!(update_summary(&[]), "Homebrew updated; status re-checked");
    }
}
