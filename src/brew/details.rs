#[derive(Clone, Debug)]
pub struct Details {
    pub desc: Option<String>,
    pub homepage: Option<String>,
    pub latest: Option<String>,
    pub installed: Vec<String>,
    pub deps: Option<Vec<String>>,
    pub uses: Option<Vec<String>>,
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
    let output = tokio::process::Command::new("brew")
        .args(["info", "--json=v2", pkg])
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            format!("brew info failed for {pkg}")
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }

    let info: BrewInfo = serde_json::from_slice(&output.stdout)?;
    let formula = info
        .formulae
        .first()
        .ok_or_else(|| anyhow::anyhow!("No formula info for {pkg}"))?;

    let installed = formula
        .installed
        .iter()
        .map(|item| item.version.clone())
        .collect();

    Ok(Details {
        desc: formula.desc.clone(),
        homepage: formula.homepage.clone(),
        latest: formula
            .versions
            .as_ref()
            .and_then(|versions| versions.stable.clone()),
        installed,
        deps: None,
        uses: None,
    })
}

pub async fn fetch_details_full(pkg: &str) -> anyhow::Result<Details> {
    let mut details = fetch_details_basic(pkg).await?;
    let deps = run_brew_lines_async(["deps", "--installed", pkg]).await?;
    let uses = run_brew_lines_async(["uses", "--installed", pkg]).await?;
    details.deps = Some(deps);
    details.uses = Some(uses);
    Ok(details)
}

async fn run_brew_lines_async<const N: usize>(args: [&str; N]) -> anyhow::Result<Vec<String>> {
    let output = tokio::process::Command::new("brew")
        .args(args)
        .output()
        .await?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let message = if stderr.is_empty() {
            "brew command failed".to_string()
        } else {
            stderr
        };
        return Err(anyhow::anyhow!(message));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect())
}
