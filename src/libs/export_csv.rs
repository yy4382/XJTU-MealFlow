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

use color_eyre::eyre::{Result, Context};
use chrono::{DateTime, FixedOffset, NaiveDate};

use super::transactions::{Transaction, TransactionManager, FilterOptions};

/// CSV 导出器
/// 
/// 提供将交易记录导出为 CSV 格式的静态方法。
/// 支持导出所有记录或根据筛选条件导出特定记录。
pub struct CsvExporter;

impl CsvExporter {
    /// 解析日期字符串，将时间设为当天开始 (00:00:00)
    /// 
    /// # 参数
    /// 
    /// * `date_str` - 格式为 `YYYY-MM-DD` 的日期字符串
    /// 
    /// # 返回值
    /// 
    /// 返回带有 UTC+8 时区的日期时间对象，时间为当天的 00:00:00
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use xjtu_mealflow::libs::export_csv::CsvExporter;
    /// 
    /// let date = CsvExporter::parse_date("2022-12-09").unwrap();
    /// // 结果: 2022-12-09 00:00:00 +0800
    /// ```
    pub fn parse_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context(format!("Failed to parse date: {}", date_str))?;
        
        // 创建带有 00:00:00 时间的 datetime
        let naive_datetime = date.and_hms_opt(0, 0, 0).unwrap();
        
        // 添加 UTC+8 时区
        let tz_offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC+8
        let datetime = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, tz_offset);
        
        Ok(datetime)
    }

    /// 解析日期字符串，将时间设为当天结束 (23:59:59)
    /// 
    /// # 参数
    /// 
    /// * `date_str` - 格式为 `YYYY-MM-DD` 的日期字符串
    /// 
    /// # 返回值
    /// 
    /// 返回带有 UTC+8 时区的日期时间对象，时间为当天的 23:59:59
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use xjtu_mealflow::libs::export_csv::CsvExporter;
    /// 
    /// let date = CsvExporter::parse_end_date("2022-12-09").unwrap();
    /// // 结果: 2022-12-09 23:59:59 +0800
    /// ```
    pub fn parse_end_date(date_str: &str) -> Result<DateTime<FixedOffset>> {
        let date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .context(format!("Failed to parse date: {}", date_str))?;
        
        // 创建带有 23:59:59 时间的 datetime
        let naive_datetime = date.and_hms_opt(23, 59, 59).unwrap();
        
        // 添加 UTC+8 时区
        let tz_offset = FixedOffset::east_opt(8 * 3600).unwrap(); // UTC+8
        let datetime = DateTime::<FixedOffset>::from_naive_utc_and_offset(naive_datetime, tz_offset);
        
        Ok(datetime)
    }

    /// 导出所有交易记录到 CSV 文件
    /// 
    /// # 参数
    /// 
    /// * `manager` - 交易管理器实例
    /// * `file_path` - CSV 文件保存路径
    /// 
    /// # 返回值
    /// 
    /// 成功时返回导出的记录数量
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use xjtu_mealflow::libs::{transactions::TransactionManager, export_csv::CsvExporter};
    /// 
    /// let manager = TransactionManager::new(None)?;
    /// let count = CsvExporter::export_all_transactions(&manager, "all_transactions.csv")?;
    /// println!("Exported {} transactions", count);
    /// ```
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
    /// # 参数
    /// 
    /// * `manager` - 交易管理器实例
    /// * `file_path` - CSV 文件保存路径
    /// * `filter_opt` - 筛选条件
    /// 
    /// # 返回值
    /// 
    /// 成功时返回导出的记录数量
    /// 
    /// # 示例
    /// 
    /// ```rust
    /// use xjtu_mealflow::libs::{
    ///     transactions::{TransactionManager, FilterOptions},
    ///     export_csv::CsvExporter
    /// };
    /// 
    /// let manager = TransactionManager::new(None)?;
    /// let filter = FilterOptions::default().merchant("超市");
    /// let count = CsvExporter::export_filtered_transactions(&manager, "supermarket.csv", &filter)?;
    /// println!("Exported {} transactions from supermarket", count);
    /// ```
    pub fn export_filtered_transactions<P: AsRef<Path>>(
        manager: &TransactionManager,
        file_path: P,
        filter_opt: &FilterOptions,
    ) -> Result<usize> {
        let transactions = manager.fetch_filtered(filter_opt)?;
        println!("Found {} transactions matching the filters", transactions.len());
        Self::write_transactions_to_csv(&transactions, file_path)?;
        Ok(transactions.len())
    }

    /// 将交易记录写入 CSV 文件
    /// 
    /// 这是一个内部辅助方法，负责实际的文件写入操作。
    /// 
    /// # 参数
    /// 
    /// * `transactions` - 要写入的交易记录列表
    /// * `file_path` - CSV 文件保存路径
    fn write_transactions_to_csv<P: AsRef<Path>>(
        transactions: &[Transaction],
        file_path: P,
    ) -> Result<()> {
        let mut file = File::create(file_path)?;
        
        // 写入 CSV 头部
        writeln!(file, "ID,Time,Amount,Merchant")?;
        
        // 写入每条交易记录
        for transaction in transactions {
            writeln!(
                file,
                "{},{},{},\"{}\"",
                transaction.id,
                transaction.time.format("%Y-%m-%d %H:%M:%S %z"),
                transaction.amount,
                transaction.merchant.replace("\"", "\"\"") // 处理 CSV 中的引号转义
            )?;
        }
        
        Ok(())
    }
}
