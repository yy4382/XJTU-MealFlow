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
use chrono::{DateTime, FixedOffset, NaiveDate};

#[cfg(not(tarpaulin_include))]
async fn run() -> Result<()> {
    use cli::{ClapSource, Commands};
    use color_eyre::eyre::Context;
    use libs::transactions::TransactionManager;

    // Helper function to parse date string with time set to beginning of day
    fn parse_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context(format!("Failed to parse date: {}", date_str))?;
        
        // Create datetime with 00:00:00 time
        let naive_datetime = date.and_hms_opt(0, 0, 0).unwrap();
        
        // Add UTC+8 timezone
        let tz_offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC+8
        let datetime = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, tz_offset);
        
        Ok(datetime)
    }

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
            println!("Visit http://localhost:8086 to view the web interface");
            let manager = TransactionManager::new(config.config.db_path())
                .context("Error when connecting to Database")?;
            web_main(manager).await?;
            Ok(())
        }
        Some(Commands::ExportCsv { output, merchant, min_amount, max_amount, time_start, time_end }) => {
            let manager = TransactionManager::new(config.config.db_path())
                .context("Error when connecting to Database")?;
            
            // Filter options init
            let mut filter_opt = libs::transactions::FilterOptions::default();
            
            // (1) Filter by merchant if provided
            if let Some(m) = merchant {
                filter_opt = filter_opt.merchant(m);
            }
            
            // (2) Filter by amount if provided
            // - When user provides min_amount or max_amount, convert them to negative values
            // - input: query for [10.00, 100.00]
            // - actual database query: [-100.00, -10.00]

            // 10.00 -> -10.00
            if let Some(min) = min_amount {
                // Ensure user input is a positive number
                if *min < 0.0 {
                    println!("Warning: min_amount should be positive, converting absolute value");
                }
                // Logic: convert positive min_amount to negative for database query
                let db_max = -min.abs();
                filter_opt = filter_opt.max(db_max);
            }
            
            // 100.00 -> -100.00
            if let Some(max) = max_amount {
                // Ensure user input is a positive number
                if *max < 0.0 {
                    println!("Warning: max_amount should be positive, converting absolute value");
                }
                // Logic: convert positive max_amount to negative for database query
                let db_min = -max.abs();
                filter_opt = filter_opt.min(db_min);
            }

            // (3) Filter by date range if provided 
            // - Input date format: YYYY-MM-DD
            // - Format: [Start Date, End Date)
            // start date
            if let Some(start_str) = time_start {
                match parse_date(&start_str) {
                    Ok(start_date) => {
                        // 2022-12-09 0:00:00
                        println!("Setting start date to: {}", start_date);
                        filter_opt = filter_opt.start(start_date);
                    },
                    Err(e) => {
                        println!("Warning: Invalid start date format '{}': {}", start_str, e);
                        println!("Date format should be YYYY-MM-DD");
                    }
                }
            }

            // end date
            if let Some(end_str) = time_end {
                match parse_date(&end_str) {
                    Ok(end_date) => {
                        // 2022-12-25 0:00:00
                        println!("Setting start date to: {}", end_date);
                        filter_opt = filter_opt.end(end_date);
                    },
                    Err(e) => {
                        println!("Warning: Invalid start date format '{}': {}", end_str, e);
                        println!("Date format should be YYYY-MM-DD");
                    }
                }
            }

            let output_path = output.clone().unwrap_or_else(|| "transactions_export.csv".to_string());
            
            if merchant.is_some() || min_amount.is_some() || max_amount.is_some() || time_start.is_some() || time_end.is_some() {
                manager.export_filtered_to_csv(&output_path, &filter_opt)
                    .context("Error when exporting filtered transactions to CSV")?;
            } else {
                manager.export_to_csv(&output_path)
                    .context("Error when exporting all transactions to CSV")?;
            }

            println!("Successfully export csv into {}", output_path);
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
    .bind("127.0.0.1:8086")?
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
