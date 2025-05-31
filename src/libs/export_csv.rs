//! # CSV 导出模块
//!
//! 提供将交易记录导出为 CSV 格式的功能。
//!
//! ## 基本用法
//!
//! ### 导出所有交易记录
//!
//! ```bash
//! # 默认路径 transactions_export.csv
//! cargo run -- export-csv
//! ```
//!
//! ### 自定义导出路径
//!
//! ```bash
//! # 自定义导出路径
//! cargo run -- export-csv --output "my_transactions.csv"
//! cargo run -- export-csv --output "data/my_transactions.csv"
//! ```
//!
//! ## 筛选功能
//!
//! ### 按消费金额筛选
//!
//! 导出消费金额在指定范围内的交易：
//!
//! ```bash
//! # 消费金额区间 [10.00, 50.00]
//! cargo run -- export-csv --min-amount=10.00 --max-amount=50.00
//! ```
//!
//! ### 按商家筛选
//!
//! 导出特定商家的交易数据：
//!
//! ```bash
//! # 筛选特定商家（如"超市"）的交易
//! cargo run -- export-csv --merchant "超市"
//! ```
//!
//! ### 按日期区间筛选
//!
//! 导出指定日期范围内的交易：
//!
//! ```bash
//! # [2022-12-09, 2024-12-09] 左闭右闭区间
//! cargo run -- export-csv --time-start "2022-12-09" --time-end "2024-12-09"
//!
//! # 从指定日期到最新记录
//! cargo run -- export-csv --time-start "2022-12-09"
//!
//! # 从最早记录到指定日期
//! cargo run -- export-csv --time-end "2024-12-09"
//! ```
//!
//! ### 组合筛选
//!
//! 可以同时使用多个筛选条件：
//!
//! ```bash
//! # 组合筛选：金额 + 商家 + 自定义输出路径
//! cargo run -- export-csv \
//!   --min-amount=10.00 \
//!   --max-amount=50.00 \
//!   --merchant "超市" \
//!   --output "filtered_transactions.csv"
//! ```
//!
//! ## 日期格式
//!
//! 所有日期参数必须使用 `YYYY-MM-DD` 格式，例如：
//! - `2022-12-09`
//! - `2024-01-15`
//!
//! ## 输出格式
//!
//! 导出的 CSV 文件包含以下列：
//! - `ID`: 交易唯一标识符
//! - `Time`: 交易时间（格式：YYYY-MM-DD HH:MM:SS +ZZZZ）
//! - `Amount`: 交易金额（负数表示消费，正数表示充值）
//! - `Merchant`: 商家名称

use std::fs::File;
use std::io::Write;
use std::path::Path;

use chrono::{DateTime, FixedOffset, NaiveDate};
use color_eyre::eyre::{Context, Result};

use super::transactions::{FilterOptions, Transaction, TransactionManager};

/// CSV 导出器
///
/// 提供将交易记录导出为 CSV 格式的静态方法。
/// 支持导出所有记录或根据筛选条件导出特定记录。
pub struct CsvExporter;

/// CSV 导出命令的参数
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// 输出文件路径
    pub output: Option<String>,
    /// 商家名称筛选
    pub merchant: Option<String>,
    /// 最小金额筛选（正数）
    pub min_amount: Option<f64>,
    /// 最大金额筛选（正数）
    pub max_amount: Option<f64>,
    /// 开始日期筛选
    pub time_start: Option<String>,
    /// 结束日期筛选
    pub time_end: Option<String>,
}

