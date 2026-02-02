use crate::brew::run_brew_command;
use std::collections::HashSet;

#[derive(Clone, Debug, Default)]
pub struct HealthStatus {
    pub doctor_ok: Option<bool>,
    pub doctor_issues: Vec<String>,
    pub outdated_count: Option<usize>,
    pub outdated_packages: Vec<String>,
    pub brew_version: Option<String>,
    pub brew_info: Option<String>,
}

pub struct HealthMessage {
    pub result: anyhow::Result<HealthStatus>,
}

pub async fn fetch_health() -> anyhow::Result<HealthStatus> {
    let mut status = HealthStatus::default();

    // Run independent commands in parallel (version, info, leaves, doctor)
    let (version_result, info_result, leaves_result, doctor_result) = tokio::join!(
        run_brew_command(&["--version"]),
        run_brew_command(&["info"]),
        run_brew_command(&["leaves"]),
        run_brew_command(&["doctor"]),
    );

    // Process version result
    if let Ok(result) = version_result {
        if result.success {
            let mut lines = result.stdout.lines().map(|s| s.trim()).filter(|s| !s.is_empty());
            status.brew_version = lines.next().map(str::to_string);
        }
    }

    // Process info result
    if let Ok(result) = info_result {
        if result.success {
            status.brew_info = result
                .stdout
                .lines()
                .map(|s| s.trim())
                .find(|line| !line.is_empty())
                .map(str::to_string);
        }
    }

    // Process doctor result
    if let Ok(result) = doctor_result {
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

    // Build leaf set from leaves result
    let leaf_set: HashSet<String> = match leaves_result {
        Ok(result) => result
            .stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|s| s.trim().to_string())
            .collect(),
        Err(_) => HashSet::new(),
    };

    // Now fetch outdated (this depends on having leaf_set ready for filtering)
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
