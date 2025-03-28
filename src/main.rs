mod actions;
mod app;
mod cli;
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
mod utils;

use app::{App, RootState};
use clap::Parser;
use color_eyre::eyre::Result;
use dotenv::dotenv;

#[cfg(not(tarpaulin_include))]
async fn run() -> Result<()> {
    use cli::ClapSource;
    use color_eyre::eyre::Context;

    let args = cli::Cli::parse();

    // application state
    let config = crate::config::Config::new(Some(ClapSource::new(&args)))
        .context("Error when loading config").unwrap();
    let mut app = App {
        state: RootState::new(Some(config.config.db_path())),
        page: Box::new(page::home::Home::default()),
        tui: tui::Tui::new()?
            .tick_rate(args.tick_rate)
            .frame_rate(args.frame_rate),
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