impl CsvExporter {
    /// 执行 CSV 导出命令
    ///
    /// 这是主入口函数，处理所有的筛选条件构建和导出逻辑
    ///
    /// # 参数
    ///
    /// * `manager` - 交易管理器实例
    /// * `options` - 导出选项的引用
    ///
    /// # 返回值
    ///
    /// 成功时返回导出的记录数量
    pub fn execute_export(manager: &TransactionManager, options: &ExportOptions) -> Result<usize> {
        // 构建筛选条件
        let filter_opt = Self::build_filter_options(options)?;

        // 确定输出路径
        let output_path = options
            .output
            .clone()
            .unwrap_or_else(|| "transactions_export.csv".to_string());

        // 执行导出
        let count = if Self::has_any_filter(options) {
            Self::export_filtered_transactions(manager, &output_path, &filter_opt)?
        } else {
            Self::export_all_transactions(manager, &output_path)?
        };

        println!(
            "Successfully exported {} transactions to {}",
            count, output_path
        );
        Ok(count)
    }

    /// 构建筛选条件
    ///
    /// 将用户输入的选项转换为数据库查询的筛选条件
    fn build_filter_options(options: &ExportOptions) -> Result<FilterOptions> {
        let mut filter_opt = FilterOptions::default();

        // (1) 商家筛选
        if let Some(merchant) = &options.merchant {
            filter_opt = filter_opt.merchant(merchant);
        }

        // (2) 金额筛选
        // 用户输入正数范围，转换为数据库中的负数范围
        if let Some(min) = options.min_amount {
            if min < 0.0 {
                println!("Warning: min_amount should be positive, converting absolute value");
            }
            // 用户最小值 -> 数据库最大值（逻辑反转）
            let db_max = -min.abs();
            filter_opt = filter_opt.max(db_max);
        }

        if let Some(max) = options.max_amount {
            if max < 0.0 {
                println!("Warning: max_amount should be positive, converting absolute value");
            }
            // 用户最大值 -> 数据库最小值（逻辑反转）
            let db_min = -max.abs();
            filter_opt = filter_opt.min(db_min);
        }

        // (3) 日期筛选
        if let Some(start_str) = &options.time_start {
            match Self::parse_date(start_str) {
                Ok(start_date) => {
                    println!("Setting start date to: {}", start_date);
                    filter_opt = filter_opt.start(start_date);
                }
                Err(e) => {
                    println!("Warning: Invalid start date format '{}': {}", start_str, e);
                    println!("Date format should be YYYY-MM-DD");
                }
            }
        }

        if let Some(end_str) = &options.time_end {
            match Self::parse_end_date(end_str) {
                Ok(end_date) => {
                    println!("Setting end date to: {}", end_date);
                    filter_opt = filter_opt.end(end_date);
                }
                Err(e) => {
                    println!("Warning: Invalid end date format '{}': {}", end_str, e);
                    println!("Date format should be YYYY-MM-DD");
                }
            }
        }

        Ok(filter_opt)
    }

    /// 检查是否有任何筛选条件
    fn has_any_filter(options: &ExportOptions) -> bool {
        options.merchant.is_some()
            || options.min_amount.is_some()
            || options.max_amount.is_some()
            || options.time_start.is_some()
            || options.time_end.is_some()
    }

    /// 解析日期字符串，将时间设为当天开始 (00:00:00)
    ///
    /// [之前的文档注释保持不变...]
    pub fn parse_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context(format!("Failed to parse date: {}", date_str))?;

