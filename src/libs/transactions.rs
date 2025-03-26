use std::{path::PathBuf, rc::Rc};

use chrono::{DateTime, Local};
use color_eyre::eyre::{Result, bail};
use rusqlite::{Connection, params};

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: u64,
    pub time: DateTime<Local>,
    pub amount: f64,
    pub merchant: String,
}

#[derive(Debug, Clone)]
pub struct TransactionManager {
    conn: Rc<Connection>,
}

impl TransactionManager {
    pub fn new(db_path: Option<PathBuf>) -> Result<Self> {
        let conn = match db_path {
            Some(db_path) => {
                std::fs::create_dir_all(db_path.parent().unwrap())?;
                Connection::open(db_path)?
            }
            None => Connection::open_in_memory()?,
        };

        Ok(TransactionManager {
            conn: Rc::new(conn),
        })
    }

    pub fn init_db(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS transactions (
                id INTEGER PRIMARY KEY,
                time TEXT NOT NULL,
                amount REAL NOT NULL,
                merchant TEXT NOT NULL
            )",
            [],
        )?;
        self.conn.execute(
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
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS cookies (
            account TEXT PRIMARY KEY,
            cookie TEXT NOT NULL
        )",
            [],
        )?;
        Ok(())
    }

    pub fn insert(&self, transactions: &Vec<Transaction>) -> Result<(), rusqlite::Error> {
        // insert at once
        let mut stmt = self
            .conn
            .prepare("INSERT INTO transactions (id, time, amount, merchant) VALUES (?, ?, ?, ?)")?;

        for transaction in transactions {
            stmt.execute(params![
                transaction.id,
                transaction.time.to_rfc3339(),
                transaction.amount,
                transaction.merchant
            ])?;
        }
        Ok(())
    }

    pub fn fetch_all(&self) -> Result<Vec<Transaction>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, time, amount, merchant FROM transactions")?;
        let transactions = stmt.query_map([], |row| {
            let time_str: String = row.get(1)?;
            let time = chrono::DateTime::parse_from_rfc3339(&time_str)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
                .with_timezone(&Local);

            Ok(Transaction {
                id: row.get(0)?,
                time,
                amount: row.get(2)?,
                merchant: row.get(3)?,
            })
        })?;

        let mut result = Vec::new();
        for transaction in transactions {
            result.push(transaction?);
        }
        Ok(result)
    }

    pub fn fetch_count(&self) -> Result<u64, rusqlite::Error> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM transactions")?;
        let count = stmt.query_map([], |row| row.get(0))?;
        let mut result = 0;
        for c in count {
            result = c?;
        }
        Ok(result)
    }

    #[allow(dead_code)]
    pub fn clear_db(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM transactions", [])?;
        Ok(())
    }

    /// Update account(optional) and cookie in cookies table
    ///
    /// If there is already a record, update it. Otherwise, insert a new record.
    /// There should always be only one records in cookies table
    pub fn update_account(&self, account: &str) -> Result<()> {
        // Check if there are existing records
        let existing = self
            .conn
            .query_row("SELECT cookie FROM cookies LIMIT 1", [], |row| {
                row.get::<_, String>(0)
            });

        // Determine account value to use
        let cookie = existing.unwrap_or_default();

        // Replace the record
        self.conn.execute("DELETE FROM cookies", [])?;
        let mut stmt = self
            .conn
            .prepare("INSERT INTO cookies (account, cookie) VALUES (?, ?)")?;
        stmt.execute(params![account, cookie])?;
        Ok(())
    }

    pub fn update_cookie(&self, cookie: &str) -> Result<()> {
        // Check if there are existing records
        let existing = self
            .conn
            .query_row("SELECT account FROM cookies LIMIT 1", [], |row| {
                row.get::<_, String>(0)
            });

        // Determine account value to use
        let account = existing.unwrap_or_default();

        // Replace the record
        self.conn.execute("DELETE FROM cookies", [])?;
        let mut stmt = self
            .conn
            .prepare("INSERT INTO cookies (account, cookie) VALUES (?, ?)")?;
        stmt.execute(params![account, cookie])?;
        Ok(())
    }

    pub fn get_account_cookie(&self) -> Result<(String, String)> {
        let mut stmt = self.conn.prepare("SELECT account, cookie FROM cookies")?;
        let mut rows = stmt.query([])?;
        let row = rows.next()?;
        match row {
            Some(row) => {
                let account = row.get(0)?;
                let cookie = row.get(1)?;
                Ok((account, cookie))
            }
            None => bail!("No account and cookie found"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_manager() {
        let manager = TransactionManager::new(None).unwrap();

        manager.init_db().unwrap();

        manager.clear_db().unwrap();

        let transactions = vec![
            Transaction {
                id: 1,
                time: Local::now(),
                amount: -100.0,
                merchant: "Amazon".to_string(),
            },
            Transaction {
                id: 2,
                time: Local::now(),
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
        manager.init_db().unwrap();

        manager.update_account("test_account").unwrap();
        let (account, cookie) = manager.get_account_cookie().unwrap();
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
        manager.init_db().unwrap();
        manager.clear_db().unwrap();

        // Initially should have zero transactions
        let count = manager.fetch_count().unwrap();
        assert_eq!(count, 0);

        // Add some transactions
        let transactions = vec![
            Transaction {
                id: 1,
                time: Local::now(),
                amount: -100.0,
                merchant: "Amazon".to_string(),
            },
            Transaction {
                id: 2,
                time: Local::now(),
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
            time: Local::now(),
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
}
