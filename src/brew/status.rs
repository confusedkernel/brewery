use super::services::{ServiceEntry, fetch_services};
use crate::brew::commands::run_background_command;
use crate::brew::graph::short_name;
use crate::brew::run_brew_command;
use crate::format::first_nonempty_line;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

const LATEST_BREWERY_CACHE_TTL: Duration = Duration::from_secs(30 * 60);

/// The last `cargo search` answer and when it was asked, so the five-minute
/// background status refresh does not hit crates.io every time.
static LATEST_BREWERY_CACHE: Mutex<Option<(SystemTime, Option<String>)>> = Mutex::new(None);

/// One row of `brew outdated`, with the version jump an upgrade would make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutdatedEntry {
    pub name: String,
    pub is_cask: bool,
    pub installed_versions: Vec<String>,
    pub current_version: String,
    pub pinned: bool,
}

impl OutdatedEntry {
    /// `1.2.0 → 1.3.0`, with the arrow left to the caller so the ASCII
    /// fallback can be honoured.
    pub fn installed_label(&self) -> String {
        if self.installed_versions.is_empty() {
            return "?".to_string();
        }
        self.installed_versions.join(", ")
    }
}

#[derive(Clone, Debug, Default)]
pub struct StatusSnapshot {
    pub outdated_count: Option<usize>,
    /// Outdated leaves and casks — the things the user asked for — which is
    /// what the Outdated tab lists and what "upgrade all" counts.
    pub outdated_packages: Vec<String>,
    /// Every outdated formula and cask, leaves or not, with versions. Feeds
    /// the list markers and the version columns in the Outdated tab.
    pub outdated: Vec<OutdatedEntry>,
    /// `brew list --pinned`: formulae held back from `brew upgrade`.
    pub pinned: Vec<String>,
    pub brew_version: Option<String>,
    pub brew_update_status: Option<String>,
    pub last_brew_update_secs_ago: Option<u64>,
    pub brewery_latest_version: Option<String>,
    pub brewery_update_available: bool,
    pub services: Vec<ServiceEntry>,
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
        pinned_result,
        services_result,
        latest_brewery_version,
    ) = tokio::join!(
        run_brew_command(&["--version"]),
        fetch_leaf_set(known_leaves),
        run_brew_command(&["--repository"]),
        run_brew_command(&["--repository", "homebrew/core"]),
        run_brew_command(&["outdated", "--json=v2"]),
        run_brew_command(&["list", "--pinned"]),
        fetch_services(),
        fetch_latest_brewery_version_cached(),
    );

    // Process version result
    if let Ok(result) = version_result
        && result.success
    {
        status.brew_version = first_nonempty_line(&result.stdout).map(str::to_string);
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

    if let Ok(result) = pinned_result
        && result.success
    {
        status.pinned = bare_formula_names(&result.stdout).collect();
    }

    if let Ok(result) = outdated_result
        && result.success
    {
        status.outdated = parse_outdated_json(&result.stdout).unwrap_or_default();
        let packages: Vec<String> = status
            .outdated
            .iter()
            .filter(|entry| entry.is_cask || leaf_set.contains(&entry.name))
            .map(|entry| entry.name.clone())
            .collect();
        status.outdated_count = Some(packages.len());
        status.outdated_packages = packages;
    }

    Ok(status)
}

#[derive(serde::Deserialize)]
struct OutdatedJson {
    #[serde(default)]
    formulae: Vec<OutdatedEntryJson>,
    #[serde(default)]
    casks: Vec<OutdatedEntryJson>,
}

#[derive(serde::Deserialize)]
struct OutdatedEntryJson {
    name: String,
    #[serde(default)]
    installed_versions: Vec<String>,
    #[serde(default)]
    current_version: String,
    #[serde(default)]
    pinned: bool,
}

/// `brew outdated --json=v2` covers formulae and casks in one call, at the
/// same cost as the plain formula listing, and is the only form that reports
/// the version an upgrade would land on.
fn parse_outdated_json(stdout: &str) -> Option<Vec<OutdatedEntry>> {
    let doc: OutdatedJson = serde_json::from_str(stdout).ok()?;

    let convert = |entry: OutdatedEntryJson, is_cask: bool| OutdatedEntry {
        name: short_name(&entry.name),
        is_cask,
        installed_versions: entry.installed_versions,
        current_version: entry.current_version,
        pinned: entry.pinned,
    };

    let mut entries: Vec<OutdatedEntry> = doc
        .formulae
        .into_iter()
        .map(|entry| convert(entry, false))
        .chain(doc.casks.into_iter().map(|entry| convert(entry, true)))
        .collect();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Some(entries)
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
    let version = line.split('"').nth(1)?.trim();
    (!version.is_empty()).then(|| version.to_string())
}

