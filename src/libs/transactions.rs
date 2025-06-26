//! # 交易记录管理模块
//!
//! 提供交易记录的数据模型、数据库存储和查询功能。支持 SQLite 数据库的本地缓存和内存数据库两种模式。
//!
//! ## 核心功能
//!
//! - **数据模型**: `Transaction` 结构体定义交易记录的标准格式
//! - **数据库管理**: `TransactionManager` 提供数据的增删改查操作
//! - **数据筛选**: `FilterOptions` 支持按时间、商家、金额范围筛选
//! - **账户管理**: Cookie 和账户信息的持久化存储
//!
//! ## 数据库架构
//!
//! ```sql
//! -- 交易记录表
//! CREATE TABLE transactions (
//!     id INTEGER PRIMARY KEY,           -- 交易唯一标识（基于内容哈希）
//!     time TEXT NOT NULL,              -- 交易时间（ISO 8601 格式）
//!     amount REAL NOT NULL,            -- 交易金额（负数=消费，正数=充值）
//!     merchant TEXT NOT NULL           -- 商家名称
//! );
//!
//! -- 账户信息表
//! CREATE TABLE cookies (
//!     account TEXT PRIMARY KEY,        -- 学号/账号
//!     cookie TEXT NOT NULL            -- 会话 Cookie
//! );
//! ```
//!
//! ## 冲突处理机制
//!
//! 使用触发器防止重复插入：
//! - **相同记录**: 静默跳过（IGNORE）
//! - **ID 冲突但数据不同**: 抛出错误（ABORT）
//!
//! ## 时区处理
//!
//! 所有时间均使用 UTC+8 (中国标准时间)：
//! ```rust
//! use crate::libs::transactions::OFFSET_UTC_PLUS8;
//! let local_time = naive_datetime.and_local_timezone(OFFSET_UTC_PLUS8);
//! ```
//!
//! ## 基本用法
//!
//! ### 创建管理器
//!
//! ```rust
//! use std::path::PathBuf;
//! use crate::libs::transactions::TransactionManager;
//!
//! // 使用文件数据库
//! let db_path = PathBuf::from("./data/transactions.db");
//! let manager = TransactionManager::new(Some(db_path))?;
//!
//! // 使用内存数据库（测试用）
//! let manager = TransactionManager::new(None)?;
//! ```
//!
//! ### 插入交易记录
//!
//! ```rust
//! use chrono::{DateTime, FixedOffset};
//! use crate::libs::transactions::{Transaction, OFFSET_UTC_PLUS8};
//!
//! // 创建交易记录
//! let time = DateTime::parse_from_str("2024-01-15 12:30:00 +0800", "%Y-%m-%d %H:%M:%S %z")?;
//! let transaction = Transaction::new(-15.50, "梧桐苑餐厅".to_string(), time);
//!
//! // 批量插入
//! let transactions = vec![transaction];
//! manager.insert(&transactions)?;
//! ```
//!
//! ### 查询与筛选
//!
//! ```rust
//! use crate::libs::transactions::FilterOptions;
//!
//! // 获取所有记录
//! let all_transactions = manager.fetch_all()?;
//!
//! // 按条件筛选
//! let filter = FilterOptions::default()
//!     .merchant("梧桐苑餐厅")
//!     .min(-50.0)  // 消费金额大于 50 元
//!     .max(-10.0); // 消费金额小于 10 元
//!
//! let filtered = manager.fetch_filtered(&filter)?;
//! ```
//!
//! ## 筛选条件详解
//!
//! ### 时间范围筛选
//!
//! ```rust
//! // [start_time, end_time) 左闭右开区间
//! let filter = FilterOptions::default()
//!     .start(start_time)
//!     .end(end_time);
//! ```
//!
//! ### 金额范围筛选
//!
//! ```rust
//! // [min_amount, max_amount) 左闭右开区间
//! // 注意：消费金额为负数
//! let filter = FilterOptions::default()
//!     .min(-100.0)  // 消费金额 >= 100 元
//!     .max(-10.0);  // 消费金额 < 10 元
//! ```

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, FixedOffset, TimeZone};
use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize}; // Added import

