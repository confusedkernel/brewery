use crate::brew::run_brew_command;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Clone, Debug, Default)]
pub struct StatusSnapshot {
    pub doctor_ok: Option<bool>,
    pub doctor_issues: Vec<String>,
    pub outdated_count: Option<usize>,
    pub outdated_packages: Vec<String>,
    pub brew_version: Option<String>,
    pub brew_info: Option<String>,
    pub brew_update_status: Option<String>,
    pub last_brew_update_secs_ago: Option<u64>,
}

pub struct StatusMessage {
    pub result: anyhow::Result<StatusSnapshot>,
}

pub async fn fetch_status() -> anyhow::Result<StatusSnapshot> {
    let mut status = StatusSnapshot::default();

    // Run independent commands in parallel (version, info, leaves, doctor, repo paths)
    let (
        version_result,
        info_result,
        leaves_result,
        doctor_result,
        brew_repo_result,
        core_repo_result,
    ) = tokio::join!(
        run_brew_command(&["--version"]),
        run_brew_command(&["info"]),
        run_brew_command(&["leaves"]),
        run_brew_command(&["doctor"]),
        run_brew_command(&["--repository"]),
        run_brew_command(&["--repository", "homebrew/core"]),
    );

    // Process version result
    if let Ok(result) = version_result {
        if result.success {
            let mut lines = result
                .stdout
                .lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty());
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

    // Process last brew update time from repository metadata
    let mut repo_paths = Vec::new();
    if let Ok(result) = brew_repo_result {
        if result.success {
            if let Some(path) = first_nonempty_line(&result.stdout) {
                repo_paths.push(path.to_string());
            }
        }
    }
    if let Ok(result) = core_repo_result {
        if result.success {
            if let Some(path) = first_nonempty_line(&result.stdout) {
                repo_paths.push(path.to_string());
            }
        }
    }
    status.last_brew_update_secs_ago = last_update_secs_ago(&repo_paths);
    status.brew_update_status = Some(match status.last_brew_update_secs_ago {
        Some(secs) if secs <= 86_400 => "Up to date".to_string(),
        Some(_) => "Update recommended".to_string(),
        None => "Unknown".to_string(),
    });

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

fn first_nonempty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

fn last_update_secs_ago(repo_paths: &[String]) -> Option<u64> {
    let mut latest: Option<SystemTime> = None;

    for repo in repo_paths {
        let fetch_head = PathBuf::from(repo).join(".git").join("FETCH_HEAD");
        if let Ok(metadata) = std::fs::metadata(fetch_head) {
            if let Ok(modified) = metadata.modified() {
                latest = Some(latest.map(|current| current.max(modified)).unwrap_or(modified));
            }
        }
    }

    latest.and_then(|time| time.elapsed().ok().map(|elapsed| elapsed.as_secs()))
}
