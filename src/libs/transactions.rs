use std::{path::PathBuf, rc::Rc};

use chrono::{DateTime, Local};
use color_eyre::eyre::Result;
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
}
