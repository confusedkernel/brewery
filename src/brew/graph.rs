//! The installed dependency graph, and the two questions it answers:
//! "why do I have this?" and "what does removing this take with it?".
//!
//! Homebrew answers neither directly. `brew uses --installed` gives one level
//! of reverse dependency, and there is no way to preview the orphan cascade of
//! an uninstall.
//!
//! The graph comes from install receipts rather than `brew deps --installed`,
//! because that command reports the dependencies formulae *currently declare*,
//! which drifts from what is actually on disk — a machine can hold `icu4c@76`
//! while every current formula declares `icu4c@78`. Each keg's
//! `INSTALL_RECEIPT.json` instead records `runtime_dependencies` as installed,
//! which is what `brew autoremove` reasons about.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use super::process::cellar_path;

/// One formula's install receipt, reduced to what the graph needs.
#[derive(Clone, Debug, Default)]
pub struct FormulaReceipt {
    /// Whether the user asked for this formula, as opposed to it arriving as a
    /// dependency. Recorded nowhere else that `brew` will tell you about.
    pub on_request: bool,
    /// Dependencies the formula declares itself, used to explain provenance.
    pub direct: Vec<String>,
    /// The full runtime closure as installed, used for orphan analysis.
    pub runtime: Vec<String>,
}

pub type Receipts = HashMap<String, FormulaReceipt>;

#[derive(Default)]
pub struct DependencyGraph {
    receipts: Receipts,
    /// Reverse of the direct edges.
    dependents: HashMap<String, Vec<String>>,
    on_request: Vec<String>,
    /// Formulae `brew autoremove` would remove right now, computed once so
    /// [`DependencyGraph::removal_impact`] reports only *newly* created orphans.
    already_orphaned: HashSet<String>,
}

/// Why a formula is present on the system.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Explicitly installed.
    OnRequest,
    /// Pulled in as a dependency. `path` runs from the formula up to the
    /// explicitly installed formula that needs it, exclusive of the formula
    /// itself.
    RequiredBy {
        path: Vec<String>,
        direct_dependents: usize,
    },
    /// Nothing explicitly installed needs this; `brew autoremove` would take it.
    Orphaned,
    /// No receipt, so not an installed formula (a cask, or not installed).
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemovalImpact {
    /// Formulae that become orphaned as a result, sorted, excluding the target
    /// and anything already orphaned beforehand.
    pub orphaned: Vec<String>,
}

pub struct GraphMessage {
    pub result: anyhow::Result<DependencyGraph>,
}

impl DependencyGraph {
    pub fn new(receipts: Receipts) -> Self {
        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        for (name, receipt) in &receipts {
            for dep in &receipt.direct {
                dependents
                    .entry(dep.clone())
                    .or_default()
                    .push(name.clone());
            }
        }
        for list in dependents.values_mut() {
            list.sort();
        }

        let mut on_request: Vec<String> = receipts
            .iter()
            .filter(|(_, receipt)| receipt.on_request)
            .map(|(name, _)| name.clone())
            .collect();
        on_request.sort();

        let mut graph = Self {
            receipts,
            dependents,
            on_request,
            already_orphaned: HashSet::new(),
        };

        graph.already_orphaned = graph.orphans_without(None);
        graph
    }

    pub fn is_known_formula(&self, pkg: &str) -> bool {
        self.receipts.contains_key(pkg)
    }

    pub fn direct_dependents(&self, pkg: &str) -> &[String] {
        self.dependents.get(pkg).map_or(&[], Vec::as_slice)
    }

    /// Why `pkg` is installed, as a chain up to an explicitly installed
    /// formula. Breadth-first, so the chain is the shortest one and therefore
    /// the most direct explanation rather than an arbitrary path.
    pub fn explain(&self, pkg: &str) -> Origin {
        let Some(receipt) = self.receipts.get(pkg) else {
            return Origin::Unknown;
        };

        if receipt.on_request {
            return Origin::OnRequest;
        }

        if self.already_orphaned.contains(pkg) {
            return Origin::Orphaned;
        }

        let direct_dependents = self.direct_dependents(pkg).len();
        let mut parents: HashMap<&str, &str> = HashMap::new();
        let mut seen: HashSet<&str> = HashSet::from([pkg]);
        let mut queue: VecDeque<&str> = VecDeque::from([pkg]);

        while let Some(current) = queue.pop_front() {
            for dependent in self.direct_dependents(current) {
                let dependent = dependent.as_str();
                if !seen.insert(dependent) {
                    continue;
                }
                parents.insert(dependent, current);

                if self.receipts.get(dependent).is_some_and(|r| r.on_request) {
                    return Origin::RequiredBy {
                        path: trace_path(&parents, pkg, dependent),
                        direct_dependents,
                    };
                }

                queue.push_back(dependent);
            }
        }

        // Not orphaned, so some requested formula's runtime closure holds it
        // even though no chain of *declared* edges reaches it. Name that
        // formula rather than leaving the chain empty.
        let requester = self.on_request.iter().find(|name| {
            self.receipts
                .get(*name)
                .is_some_and(|receipt| receipt.runtime.iter().any(|dep| dep == pkg))
        });

        Origin::RequiredBy {
            path: requester.cloned().into_iter().collect(),
            direct_dependents,
        }
    }