fn is_newer_version(latest: &str, current: &str) -> bool {
    parse_semver_triplet(latest) > parse_semver_triplet(current)
}

fn parse_semver_triplet(version: &str) -> (u64, u64, u64) {
    let core = version.split('-').next().unwrap_or(version);
    let mut parts = core.split('.').map(|part| part.parse().unwrap_or(0));
    let mut next = || parts.next().unwrap_or(0);
    (next(), next(), next())
}

async fn fetch_latest_brewery_version_cached() -> Option<String> {
    if let Ok(cache) = LATEST_BREWERY_CACHE.lock()
        && let Some((checked_at, version)) = cache.as_ref()
        && checked_at
            .elapsed()
            .is_ok_and(|age| age <= LATEST_BREWERY_CACHE_TTL)
    {
        return version.clone();
    }

    let fetched_version =
        match run_background_command("cargo", &["search", "brewery", "--limit", "1"]).await {
            Ok(result) if result.success => parse_latest_brewery_version(&result.stdout),
            _ => None,
        };

    if let Ok(mut cache) = LATEST_BREWERY_CACHE.lock() {
        *cache = Some((SystemTime::now(), fetched_version.clone()));
    }
    fetched_version
}

#[cfg(test)]
mod tests {
    use super::{
        is_newer_version, parse_latest_brewery_version, parse_outdated_json, parse_semver_triplet,
    };

    #[test]
    fn parses_outdated_formulae_and_casks_with_versions() {
        let stdout = r#"{
            "formulae": [
                {
                    "name": "homebrew/core/btop",
                    "installed_versions": ["1.4.6"],
                    "current_version": "1.4.7",
                    "pinned": false,
                    "pinned_version": null
                },
                {
                    "name": "node",
                    "installed_versions": ["22.1.0", "24.0.0"],
                    "current_version": "24.2.0",
                    "pinned": true,
                    "pinned_version": "24.0.0"
                }
            ],
            "casks": [
                {
                    "name": "obsidian",
                    "installed_versions": ["1.12.4"],
                    "current_version": "1.13.7",
                    "pinned": false,
                    "pinned_version": null
                }
            ]
        }"#;

        let entries = parse_outdated_json(stdout).expect("json should parse");
        assert_eq!(entries.len(), 3);

        let btop = &entries[0];
        assert_eq!(btop.name, "btop", "tap prefix should be stripped");
        assert!(!btop.is_cask);
        assert_eq!(btop.installed_label(), "1.4.6");
        assert_eq!(btop.current_version, "1.4.7");

        let node = &entries[1];
        assert!(node.pinned);
        assert_eq!(node.installed_label(), "22.1.0, 24.0.0");

        let obsidian = &entries[2];
        assert!(obsidian.is_cask);
    }

    #[test]
    fn rejects_non_json_outdated_output() {
        assert!(
            parse_outdated_json(
                "btop
node
"
            )
            .is_none()
        );
    }

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

    /// The JSON form of `brew outdated` has to agree with the plain listing,
    /// since the plain one is what the app used to trust. Hits the real
    /// system, so it is opt-in: `cargo test -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "requires a local Homebrew installation"]
    async fn outdated_json_agrees_with_plain_listing() {
        use super::{bare_formula_names, fetch_status, run_brew_command};

        let status = fetch_status(None).await.expect("status should load");

        let plain = run_brew_command(&["outdated", "--formula"])
            .await
            .expect("brew outdated should run");
        let mut expected: Vec<String> = bare_formula_names(&plain.stdout).collect();
        expected.sort();

        let mut parsed: Vec<String> = status
            .outdated
            .iter()
            .filter(|entry| !entry.is_cask)
            .map(|entry| entry.name.clone())
            .collect();
        parsed.sort();

        println!(
            "{} outdated formulae, {} outdated casks, {} pinned, {} listed in the tab",
            parsed.len(),
            status.outdated.iter().filter(|entry| entry.is_cask).count(),
            status.pinned.len(),
            status.outdated_packages.len(),
        );
        assert_eq!(
            parsed, expected,
            "JSON and plain outdated listings disagree"
        );
        assert!(
            status
                .outdated
                .iter()
                .all(|entry| !entry.current_version.is_empty()),
            "every outdated entry should carry the version an upgrade lands on"
        );
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