/// 交易记录数据结构
///
/// 表示校园卡的单笔交易信息，包含唯一标识、时间、金额和商家。
/// 支持序列化和反序列化，便于存储和网络传输。
///
/// ## 字段说明
///
/// - `id`: 基于交易内容计算的哈希值，用作唯一标识
/// - `time`: 交易发生时间，统一使用 UTC+8 时区
/// - `amount`: 交易金额，负数表示消费，正数表示充值
/// - `merchant`: 商家名称，如"梧桐苑餐厅"、"文治书院超市"
///
/// ## ID 生成策略
///
/// 交易 ID 通过对 `时间戳 + 金额 + 商家名称` 进行哈希计算生成，
/// 确保相同内容的交易具有相同的 ID，便于去重和冲突检测。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// 交易唯一标识符
    ///
    /// 基于交易内容（时间戳 + 金额 + 商家）的哈希值
    pub id: i64,

    /// 交易发生时间
    ///
    /// 统一使用 UTC+8 时区（中国标准时间）
    pub time: DateTime<FixedOffset>,

    /// 交易金额
    ///
    /// - 负数: 消费（如 -15.50 表示消费 15.50 元）
    /// - 正数: 充值（如 100.00 表示充值 100 元）
    pub amount: f64,

    /// 商家名称
    ///
    /// 消费场所的名称，如"梧桐苑餐厅"、"文治书院超市"
    pub merchant: String,
}

/// 中国标准时间偏移量（UTC+8）
///
/// 所有交易时间统一使用此时区，确保时间的一致性
pub const OFFSET_UTC_PLUS8: FixedOffset =
    FixedOffset::east_opt(8 * 3600).expect("Failed to create FixedOffset +8");

impl Transaction {
    /// 创建新的交易记录
    ///
    /// 根据提供的金额、商家和时间信息创建交易记录，
    /// 自动计算基于内容的唯一 ID。
    ///
    /// # 参数
    ///
    /// * `amount` - 交易金额（负数表示消费，正数表示充值）
    /// * `merchant` - 商家名称
    /// * `time` - 交易时间（带时区信息）
    ///
    /// # 返回值
    ///
    /// 返回新创建的 `Transaction` 实例
    ///
    /// # 示例
    ///
    /// ```rust
    /// use chrono::{DateTime, FixedOffset};
    /// use crate::libs::transactions::{Transaction, OFFSET_UTC_PLUS8};
    ///
    /// let time = DateTime::parse_from_str("2024-01-15 12:30:00 +0800",
    ///                                     "%Y-%m-%d %H:%M:%S %z")?;
    /// let transaction = Transaction::new(
    ///     -15.50,
    ///     "梧桐苑餐厅".to_string(),
    ///     time
    /// );
    /// ```
    pub fn new(amount: f64, merchant: String, time: DateTime<FixedOffset>) -> Self {
        Transaction {
            id: Transaction::hash(&format!("{}&{}&{}", time.timestamp(), amount, &merchant)),
            time,
            amount,
            merchant,
        }
    }

