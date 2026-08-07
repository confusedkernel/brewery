//! Graph-derived answers the UI asks for: why a formula is installed, and what
//! uninstalling it would drag along with it.

use super::*;
use crate::brew::Origin;

pub struct ImpactSummary {
    /// Formulae that would become orphaned, sorted.
    pub orphaned: Vec<String>,
    /// Disk freed by removing the target and its newly orphaned dependencies.
    /// `None` until sizes have been loaded.
    pub freed_kb: Option<u64>,
}

impl ImpactSummary {
    pub fn is_empty(&self) -> bool {
        self.orphaned.is_empty()
    }
}

impl App {
    /// Why `pkg` is on the system. `None` while the graph is still loading.
    pub fn origin_of(&self, pkg: &str) -> Option<Origin> {
        let graph = self.dependency_graph.as_ref()?;
        match graph.explain(pkg) {
            // Casks and uninstalled names have no provenance to report.
            Origin::Unknown => None,
            origin => Some(origin),
        }
    }

    /// What uninstalling `pkg` would orphan. `None` while the graph is still
    /// loading, or for anything that is not an installed formula.
    pub fn removal_impact_of(&self, pkg: &str) -> Option<ImpactSummary> {
        let graph = self.dependency_graph.as_ref()?;
        if !graph.is_known_formula(pkg) {
            return None;
        }

        let orphaned = graph.removal_impact(pkg).orphaned;
        let freed_kb =
            self.total_size_kb(std::iter::once(pkg).chain(orphaned.iter().map(String::as_str)));

        Some(ImpactSummary { orphaned, freed_kb })
    }

    /// A one-line summary for the uninstall confirmation prompt, omitted
    /// entirely when nothing else would be affected.
    pub fn removal_impact_hint(&self, pkg: &str) -> Option<String> {
        let impact = self.removal_impact_of(pkg)?;
        if impact.is_empty() {
            return None;
        }

        let count = impact.orphaned.len();
        let noun = if count == 1 { "orphan" } else { "orphans" };
        Some(match impact.freed_kb {
            Some(kb) => format!("+{count} {noun} (~{})", crate::format::format_size(kb)),
            None => format!("+{count} {noun}"),
        })
    }

    fn total_size_kb<'a>(&self, names: impl Iterator<Item = &'a str>) -> Option<u64> {
        if self.sizes.is_empty() {
            return None;
        }

        let mut total = 0;
        let mut matched = false;
        for name in names {
            if let Some(entry) = self.sizes.iter().find(|entry| entry.name == name) {
                total += entry.size_kb;
                matched = true;
            }
        }

        matched.then_some(total)
    }
}
