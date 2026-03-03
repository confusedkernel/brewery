use super::commands::run_brew_command;

#[derive(Clone, Debug, Default)]
pub struct ServiceEntry {
    pub name: String,
    pub status: String,
}

pub async fn fetch_services() -> anyhow::Result<Vec<ServiceEntry>> {
    let result = run_brew_command(&["services", "list"]).await?;
    if !result.success {
        return Ok(Vec::new());
    }

    Ok(parse_services_list(&result.stdout))
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

        entries.push(ServiceEntry {
            name: name.to_string(),
            status: status.to_string(),
        });
    }

    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

#[cfg(test)]
mod tests {
    use super::parse_services_list;

    #[test]
    fn parses_services_list_rows() {
        let stdout = "Name          Status  User File\nredis         started me   ~/Library/LaunchAgents/homebrew.mxcl.redis.plist\npostgresql@14 none\n";
        let parsed = parse_services_list(stdout);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "postgresql@14");
        assert_eq!(parsed[0].status, "none");
        assert_eq!(parsed[1].name, "redis");
        assert_eq!(parsed[1].status, "started");
    }
}
