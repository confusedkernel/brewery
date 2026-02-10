use crate::brew::{run_brew_command, run_command};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

const LATEST_BREWERY_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

#[derive(Clone)]
struct LatestBreweryCacheEntry {
    version: Option<String>,
    checked_at: SystemTime,
}

static LATEST_BREWERY_CACHE: OnceLock<Mutex<Option<LatestBreweryCacheEntry>>> = OnceLock::new();

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
    pub brewery_latest_version: Option<String>,
    pub brewery_update_available: bool,
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
        latest_brewery_version,
    ) = tokio::join!(
        run_brew_command(&["--version"]),
        run_brew_command(&["info"]),
        run_brew_command(&["leaves"]),
        run_brew_command(&["doctor"]),
        run_brew_command(&["--repository"]),
        run_brew_command(&["--repository", "homebrew/core"]),
        fetch_latest_brewery_version_cached(),
    );

    // Process version result
    if let Ok(result) = version_result
        && result.success
    {
        let mut lines = result
            .stdout
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        status.brew_version = lines.next().map(str::to_string);
    }

    // Process info result
    if let Ok(result) = info_result
        && result.success
    {
        status.brew_info = result
            .stdout
            .lines()
            .map(|s| s.trim())
            .find(|line| !line.is_empty())
            .map(str::to_string);
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
    if let Ok(result) = brew_repo_result
        && result.success
        && let Some(path) = first_nonempty_line(&result.stdout)
    {
        repo_paths.push(path.to_string());
    }
    if let Ok(result) = core_repo_result
        && result.success
        && let Some(path) = first_nonempty_line(&result.stdout)
    {
        repo_paths.push(path.to_string());
    }
    status.last_brew_update_secs_ago = last_update_secs_ago(&repo_paths);
    status.brew_update_status = Some(match status.last_brew_update_secs_ago {
        Some(secs) if secs <= 86_400 => "Up to date".to_string(),
        Some(_) => "Update recommended".to_string(),
        None => "Unknown".to_string(),
    });

    if let Some(latest) = latest_brewery_version {
        status.brewery_update_available = is_newer_version(&latest, env!("CARGO_PKG_VERSION"));
        status.brewery_latest_version = Some(latest);
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

fn first_nonempty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

fn last_update_secs_ago(repo_paths: &[String]) -> Option<u64> {
    let mut latest: Option<SystemTime> = None;

    for repo in repo_paths {
        let fetch_head = PathBuf::from(repo).join(".git").join("FETCH_HEAD");
        if let Ok(metadata) = std::fs::metadata(fetch_head)
            && let Ok(modified) = metadata.modified()
        {
            latest = Some(
                latest
                    .map(|current| current.max(modified))
                    .unwrap_or(modified),
            );
        }
    }

    latest.and_then(|time| time.elapsed().ok().map(|elapsed| elapsed.as_secs()))
}

fn parse_latest_brewery_version(stdout: &str) -> Option<String> {
    let line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("brewery "))?;
    let first_quote = line.find('"')?;
    let rest = &line[first_quote + 1..];
    let second_quote = rest.find('"')?;
    let version = rest[..second_quote].trim();
    if version.is_empty() {
        None
    } else {
        Some(version.to_string())
    }
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_semver_triplet(latest) > parse_semver_triplet(current)
}

fn parse_semver_triplet(version: &str) -> (u64, u64, u64) {
    let core = version.split('-').next().unwrap_or(version);
    let mut parts = core.split('.');
    let major = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let minor = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(0);
    (major, minor, patch)
}

async fn fetch_latest_brewery_version_cached() -> Option<String> {
    if let Some(version) = read_cached_latest_brewery_version() {
        return version;
    }

    let fetched_version = match run_command("cargo", &["search", "brewery", "--limit", "1"]).await {
        Ok(result) if result.success => parse_latest_brewery_version(&result.stdout),
        _ => None,
    };

    write_cached_latest_brewery_version(fetched_version.clone());
    fetched_version
}

fn read_cached_latest_brewery_version() -> Option<Option<String>> {
    let cache = LATEST_BREWERY_CACHE.get_or_init(|| Mutex::new(None));
    let guard = cache.lock().ok()?;
    let entry = guard.as_ref()?;
    let age = entry.checked_at.elapsed().ok()?;
    if age <= LATEST_BREWERY_CACHE_TTL {
        Some(entry.version.clone())
    } else {
        None
    }
}

fn write_cached_latest_brewery_version(version: Option<String>) {
    let cache = LATEST_BREWERY_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = cache.lock() {
        *guard = Some(LatestBreweryCacheEntry {
            version,
            checked_at: SystemTime::now(),
        });
    }
}
