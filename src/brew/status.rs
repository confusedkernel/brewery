use super::services::{ServiceEntry, fetch_services};
use crate::brew::graph::short_name;
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
    pub outdated_count: Option<usize>,
    pub outdated_packages: Vec<String>,
    pub brew_version: Option<String>,
    pub brew_update_status: Option<String>,
    pub last_brew_update_secs_ago: Option<u64>,
    pub brewery_latest_version: Option<String>,
    pub brewery_update_available: bool,
    pub services: Vec<ServiceEntry>,
}

pub struct StatusMessage {
    pub result: anyhow::Result<StatusSnapshot>,
}

/// `known_leaves` is the leaf set the app has already loaded, used to keep the
/// outdated list to packages the user asked for. Only the first status check —
/// which races the initial load — has to ask `brew` for it again.
pub async fn fetch_status(known_leaves: Option<Vec<String>>) -> anyhow::Result<StatusSnapshot> {
    let mut status = StatusSnapshot::default();

    // Every one of these is an independent `brew` invocation costing hundreds of
    // milliseconds, so the whole set runs at once and the snapshot lands at the
    // cost of the slowest rather than their sum.
    let (
        version_result,
        leaf_set,
        brew_repo_result,
        core_repo_result,
        outdated_result,
        services_result,
        latest_brewery_version,
    ) = tokio::join!(
        run_brew_command(&["--version"]),
        fetch_leaf_set(known_leaves),
        run_brew_command(&["--repository"]),
        run_brew_command(&["--repository", "homebrew/core"]),
        run_brew_command(&["outdated", "--formula"]),
        fetch_services(),
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

    if let Ok(services) = services_result {
        status.services = services;
    }

    if let Ok(result) = outdated_result {
        let packages: Vec<String> = bare_formula_names(&result.stdout)
            .filter(|name| leaf_set.contains(name))
            .collect();
        status.outdated_count = Some(packages.len());
        status.outdated_packages = packages;
    }

    Ok(status)
}

/// Leaves the app already has, or a fresh `brew leaves` when it has none yet.
async fn fetch_leaf_set(known: Option<Vec<String>>) -> HashSet<String> {
    if let Some(leaves) = known.filter(|leaves| !leaves.is_empty()) {
        return leaves.into_iter().collect();
    }

    match run_brew_command(&["leaves"]).await {
        Ok(result) if result.success => bare_formula_names(&result.stdout).collect(),
        _ => HashSet::new(),
    }
}

/// Formula names as the rest of the app spells them: bare, never tap-qualified.
/// `brew leaves` and `brew outdated` both print `org/tap/pkg` for tap installs,
/// and the two sets are compared against each other and against the list panel.
fn bare_formula_names(stdout: &str) -> impl Iterator<Item = String> + '_ {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(short_name)
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

#[cfg(test)]
mod tests {
    use super::{is_newer_version, parse_latest_brewery_version, parse_semver_triplet};

    #[test]
    fn parses_latest_brewery_version_from_cargo_search_output() {
        let stdout = "brewery = \"0.3.3\"    # A fast, friendly TUI for Homebrew\n";
        assert_eq!(
            parse_latest_brewery_version(stdout),
            Some("0.3.3".to_string())
        );
    }

    #[test]
    fn ignores_unrelated_cargo_search_output() {
        let stdout = "othercrate = \"1.2.3\"\n";
        assert_eq!(parse_latest_brewery_version(stdout), None);
    }

    #[test]
    fn parse_semver_triplet_drops_prerelease_suffix() {
        assert_eq!(parse_semver_triplet("1.2.3-beta.1"), (1, 2, 3));
    }

    #[test]
    fn compares_versions_using_semver_ordering() {
        assert!(is_newer_version("0.3.3", "0.3.2"));
        assert!(!is_newer_version("0.3.2", "0.3.2"));
        assert!(!is_newer_version("0.3.1", "0.3.2"));
    }
}
