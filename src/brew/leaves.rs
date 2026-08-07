use super::process::{ensure_success, nonempty_lines, run_brew};

/// The installed formulae, in both the scopes the list can show.
///
/// `brew leaves` is a pruning aid — it hides anything another formula depends
/// on — so it is a poor default for browsing. Both scopes are fetched together
/// so they can never disagree about what is installed.
pub struct InstalledFormulae {
    pub leaves: Vec<String>,
    pub all: Vec<String>,
}

pub struct LeavesMessage {
    pub result: anyhow::Result<InstalledFormulae>,
}

pub async fn fetch_leaves() -> anyhow::Result<InstalledFormulae> {
    let (leaves, all) = tokio::try_join!(
        fetch_list(&["leaves"], "brew leaves failed"),
        fetch_list(&["list", "--formula"], "brew list --formula failed"),
    )?;

    Ok(InstalledFormulae { leaves, all })
}

async fn fetch_list(args: &[&str], fallback: &str) -> anyhow::Result<Vec<String>> {
    let output = run_brew(args).await?;
    ensure_success(&output, fallback)?;
    Ok(nonempty_lines(&output.stdout))
}
