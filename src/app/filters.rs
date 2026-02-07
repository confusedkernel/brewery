use super::*;

impl App {
    pub fn filtered_leaves(&self) -> Vec<(usize, &str)> {
        self.filtered_leaves
            .iter()
            .filter_map(|idx| self.leaves.get(*idx).map(|item| (*idx, item.as_str())))
            .collect()
    }

    pub fn is_outdated_leaf(&self, pkg: &str) -> bool {
        self.system_status
            .as_ref()
            .map(|status| status.outdated_packages.iter().any(|name| name == pkg))
            .unwrap_or(false)
    }

    pub fn toggle_outdated_filter(&mut self) {
        self.leaves_outdated_only = !self.leaves_outdated_only;
        self.update_filtered_leaves();
        if self.leaves_outdated_only {
            self.status = "Filter: outdated only".to_string();
        } else {
            self.status = "Filter: all leaves".to_string();
        }
        self.last_refresh = Instant::now();
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

        let needle = self.leaves_query.to_lowercase();
        self.filtered_leaves = self
            .leaves
            .iter()
            .enumerate()
            .filter(|(_, item)| !self.leaves_outdated_only || self.is_outdated_leaf(item))
            .filter(|(_, item)| {
                self.leaves_query.is_empty() || item.to_lowercase().contains(&needle)
            })
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
}