    /// 解析日期字符串为 UTC+8 时区的 DateTime
    ///
    /// 将字符串格式的日期时间解析为带有 UTC+8 时区信息的 DateTime 对象。
    /// 主要用于处理从 XJTU 校园卡 API 获取的时间数据。
    ///
    /// # 参数
    ///
    /// * `s` - 日期时间字符串
    /// * `format` - 解析格式，使用 chrono 的格式化语法
    ///
    /// # 返回值
    ///
    /// 成功时返回 `DateTime<FixedOffset>`，失败时返回解析错误
    ///
    /// # 示例
    ///
    /// ```rust
    /// use crate::libs::transactions::Transaction;
    ///
    /// // 解析 XJTU API 格式的时间
    /// let datetime = Transaction::parse_to_fixed_utc_plus8(
    ///     "2024-01-15 12:30:00",
    ///     "%Y-%m-%d %H:%M:%S"
    /// )?;
    /// ```
    ///
    /// # 错误
    ///
    /// - 当日期字符串格式不匹配时返回解析错误
    /// - 当时区转换模糊时返回时区错误
    pub fn parse_to_fixed_utc_plus8(s: &str, format: &str) -> Result<DateTime<FixedOffset>> {
        let naive_dt = chrono::NaiveDateTime::parse_from_str(s, format).with_context(|| {
            format!("Failed to parse date string: {} with format: {}", s, format)
        })?;

        naive_dt
            .and_local_timezone(OFFSET_UTC_PLUS8)
            .single()
            .with_context(|| format!("Ambiguous result when adding TZ info to {}", naive_dt))
    }

    /// 计算字符串的哈希值
    ///
    /// 使用 Rust 默认的哈希算法计算字符串的 64 位哈希值，
    /// 用于生成交易的唯一标识符。
    ///
    /// # 参数
    ///
    /// * `s` - 待哈希的字符串
    ///
    /// # 返回值
    ///
    /// 返回 64 位有符号整数哈希值
    fn hash(s: &str) -> i64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        i64::from_ne_bytes(hasher.finish().to_ne_bytes())
    }
}

#[derive(Debug, Clone)]
pub struct TransactionManager {
    conn: Arc<Mutex<Connection>>,
}

impl TransactionManager {
    pub fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let conn = match db_path.as_ref() {
            Some(db_path) => {
                std::fs::create_dir_all(db_path.parent().unwrap())
                    .context("Failed to create dir for local cache DB")?;
                Connection::open(db_path.clone()).with_context(|| {
                    format!(
                        "Failed to open local cache DB at {}",
                        db_path.to_str().unwrap_or("INVALID PATH")
                    )
                })?
            }
            None => Connection::open_in_memory()?,
        };

        // Initialize the database
        TransactionManager::init_db(&conn)
            .with_context(|| "Failed to initialize local cache DB")?;

