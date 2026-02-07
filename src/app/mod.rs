mod filters;
mod reducers;
mod requests;
mod state;
mod types;

pub use types::{
    FocusedPanel, IconMode, InputMode, PackageAction, PendingPackageAction, StatusTab, Toast,
    ToastLevel, ViewMode,
};

use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use lru::LruCache;

use crate::brew::{
    CommandMessage, Details, DetailsLoad, DetailsMessage, LeavesMessage, SizeEntry, SizesMessage,
    StatusMessage, StatusSnapshot, fetch_details_basic, fetch_details_full, fetch_leaves,
    fetch_sizes, fetch_status, run_brew_command,
};
use crate::theme::{Theme, ThemeMode, detect_system_theme};

/// Maximum number of package details to cache
const DETAILS_CACHE_CAPACITY: usize = 64;
const TOAST_DURATION: Duration = Duration::from_secs(5);

pub struct App {
    pub started_at: Instant,
    pub last_refresh: Instant,
    pub status: String,
    pub toast: Option<Toast>,
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
    pub pending_upgrade_all_outdated: bool,
    pub pending_leaves: bool,
    pub last_leaves_refresh: Option<Instant>,
    pub last_sizes_refresh: Option<Instant>,
    pub focus_panel: FocusedPanel,
    pub sizes_scroll_offset: usize,
    pub details_scroll_offset: usize,
    pub status_scroll_offset: usize,
    pub system_status: Option<StatusSnapshot>,
    pub pending_status: bool,
    pub last_status_check: Option<Instant>,
    pub status_tab: StatusTab,
    pub leaves_outdated_only: bool,
    pub show_help_popup: bool,
    pub help_scroll_offset: usize,
    pub needs_redraw: bool,
    pub last_selection_change: Option<Instant>,
    /// Count of recent selection changes (for detecting rapid scrolling)
    pub recent_selection_count: u8,
}
