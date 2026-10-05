use std::cmp::Ordering;
use std::collections::HashMap;

use super::*;
use crate::brew::ServiceEntry;

impl App {
    pub fn is_cask_mode(&self) -> bool {
        self.active_package_kind == PackageKind::Cask
    }

    pub fn is_outdated_leaf(&self, pkg: &str) -> bool {
        self.outdated_leaves.contains(pkg)
    }

    pub fn is_outdated_cask(&self, cask: &str) -> bool {
        self.outdated_casks.contains(cask)
    }

    pub fn is_pinned(&self, pkg: &str) -> bool {
        self.pinned.contains(pkg)
    }

    /// Rebuilds the displayed formula list from the active scope, keeping the
    /// selection on the same package where it still exists — the two scopes
    /// index differently, so the raw index is meaningless across a switch.
    pub fn sync_installed_list(&mut self) {
        let previous = self.selected_leaf().map(str::to_string);

        self.leaves = if self.leaves_only {
            self.leaf_formulae.clone()
        } else {
            self.all_formulae.clone()
        };
        self.sort_leaves();

        // The old index means something different in the new scope, and would
        // otherwise survive reconciliation while pointing at an unrelated
        // package. Drop it, then restore by name.
        self.selected_index = None;
        self.update_filtered_leaves();

        if let Some(previous) = previous
            && let Some(index) = self.leaves.iter().position(|name| *name == previous)
            && self.filtered_leaves.contains(&index)
        {
            self.selected_index = Some(index);
        }
    }

    /// Orders `leaves` by the active sort. Name order is what the fetch
    /// already delivers; the other modes fall back to it for ties and for
    /// entries the sizes or receipts know nothing about, so the list is
    /// stable while that data is still on its way.
    fn sort_leaves(&mut self) {
        match self.sort_mode {
            SortMode::Name => {}
            SortMode::Size => {
                let sizes: HashMap<&str, u64> = self
                    .sizes
                    .iter()
                    .map(|entry| (entry.name.as_str(), entry.size_kb))
                    .collect();
                self.leaves.sort_by(|left, right| {
                    let left_size = sizes.get(left.as_str());
                    let right_size = sizes.get(right.as_str());
                    descending_then_name(left_size, right_size, left, right)
                });
            }
            SortMode::Recent => {
                let graph = self.dependency_graph.as_ref();
                self.leaves.sort_by(|left, right| {
                    let left_at = graph.and_then(|graph| graph.installed_at(left));
                    let right_at = graph.and_then(|graph| graph.installed_at(right));
                    descending_then_name(left_at.as_ref(), right_at.as_ref(), left, right)
                });
            }
        }
    }

    /// Re-sorts the list when the data a non-name sort depends on arrives.
    pub fn resync_sorted_list(&mut self) {
        if self.sort_mode != SortMode::Name {
            self.sync_installed_list();
        }
    }

    pub fn toggle_leaves_scope(&mut self) {
        if self.is_cask_mode() {
            self.set_status("List scope only applies to formulae");
            return;
        }

        self.leaves_only = !self.leaves_only;
        self.sync_installed_list();

        self.set_status(if self.leaves_only {
            format!("Showing {} leaves", self.leaves.len())
        } else {
            format!("Showing all {} formulae", self.leaves.len())
        });
    }

