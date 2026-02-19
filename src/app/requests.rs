use tokio::sync::mpsc;

use super::*;

impl App {
    pub fn request_leaves(&mut self, tx: &mpsc::UnboundedSender<LeavesMessage>) {
        if self.pending_leaves {
            return;
        }

        self.pending_leaves = true;
        self.status = "Loading leaves...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_leaves().await;
            let _ = tx.send(LeavesMessage { result });
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

    pub fn request_sizes(&mut self, tx: &mpsc::UnboundedSender<SizesMessage>) {
        if self.pending_sizes {
            return;
        }

        self.pending_sizes = true;
        self.status = "Loading sizes...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_sizes().await;
            let _ = tx.send(SizesMessage { result });
        });
    }

    pub fn request_casks(&mut self, tx: &mpsc::UnboundedSender<CasksMessage>) {
        if self.pending_casks {
            return;
        }

        self.pending_casks = true;
        self.status = "Loading casks...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_casks().await;
            let _ = tx.send(CasksMessage { result });
        });
    }

    pub fn request_status(&mut self, tx: &mpsc::UnboundedSender<StatusMessage>) {
        if self.pending_status {
            return;
        }

        self.pending_status = true;
        self.status = "Checking status...".to_string();
        self.last_refresh = Instant::now();
        self.needs_redraw = true;

        let tx = tx.clone();
        tokio::spawn(async move {
            let result = fetch_status().await;
            let _ = tx.send(StatusMessage { result });
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