        Ok(TransactionManager {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
        conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY,
                time TEXT NOT NULL,
                amount REAL NOT NULL,
                merchant TEXT NOT NULL
            )",
            [],
        )?;
        conn.execute(
            "CREATE TRIGGER IF NOT EXISTS prevent_transaction_conflict 
                BEFORE INSERT ON transactions
                FOR EACH ROW
                BEGIN
                    SELECT CASE
                    WHEN EXISTS (
                        SELECT 1 FROM transactions 
                        WHERE id = NEW.id 
                        AND time = NEW.time 
                        AND amount = NEW.amount 
                        AND merchant = NEW.merchant
                    ) THEN
                        RAISE(IGNORE)  -- 完全相同的记录则静默跳过
                    WHEN EXISTS (
                        SELECT 1 FROM transactions 
                        WHERE id = NEW.id
                    ) THEN
                        RAISE(ABORT, 'Conflict: Existing transaction with different data')  -- ID存在但数据不同时终止
                    END;
                END;",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS cookies (
            account TEXT PRIMARY KEY,
            cookie TEXT NOT NULL
        )",
            [],
        )?;
        Ok(())
    }

    pub fn insert(&self, transactions: &Vec<Transaction>) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // insert at once
        let mut stmt = conn
            .prepare("INSERT INTO transactions (id, time, amount, merchant) VALUES (?, ?, ?, ?)")?;

        for transaction in transactions {
            stmt.execute(params![
                transaction.id,
                transaction.time,
                transaction.amount,
                transaction.merchant
            ])
            .with_context(|| {
                format!(
                    "Error when inserting transactions into Database, transaction: {:?}",
                    transaction
                )
            })?;
        }
        Ok(())
    }

    /// Fetch all transactions from the database
    ///
    /// Do not guarantee the order of transactions
    pub fn fetch_all(&self) -> Result<Vec<Transaction>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT id, time, amount, merchant FROM transactions")?;
        let transactions = stmt.query_map([], |row| {
            Ok(Transaction {
                id: row.get(0)?,
                time: row.get(1)?,
                amount: row.get(2)?,
                merchant: row.get(3)?,
            })
        })?;

        Ok(transactions.filter_map(|t| t.ok()).collect())
    }

    pub fn fetch_filtered(&self, filter_opt: &FilterOptions) -> Result<Vec<Transaction>> {
        let conn = self.conn.lock().unwrap();

        let mut conditions = Vec::new();
        let mut params = Vec::new();

        if let Some((start, end)) = &filter_opt.time {
            conditions.push("time >= ? AND time < ?");
            params.push(start.to_string());
            params.push(end.to_string());
        }

        if let Some(merchant) = &filter_opt.merchant {
            conditions.push("merchant = ?");
            params.push(merchant.to_string());
        }

        if let Some((min, max)) = &filter_opt.amount {
            conditions.push("amount >= ? AND amount < ?");
            params.push(min.to_string());
            params.push(max.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let query = format!(
            "SELECT id, time, amount, merchant FROM transactions {}",
            where_clause
        );

        let mut stmt = conn.prepare(&query)?;

        let param_refs: Vec<&dyn rusqlite::ToSql> =
            params.iter().map(|p| p as &dyn rusqlite::ToSql).collect();

        let transactions = stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
            Ok(Transaction {
                id: row.get(0)?,
                time: row.get(1)?,
                amount: row.get(2)?,
                merchant: row.get(3)?,
            })
        })?;

        Ok(transactions.filter_map(|t| t.ok()).collect())
    }

    pub fn fetch_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM transactions")?;
        let count: i64 = stmt.query_row([], |row| row.get(0))?;
        Ok(count as u64)
    }

    #[allow(dead_code)]
    pub fn clear_db(&self) -> Result<(), rusqlite::Error> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM transactions", [])?;
        Ok(())
    }

    /// Update account(optional) and cookie in cookies table
    ///
    /// If there is already a record, update it. Otherwise, insert a new record.
    /// There should always be only one records in cookies table
    pub fn update_account(&self, account: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if there are existing records
        let existing = conn.query_row("SELECT cookie FROM cookies LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        });

        // Determine account value to use
        let cookie = existing.unwrap_or_default();

        // Replace the record
        conn.execute("DELETE FROM cookies", [])?;
        let mut stmt = conn.prepare("INSERT INTO cookies (account, cookie) VALUES (?, ?)")?;
        stmt.execute(params![account, cookie])?;
        Ok(())
    }

    pub fn update_cookie(&self, cookie: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();

        // Check if there are existing records
        let existing = conn.query_row("SELECT account FROM cookies LIMIT 1", [], |row| {
            row.get::<_, String>(0)
        });

        // Determine account value to use
        let account = existing.unwrap_or_default();

        // Replace the record
        conn.execute("DELETE FROM cookies", [])?;
        let mut stmt = conn.prepare("INSERT INTO cookies (account, cookie) VALUES (?, ?)")?;
        stmt.execute(params![account, cookie])?;
        Ok(())
    }

    pub fn update_hallticket(&self, hallticket: &str) -> Result<()> {
        let cookie = format!("hallticket={}", hallticket);
        self.update_cookie(&cookie)
    }

    pub fn get_account_cookie(&self) -> Result<(String, String)> {
        let (account, cookie) = self.get_account_cookie_may_empty()?;

        if account.is_empty() || cookie.is_empty() {
            bail!("Account or cookie is empty");
        }

        Ok((account, cookie))
    }

    pub fn get_account_cookie_may_empty(&self) -> Result<(String, String)> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT account, cookie FROM cookies")?;
        let mut rows = stmt.query([])?;
        let row = rows.next()?;
        match row {
            Some(row) => {
                let account: String = row.get(0)?;
                let cookie: String = row.get(1)?;
                Ok((account, cookie))
            }
            None => bail!("No account and cookie found"),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)] // Added Serialize, Deserialize, made pub