    pub fn toggle_outdated_filter(&mut self) {
        if self.is_cask_mode() {
            self.set_status("Outdated filter only applies to formulae");
            return;
        }

        self.leaves_outdated_only = !self.leaves_outdated_only;
        self.update_filtered_leaves();
        self.set_status(if self.leaves_outdated_only {
            "Filter: outdated only"
        } else {
            "Filter: all leaves"
        });
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

    pub fn selected_service(&self) -> Option<&str> {
        self.selected_service_entry()
            .map(|service| service.name.as_str())
    }

    pub fn selected_service_entry(&self) -> Option<&ServiceEntry> {
        let selected = self.services_selected_index?;
        let snapshot = self.system_status.as_ref()?;
        snapshot.services.get(selected)
    }

    pub fn filtered_service_indices(&self) -> Vec<usize> {
        let Some(snapshot) = self.system_status.as_ref() else {
            return Vec::new();
        };

        snapshot
            .services
            .iter()
            .enumerate()
            .filter(|(_, service)| self.service_matches_filters(service))
            .map(|(index, _)| index)
            .collect()
    }

    pub fn select_next_service(&mut self) {
        self.step_service_selection(StepDirection::Next);
    }

    pub fn select_prev_service(&mut self) {
        self.step_service_selection(StepDirection::Prev);
    }

    pub fn reconcile_service_selection(&mut self) {
        let filtered = self.filtered_service_indices();
        if filtered.is_empty() {
            self.services_selected_index = None;
            self.status_scroll_offset = 0;
            return;
        }

        if let Some(selected) = self.services_selected_index
            && let Some(position) = filtered.iter().position(|candidate| *candidate == selected)
        {
            self.status_scroll_offset = position;
            return;
        }

        self.services_selected_index = filtered.first().copied();
        self.status_scroll_offset = 0;
    }

    pub fn toggle_services_failed_filter(&mut self) {
        self.services_failed_only = !self.services_failed_only;
        self.reconcile_service_selection();
        self.set_services_filter_status();
    }

    pub fn toggle_services_autostart_filter(&mut self) {
        self.services_autostart_only = !self.services_autostart_only;
        self.reconcile_service_selection();
        self.set_services_filter_status();
    }

    pub fn cycle_services_kind_filter(&mut self) {
        self.services_kind_filter = self.services_kind_filter.next();
        self.reconcile_service_selection();
        self.set_services_filter_status();
    }

    fn set_services_filter_status(&mut self) {
        self.set_status(format!(
            "Services filter: {}",
            self.services_filter_summary()
        ));
    }

    pub fn services_filter_summary(&self) -> String {
        let failed = if self.services_failed_only {
            "failed"
        } else {
            "all"
        };
        let autostart = if self.services_autostart_only {
            "auto-start"
        } else {
            "any start mode"
        };
        format!(
            "{failed}, {autostart}, {}",
            self.services_kind_filter.label()
        )
    }

    pub fn is_service_cask_backed(&self, service_name: &str) -> bool {
        self.casks
            .binary_search_by(|candidate| candidate.as_str().cmp(service_name))
            .is_ok()
    }

    pub fn service_backend_label(&self, service_name: &str) -> &'static str {
        if self.is_service_cask_backed(service_name) {
            "cask"
        } else {
            "formula"
        }
    }

    pub fn select_next_result(&mut self) {
        self.step_result_selection(StepDirection::Next);
    }

    pub fn select_prev_result(&mut self) {
        self.step_result_selection(StepDirection::Prev);
    }

    fn step_result_selection(&mut self, direction: StepDirection) {
        let len = self.package_results.len();
        self.package_results_selected =
            (len > 0).then(|| step_position(self.package_results_selected, len, direction));
        if len > 0 {
            self.last_result_details_pkg = None;
        }
    }

    pub fn clear_package_results(&mut self) {
        self.package_results.clear();
        self.package_results_selected = None;
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
        self.step_installed_selection(StepDirection::Next);
    }

    pub fn select_prev(&mut self) {
        self.step_installed_selection(StepDirection::Prev);
    }

    fn step_installed_selection(&mut self, direction: StepDirection) {
        let (filtered, selected) = if self.is_cask_mode() {
            (&self.filtered_casks, &mut self.selected_cask_index)
        } else {
            (&self.filtered_leaves, &mut self.selected_index)
        };
        *selected = step_filtered(filtered, *selected, direction);
    }

    pub fn update_filtered_casks(&mut self) {
        self.filtered_casks = build_filtered_indices(&self.casks, &self.leaves_query, |_| true);
        reconcile_selection(&self.filtered_casks, &mut self.selected_cask_index);
    }

    fn step_service_selection(&mut self, direction: StepDirection) {
        let filtered = self.filtered_service_indices();
        self.services_selected_index =
            step_filtered(&filtered, self.services_selected_index, direction);
        // The services tab scrolls with its selection, one line per service.
        self.status_scroll_offset = self
            .services_selected_index
            .and_then(|selected| filtered.iter().position(|idx| *idx == selected))
            .unwrap_or(0);
    }

