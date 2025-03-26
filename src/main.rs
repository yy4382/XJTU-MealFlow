mod actions;
mod app;
mod component;
mod config;
#[cfg(not(tarpaulin_include))]
mod errors;
mod libs;
#[cfg(not(tarpaulin_include))]
mod logging;
mod page;
#[cfg(not(tarpaulin_include))]
mod tui;

use app::{App, RootState};
use color_eyre::eyre::Result;
use dotenv::dotenv;

#[cfg(not(tarpaulin_include))]
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
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<()> {
    dotenv().ok();
    errors::init()?;
    logging::init()?;

    let result = run().await;

    result?;

    Ok(())
}
