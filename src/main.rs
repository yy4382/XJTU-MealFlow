mod actions;
mod app;
mod errors;
mod libs;
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
    let mut app = App {
        state: RootState {
            should_quit: false,
            action_tx: action_tx.clone(),
            action_rx: action_rx,
            manager: TransactionManager::new().unwrap(),
            input_mode: false,
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

    let result = run().await;

    result?;

    Ok(())
}
