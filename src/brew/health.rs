use crate::brew::run_brew_command;
use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct HealthStatus {
    pub doctor_ok: Option<bool>,
    pub doctor_issues: Vec<String>,
    pub outdated_count: Option<usize>,
    pub outdated_packages: Vec<String>,
    pub brew_version: Option<String>,
}

pub struct HealthMessage {
    pub result: anyhow::Result<HealthStatus>,
}

pub async fn fetch_health() -> anyhow::Result<HealthStatus> {
    let mut status = HealthStatus::default();

    // Get brew version
    if let Ok(result) = run_brew_command(&["--version"]).await {
        if result.success {
            status.brew_version = result.stdout.lines().next().map(|s| s.trim().to_string());
        }
    }

    // Run brew doctor
    if let Ok(result) = run_brew_command(&["doctor"]).await {
        status.doctor_ok = Some(result.success);
        if !result.success {
            // Parse warnings/errors from stderr or stdout
            let output = if result.stderr.is_empty() {
                &result.stdout
            } else {
                &result.stderr
            };
            status.doctor_issues = output
                .lines()
                .filter(|line| line.starts_with("Warning:") || line.starts_with("Error:"))
                .take(5)
                .map(|s| s.to_string())
                .collect();
        }
    }

    // Get outdated leaves
    let leaf_set: HashSet<String> = match run_brew_command(&["leaves"]).await {
        Ok(result) => result
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
        Err(_) => HashSet::new(),
    };

    if let Ok(result) = run_brew_command(&["outdated", "--formula"]).await {
        let packages: Vec<String> = result
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.trim().to_string())
            .filter(|name| leaf_set.contains(name))
            .collect();
        status.outdated_count = Some(packages.len());
        status.outdated_packages = packages;
    }

    Ok(status)
}