pub struct FilterOptions {
    // Made pub
    /// Time range, closed on left, open on right
    pub time: Option<(DateTime<FixedOffset>, DateTime<FixedOffset>)>, // Made pub
    /// Merchant name
    pub merchant: Option<String>, // Made pub
    /// Amount range, closed on left, open on right
    pub amount: Option<(f64, f64)>, // Made pub
}

impl FilterOptions {
    #[allow(dead_code)]
    pub fn start(mut self, start: DateTime<FixedOffset>) -> Self {
        // Made pub
        self.time = Some(match self.time {
            Some((_, end)) => (start, end),
            None => (
                start,
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(9999, 1, 1, 0, 0, 0)
                    .unwrap(),
            ),
        });
        self
    }
    #[allow(dead_code)]
    pub fn end(mut self, end: DateTime<FixedOffset>) -> Self {
        // Made pub
        self.time = Some(match self.time {
            Some((start, _)) => (start, end),
            None => (
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
                    .unwrap(),
                end,
            ),
        });
        self
    }
    pub fn merchant<T: Into<String>>(mut self, merchant: T) -> Self {
        // Made pub
        self.merchant = Some(merchant.into());
        self
    }
    #[allow(dead_code)]
    pub fn min(mut self, amount: f64) -> Self {
        // Made pub
        self.amount = Some(match self.amount {
            Some((_, max)) => (amount, max),
            None => (amount, f64::INFINITY), // Use a safe default for the maximum value
        });
        self
    }
    #[allow(dead_code)]
    pub fn max(mut self, amount: f64) -> Self {
        // Made pub
        self.amount = Some(match self.amount {
            Some((min, _)) => (min, amount),
            None => (f64::NEG_INFINITY, amount), // Use a safe default for the minimum value
        });
        self
    }
}

