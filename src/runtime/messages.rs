use std::time::Instant;

use tokio::sync::mpsc;

use crate::app::{App, InputMode};
use crate::brew::{DetailsLoad, LeavesMessage};

pub struct RuntimeChannels {
    pub leaves_tx: mpsc::UnboundedSender<LeavesMessage>,
    pub details_tx: mpsc::UnboundedSender<crate::brew::DetailsMessage>,
    pub sizes_tx: mpsc::UnboundedSender<crate::brew::SizesMessage>,
    pub command_tx: mpsc::UnboundedSender<crate::brew::CommandMessage>,
    pub status_tx: mpsc::UnboundedSender<crate::brew::StatusMessage>,
    pub leaves_rx: mpsc::UnboundedReceiver<LeavesMessage>,
    pub details_rx: mpsc::UnboundedReceiver<crate::brew::DetailsMessage>,
    pub sizes_rx: mpsc::UnboundedReceiver<crate::brew::SizesMessage>,
    pub command_rx: mpsc::UnboundedReceiver<crate::brew::CommandMessage>,
    pub status_rx: mpsc::UnboundedReceiver<crate::brew::StatusMessage>,
}

pub fn create_channels() -> RuntimeChannels {
    let (leaves_tx, leaves_rx) = mpsc::unbounded_channel::<LeavesMessage>();
    let (details_tx, details_rx) = mpsc::unbounded_channel();
    let (sizes_tx, sizes_rx) = mpsc::unbounded_channel();
    let (command_tx, command_rx) = mpsc::unbounded_channel();
    let (status_tx, status_rx) = mpsc::unbounded_channel();

    RuntimeChannels {
        leaves_tx,
        details_tx,
        sizes_tx,
        command_tx,
        status_tx,
        leaves_rx,
        details_rx,
        sizes_rx,
        command_rx,
        status_rx,
    }
}

pub fn process_pending_messages(app: &mut App, channels: &mut RuntimeChannels) {
    let mut received_message = false;

    while let Ok(message) = channels.leaves_rx.try_recv() {
        app.apply_leaves_message(message);
        received_message = true;
    }
    while let Ok(message) = channels.details_rx.try_recv() {
        app.apply_details_message(message);
        received_message = true;
    }
    while let Ok(message) = channels.sizes_rx.try_recv() {
        app.apply_sizes_message(message);
        received_message = true;
    }
    while let Ok(message) = channels.command_rx.try_recv() {
        let mut should_refresh_leaves = false;
        let mut should_refresh_status = false;
        let mut refresh_details_pkg = None;
        if let Ok(result) = &message.result
            && result.success
        {
            should_refresh_leaves = matches!(
                message.label.as_str(),
                "install" | "uninstall" | "upgrade" | "upgrade-all"
            );
            should_refresh_status = matches!(
                message.label.as_str(),
                "install" | "uninstall" | "upgrade" | "upgrade-all"
            );
            if message.label == "upgrade" {
                refresh_details_pkg = app.last_command_target.clone();
            }
        }
        app.apply_command_message(message);
        if should_refresh_leaves {
            app.request_leaves(&channels.leaves_tx);
        }
        if should_refresh_status {
            app.request_status(&channels.status_tx);
        }
        if let Some(pkg) = refresh_details_pkg {
            app.request_details_forced(&pkg, DetailsLoad::Basic, &channels.details_tx);
        }
        received_message = true;
    }
    while let Ok(message) = channels.status_rx.try_recv() {
        app.apply_status_message(message);
        received_message = true;
    }

    if received_message {
        app.needs_redraw = true;
    }
}

pub fn handle_auto_details(
    app: &mut App,
    last_fetched_leaf: &mut Option<String>,
    details_tx: &mpsc::UnboundedSender<crate::brew::DetailsMessage>,
    debounce: std::time::Duration,
) {
    if matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) && let Some(pkg) = app.selected_package_result().map(str::to_string)
    {
        let already_fetched = app.last_result_details_pkg.as_deref() == Some(pkg.as_str());
        let debounce_elapsed = app
            .last_selection_change
            .map(|t| t.elapsed() >= debounce)
            .unwrap_or(true);
        let not_pending = app.pending_details.is_none();
        let not_scrolling = !app.is_rapid_scrolling();

        if !already_fetched && debounce_elapsed && not_pending && not_scrolling {
            app.request_details_for(&pkg, DetailsLoad::Basic, details_tx);
            app.last_result_details_pkg = Some(pkg);
        }
    }

    if !matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    ) {
        let selected = app.selected_leaf().map(str::to_string);
        if let Some(ref pkg) = selected {
            let already_fetched = last_fetched_leaf.as_ref() == Some(pkg);
            let debounce_elapsed = app
                .last_selection_change
                .map(|t| t.elapsed() >= debounce)
                .unwrap_or(true);
            let not_pending = app.pending_details.is_none();
            let not_scrolling = !app.is_rapid_scrolling();

            if !already_fetched && debounce_elapsed && not_pending && not_scrolling {
                app.request_details(DetailsLoad::Basic, details_tx);
                *last_fetched_leaf = selected.clone();
            }
        }
    }
}

pub fn handle_focus_backtab(app: &mut App) {
    app.focus_panel = match app.focus_panel {
        crate::app::FocusedPanel::Leaves => crate::app::FocusedPanel::Details,
        crate::app::FocusedPanel::Sizes => crate::app::FocusedPanel::Leaves,
        crate::app::FocusedPanel::Status => crate::app::FocusedPanel::Sizes,
        crate::app::FocusedPanel::Details => crate::app::FocusedPanel::Status,
    };
    app.status = format!("Focus: {:?}", app.focus_panel);
    app.last_refresh = Instant::now();
}
