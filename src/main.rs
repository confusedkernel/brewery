mod app;
mod brew;
mod runtime;
mod theme;
mod ui;

use app::App;
use runtime::event_loop::run_app;
use runtime::terminal::{restore_terminal, setup_terminal};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = App::new();
    let mut terminal = setup_terminal(app.mouse_enabled)?;

    let result = run_app(&mut terminal, app).await;
    restore_terminal(&mut terminal)?;

    result
}