    fn service_matches_filters(&self, service: &ServiceEntry) -> bool {
        if self.services_failed_only && !service.has_failed() {
            return false;
        }

        if self.services_autostart_only && !service.auto_start_enabled() {
            return false;
        }

        match self.services_kind_filter {
            ServiceKindFilter::All => true,
            ServiceKindFilter::Formula => !self.is_service_cask_backed(&service.name),
            ServiceKindFilter::Cask => self.is_service_cask_backed(&service.name),
        }
    }
}

#[derive(Clone, Copy)]
enum StepDirection {
    Next,
    Prev,
}

/// Larger keys first; missing keys after every present one; names break ties.
fn descending_then_name<K: Ord>(
    left_key: Option<&K>,
    right_key: Option<&K>,
    left_name: &str,
    right_name: &str,
) -> Ordering {
    match (left_key, right_key) {
        (Some(left), Some(right)) => right.cmp(left).then_with(|| left_name.cmp(right_name)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left_name.cmp(right_name),
    }
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

/// Moves a selection of absolute indices one step through the filtered
/// subset, landing on its first entry when nothing visible was selected.
fn step_filtered(
    filtered: &[usize],
    selected: Option<usize>,
    direction: StepDirection,
) -> Option<usize> {
    let current_pos =
        selected.and_then(|idx| filtered.iter().position(|candidate| *candidate == idx));
    let next_pos = step_position(current_pos, filtered.len(), direction);
    filtered.get(next_pos).copied()
}

fn step_position(current: Option<usize>, len: usize, direction: StepDirection) -> usize {
    match direction {
        StepDirection::Next => current.map_or(0, |idx| (idx + 1).min(len.saturating_sub(1))),
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
    use super::{App, PackageKind, SortMode, contains_ascii_case_insensitive, leaf_matches_query};
    use crate::brew::{DependencyGraph, FormulaReceipt, Receipts, SizeEntry};

    /// Two leaves, plus the dependencies they pulled in.
    fn app_with_both_scopes() -> App {
        let mut app = App::new();
        app.leaf_formulae = vec!["imagemagick".to_string(), "wget".to_string()];
        app.all_formulae = vec![
            "imagemagick".to_string(),
            "libpng".to_string(),
            "wget".to_string(),
            "zlib".to_string(),
        ];
        app.sync_installed_list();
        app
    }

    #[test]
    fn shows_only_leaves_by_default() {
        let app = app_with_both_scopes();
        assert!(app.leaves_only);
        assert_eq!(app.leaves, ["imagemagick", "wget"]);
    }

    #[test]
    fn widening_the_scope_reveals_dependencies() {
        let mut app = app_with_both_scopes();
        app.toggle_leaves_scope();

        assert!(!app.leaves_only);
        assert_eq!(app.leaves, ["imagemagick", "libpng", "wget", "zlib"]);
        assert!(app.status.contains('4'), "status should report the count");
    }

    /// The two scopes index differently, so a preserved index would silently
    /// move the selection to an unrelated package.
    #[test]
    fn keeps_the_selection_on_the_same_package_when_widening() {
        let mut app = app_with_both_scopes();
        app.selected_index = Some(1); // wget
        assert_eq!(app.selected_leaf(), Some("wget"));

        app.toggle_leaves_scope();

        assert_eq!(
            app.selected_leaf(),
            Some("wget"),
            "wget moved to index 2 in the wider scope"
        );
    }

    #[test]
    fn keeps_the_selection_when_narrowing_back() {
        let mut app = app_with_both_scopes();
        app.toggle_leaves_scope();
        app.selected_index = Some(2); // wget, in the wider scope
        app.toggle_leaves_scope();

        assert!(app.leaves_only);
        assert_eq!(app.selected_leaf(), Some("wget"));
    }

    #[test]
    fn falls_back_when_the_selection_leaves_the_scope() {
        let mut app = app_with_both_scopes();
        app.toggle_leaves_scope();
        app.selected_index = Some(1); // libpng, which is not a leaf
        assert_eq!(app.selected_leaf(), Some("libpng"));

        app.toggle_leaves_scope();

        assert_eq!(
            app.selected_leaf(),
            Some("imagemagick"),
            "a selection that no longer exists should land on the first entry"
        );
    }

    #[test]
    fn refuses_to_change_scope_in_cask_mode() {
        let mut app = app_with_both_scopes();
        app.active_package_kind = PackageKind::Cask;
        app.toggle_leaves_scope();

        assert!(app.leaves_only, "cask mode has no leaves/all distinction");
        assert!(app.status.contains("formulae"));
    }

    #[test]
    fn respects_the_active_search_filter_across_a_scope_change() {
        let mut app = app_with_both_scopes();
        app.leaves_query = "png".to_string();
        app.toggle_leaves_scope();

        let matches: Vec<&String> = app
            .filtered_leaves
            .iter()
            .filter_map(|idx| app.leaves.get(*idx))
            .collect();
        assert_eq!(matches, ["libpng"]);
    }

    fn size(name: &str, size_kb: u64) -> SizeEntry {
        SizeEntry {
            name: name.to_string(),
            size_kb,
        }
    }

    #[test]
    fn sorts_by_size_with_unsized_entries_trailing_by_name() {
        let mut app = app_with_both_scopes();
        app.toggle_leaves_scope();
        app.sizes = vec![size("libpng", 300), size("imagemagick", 900)];

        app.cycle_sort_mode();

        assert_eq!(app.sort_mode, SortMode::Size);
        assert_eq!(app.leaves, ["imagemagick", "libpng", "wget", "zlib"]);
    }

    #[test]
    fn sorts_by_install_date_newest_first() {
        let receipt = |at: u64| FormulaReceipt {
            installed_at: Some(at),
            ..FormulaReceipt::default()
        };
        let receipts = Receipts::from([
            ("imagemagick".to_string(), receipt(100)),
            ("wget".to_string(), receipt(300)),
            ("zlib".to_string(), receipt(200)),
        ]);

        let mut app = app_with_both_scopes();
        app.toggle_leaves_scope();
        app.dependency_graph = Some(DependencyGraph::new(receipts));
        app.sort_mode = SortMode::Size;

        app.cycle_sort_mode();

        assert_eq!(app.sort_mode, SortMode::Recent);
        assert_eq!(
            app.leaves,
            ["wget", "zlib", "imagemagick", "libpng"],
            "libpng has no receipt, so it trails"
        );
    }

    #[test]
    fn keeps_the_selection_across_a_sort_change() {
        let mut app = app_with_both_scopes();
        app.sizes = vec![size("wget", 900), size("imagemagick", 100)];
        app.selected_index = Some(1); // wget
        app.cycle_sort_mode();

        assert_eq!(app.leaves, ["wget", "imagemagick"]);
        assert_eq!(app.selected_leaf(), Some("wget"));
    }

    #[test]
    fn re_sorts_when_sizes_arrive_after_the_sort_was_chosen() {
        let mut app = app_with_both_scopes();
        app.cycle_sort_mode();
        assert_eq!(app.leaves, ["imagemagick", "wget"], "no sizes yet");

        app.sizes = vec![size("wget", 900), size("imagemagick", 100)];
        app.resync_sorted_list();
        assert_eq!(app.leaves, ["wget", "imagemagick"]);
    }

    #[test]
    fn cycles_back_to_name_order() {
        let mut app = app_with_both_scopes();
        app.sizes = vec![size("wget", 900), size("imagemagick", 100)];
        app.cycle_sort_mode();
        app.cycle_sort_mode();
        app.cycle_sort_mode();

        assert_eq!(app.sort_mode, SortMode::Name);
        assert_eq!(app.leaves, ["imagemagick", "wget"]);
    }

    #[test]
    fn refuses_to_sort_in_cask_mode() {
        let mut app = app_with_both_scopes();
        app.active_package_kind = PackageKind::Cask;
        app.cycle_sort_mode();

        assert_eq!(app.sort_mode, SortMode::Name);
        assert!(app.status.contains("formulae"));
    }

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
