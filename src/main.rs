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
    use cli::{ClapSource, Commands};
    use color_eyre::eyre::Context;
    use libs::transactions::TransactionManager;

    let args = cli::Cli::parse();

    // application state
    let config = crate::config::Config::new(Some(ClapSource::new(&args)))
        .context("Error when loading config")
        .unwrap();

    match &args.command {
        Some(Commands::ClearDb) => {
            let manager = TransactionManager::new(config.config.db_path())
                .context("Error when connecting to Database")?;
            manager.clear_db().context("Error when clearing database")?;
            println!("Database cleared");
            Ok(())
        }
        None => {
            let state = RootState::new(config);
            let mut app = App {
                page: vec![Box::new(page::home::Home {
                    tx: state.clone_tx().into(),
                })],
                state,
                tui: tui::Tui::new()?
                    .tick_rate(args.tick_rate)
                    .frame_rate(args.frame_rate)
                    .into(),
            };

            app.run().await?;
            Ok(())
        }
    }
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