    /// What uninstalling `target` would orphan, beyond what is already garbage.
    pub fn removal_impact(&self, target: &str) -> RemovalImpact {
        if !self.receipts.contains_key(target) {
            return RemovalImpact::default();
        }

        let mut orphaned: Vec<String> = self
            .orphans_without(Some(target))
            .into_iter()
            .filter(|name| name != target && !self.already_orphaned.contains(name))
            .collect();
        orphaned.sort();

        RemovalImpact { orphaned }
    }

    /// Formulae no explicitly installed formula needs, optionally pretending
    /// `excluded` has been uninstalled. This mirrors `brew autoremove`: keep
    /// everything requested, plus everything in a requested formula's runtime
    /// closure, and treat the rest as garbage.
    fn orphans_without(&self, excluded: Option<&str>) -> HashSet<String> {
        let mut needed: HashSet<&str> = HashSet::new();

        for name in &self.on_request {
            if Some(name.as_str()) == excluded {
                continue;
            }

            needed.insert(name.as_str());
            if let Some(receipt) = self.receipts.get(name) {
                needed.extend(receipt.runtime.iter().map(String::as_str));
            }
        }

        self.receipts
            .keys()
            .filter(|name| Some(name.as_str()) != excluded)
            .filter(|name| !needed.contains(name.as_str()))
            .cloned()
            .collect()
    }
}

/// Walks parent links back from an ancestor to `start`, returning the chain
/// from `start`'s dependent up to and including `ancestor`.
fn trace_path(parents: &HashMap<&str, &str>, start: &str, ancestor: &str) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = ancestor;

    while current != start {
        path.push(current.to_string());
        match parents.get(current) {
            Some(parent) => current = parent,
            None => break,
        }
    }

    path.reverse();
    path
}

/// Receipts record tap-qualified names (`org/tap/pkg`); everything else in the
/// app uses the bare name.
pub(super) fn short_name(name: &str) -> String {
    name.rsplit('/').next().unwrap_or(name).trim().to_string()
}

pub async fn fetch_dependency_graph() -> anyhow::Result<DependencyGraph> {
    let cellar = cellar_path().await?;
    let receipts = tokio::task::spawn_blocking(move || read_receipts(&cellar)).await??;
    Ok(DependencyGraph::new(receipts))
}

#[derive(serde::Deserialize)]
struct InstallReceipt {
    #[serde(default)]
    installed_on_request: bool,
    #[serde(default)]
    runtime_dependencies: Vec<RuntimeDependency>,
}

#[derive(serde::Deserialize)]
struct RuntimeDependency {
    full_name: Option<String>,
    #[serde(default)]
    declared_directly: bool,
}

