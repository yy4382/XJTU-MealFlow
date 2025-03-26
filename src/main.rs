mod actions;
mod app;
mod component;
mod config;
mod errors;
mod libs;
mod logging;
mod page;
mod tui;

use app::{App, RootState};
use color_eyre::eyre::Result;
use dotenv::dotenv;

async fn run() -> Result<()> {
    // application state
    let config = crate::config::Config::new()?;
    let mut app = App {
        state: RootState::new(Some(config.config.db_path())),
        page: Box::new(page::home::Home::default()),
        tui: tui::Tui::new()?.tick_rate(1.0).frame_rate(30.0),
    };

    app.run().await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    errors::init()?;
    logging::init()?;

    let result = run().await;

    result?;

    Ok(())
}
