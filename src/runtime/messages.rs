use std::time::Duration;

use tokio::sync::mpsc;

use crate::app::{App, InputMode, Message, MessageTx};
use crate::brew::{CommandKind, DetailsLoad};

pub struct RuntimeChannels {
    pub tx: MessageTx,
    pub rx: mpsc::UnboundedReceiver<Message>,
}

pub fn create_channels() -> RuntimeChannels {
    let (tx, rx) = mpsc::unbounded_channel();
    RuntimeChannels { tx, rx }
}

pub fn process_pending_messages(app: &mut App, channels: &mut RuntimeChannels) {
    while let Ok(message) = channels.rx.try_recv() {
        let Message::Command { kind, result } = &message else {
            app.apply_message(message);
            continue;
        };

        let kind = *kind;
        let succeeded = result.as_ref().is_ok_and(|result| result.success);
        let upgraded_pkg = app
            .last_command_target
            .clone()
            .filter(|_| succeeded && kind == CommandKind::Upgrade);
        app.apply_message(message);

        if succeeded && kind.refreshes_lists_on_success() {
            app.request_leaves(&channels.tx);
            app.request_casks(&channels.tx);
            // Anything that adds or removes kegs invalidates the graph.
            app.request_graph(&channels.tx);
        }
        if succeeded && kind.refreshes_status_on_success() {
            app.request_status(&channels.tx);
            app.request_doctor(&channels.tx);
        }
        if let Some(pkg) = upgraded_pkg {
            app.request_details_forced(&pkg, DetailsLoad::Basic, &channels.tx);
        }
    }
}

/// Loads basic details for whatever is selected once the selection has
/// settled, so scrolling through a list does not fire a request per row.
pub fn handle_auto_details(
    app: &mut App,
    last_fetched_leaf: &mut Option<String>,
    tx: &MessageTx,
    debounce: Duration,
) {
    let settled = app.pending_details.is_none()
        && !app.is_rapid_scrolling()
        && app
            .last_selection_change
            .is_none_or(|changed| changed.elapsed() >= debounce);
    if !settled {
        return;
    }

    let searching = matches!(
        app.input_mode,
        InputMode::PackageSearch | InputMode::PackageResults
    );
    let selected = if searching {
        app.selected_package_result()
    } else {
        app.selected_installed_package()
    };
    let Some(pkg) = selected.map(str::to_string) else {
        return;
    };

    let last_fetched = if searching {
        &mut app.last_result_details_pkg
    } else {
        last_fetched_leaf
    };
    if last_fetched.as_deref() == Some(pkg.as_str()) {
        return;
    }

    *last_fetched = Some(pkg.clone());
    app.request_details_for(&pkg, DetailsLoad::Basic, tx);
}
