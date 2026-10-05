mod filters;
mod insights;
mod reducers;
mod requests;
mod state;
mod types;

pub use requests::MessageTx;
pub use types::{
    CommandHistoryEntry, Confirmation, FocusedPanel, InputMode, Job, Message, PackageAction,
    PackageKind, PendingPackageAction, PendingServiceAction, ServiceAction, ServiceKindFilter,
    SortMode, StatusTab, Toast, ToastLevel, ViewMode,
};

use std::collections::{HashSet, VecDeque};
use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use lru::LruCache;

use crate::brew::{
    CommandKind, DependencyGraph, Details, DetailsLoad, DoctorReport, SizeEntry, StatusSnapshot,
    fetch_casks, fetch_dependency_graph, fetch_details_basic, fetch_details_full, fetch_doctor,
    fetch_leaves, fetch_sizes, fetch_status, run_command,
};
use crate::theme::{Theme, ThemeMode, detect_system_theme};

/// Maximum number of package details to cache
const DETAILS_CACHE_CAPACITY: usize = 64;
const COMMAND_HISTORY_CAPACITY: usize = 24;
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
    pub active_package_kind: PackageKind,
    /// The formula list currently on display, derived from one of the two
    /// scopes below via [`App::sync_installed_list`].
    pub leaves: Vec<String>,
    /// `brew leaves` — formulae nothing else depends on.
    pub leaf_formulae: Vec<String>,
    /// Every installed formula, which is where dependency provenance gets
    /// interesting.
    pub all_formulae: Vec<String>,
    pub leaves_job: Job,
    pub leaves_only: bool,
    pub sort_mode: SortMode,
    pub casks: Vec<String>,
    pub casks_job: Job,
    pub filtered_leaves: Vec<usize>,
    pub filtered_casks: Vec<usize>,
    /// Every outdated formula, whichever scope is showing.
    pub outdated_leaves: HashSet<String>,
    pub outdated_casks: HashSet<String>,
    /// Formulae held back from `brew upgrade` via `brew pin`.
    pub pinned: HashSet<String>,
    pub package_results_selected: Option<usize>,
    pub last_result_details_pkg: Option<String>,
    pub selected_index: Option<usize>,
    pub selected_cask_index: Option<usize>,
    pub details_cache: LruCache<String, Details>,
    pub pending_details: Option<String>,
    pub package_results: Vec<String>,
    pub view_mode: ViewMode,
    pub sizes: Vec<SizeEntry>,
    pub sizes_job: Job,
    pub icons_ascii: bool,
    pub mouse_enabled: bool,
    pub command_job: Job,
    pub last_command: Option<CommandKind>,
    pub last_command_target: Option<String>,
    pub last_command_completed: Option<(CommandKind, String, Instant)>,
    pub last_command_output: Vec<String>,
    pub last_command_error: Option<String>,
    pub last_error: Option<String>,
    pub pending_confirmation: Option<Confirmation>,
    pub command_history: VecDeque<CommandHistoryEntry>,
    pub last_command_args: Vec<String>,
    pub focus_panel: FocusedPanel,
    pub sizes_scroll_offset: usize,
    pub details_scroll_offset: usize,
    pub status_scroll_offset: usize,
    /// Installed dependency graph, backing "why do I have this?" and the
    /// uninstall impact preview.
    pub dependency_graph: Option<DependencyGraph>,
    pub graph_job: Job,
    pub system_status: Option<StatusSnapshot>,
    pub status_job: Job,
    /// `brew doctor`, on its own clock. Fills in after the rest of the status
    /// panel rather than holding it up.
    pub doctor: Option<DoctorReport>,
    pub doctor_job: Job,
    pub status_tab: StatusTab,
    pub services_selected_index: Option<usize>,
    pub services_failed_only: bool,
    pub services_autostart_only: bool,
    pub services_kind_filter: ServiceKindFilter,
    pub leaves_outdated_only: bool,
    pub show_help_popup: bool,
    pub help_scroll_offset: usize,
    pub help_selected_command: usize,
    pub needs_redraw: bool,
    pub last_selection_change: Option<Instant>,
    /// Count of recent selection changes (for detecting rapid scrolling)
    pub recent_selection_count: u8,
}
