use std::fs::File;
use std::io::Write;
use std::path::Path;

use color_eyre::eyre::{Result, Context};
use chrono::{DateTime, FixedOffset, NaiveDate};

use super::transactions::{Transaction, TransactionManager, FilterOptions};

/// CSV 导出器
pub struct CsvExporter;

impl CsvExporter {
    /// 解析日期字符串，将时间设为当天开始 (00:00:00)
    /// 
    /// # Args
    /// * `date_str` - 格式为 YYYY-MM-DD 的日期字符串
    /// 
    /// # Return Value
    /// * `Result<DateTime<FixedOffset>>` - 带有 UTC+8 时区的日期时间对象
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
    /// # Args
    /// * `date_str` - 格式为 YYYY-MM-DD 的日期字符串
    /// 
    /// # Return Value
    /// * `Result<DateTime<FixedOffset>>` - 带有 UTC+8 时区的日期时间对象
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
    /// # Args
    /// * `manager` - 交易管理器
    /// * `file_path` - CSV 文件保存路径
    /// 
    /// # Return Value
    /// * `Result<usize>` - 成功时返回导出的记录数量
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
    /// # Args
    /// * `manager` - 交易管理器
    /// * `file_path` - CSV 文件保存路径
    /// * `filter_opt` - 筛选条件
    /// 
    /// # Return Value
    /// * `Result<usize>` - 成功时返回导出的记录数量
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
    /// # Args
    /// * `transactions` - 交易记录列表
    /// * `file_path` - CSV 文件保存路径
    /// 
    /// # Return Value
    /// * `Result<()>`
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

