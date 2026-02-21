use super::process::{ensure_success, nonempty_lines, run_brew};

#[derive(Clone, Debug)]
pub struct Details {
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub latest: Option<String>,
    pub installed: Vec<String>,
    pub deps: Option<Vec<String>>,
    pub uses: Option<Vec<String>>,
    pub artifacts: Option<Vec<String>>,
}

#[derive(Clone, Copy, Debug)]
pub enum DetailsLoad {
    Basic,
    Full,
}

pub struct DetailsMessage {
    pub pkg: String,
    pub load: DetailsLoad,
    pub result: anyhow::Result<Details>,
}

#[derive(serde::Deserialize)]
struct BrewInfo {
    #[serde(default)]
    formulae: Vec<FormulaInfo>,
}

#[derive(serde::Deserialize)]
struct FormulaInfo {
    desc: Option<String>,
    homepage: Option<String>,
    versions: Option<FormulaVersions>,
    #[serde(default)]
    installed: Vec<InstalledInfo>,
}

#[derive(serde::Deserialize)]
struct FormulaVersions {
    stable: Option<String>,
}

#[derive(serde::Deserialize)]
struct InstalledInfo {
    version: String,
}

pub async fn fetch_details_basic(pkg: &str) -> anyhow::Result<Details> {
    let args = ["info", "--json=v2", pkg];
    let output = run_brew(&args).await?;
    let fallback = format!("brew info failed for {pkg}");
    ensure_success(&output, &fallback)?;

    let info: BrewInfo = serde_json::from_slice(&output.stdout)?;
    if let Some(formula) = info.formulae.first() {
        let installed = formula
            .installed
            .iter()
            .map(|item| item.version.clone())
            .collect();

        return Ok(Details {
            desc: formula.desc.clone(),
            homepage: formula.homepage.clone(),
            latest: formula
                .versions
                .as_ref()
                .and_then(|versions| versions.stable.clone()),
            installed,
            deps: None,
            uses: None,
            artifacts: None,
        });
    }

    let doc: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    parse_cask_details(&doc).ok_or_else(|| anyhow::anyhow!("No package info for {pkg}"))
}

pub async fn fetch_details_full(pkg: &str) -> anyhow::Result<Details> {
    let mut details = fetch_details_basic(pkg).await?;
    if details.artifacts.is_some() {
        return Ok(details);
    }

    let deps = run_brew_lines_async(["deps", "--installed", pkg]).await?;
    let uses = run_brew_lines_async(["uses", "--installed", pkg]).await?;
    details.deps = Some(deps);
    details.uses = Some(uses);
    Ok(details)
}

async fn run_brew_lines_async<const N: usize>(args: [&str; N]) -> anyhow::Result<Vec<String>> {
    let output = run_brew(&args).await?;
    ensure_success(&output, "brew command failed")?;
    Ok(nonempty_lines(&output.stdout))
}

fn parse_cask_details(doc: &serde_json::Value) -> Option<Details> {
    let cask = doc.get("casks")?.as_array()?.first()?;

    let desc = cask
        .get("desc")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let homepage = cask
        .get("homepage")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let latest = cask
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    let installed = cask
        .get("installed")
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    entry
                        .get("version")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| entry.as_str())
                        .map(str::to_string)
                })
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    let artifacts = cask
        .get("artifacts")
        .and_then(serde_json::Value::as_array)
        .map(|entries| {
            entries
                .iter()
                .filter_map(format_cask_artifact)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();

    Some(Details {
        desc,
        homepage,
        latest,
        installed,
        deps: None,
        uses: None,
        artifacts: Some(artifacts),
    })
}

fn format_cask_artifact(value: &serde_json::Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }

    if let Some(array) = value.as_array() {
        let kind = array
            .first()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("artifact");
        let label = array
            .get(1)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if label.is_empty() {
            return Some(kind.to_string());
        }
        return Some(format!("{kind}: {label}"));
    }

    if let Some(object) = value.as_object() {
        let key = object.keys().next()?.to_string();
        let first_value = object
            .values()
            .next()
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if first_value.is_empty() {
            return Some(key);
        }
        return Some(format!("{key}: {first_value}"));
    }

    None
}
