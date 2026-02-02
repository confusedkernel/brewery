mod app;
mod brew;
mod runtime;
mod theme;
mod ui;

use runtime::event_loop::run_app;
use runtime::terminal::{restore_terminal, setup_terminal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;

    let result = run_app(&mut terminal).await;
    restore_terminal(&mut terminal)?;

    result
}
