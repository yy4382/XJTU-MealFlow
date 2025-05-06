use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, FixedOffset, TimeZone};
use color_eyre::eyre::{Context, ContextCompat, Result, bail};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize}; // Added import

#[derive(Debug, Clone, Serialize, Deserialize)] // Added Serialize, Deserialize
pub struct Transaction {
    pub id: i64,
    /// Time of the transaction in UTC+8
    pub time: DateTime<FixedOffset>,
    pub amount: f64,
    pub merchant: String,
}

pub const OFFSET_UTC_PLUS8: FixedOffset =
    FixedOffset::east_opt(8 * 3600).expect("Failed to create FixedOffset +8");

impl Transaction {
    pub fn new(amount: f64, merchant: String, time: DateTime<FixedOffset>) -> Self {
        Transaction {
            id: Transaction::hash(&format!("{}&{}&{}", time.timestamp(), amount, &merchant)),
            time,
            amount,
            merchant,
        }
    }

    /// Parse a date string in UTC+8 timezone
    pub fn parse_to_fixed_utc_plus8(s: &str, format: &str) -> Result<DateTime<FixedOffset>> {
        let naive_dt = chrono::NaiveDateTime::parse_from_str(s, format).with_context(|| {
            format!("Failed to parse date string: {} with format: {}", s, format)
        })?;

        naive_dt
            .and_local_timezone(OFFSET_UTC_PLUS8)
            .single()
            .with_context(|| format!("Ambiguous result when adding TZ info to {}", naive_dt))
    }

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
