use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::brew::{
    fetch_details_basic, fetch_details_full, fetch_leaves, Details, DetailsLoad, DetailsMessage,
};
use crate::theme::{detect_system_theme, Theme, ThemeMode};

pub struct App {
    pub started_at: Instant,
    pub last_refresh: Instant,
    pub status: String,
    pub theme_mode: ThemeMode,
    pub theme: Theme,
    pub input_mode: InputMode,
    pub search_query: String,
    pub leaves: Vec<String>,
    pub selected_index: Option<usize>,
    pub details_cache: HashMap<String, Details>,
    pub pending_details: Option<String>,
    pub last_error: Option<String>,
    pub last_leaves_refresh: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        let theme = detect_system_theme();
        Self {
            started_at: Instant::now(),
            last_refresh: Instant::now(),
            status: "Ready".to_string(),
            theme_mode: ThemeMode::Auto,
            theme,
            input_mode: InputMode::Normal,
            search_query: String::new(),
            leaves: Vec::new(),
            selected_index: Some(0),
            details_cache: HashMap::new(),
            pending_details: None,
            last_error: None,
            last_leaves_refresh: None,
        }
    }

    pub fn on_tick(&mut self) {
        if self.last_refresh.elapsed() >= Duration::from_secs(5) {
            self.last_refresh = Instant::now();
            self.status = "Idle".to_string();
        }
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

    pub fn refresh_leaves(&mut self) {
        match fetch_leaves() {
            Ok(mut leaves) => {
                leaves.sort();
                self.leaves = leaves;
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
        self.last_refresh = Instant::now();
    }

    pub fn filtered_leaves(&self) -> Vec<(usize, &str)> {
        if self.search_query.is_empty() {
            return self
                .leaves
                .iter()
                .enumerate()
                .map(|(idx, item)| (idx, item.as_str()))
                .collect();
        }
        let needle = self.search_query.to_lowercase();
        self.leaves
            .iter()
            .enumerate()
            .filter(|(_, item)| item.to_lowercase().contains(&needle))
            .map(|(idx, item)| (idx, item.as_str()))
            .collect()
    }

    pub fn selected_leaf(&self) -> Option<&str> {
        let selected = self.selected_index?;
        self.leaves.get(selected).map(String::as_str)
    }

    pub fn select_next(&mut self) {
        if self.leaves.is_empty() {
            self.selected_index = None;
            return;
        }
        let next = match self.selected_index {
            Some(idx) => (idx + 1).min(self.leaves.len() - 1),
            None => 0,
        };
        self.selected_index = Some(next);
    }

    pub fn select_prev(&mut self) {
        if self.leaves.is_empty() {
            self.selected_index = None;
            return;
        }
        let prev = match self.selected_index {
            Some(idx) => idx.saturating_sub(1),
            None => 0,
        };
        self.selected_index = Some(prev);
    }

    pub fn request_details(
        &mut self,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        let Some(pkg) = self.selected_leaf().map(str::to_string) else {
            return;
        };

        if let Some(pending) = self.pending_details.as_ref() {
            if pending == &pkg {
                return;
            }
        }

        if let Some(existing) = self.details_cache.get(&pkg) {
            match load {
                DetailsLoad::Basic => return,
                DetailsLoad::Full => {
                    if existing.deps.is_some() && existing.uses.is_some() {
                        return;
                    }
                }
            }
        }

        self.pending_details = Some(pkg.clone());
        self.status = match load {
            DetailsLoad::Basic => "Loading details...".to_string(),
            DetailsLoad::Full => "Loading deps/uses...".to_string(),
        };
        self.last_refresh = Instant::now();

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = match load {
                DetailsLoad::Basic => fetch_details_basic(&pkg).await,
                DetailsLoad::Full => fetch_details_full(&pkg).await,
            };
            let _ = tx.send(DetailsMessage { pkg, load, result });
        });
    }

    pub fn apply_details_message(&mut self, message: DetailsMessage) {
        match message.result {
            Ok(details) => {
                self.details_cache
                    .entry(message.pkg.clone())
                    .and_modify(|existing| merge_details(existing, &details))
                    .or_insert(details);
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
    }
}

fn merge_details(existing: &mut Details, incoming: &Details) {
    existing.desc = incoming.desc.clone();
    existing.homepage = incoming.homepage.clone();
    existing.installed = incoming.installed.clone();
    if incoming.deps.is_some() {
        existing.deps = incoming.deps.clone();
    }
    if incoming.uses.is_some() {
        existing.uses = incoming.uses.clone();
    }
}

#[derive(Clone, Copy)]
pub enum InputMode {
    Normal,
    Search,
}
