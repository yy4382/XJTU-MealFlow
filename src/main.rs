mod actions;
mod app;
mod config;
mod errors;
mod libs;
mod logging;
mod page;
mod tui;

use app::{App, RootState};
use color_eyre::eyre::Result;
use dotenv::dotenv;
use libs::transactions::TransactionManager;
use tokio::sync::mpsc::{self};

async fn run() -> Result<()> {
    let (action_tx, action_rx) = mpsc::unbounded_channel(); // new

    // application state
    let config = crate::config::Config::new()?;
    let mut app = App {
        state: RootState {
            should_quit: false,
            action_tx: action_tx.clone(),
            action_rx,
            manager: TransactionManager::new(Some(config.config.db_path()))?,
            input_mode: false,
            config,
        },
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
