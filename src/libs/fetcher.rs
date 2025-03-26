use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, TimeZone};
use reqwest::{blocking::Client, header};
use serde::Deserialize;
use std::{
    str::{self},
    thread::sleep,
    time::Duration,
};

use crate::{libs::transactions::Transaction, page::fetch::FetchProgress};

pub const API_ORIGIN: &str = "http://card.xjtu.edu.cn";
pub const API_PATH: &str = "/Report/GetPersonTrjn";

#[derive(Deserialize, Debug)]
struct ApiResponse {
    rows: Vec<TransactionRow>,
}

#[derive(Deserialize, Debug)]
struct TransactionRow {
    #[serde(rename = "OCCTIME")]
    time: String,
    #[serde(rename = "TRANAMT")]
    amount: f64,
    #[serde(rename = "MERCNAME")]
    merchant: String,

    #[serde(rename = "JNUM")]
    id: u64,
}

pub trait MealFetcher {
    fn fetch_transaction_one_page(&self, page: u32) -> Result<String>;
}

pub struct RealMealFetcher {
    cookie: Option<String>,
    account: Option<String>,
    origin: String,
    per_page: u32,
}

impl Default for RealMealFetcher {
    fn default() -> Self {
        Self {
            cookie: Default::default(),
            account: Default::default(),
            origin: API_ORIGIN.into(),
            per_page: 50,
        }
    }
}

impl RealMealFetcher {
    pub fn cookie<T: Into<String>>(self, cookie: T) -> Self {
        Self {
            cookie: Some(cookie.into()),
            ..self
        }
    }
    pub fn account<T: Into<String>>(self, account: T) -> Self {
        Self {
            account: Some(account.into()),
            ..self
        }
    }

    #[cfg(test)]
    pub fn origin<T: Into<String>>(self, origin: T) -> Self {
        Self {
            origin: origin.into(),
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn per_page(self, size: u32) -> Self {
        Self {
            per_page: size,
            ..self
        }
    }
}

impl MealFetcher for RealMealFetcher {
    fn fetch_transaction_one_page(&self, page: u32) -> Result<String> {
        let client = Client::new();

        let cookie = self
            .cookie
            .clone()
            .ok_or(anyhow::anyhow!("Cookie not set"))?;
        let account = self
            .account
            .clone()
            .ok_or(anyhow::anyhow!("Account not set"))?;

        let mut headers = header::HeaderMap::new();
        headers.insert(header::HOST, "card.xjtu.edu.cn".parse().unwrap());
        headers.insert(
            header::ACCEPT,
            "application/json, text/javascript, */*; q=0.01"
                .parse()
                .unwrap(),
        );
        headers.insert("X-Requested-With", "XMLHttpRequest".parse().unwrap());
        headers.insert(
            header::ACCEPT_LANGUAGE,
            "zh-CN,zh-Hans;q=0.9".parse().unwrap(),
        );
        headers.insert(header::ACCEPT_ENCODING, "gzip, deflate".parse().unwrap());
        headers.insert(
            header::CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=UTF-8"
                .parse()
                .unwrap(),
        );
        headers.insert(header::ORIGIN, self.origin.parse().unwrap());
        headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());
        headers.insert(
            header::REFERER,
            "http://card.xjtu.edu.cn/PPage/ComePage?flowID=15"
                .parse()
                .unwrap(),
        );
        headers.insert(header::USER_AGENT, "".parse().unwrap());
        headers.insert(header::COOKIE, cookie.parse().context("Invalid cookie")?);

        let body = format!(
            "account={}&page={}&json=true&rows={}",
            account, page, self.per_page
        );

        // Attempt request with retry logic
        let mut attempts = 0;
        let max_attempts = 3;
        let mut last_error = None;

        while attempts < max_attempts {
            match client
                .post(format!("{}{}", &self.origin, API_PATH))
                .headers(headers.clone())
                .body(body.clone())
                .send()
            {
                Ok(response) => {
                    if response.status().is_success() {
                        match response.text() {
                            Ok(api_response) => {
                                return Ok(api_response);
                            }
                            Err(e) => {
                                last_error =
                                    Some(anyhow::anyhow!("Failed to parse response: {}", e));
                            }
                        }
                    } else {
                        last_error = Some(anyhow::anyhow!(
                            "Request failed with status: {}",
                            response.status()
                        ));
                    }
                }
                Err(e) => {
                    last_error = Some(anyhow::anyhow!("Request error: {}", e));
                }
            }

            // Retry after delay 1000 (blocking)
            sleep(Duration::new(1, 0));
            attempts += 1;
        }

        // If we get here, all attempts failed
        bail!(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to fetch transactions")))
    }
}

fn api_response_to_transactions(s: &str) -> Result<Vec<Transaction>> {
    let api_response: ApiResponse = serde_json::from_str(s)?;

    let row_map = |row: TransactionRow| {
        // Parse the date
        let time_str = &row.time.trim();
        let time: DateTime<Local> =
            match chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
                Ok(dt) => match chrono::Local::now().timezone().from_local_datetime(&dt) {
                    chrono::LocalResult::Single(t) => t,
                    _ => return None,
                },
                Err(_) => return None,
            };

        let amount = row.amount;
        let merchant = row.merchant.trim().to_string();

        Some(Transaction {
            time,
            amount,
            merchant,
            id: row.id,
        })
    };

    Ok(api_response
        .rows
        .into_iter()
        .filter_map(row_map)
        .filter(|t| t.amount < 0.0)
        .collect())
}

pub fn fetch<F>(
    end_time: DateTime<Local>,
    client: Box<dyn MealFetcher>,
    progress_cb: F,
) -> Result<Vec<Transaction>>
where
    F: Fn(FetchProgress),
{
    let mut all_transactions: Vec<Transaction> = Vec::new();
    let max_pages = 200;

    progress_cb(FetchProgress {
        current_page: 0,
        total_entries_fetched: 0,
        oldest_date: None,
    });

    for page in 1..=max_pages {
        let page_transactions = client.fetch_transaction_one_page(page)?;
        let page_transactions = api_response_to_transactions(&page_transactions)?;
        if page_transactions.is_empty() {
            break;
        }

        all_transactions.extend(page_transactions);

        // Check if we've reached transactions older than the end timestamp
        if let Some(last_transaction) = all_transactions.last() {
            progress_cb(FetchProgress {
                current_page: page,
                total_entries_fetched: all_transactions.len() as u32,
                oldest_date: Some(last_transaction.time),
            });

            let last_timestamp = last_transaction.time.timestamp();
            if last_timestamp <= end_time.timestamp() {
                // Filter out transactions older than the end timestamp
                all_transactions.retain(|t| (t.time.timestamp()) > end_time.timestamp());
                break;
            }
        } else {
            bail!("No transactions fetched");
        }
    }

    Ok(all_transactions)
}

macro_rules! test_file {
    ($a:expr) => {
        include_str!(concat!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/test/mock-data/api-resp/"),
            $a
        ))
    };
}

