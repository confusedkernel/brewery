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
        step_linear_selection(
            self.package_results.len(),
            &mut self.package_results_selected,
            StepDirection::Next,
        );
        self.last_result_details_pkg = None;
    }

    pub fn select_prev_result(&mut self) {
        if self.package_results.is_empty() {
            self.package_results_selected = None;
            return;
        }
        step_linear_selection(
            self.package_results.len(),
            &mut self.package_results_selected,
            StepDirection::Prev,
        );
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

        self.filtered_leaves = build_filtered_indices(&self.leaves, &self.leaves_query, |item| {
            !self.leaves_outdated_only || self.is_outdated_leaf(item)
        });

        reconcile_selection(&self.filtered_leaves, &mut self.selected_index);
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
            step_filtered_selection(
                &self.filtered_casks,
                &mut self.selected_cask_index,
                StepDirection::Next,
            );
            return;
        }

        step_filtered_selection(
            &self.filtered_leaves,
            &mut self.selected_index,
            StepDirection::Next,
        );
    }

    pub fn select_prev(&mut self) {
        if self.is_cask_mode() {
            step_filtered_selection(
                &self.filtered_casks,
                &mut self.selected_cask_index,
                StepDirection::Prev,
            );
            return;
        }

        step_filtered_selection(
            &self.filtered_leaves,
            &mut self.selected_index,
            StepDirection::Prev,
        );
    }

    pub fn update_filtered_casks(&mut self) {
        self.filtered_casks = build_filtered_indices(&self.casks, &self.leaves_query, |_| true);
        reconcile_selection(&self.filtered_casks, &mut self.selected_cask_index);
    }
}

#[derive(Clone, Copy)]
enum StepDirection {
    Next,
    Prev,
}

fn build_filtered_indices<F>(items: &[String], query: &str, mut include: F) -> Vec<usize>
where
    F: FnMut(&str) -> bool,
{
    let query = query.trim();
    let has_query = !query.is_empty();
    let query_is_ascii = query.is_ascii();
    let query_lower = (!query_is_ascii && has_query).then(|| query.to_lowercase());

    items
        .iter()
        .enumerate()
        .filter(|(_, item)| include(item.as_str()))
        .filter(|(_, item)| {
            !has_query || leaf_matches_query(item, query, query_lower.as_deref(), query_is_ascii)
        })
        .map(|(idx, _)| idx)
        .collect()
}

fn reconcile_selection(filtered: &[usize], selected: &mut Option<usize>) {
    if filtered.is_empty() {
        *selected = None;
        return;
    }

    if selected.is_some_and(|idx| filtered.contains(&idx)) {
        return;
    }

    *selected = filtered.first().copied();
}

fn step_filtered_selection(
    filtered: &[usize],
    selected: &mut Option<usize>,
    direction: StepDirection,
) {
    if filtered.is_empty() {
        *selected = None;
        return;
    }

    let current_pos =
        selected.and_then(|idx| filtered.iter().position(|candidate| *candidate == idx));
    let next_pos = step_position(current_pos, filtered.len(), direction);
    *selected = filtered.get(next_pos).copied();
}

fn step_linear_selection(len: usize, selected: &mut Option<usize>, direction: StepDirection) {
    let next = step_position(*selected, len, direction);
    *selected = Some(next);
}

fn step_position(current: Option<usize>, len: usize, direction: StepDirection) -> usize {
    match direction {
        StepDirection::Next => current.map_or(0, |idx| (idx + 1).min(len - 1)),
        StepDirection::Prev => current.map_or(0, |idx| idx.saturating_sub(1)),
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
