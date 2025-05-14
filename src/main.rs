mod actions;
mod app;
mod cli;
mod component;
mod config;
mod libs;
mod page;
mod server;
#[cfg(not(tarpaulin_include))]
mod tui;
mod utils;

use actix_web::{HttpServer, middleware::Logger, web};
use app::{App, RootState};
use clap::Parser;
use color_eyre::eyre::Result;
use dotenv::dotenv;
use libs::transactions::TransactionManager;

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
        Some(Commands::Web) => {
            println!("Visit http://localhost:8080 to view the web interface");
            let manager = TransactionManager::new(config.config.db_path())
                .context("Error when connecting to Database")?;
            web_main(manager).await?;
            Ok(())
        }
        None => {
            let state = RootState::new(config);
            let mut app = App::new(
                state,
                tui::Tui::new()?
                    .tick_rate(args.tick_rate)
                    .frame_rate(args.frame_rate)
                    .into(),
            );

            app.run().await?;
            Ok(())
        }
    }
}

async fn web_main(manager: TransactionManager) -> std::io::Result<()> {
    let transaction_manager = web::Data::new(manager);

    HttpServer::new(move || {
        actix_web::App::new()
            .wrap(Logger::default()) // Add Logger middleware
            .app_data(transaction_manager.clone()) // Add TransactionManager to app data
            .configure(server::api::config_routes) // Configure routes from server.rs
            .default_service(web::route().to(server::serve_frontend)) // Serve frontend
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

#[tokio::main]
#[cfg(not(tarpaulin_include))]
async fn main() -> Result<()> {
    dotenv().ok();
    utils::errors::init()?;
    utils::logging::init()?;

    let result = run().await;

    result?;

    Ok(())
}
