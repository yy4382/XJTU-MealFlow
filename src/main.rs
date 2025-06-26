//! # XJTU MealFlow
//!
//! XJTU MealFlow 是一个用于西安交通大学校园卡消费记录管理的应用程序。
//! 该程序提供了多种运行模式：命令行界面(CLI)、终端用户界面(TUI)和Web界面。
//!
//! ## 主要功能
//!
//! - **数据获取**: 从西安交通大学校园卡系统获取消费记录
//! - **数据分析**: 提供多维度的消费数据分析（时间段、商家、类别等）
//! - **数据导出**: 支持将消费记录导出为CSV格式
//! - **多种界面**: 支持TUI、Web和CLI多种用户界面
//! - **数据存储**: 使用SQLite数据库进行本地数据缓存
//!
//! ## 使用方式
//!
//! ### TUI模式（默认）
//! ```bash
//! cargo run
//! ```
//!
//! ### Web模式
//! ```bash
//! cargo run -- web
//! ```
//!
//! ### CSV导出
//! ```bash
//! cargo run -- export-csv --output transactions.csv
//! ```
//!
//! ### 清理数据库
//! ```bash
//! cargo run -- clear-db
//! ```
//!
//! ## 配置选项
//!
//! - `--account`: 校园卡账号
//! - `--hallticket`: 校园卡认证票据
//! - `--data-dir`: 数据目录路径
//! - `--db-in-mem`: 使用内存数据库（数据不会持久化）
//! - `--use-mock-data`: 使用模拟数据进行测试

/// 应用程序的核心动作定义和状态管理
mod actions;

/// 应用程序主体和层级管理系统
mod app;

/// 命令行参数解析和配置
mod cli;

/// 可重用的UI组件库
mod component;

/// 配置文件和环境变量管理
mod config;

/// 核心业务逻辑库，包含数据获取、存储和导出功能
mod libs;

/// 用户界面页面和组件
mod page;

/// Web服务器和API路由
mod server;

/// 终端用户界面(TUI)实现
#[cfg(not(tarpaulin_include))]
mod tui;

/// 实用工具模块，包含错误处理、日志记录等
mod utils;

use actix_web::{HttpServer, middleware::Logger, web};
use app::{App, RootState};
use clap::Parser;
use color_eyre::eyre::Result;
use dotenv::dotenv;
use libs::export_csv::CsvExporter;
use libs::transactions::TransactionManager;

/// 应用程序的主运行函数
///
/// 根据命令行参数决定运行模式：
/// - 无子命令：启动TUI模式
/// - `clear-db`：清理本地数据库
/// - `web`：启动Web服务器
/// - `export-csv`：导出数据为CSV格式
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回相应的错误信息
///
/// # 错误
///
/// - 配置加载失败
/// - 数据库连接失败
/// - 网络服务启动失败
/// - CSV导出失败
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
        Some(Commands::ExportCsv {
            output,
            merchant,
            min_amount,
            max_amount,
            time_start,
            time_end,
        }) => {
            let manager = TransactionManager::new(config.config.db_path())
                .context("Error when connecting to Database")?;

            let export_options = libs::export_csv::ExportOptions {
                output: output.clone(),
                merchant: merchant.clone(),
                min_amount: *min_amount,
                max_amount: *max_amount,
                time_start: time_start.clone(),
                time_end: time_end.clone(),
            };

            CsvExporter::execute_export(&manager, &export_options)
                .context("Error when exporting transactions to CSV")?;
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

/// 启动Web服务器
///
/// 创建一个Actix Web服务器，提供REST API和静态文件服务。
/// 服务器监听在 `127.0.0.1:8080` 地址上。
///
/// # 参数
///
/// * `manager` - 事务管理器实例，用于处理数据库操作
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回IO错误
///
/// # 功能
///
/// - 提供RESTful API用于数据操作
/// - 服务前端静态文件
/// - 支持CSV导出的Web接口
/// - 自动请求日志记录
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

/// 应用程序入口点
///
/// 执行以下初始化步骤：
/// 1. 加载.env文件中的环境变量
/// 2. 初始化错误处理系统
/// 3. 初始化日志记录系统
/// 4. 调用主运行函数
///
/// # 返回值
///
/// 程序执行成功时返回 `Ok(())`，否则返回错误信息
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
