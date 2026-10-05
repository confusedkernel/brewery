use std::future::Future;

use tokio::sync::mpsc;

use super::*;

pub type MessageTx = mpsc::UnboundedSender<Message>;

impl App {
    pub fn request_leaves(&mut self, tx: &MessageTx) {
        if !self.leaves_job.start() {
            return;
        }

        self.set_request_status("Loading leaves...");
        spawn_request(tx, async { Message::Leaves(fetch_leaves().await) });
    }

    pub fn request_details(&mut self, load: DetailsLoad, tx: &MessageTx) {
        let Some(pkg) = self.selected_installed_package().map(str::to_string) else {
            return;
        };

        self.request_details_for(&pkg, load, tx);
    }

    pub fn request_details_for(&mut self, pkg: &str, load: DetailsLoad, tx: &MessageTx) {
        self.request_details_inner(pkg, load, tx, false);
    }

    pub fn request_details_forced(&mut self, pkg: &str, load: DetailsLoad, tx: &MessageTx) {
        self.request_details_inner(pkg, load, tx, true);
    }

    fn request_details_inner(&mut self, pkg: &str, load: DetailsLoad, tx: &MessageTx, force: bool) {
        if self.pending_details.as_deref() == Some(pkg) {
            return;
        }

        if !force && let Some(existing) = self.details_cache.get(pkg) {
            let has_everything = match load {
                DetailsLoad::Basic => true,
                DetailsLoad::Full => existing.deps.is_some() && existing.uses.is_some(),
            };
            if has_everything {
                return;
            }
        }

        let pkg = pkg.to_string();
        self.pending_details = Some(pkg.clone());
        // Details load on every selection change, so they update the status
        // line without forcing a redraw of their own.
        self.status = match load {
            DetailsLoad::Basic => "Loading details...",
            DetailsLoad::Full => "Loading deps/uses...",
        }
        .to_string();
        self.last_refresh = Instant::now();

        spawn_request(tx, async move {
            let result = match load {
                DetailsLoad::Basic => fetch_details_basic(&pkg).await,
                DetailsLoad::Full => fetch_details_full(&pkg).await,
            };
            Message::Details { pkg, load, result }
        });
    }

    pub fn request_sizes(&mut self, tx: &MessageTx) {
        if !self.sizes_job.start() {
            return;
        }

        self.set_request_status("Loading sizes...");
        spawn_request(tx, async { Message::Sizes(fetch_sizes().await) });
    }

    pub fn request_casks(&mut self, tx: &MessageTx) {
        if !self.casks_job.start() {
            return;
        }

        self.set_request_status("Loading casks...");
        spawn_request(tx, async { Message::Casks(fetch_casks().await) });
    }

    pub fn request_graph(&mut self, tx: &MessageTx) {
        if !self.graph_job.start() {
            return;
        }

        spawn_request(tx, async { Message::Graph(fetch_dependency_graph().await) });
    }

    pub fn request_status(&mut self, tx: &MessageTx) {
        if !self.status_job.start() {
            return;
        }

        self.set_request_status("Checking status...");
        let known_leaves = (!self.leaf_formulae.is_empty()).then(|| self.leaf_formulae.clone());
        spawn_request(tx, async move {
            Message::Status(fetch_status(known_leaves).await)
        });
    }

    /// Deliberately quiet: no status-line text and no spinner. The doctor run
    /// outlives the rest of the status check, and announcing it would put the
    /// panel back to looking busy for the second it saves.
    pub fn request_doctor(&mut self, tx: &MessageTx) {
        if !self.doctor_job.start() {
            return;
        }

        spawn_request(tx, async { Message::Doctor(fetch_doctor().await) });
    }

    pub fn request_command(&mut self, kind: CommandKind, args: &[&str], tx: &MessageTx) {
        if !self.command_job.start() {
            return;
        }

        self.last_command = Some(kind);
        self.last_command_target = if kind.has_named_target() {
            args.last().map(|value| (*value).to_string())
        } else {
            None
        };
        self.last_command_args = args.iter().map(|arg| (*arg).to_string()).collect();
        self.last_command_output.clear();
        self.last_command_error = None;
        self.set_request_status(format!("Running {kind}..."));

        let args = self.last_command_args.clone();
        spawn_request(tx, async move {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let result = run_command(kind.binary(), &arg_refs).await;
            Message::Command { kind, result }
        });
    }

    fn set_request_status(&mut self, status: impl Into<String>) {
        self.set_status(status);
        self.needs_redraw = true;
    }
}

fn spawn_request<Fut>(tx: &MessageTx, task: Fut)
where
    Fut: Future<Output = Message> + Send + 'static,
{
    let tx = tx.clone();
    tokio::spawn(async move {
        let _ = tx.send(task.await);
    });
}