impl std::fmt::Display for FilterOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut result = String::new();
        if let Some((start, end)) = &self.time {
            result.push_str(&format!("Time: {} - {}\n", start, end));
        }
        if let Some(merchant) = &self.merchant {
            result.push_str(&format!("Merchant: {}\n", merchant));
        }
        if let Some((min, max)) = &self.amount {
            result.push_str(&format!("Amount: {} - {}\n", min, max));
        }
        if result.is_empty() {
            result.push_str("No filters applied\n");
        }
        write!(f, "{}", result)
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn transaction_parse_time() {
        let time_str = "2025-03-01 00:00:00";
        let format = "%Y-%m-%d %H:%M:%S";
        let time = Transaction::parse_to_fixed_utc_plus8(time_str, format).unwrap();
        assert_eq!(
            time,
            OFFSET_UTC_PLUS8
                .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn transaction_new() {
        let time = OFFSET_UTC_PLUS8
            .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
            .unwrap();
        let transaction = Transaction::new(-100.0, "Amazon".to_string(), time);
        assert_eq!(transaction.amount, -100.0);
        assert_eq!(transaction.merchant, "Amazon");
        assert_eq!(transaction.time, time);
        assert_eq!(transaction.id, 2865793625909541060);
    }

    #[test]
    fn test_transaction_manager() {
        let manager = TransactionManager::new(None).unwrap();

        manager.clear_db().unwrap();

        let transactions = vec![
            Transaction {
                id: 1,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -100.0,
                merchant: "Amazon".to_string(),
            },
            Transaction {
                id: 2,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -200.0,
                merchant: "Google".to_string(),
            },
        ];

        manager.insert(&transactions).unwrap();

        let fetched = manager.fetch_all().unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].id, 1);
        assert_eq!(fetched[0].amount, -100.0);
        assert_eq!(fetched[0].merchant, "Amazon");

        assert_eq!(fetched[1].id, 2);
        assert_eq!(fetched[1].amount, -200.0);
        assert_eq!(fetched[1].merchant, "Google");
    }

    #[test]
    fn test_account_cookie() {
        let manager = TransactionManager::new(None).unwrap();

        manager.update_account("test_account").unwrap();
        let (account, cookie) = manager.get_account_cookie_may_empty().unwrap();
        assert_eq!(account, "test_account");
        assert_eq!(cookie, "");

        manager.update_cookie("test_cookie").unwrap();
        let (account, cookie) = manager.get_account_cookie().unwrap();
        assert_eq!(account, "test_account");
        assert_eq!(cookie, "test_cookie");

        manager.update_account("test_account2").unwrap();
        let (account, cookie) = manager.get_account_cookie().unwrap();
        assert_eq!(account, "test_account2");
        assert_eq!(cookie, "test_cookie");
    }

    #[test]
    fn test_fetch_count() {
        let manager = TransactionManager::new(None).unwrap();
        manager.clear_db().unwrap();

        // Initially should have zero transactions
        let count = manager.fetch_count().unwrap();
        assert_eq!(count, 0);

        // Add some transactions
        let transactions = vec![
            Transaction {
                id: 1,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -100.0,
                merchant: "Amazon".to_string(),
            },
            Transaction {
                id: 2,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -200.0,
                merchant: "Google".to_string(),
            },
        ];

        manager.insert(&transactions).unwrap();

        // Should now have 2 transactions
        let count = manager.fetch_count().unwrap();
        assert_eq!(count, 2);

        // Add more transactions
        let more_transactions = vec![Transaction {
            id: 3,
            time: OFFSET_UTC_PLUS8
                .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                .unwrap(),
            amount: -300.0,
            merchant: "Apple".to_string(),
        }];

        manager.insert(&more_transactions).unwrap();

        // Should now have 3 transactions
        let count = manager.fetch_count().unwrap();
        assert_eq!(count, 3);

        // Clear the database
        manager.clear_db().unwrap();

        // Should now have 0 transactions again
        let count = manager.fetch_count().unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn multithread_access() {
        let manager = TransactionManager::new(None).unwrap();

        let transactions = vec![
            Transaction {
                id: 1,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -100.0,
                merchant: "Amazon".to_string(),
            },
            Transaction {
                id: 2,
                time: OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
                amount: -200.0,
                merchant: "Google".to_string(),
            },
        ];

        let manager_clone = manager.clone();
        std::thread::spawn(move || {
            manager_clone.insert(&transactions).unwrap();
        })
        .join()
        .unwrap();

        let fetched = manager.fetch_all().unwrap();
        assert_eq!(fetched.len(), 2);
        assert_eq!(fetched[0].id, 1);
    }

    #[test]
    fn test_fetch_filtered() {
        let manager = TransactionManager::new(None).unwrap();
        manager.clear_db().unwrap();

        // Insert test data
        let transactions = vec![
            Transaction::new(
                -100.0,
                "Amazon".to_string(),
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
            ),
            Transaction::new(
                -200.0,
                "Google".to_string(),
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 2, 0, 0, 0)
                    .unwrap(),
            ),
            Transaction::new(
                -300.0,
                "Amazon".to_string(),
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 3, 0, 0, 0)
                    .unwrap(),
            ),
        ];
        manager.insert(&transactions).unwrap();

        // Test filtering by merchant
        let filter = FilterOptions::default().merchant("Amazon".to_string());
        let results = manager.fetch_filtered(&filter).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|t| t.merchant == "Amazon"));

        // Test filtering by amount range
        let filter = FilterOptions::default().min(-250.0).max(-100.0);
        let results = manager.fetch_filtered(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].amount, -200.0);

        // Test filtering by time range
        let filter = FilterOptions::default()
            .start(
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
                    .unwrap(),
            )
            .end(
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2025, 3, 2, 0, 0, 0)
                    .unwrap(),
            );
        let results = manager.fetch_filtered(&filter).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].merchant, "Amazon");
    }
}
