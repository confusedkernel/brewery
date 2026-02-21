use std::future::Future;

use tokio::sync::mpsc;

use super::*;

impl App {
    pub fn request_leaves(&mut self, tx: &mpsc::UnboundedSender<LeavesMessage>) {
        if self.pending_leaves {
            return;
        }

        self.pending_leaves = true;
        set_request_status(self, "Loading leaves...", true);

        spawn_request(tx, async {
            LeavesMessage {
                result: fetch_leaves().await,
            }
        });
    }

    pub fn request_details(
        &mut self,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        let Some(pkg) = self.selected_installed_package().map(str::to_string) else {
            return;
        };

        self.request_details_for(&pkg, load, tx);
    }

    pub fn request_details_for(
        &mut self,
        pkg: &str,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        self.request_details_for_inner(pkg, load, tx, false);
    }

    pub fn request_details_forced(
        &mut self,
        pkg: &str,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
    ) {
        self.request_details_for_inner(pkg, load, tx, true);
    }

    fn request_details_for_inner(
        &mut self,
        pkg: &str,
        load: DetailsLoad,
        tx: &mpsc::UnboundedSender<DetailsMessage>,
        force: bool,
    ) {
        let pkg = pkg.to_string();

        if let Some(pending) = self.pending_details.as_ref()
            && pending == &pkg
        {
            return;
        }

        if !force && let Some(existing) = self.details_cache.get(&pkg) {
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
        set_request_status(
            self,
            match load {
                DetailsLoad::Basic => "Loading details...",
                DetailsLoad::Full => "Loading deps/uses...",
            },
            false,
        );

        spawn_request(tx, async move {
            let result = match load {
                DetailsLoad::Basic => fetch_details_basic(&pkg).await,
                DetailsLoad::Full => fetch_details_full(&pkg).await,
            };
            DetailsMessage { pkg, load, result }
        });
    }

    pub fn request_sizes(&mut self, tx: &mpsc::UnboundedSender<SizesMessage>) {
        if self.pending_sizes {
            return;
        }

        self.pending_sizes = true;
        set_request_status(self, "Loading sizes...", true);

        spawn_request(tx, async {
            SizesMessage {
                result: fetch_sizes().await,
            }
        });
    }

    pub fn request_casks(&mut self, tx: &mpsc::UnboundedSender<CasksMessage>) {
        if self.pending_casks {
            return;
        }

        self.pending_casks = true;
        set_request_status(self, "Loading casks...", true);

        spawn_request(tx, async {
            CasksMessage {
                result: fetch_casks().await,
            }
        });
    }

    pub fn request_status(&mut self, tx: &mpsc::UnboundedSender<StatusMessage>) {
        if self.pending_status {
            return;
        }

        self.pending_status = true;
        set_request_status(self, "Checking status...", true);

        spawn_request(tx, async {
            StatusMessage {
                result: fetch_status().await,
            }
        });
    }

    pub fn request_command(
        &mut self,
        kind: CommandKind,
        args: &[&str],
        tx: &mpsc::UnboundedSender<CommandMessage>,
    ) {
        if self.pending_command {
            return;
        }

        self.pending_command = true;
        self.last_command = Some(kind);
        self.last_command_target = if kind.is_package_action() {
            args.last().map(|value| (*value).to_string())
        } else {
            None
        };
        self.last_command_target_is_cask = kind.is_package_action() && args.contains(&"--cask");
        self.command_started_at = Some(Instant::now());
        self.last_command_output.clear();
        self.last_command_error = None;
        self.status = format!("Running {kind}...");
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        tokio::spawn(async move {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            let result = if kind == CommandKind::SelfUpdate {
                run_command("cargo", &arg_refs).await
            } else {
                run_brew_command(&arg_refs).await
            };
            let _ = tx.send(CommandMessage { kind, result });
        });
    }
}

fn set_request_status(app: &mut App, status: &str, needs_redraw: bool) {
    app.status = status.to_string();
    app.last_refresh = Instant::now();
    if needs_redraw {
        app.needs_redraw = true;
    }
}

fn spawn_request<Message, Fut>(tx: &mpsc::UnboundedSender<Message>, task: Fut)
where
    Message: Send + 'static,
    Fut: Future<Output = Message> + Send + 'static,
{
    let tx = tx.clone();
    tokio::spawn(async move {
        let message = task.await;
        let _ = tx.send(message);
    });
}
