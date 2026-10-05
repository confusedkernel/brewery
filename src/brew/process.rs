use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use tokio::sync::OnceCell;

/// Resolved once per process. The Cellar cannot move while the app is running,
/// and three separate scans want the path.
static CELLAR: OnceCell<PathBuf> = OnceCell::const_new();

/// The Cellar root, where per-formula keg directories and their install
/// receipts live.
pub(super) async fn cellar_path() -> anyhow::Result<PathBuf> {
    CELLAR
        .get_or_try_init(|| async {
            let output = run_brew(&["--cellar"]).await?;
            ensure_success(&output, "brew --cellar failed")?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(PathBuf::from(stdout.trim()))
        })
        .await
        .cloned()
}

pub(super) async fn run_brew(args: &[&str]) -> anyhow::Result<Output> {
    Ok(run_background("brew", args).await?)
}

/// Runs a background query to completion and collects its output.
///
/// If the request is abandoned first — the app quits and the runtime drops its
/// tasks — the command is killed along with everything it started. Killing
/// `brew` alone is not enough: it fetches through `curl`, which would outlive
/// it as an orphan. On a slow network those orphans used to pile up, every
/// launch adding another handful of downloads competing for the same link.
/// Signalling the process group is not enough either, since Homebrew starts
/// `curl` in a group of its own; see [`KillTreeOnDrop`].
pub(super) async fn run_background<I, S>(program: &str, args: I) -> std::io::Result<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = tokio::process::Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Not `kill_on_drop`: that kills only the command, and does it before
        // the guard below runs, re-parenting the command's children to init
        // so the guard can no longer find them.
        .spawn()?;

    let mut tree = KillTreeOnDrop(child.id());
    let output = child.wait_with_output().await;
    tree.0 = None; // Finished on its own; nothing left to kill.
    output
}

/// Kills a process and all of its descendants when dropped, unless disarmed
/// by clearing it.
///
/// Descendants are found by walking parent links rather than by process
/// group, because Homebrew puts each `curl` it runs in a new group.
struct KillTreeOnDrop(Option<u32>);

impl Drop for KillTreeOnDrop {
    fn drop(&mut self) {
        let Some(root) = self.0.and_then(|pid| libc::pid_t::try_from(pid).ok()) else {
            return;
        };

        // Freeze the root first so it cannot start anything new while its
        // tree is collected. Its descendants get SIGTERM so `curl` can clean
        // up its partial download; the frozen root could not act on that, so
        // it gets SIGKILL.
        signal(root, libc::SIGSTOP);
        for pid in descendants_of(root) {
            signal(pid, libc::SIGTERM);
        }
        signal(root, libc::SIGKILL);
    }
}

fn signal(pid: libc::pid_t, signal: libc::c_int) {
    // SAFETY: `kill` takes no pointers. `pid` is either the root, which has
    // not been reaped and so cannot have been reused, or a descendant seen in
    // the process table a moment ago.
    unsafe {
        libc::kill(pid, signal);
    }
}

/// Every live descendant of `root`, from the process table.
fn descendants_of(root: libc::pid_t) -> Vec<libc::pid_t> {
    let Ok(output) = std::process::Command::new("ps")
        .args(["-A", "-o", "pid=", "-o", "ppid="])
        .output()
    else {
        return Vec::new();
    };

    let links: Vec<(libc::pid_t, libc::pid_t)> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace().map(str::parse);
            Some((fields.next()?.ok()?, fields.next()?.ok()?))
        })
        .collect();

    let mut found = vec![root];
    let mut next = 0;
    while let Some(&parent) = found.get(next) {
        found.extend(
            links
                .iter()
                .filter(|(_, ppid)| *ppid == parent)
                .map(|(pid, _)| *pid),
        );
        next += 1;
    }
    found.remove(0);
    found
}

pub(super) fn ensure_success(output: &Output, fallback: &str) -> anyhow::Result<()> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let message = if stderr.is_empty() {
        fallback.to_string()
    } else {
        stderr
    };
    Err(anyhow::anyhow!(message))
}

pub(super) fn nonempty_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::run_background;

    fn is_alive(pid: libc::pid_t) -> bool {
        // SAFETY: signal 0 only checks that the process exists.
        unsafe { libc::kill(pid, 0) == 0 }
    }

    /// The case that piled up orphans: the command itself has a child (`brew`
    /// running `curl`), and the app quits while both are still running.
    #[tokio::test]
    async fn abandoning_a_query_kills_what_it_started() {
        let pid_file =
            std::env::temp_dir().join(format!("brewery-grandchild-{}", std::process::id()));
        // `set -m` puts the background job in a process group of its own,
        // the way Homebrew runs `curl`, so killing the group would miss it.
        let script = format!(
            "set -m; sleep 30 & echo $! > '{}'; wait",
            pid_file.display()
        );

        let query = run_background("sh", ["-c", script.as_str()]);
        let abandoned = tokio::time::timeout(Duration::from_millis(500), query).await;
        assert!(
            abandoned.is_err(),
            "the query should still have been running"
        );

        let grandchild: libc::pid_t = std::fs::read_to_string(&pid_file)
            .expect("the shell should have recorded its child")
            .trim()
            .parse()
            .expect("pid should be numeric");
        let _ = std::fs::remove_file(&pid_file);

        // The orphaned grandchild is reaped by init, which can take a moment.
        let deadline = Instant::now() + Duration::from_secs(2);
        while is_alive(grandchild) && Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            !is_alive(grandchild),
            "pid {grandchild} outlived the abandoned query"
        );
    }

    #[tokio::test]
    async fn collects_output_from_a_finished_query() {
        let output = run_background("sh", ["-c", "echo hi"])
            .await
            .expect("sh should run");
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "hi\n");
    }
}
