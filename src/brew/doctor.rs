//! `brew doctor`, kept apart from the rest of the status snapshot.
//!
//! It is the slowest thing the app runs — around 1.3s against 400-500ms for
//! every other status command — and nothing else in the panel depends on it.
//! Joined into the status fetch it set the floor for the whole panel; on its
//! own channel the panel lands at the cost of its second-slowest command and
//! the diagnostics fill in when they are ready.

use crate::brew::run_brew_command;

/// How many warnings to keep. `brew doctor` can produce a wall of them, and the
/// tab shows a handful of lines.
const MAX_REPORTED_ISSUES: usize = 5;

#[derive(Clone, Debug, Default)]
pub struct DoctorReport {
    pub ok: bool,
    pub issues: Vec<String>,
}

pub async fn fetch_doctor() -> anyhow::Result<DoctorReport> {
    let result = run_brew_command(&["doctor"]).await?;

    // A clean bill of health exits zero; anything to report exits non-zero and
    // lands on stderr, though older versions use stdout.
    if result.success {
        return Ok(DoctorReport {
            ok: true,
            issues: Vec::new(),
        });
    }

    let output = if result.stderr.is_empty() {
        &result.stdout
    } else {
        &result.stderr
    };

    Ok(DoctorReport {
        ok: false,
        issues: parse_issues(output),
    })
}

fn parse_issues(output: &str) -> Vec<String> {
    output
        .lines()
        .filter(|line| line.starts_with("Warning:") || line.starts_with("Error:"))
        .take(MAX_REPORTED_ISSUES)
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{MAX_REPORTED_ISSUES, parse_issues};

    #[test]
    fn keeps_only_warning_and_error_lines() {
        let output = "Please note that these warnings are just used\n\
                      Warning: Some installed formulae are deprecated.\n\
                      \n\
                      Error: Cannot link openssl@3\n";

        assert_eq!(
            parse_issues(output),
            [
                "Warning: Some installed formulae are deprecated.",
                "Error: Cannot link openssl@3",
            ]
        );
    }

    #[test]
    fn caps_a_wall_of_warnings() {
        let output = (0..20)
            .map(|i| format!("Warning: issue {i}\n"))
            .collect::<String>();

        assert_eq!(parse_issues(&output).len(), MAX_REPORTED_ISSUES);
    }
}
