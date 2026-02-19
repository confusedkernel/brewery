use super::*;

impl App {
    pub fn is_cask_mode(&self) -> bool {
        self.active_package_kind == PackageKind::Cask
    }

    pub fn is_outdated_leaf(&self, pkg: &str) -> bool {
        self.outdated_leaves.contains(pkg)
    }

    pub fn toggle_outdated_filter(&mut self) {
        if self.is_cask_mode() {
            self.status = "Outdated filter only applies to formulae".to_string();
            self.last_refresh = Instant::now();
            return;
        }

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
            self.selected_installed_package()
        }
    }

    pub fn selected_installed_package(&self) -> Option<&str> {
        if self.is_cask_mode() {
            self.selected_cask()
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

    pub fn update_all_installed_filters(&mut self) {
        self.update_filtered_leaves();
        self.update_filtered_casks();
    }

    pub fn update_active_installed_filter(&mut self) {
        if self.is_cask_mode() {
            self.update_filtered_casks();
        } else {
            self.update_filtered_leaves();
        }
    }

    pub fn update_filtered_leaves(&mut self) {
        self.filtered_leaves_dirty = false;

        let query = self.leaves_query.trim();
        let has_query = !query.is_empty();
        let query_is_ascii = query.is_ascii();
        let query_lower = (!query_is_ascii && has_query).then(|| query.to_lowercase());
        self.filtered_leaves = self
            .leaves
            .iter()
            .enumerate()
            .filter(|(_, item)| !self.leaves_outdated_only || self.is_outdated_leaf(item))
            .filter(|(_, item)| {
                !has_query
                    || leaf_matches_query(item, query, query_lower.as_deref(), query_is_ascii)
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_leaves.is_empty() {
            self.selected_index = None;
        } else if self
            .selected_index
            .is_some_and(|selected| self.filtered_leaves.contains(&selected))
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

    pub fn selected_cask(&self) -> Option<&str> {
        let selected = self.selected_cask_index?;
        self.casks.get(selected).map(String::as_str)
    }

    pub fn select_next(&mut self) {
        if self.is_cask_mode() {
            self.select_next_cask();
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
        if self.is_cask_mode() {
            self.select_prev_cask();
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

    pub fn update_filtered_casks(&mut self) {
        let query = self.leaves_query.trim();
        let has_query = !query.is_empty();
        let query_is_ascii = query.is_ascii();
        let query_lower = (!query_is_ascii && has_query).then(|| query.to_lowercase());

        self.filtered_casks = self
            .casks
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                !has_query
                    || leaf_matches_query(item, query, query_lower.as_deref(), query_is_ascii)
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.filtered_casks.is_empty() {
            self.selected_cask_index = None;
        } else if self
            .selected_cask_index
            .is_some_and(|selected| self.filtered_casks.contains(&selected))
        {
            // keep current selection
        } else {
            self.selected_cask_index = self.filtered_casks.first().copied();
        }
    }

    fn select_next_cask(&mut self) {
        if self.filtered_casks.is_empty() {
            self.selected_cask_index = None;
            return;
        }

        let current_pos = self
            .selected_cask_index
            .and_then(|selected| self.filtered_casks.iter().position(|idx| *idx == selected));
        let next_pos = match current_pos {
            Some(pos) => (pos + 1).min(self.filtered_casks.len() - 1),
            None => 0,
        };
        self.selected_cask_index = self.filtered_casks.get(next_pos).copied();
    }

    fn select_prev_cask(&mut self) {
        if self.filtered_casks.is_empty() {
            self.selected_cask_index = None;
            return;
        }

        let current_pos = self
            .selected_cask_index
            .and_then(|selected| self.filtered_casks.iter().position(|idx| *idx == selected));
        let prev_pos = match current_pos {
            Some(pos) => pos.saturating_sub(1),
            None => 0,
        };
        self.selected_cask_index = self.filtered_casks.get(prev_pos).copied();
    }
}

fn leaf_matches_query(
    item: &str,
    query: &str,
    query_lower: Option<&str>,
    query_is_ascii: bool,
) -> bool {
    if query_is_ascii && item.is_ascii() {
        return contains_ascii_case_insensitive(item.as_bytes(), query.as_bytes());
    }

    let Some(query_lower) = query_lower else {
        return true;
    };
    item.to_lowercase().contains(query_lower)
}

fn contains_ascii_case_insensitive(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return true;
    }
    if needle.len() > haystack.len() {
        return false;
    }

    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

#[cfg(test)]
mod tests {
    use super::{contains_ascii_case_insensitive, leaf_matches_query};

    #[test]
    fn matches_ascii_query_case_insensitively() {
        assert!(contains_ascii_case_insensitive(b"OpenSSL", b"ssl"));
        assert!(leaf_matches_query("OpenSSL", "ssl", None, true));
    }

    #[test]
    fn rejects_ascii_query_when_not_present() {
        assert!(!contains_ascii_case_insensitive(b"sqlite", b"brew"));
        assert!(!leaf_matches_query("sqlite", "brew", None, true));
    }

    #[test]
    fn matches_non_ascii_query_using_lowercased_forms() {
        assert!(leaf_matches_query(
            "CAFETIERE",
            "cafetiere",
            Some("cafetiere"),
            false
        ));
        assert!(leaf_matches_query("naive", "NAIVE", Some("naive"), false));
    }
}
