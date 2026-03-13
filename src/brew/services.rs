use super::commands::run_brew_command;
use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ServiceState {
    Running,
    Stopped,
    Error,
}

#[derive(Clone, Debug, Default)]
pub struct ServiceEntry {
    pub name: String,
    pub status: String,
    pub user: Option<String>,
    pub file: Option<String>,
    pub exit_code: Option<i32>,
}

impl ServiceEntry {
    fn state(&self) -> ServiceState {
        if self.status.eq_ignore_ascii_case("started")
            || self.status.eq_ignore_ascii_case("running")
        {
            ServiceState::Running
        } else if self.status.eq_ignore_ascii_case("error")
            || self.exit_code.is_some_and(|code| code != 0)
        {
            ServiceState::Error
        } else {
            ServiceState::Stopped
        }
    }

    pub fn state_label(&self) -> &'static str {
        match self.state() {
            ServiceState::Running => "running",
            ServiceState::Stopped => "stopped",
            ServiceState::Error => "error",
        }
    }

    pub fn is_running(&self) -> bool {
        matches!(self.state(), ServiceState::Running)
    }

    pub fn has_failed(&self) -> bool {
        matches!(self.state(), ServiceState::Error)
    }

    pub fn auto_start_enabled(&self) -> bool {
        self.file
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }
}

#[derive(Deserialize)]
struct ServiceEntryJson {
    name: String,
    status: String,
    user: Option<String>,
    file: Option<String>,
    exit_code: Option<i32>,
}

pub async fn fetch_services() -> anyhow::Result<Vec<ServiceEntry>> {
    let json_result = run_brew_command(&["services", "list", "--json"]).await?;
    if json_result.success
        && let Some(entries) = parse_services_json(&json_result.stdout)
    {
        return Ok(entries);
    }

    let plain_result = run_brew_command(&["services", "list"]).await?;
    if !plain_result.success {
        return Ok(Vec::new());
    }

    Ok(parse_services_list(&plain_result.stdout))
}

fn parse_services_json(stdout: &str) -> Option<Vec<ServiceEntry>> {
    let mut entries: Vec<ServiceEntry> = serde_json::from_str::<Vec<ServiceEntryJson>>(stdout)
        .ok()?
        .into_iter()
        .map(|entry| ServiceEntry {
            name: entry.name,
            status: entry.status,
            user: entry.user,
            file: entry.file,
            exit_code: entry.exit_code,
        })
        .collect();

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    Some(entries)
}

fn parse_services_list(stdout: &str) -> Vec<ServiceEntry> {
    let mut entries = Vec::new();

    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        if line.starts_with("Name ") || line.starts_with("name ") {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(status) = parts.next() else {
            continue;
        };
        let user = parts.next().and_then(normalize_optional_field);
        let file = parts.next().and_then(normalize_optional_field);

        entries.push(ServiceEntry {
            name: name.to_string(),
            status: status.to_string(),
            user,
            file,
            exit_code: None,
        });
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn normalize_optional_field(value: &str) -> Option<String> {
    if value == "-" || value.eq_ignore_ascii_case("none") {
        None
    } else {
        Some(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{ServiceEntry, ServiceState, parse_services_json, parse_services_list};

    #[test]
    fn parses_services_list_rows() {
        let stdout = "Name          Status  User File\nredis         started me   ~/Library/LaunchAgents/homebrew.mxcl.redis.plist\npostgresql@14 none\n";
        let parsed = parse_services_list(stdout);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "postgresql@14");
        assert_eq!(parsed[0].status, "none");
        assert_eq!(parsed[0].user, None);
        assert_eq!(parsed[0].file, None);
        assert_eq!(parsed[1].name, "redis");
        assert_eq!(parsed[1].status, "started");
        assert_eq!(parsed[1].user.as_deref(), Some("me"));
    }

    #[test]
    fn parses_services_json_rows() {
        let stdout = r#"[
            {
                "name": "unbound",
                "status": "none",
                "user": null,
                "file": "/opt/homebrew/opt/unbound/homebrew.mxcl.unbound.plist",
                "exit_code": null
            },
            {
                "name": "redis",
                "status": "started",
                "user": "me",
                "file": "/Users/me/Library/LaunchAgents/homebrew.mxcl.redis.plist",
                "exit_code": 0
            }
        ]"#;

        let parsed = parse_services_json(stdout).expect("json should parse");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "redis");
        assert_eq!(parsed[0].status, "started");
        assert_eq!(parsed[0].exit_code, Some(0));
        assert_eq!(parsed[1].name, "unbound");
        assert_eq!(
            parsed[1].file.as_deref(),
            Some("/opt/homebrew/opt/unbound/homebrew.mxcl.unbound.plist")
        );
    }

    #[test]
    fn marks_non_zero_exit_as_error_state() {
        let entry = ServiceEntry {
            name: "redis".to_string(),
            status: "none".to_string(),
            user: None,
            file: None,
            exit_code: Some(2),
        };

        assert_eq!(entry.state(), ServiceState::Error);
        assert!(entry.has_failed());
        assert_eq!(entry.state_label(), "error");
    }
}