        let naive_datetime = date.and_hms_opt(0, 0, 0).unwrap();
        let tz_offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC+8
        let datetime =
            DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, tz_offset);

        Ok(datetime)
    }

    /// 解析日期字符串，将时间设为当天结束 (23:59:59)
    ///
    /// [之前的文档注释保持不变...]
    pub fn parse_end_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context(format!("Failed to parse date: {}", date_str))?;

        let naive_datetime = date.and_hms_opt(23, 59, 59).unwrap();
        let tz_offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC+8
        let datetime =
            DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, tz_offset);

        Ok(datetime)
    }

    /// 导出所有交易记录到 CSV 文件
    ///
    /// [之前的文档注释和实现保持不变...]
    pub fn export_all_transactions<P: AsRef<Path>>(
        manager: &TransactionManager,
        file_path: P,
    ) -> Result<usize> {
        let transactions = manager.fetch_all()?;
        Self::write_transactions_to_csv(&transactions, file_path)?;
        Ok(transactions.len())
    }

    /// 导出筛选后的交易记录到 CSV 文件
    ///
    /// [之前的文档注释和实现保持不变...]
    pub fn export_filtered_transactions<P: AsRef<Path>>(
        manager: &TransactionManager,
        file_path: P,
        filter_opt: &FilterOptions,
    ) -> Result<usize> {
        let transactions = manager.fetch_filtered(filter_opt)?;
        println!(
            "Found {} transactions matching the filters",
            transactions.len()
        );
        Self::write_transactions_to_csv(&transactions, file_path)?;
        Ok(transactions.len())
    }

    /// 将交易记录写入 CSV 文件
    ///
    /// [之前的实现保持不变...]
    fn write_transactions_to_csv<P: AsRef<Path>>(
        transactions: &[Transaction],
        file_path: P,
    ) -> Result<()> {
        let mut file = File::create(file_path)?;

        writeln!(file, "ID,Time,Amount,Merchant")?;

        for transaction in transactions {
            writeln!(
                file,
                "{},{},{},\"{}\"",
                transaction.id,
                transaction.time.format("%Y-%m-%d %H:%M:%S %z"),
                transaction.amount,
                transaction.merchant.replace("\"", "\"\"")
            )?;
        }
        Ok(())
    }

    /// 导出交易记录为 CSV 字符串（用于 Web API）
    ///
    /// 这个方法专门为 Web API 设计，返回 CSV 内容字符串而不是写入文件。
    /// 保留了与命令行版本相同的筛选逻辑。
    ///
    /// # 参数
    ///
    /// * `manager` - 交易管理器实例
    /// * `options` - 导出选项的引用
    ///
    /// # 返回值
    ///
    /// 成功时返回 (CSV字符串内容, 记录数量)
    ///
    /// # 示例
    ///
    /// ```rust
    /// let manager = TransactionManager::new()?;
    /// let options = ExportOptions {
    ///     output: None, // Web API 不需要文件输出
    ///     merchant: Some("超市".to_string()),
    ///     min_amount: Some(10.0),
    ///     max_amount: Some(50.0),
    ///     time_start: None,
    ///     time_end: None,
    /// };
    ///
    /// let (csv_content, count) = CsvExporter::export_to_string(&manager, &options)?;
    /// println!("Generated CSV with {} records", count);
    /// ```
    pub fn export_to_string(
        manager: &TransactionManager,
        options: &ExportOptions,
    ) -> Result<(String, usize)> {
        // 复用现有的筛选条件构建逻辑
        let filter_opt = Self::build_filter_options(options)?;

        // 获取交易记录（复用现有逻辑）
        let transactions = if Self::has_any_filter(options) {
            manager.fetch_filtered(&filter_opt)?
        } else {
            manager.fetch_all()?
        };

        // 生成 CSV 字符串
        let csv_content = Self::transactions_to_csv_string(&transactions)?;

        Ok((csv_content, transactions.len()))
    }

    /// 将交易记录转换为 CSV 字符串
    ///
    /// 与 write_transactions_to_csv 保持相同的格式，但输出到字符串而不是文件
    ///
    /// # 参数
    ///
    /// * `transactions` - 交易记录数组
    ///
    /// # 返回值
    ///
    /// CSV 格式的字符串
    fn transactions_to_csv_string(transactions: &[Transaction]) -> Result<String> {
        let mut csv_content = String::new();

        // 写入表头（与文件版本格式一致）
        csv_content.push_str("ID,Time,Amount,Merchant\n");

        // 写入数据行（与文件版本格式一致）
        for transaction in transactions {
            csv_content.push_str(&format!(
                "{},{},{},\"{}\"\n",
                transaction.id,
                transaction.time.format("%Y-%m-%d %H:%M:%S %z"),
                transaction.amount,
                transaction.merchant.replace("\"", "\"\"")
            ));
        }

        Ok(csv_content)
    }
}
