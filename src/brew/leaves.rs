use std::path::Path;

use super::graph::short_name;
use super::process::{cellar_path, ensure_success, nonempty_lines, run_brew};

/// The installed formulae, in both the scopes the list can show.
///
/// `brew leaves` is a pruning aid — it hides anything another formula depends
/// on — so it is a poor default for browsing. Both scopes are fetched together
/// so they can never disagree about what is installed.
pub struct InstalledFormulae {
    pub leaves: Vec<String>,
    pub all: Vec<String>,
}

pub async fn fetch_leaves() -> anyhow::Result<InstalledFormulae> {
    let (leaves, all) = tokio::try_join!(fetch_leaf_names(), fetch_installed_names())?;

    Ok(InstalledFormulae { leaves, all })
}

/// `brew leaves` reports tap-qualified names (`org/tap/pkg`) for anything
/// installed from a tap, while the Cellar — and therefore every receipt-derived
/// lookup in the app — uses the bare name. Normalizing here is what lets a tap
/// formula match its own receipt, so its provenance and uninstall impact render
/// instead of coming back empty.
async fn fetch_leaf_names() -> anyhow::Result<Vec<String>> {
    let output = run_brew(&["leaves"]).await?;
    ensure_success(&output, "brew leaves failed")?;

    let mut leaves: Vec<String> = nonempty_lines(&output.stdout)
        .iter()
        .map(|name| short_name(name))
        .collect();
    leaves.sort();
    leaves.dedup();
    Ok(leaves)
}

/// Every installed formula, read from the Cellar rather than by shelling out to
/// `brew list --formula`. The keg directory names are exactly what that command
/// prints, and reading them costs ~10ms against ~600ms for the subprocess —
/// which is the difference between the list panel being there on the first
/// frame and arriving after a visible pause.
async fn fetch_installed_names() -> anyhow::Result<Vec<String>> {
    let cellar = cellar_path().await?;
    tokio::task::spawn_blocking(move || read_installed_names(&cellar)).await?
}

fn read_installed_names(cellar: &Path) -> anyhow::Result<Vec<String>> {
    let mut names = Vec::new();

    let Ok(entries) = std::fs::read_dir(cellar) else {
        return Ok(names);
    };

    for entry in entries.flatten() {
        // Follows symlinks on purpose: an alias formula such as `rustfmt` is a
        // symlink to the keg directory it aliases (`rust`), and `brew list`
        // counts it as installed.
        if !entry.path().is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        // A formula directory holding no keg is a leftover from a failed or
        // partial uninstall, which `brew list` does not count as installed.
        if !has_keg(&entry.path()) {
            continue;
        }

        names.push(name);
    }

    names.sort();
    Ok(names)
}

fn has_keg(formula_dir: &Path) -> bool {
    let Ok(mut versions) = std::fs::read_dir(formula_dir) else {
        return false;
    };

    versions.any(|version| version.is_ok_and(|version| version.path().is_dir()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Cellar scan has to agree with `brew list --formula` exactly, since
    /// it now stands in for that command. Hits the real system, so it is
    /// opt-in: `cargo test -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "requires a local Homebrew installation"]
    async fn cellar_scan_agrees_with_brew_list() {
        let scanned = fetch_installed_names()
            .await
            .expect("cellar scan should succeed");

        let output = run_brew(&["list", "--formula"])
            .await
            .expect("brew list should run");
        let mut expected = nonempty_lines(&output.stdout);
        expected.sort();

        println!("{} formulae scanned from the Cellar", scanned.len());
        assert_eq!(
            scanned, expected,
            "the Cellar scan disagrees with brew list --formula",
        );
    }
}