pub struct MockMealFetcher {
    sim_delay: Option<Duration>,
}

impl MockMealFetcher {
    fn new(duration: Option<Duration>) -> Self {
        Self {
            sim_delay: duration,
        }
    }
}

impl MealFetcher for MockMealFetcher {
    fn fetch_transaction_one_page(&self, page: u32) -> Result<String> {
        if let Some(d) = self.sim_delay {
            sleep(d);
        }

        Ok(if page == 1 {
            test_file!("1.json")
        } else if page == 2 {
            test_file!("2.json")
        } else if page == 3 {
            test_file!("3.json")
        } else {
            bail!("page too large")
        }
        .into())
    }
}

#[cfg(test)]
mod tests {

    use chrono::Duration as CDuration;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn test_api_response_to_transactions() {
        let transactions = api_response_to_transactions(test_file!("1.json"));
        println!("{:?}", transactions);
    }

    #[test]
    fn test_fetch_mock() {
        let fetcher = MockMealFetcher::new(None);
        let end_time = Local.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();

        let transactions = fetch(end_time, Box::new(fetcher), |_| ()).unwrap();
        assert_eq!(transactions.len(), 42)
    }

    #[tokio::test]
    async fn test_fetch_mock_progress() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<FetchProgress>(1);
        tokio::task::spawn_blocking(move || {
            let fetcher = MockMealFetcher::new(Some(Duration::from_millis(200)));
            let end_time = Local.with_ymd_and_hms(2025, 3, 6, 0, 0, 0).unwrap();
            let _ = fetch(end_time, Box::new(fetcher), |fp| {
                tx.blocking_send(fp).unwrap()
            })
            .unwrap();
            drop(tx)
        });

        let mut received = Vec::<(FetchProgress, Instant)>::new();

        loop {
            match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Some(p)) => received.push((p, Instant::now())),
                Ok(None) => break,
                Err(_) => println!("接收超时"),
            }
        }

        assert_eq!(received.len(), 3);

        let gaps = if received.len() > 1 {
            received
                .windows(2)
                .map(|window| {
                    let (_, first_instant) = &window[0];
                    let (_, second_instant) = &window[1];
                    second_instant.duration_since(*first_instant)
                })
                .collect::<Vec<Duration>>()
        } else {
            vec![]
        };

        // Verify we have appropriate delays between progress updates
        assert!(
            gaps.iter()
                .all(|duration| duration.as_millis() > 150 && duration.as_millis() < 300),
            "Progress updates should be appropriate due to simulated delay"
        );

        println!("{:?}\n{:?}", received, gaps);
    }

    #[test]
    fn test_something() {
        // Request a new server from the pool
        let mut server = mockito::Server::new();

        // Use one of these addresses to configure your client
        let _host = server.host_with_port();
        let url = server.url();

        // Create a mock
        let mock = server
            .mock("POST", "/Report/GetPersonTrjn")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(test_file!("1.json"))
            .create();

        let fetch = RealMealFetcher::default()
            .account("Account")
            .cookie("Cookie")
            .origin(url);

        let t = fetch.fetch_transaction_one_page(1).unwrap();

        assert_eq!(api_response_to_transactions(&t).unwrap().len(), 19);

        // You can use `Mock::assert` to verify that your mock was called
        mock.assert();
    }

    #[test]
    #[ignore]
    fn test_fetch_transactions() {
        dotenv::dotenv().ok();

        let cookie = std::env::var("XMF_COOKIE").unwrap();
        let account = std::env::var("XMF_ACCOUNT").unwrap();
        let end_time = Local::now() - CDuration::days(3);
        let fetch = RealMealFetcher::default().account(account).cookie(cookie);

        let transactions = super::fetch(end_time, Box::new(fetch), |_| ()).unwrap();
        println!("{:?}", transactions);
        assert!(!transactions.is_empty());
    }
}