fn read_receipts(cellar: &Path) -> anyhow::Result<Receipts> {
    let mut receipts = Receipts::new();

    let Ok(entries) = std::fs::read_dir(cellar) else {
        return Ok(receipts);
    };

    for entry in entries.flatten() {
        // Follows symlinks so alias formulae (`rustfmt` -> `rust`) get a
        // receipt too. Without one they appear in the list panel but have no
        // provenance to show.
        if !entry.path().is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }

        let Ok(versions) = std::fs::read_dir(entry.path()) else {
            continue;
        };

        // A formula can have several kegs installed. Treat it as requested if
        // any of them was, and take the richest dependency list on offer.
        let mut merged: Option<FormulaReceipt> = None;

        for version in versions.flatten() {
            let Ok(bytes) = std::fs::read(version.path().join("INSTALL_RECEIPT.json")) else {
                continue;
            };
            let Ok(receipt) = serde_json::from_slice::<InstallReceipt>(&bytes) else {
                // Keg exists but the receipt is unreadable; still an installed
                // formula, so record it with no edges.
                merged.get_or_insert_with(FormulaReceipt::default);
                continue;
            };

            let mut direct = Vec::new();
            let mut runtime = Vec::new();
            for dep in &receipt.runtime_dependencies {
                let Some(full_name) = dep.full_name.as_deref() else {
                    continue;
                };
                let dep_name = short_name(full_name);
                if dep.declared_directly {
                    direct.push(dep_name.clone());
                }
                runtime.push(dep_name);
            }

            let slot = merged.get_or_insert_with(FormulaReceipt::default);
            slot.on_request |= receipt.installed_on_request;
            if runtime.len() > slot.runtime.len() {
                slot.direct = direct;
                slot.runtime = runtime;
            }
        }

        if let Some(receipt) = merged {
            receipts.insert(name, receipt);
        }
    }

    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds receipts from `(name, on_request, direct deps)`, deriving each
    /// runtime closure from the direct edges the way Homebrew records it.
    fn graph(entries: &[(&str, bool, &[&str])]) -> DependencyGraph {
        let direct: HashMap<String, Vec<String>> = entries
            .iter()
            .map(|(name, _, deps)| {
                (
                    name.to_string(),
                    deps.iter().map(|dep| dep.to_string()).collect(),
                )
            })
            .collect();

        let receipts = entries
            .iter()
            .map(|(name, on_request, deps)| {
                let mut runtime = Vec::new();
                let mut queue: VecDeque<String> = deps.iter().map(|dep| dep.to_string()).collect();
                while let Some(current) = queue.pop_front() {
                    if runtime.contains(&current) {
                        continue;
                    }
                    if let Some(next) = direct.get(&current) {
                        queue.extend(next.iter().cloned());
                    }
                    runtime.push(current);
                }

                (
                    name.to_string(),
                    FormulaReceipt {
                        on_request: *on_request,
                        direct: deps.iter().map(|dep| dep.to_string()).collect(),
                        runtime,
                    },
                )
            })
            .collect();

        DependencyGraph::new(receipts)
    }

    /// `imagemagick` was asked for and pulls in `libpng`, which needs `zlib`.
    /// `wget` was asked for and stands alone.
    fn sample() -> DependencyGraph {
        graph(&[
            ("imagemagick", true, &["libpng"]),
            ("libpng", false, &["zlib"]),
            ("zlib", false, &[]),
            ("wget", true, &[]),
        ])
    }

    #[test]
    fn builds_reverse_edges_from_direct_dependencies() {
        let graph = sample();
        assert_eq!(graph.direct_dependents("libpng"), ["imagemagick"]);
        assert_eq!(graph.direct_dependents("zlib"), ["libpng"]);
        assert!(graph.direct_dependents("imagemagick").is_empty());
    }

    #[test]
    fn explains_an_explicitly_installed_formula() {
        assert_eq!(sample().explain("imagemagick"), Origin::OnRequest);
    }

    #[test]
    fn explains_a_dependency_by_chain_to_its_requester() {
        let graph = sample();

        assert_eq!(
            graph.explain("libpng"),
            Origin::RequiredBy {
                path: vec!["imagemagick".to_string()],
                direct_dependents: 1,
            }
        );
        assert_eq!(
            graph.explain("zlib"),
            Origin::RequiredBy {
                path: vec!["libpng".to_string(), "imagemagick".to_string()],
                direct_dependents: 1,
            }
        );
    }

    #[test]
    fn reports_the_shortest_explanation_when_several_exist() {
        let graph = graph(&[
            ("direct", true, &["zlib"]),
            ("deep", true, &["middle"]),
            ("middle", false, &["zlib"]),
            ("zlib", false, &[]),
        ]);

        let Origin::RequiredBy {
            path,
            direct_dependents,
        } = graph.explain("zlib")
        else {
            panic!("zlib should be explained as a dependency");
        };
        assert_eq!(path, vec!["direct".to_string()]);
        assert_eq!(direct_dependents, 2);
    }

    #[test]
    fn reports_a_formula_nothing_requested_as_orphaned() {
        let graph = graph(&[("stale", false, &[])]);
        assert_eq!(graph.explain("stale"), Origin::Orphaned);
        assert!(graph.already_orphaned.contains("stale"));
    }

    #[test]
    fn treats_casks_and_unknown_names_as_unknown() {
        let graph = sample();
        assert_eq!(graph.explain("obsidian"), Origin::Unknown);
        assert_eq!(graph.explain("not-installed"), Origin::Unknown);
    }

    #[test]
    fn removing_a_requester_orphans_its_exclusive_dependencies() {
        let impact = sample().removal_impact("imagemagick");
        assert_eq!(
            impact.orphaned,
            vec!["libpng".to_string(), "zlib".to_string()]
        );
    }

    #[test]
    fn keeps_dependencies_that_something_else_still_needs() {
        let graph = graph(&[
            ("imagemagick", true, &["libpng"]),
            ("ffmpeg", true, &["libpng"]),
            ("libpng", false, &[]),
        ]);

        assert!(graph.removal_impact("imagemagick").orphaned.is_empty());
    }

    /// The case that makes receipts worth reading: on this machine 63 formulae
    /// were installed on request but only 53 are leaves, so ten are both
    /// requested *and* depended on. Those must never be reported as collateral.
    #[test]
    fn never_orphans_an_explicitly_installed_dependency() {
        let graph = graph(&[("imagemagick", true, &["jpeg"]), ("jpeg", true, &[])]);

        assert!(
            graph.removal_impact("imagemagick").orphaned.is_empty(),
            "jpeg was installed on request, so removing imagemagick must not claim it"
        );
    }

    #[test]
    fn excludes_formulae_that_were_already_orphaned() {
        let graph = graph(&[
            ("imagemagick", true, &["libpng"]),
            ("libpng", false, &[]),
            ("stale", false, &[]),
        ]);

        assert!(graph.already_orphaned.contains("stale"));
        assert_eq!(graph.removal_impact("imagemagick").orphaned, ["libpng"]);
    }

    #[test]
    fn cascades_through_a_chain_of_exclusive_dependencies() {
        let graph = graph(&[
            ("top", true, &["a"]),
            ("a", false, &["b"]),
            ("b", false, &["c"]),
            ("c", false, &[]),
        ]);

        assert_eq!(
            graph.removal_impact("top").orphaned,
            ["a".to_string(), "b".to_string(), "c".to_string()]
        );
    }

    #[test]
    fn removing_a_shared_requester_orphans_only_what_it_alone_held() {
        let graph = graph(&[
            ("one", true, &["shared", "exclusive"]),
            ("two", true, &["shared"]),
            ("shared", false, &[]),
            ("exclusive", false, &[]),
        ]);

        assert_eq!(graph.removal_impact("one").orphaned, ["exclusive"]);
    }

    /// Orphan analysis must follow the recorded runtime closure, not the
    /// direct edges, since the two can disagree on a long-lived install.
    #[test]
    fn honors_the_recorded_runtime_closure_over_direct_edges() {
        let receipts = Receipts::from([
            (
                "app".to_string(),
                FormulaReceipt {
                    on_request: true,
                    direct: vec![],
                    // Nothing is declared, but this keg was built against it.
                    runtime: vec!["hidden".to_string()],
                },
            ),
            ("hidden".to_string(), FormulaReceipt::default()),
        ]);

        let graph = DependencyGraph::new(receipts);
        assert!(
            graph.already_orphaned.is_empty(),
            "a formula in a requested keg's runtime closure is not garbage"
        );
        assert_eq!(graph.removal_impact("app").orphaned, ["hidden"]);

        // No declared edge reaches `hidden`, so the explanation has to fall
        // back to whichever requested formula's closure contains it.
        assert_eq!(
            graph.explain("hidden"),
            Origin::RequiredBy {
                path: vec!["app".to_string()],
                direct_dependents: 0,
            }
        );
    }

    /// Validates the orphan analysis against Homebrew itself: what we consider
    /// already-garbage must be exactly what `brew autoremove` would remove.
    /// Hits the real system, so it is opt-in:
    /// `cargo test -- --ignored --nocapture`
    #[tokio::test]
    #[ignore = "requires a local Homebrew installation"]
    async fn baseline_orphans_agree_with_brew_autoremove() {
        use super::super::process::run_brew;

        let graph = fetch_dependency_graph()
            .await
            .expect("dependency graph should load");

        let output = run_brew(&["autoremove", "--dry-run"])
            .await
            .expect("autoremove dry-run should run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let expected: HashSet<String> = stdout
            .lines()
            .skip_while(|line| !line.contains("Would remove"))
            .skip(1)
            .flat_map(|line| line.split_whitespace())
            .map(str::to_string)
            .collect();

        println!(
            "{} formulae, {} on request, {} orphaned",
            graph.receipts.len(),
            graph.on_request.len(),
            graph.already_orphaned.len(),
        );

        assert_eq!(
            graph.already_orphaned, expected,
            "orphan analysis disagrees with brew autoremove",
        );
    }
}
