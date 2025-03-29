use chrono::{DateTime, Local, TimeZone};
use color_eyre::{
    Result, Section, SectionExt,
    eyre::{WrapErr, bail, eyre},
};
use reqwest::{blocking::Client, header};
use serde::{Deserialize, Serialize};
use std::{
    str::{self},
    thread::sleep,
    time::Duration,
};

use crate::{libs::transactions::Transaction, page::fetch::FetchProgress};

pub const API_ORIGIN: &str = "http://card.xjtu.edu.cn";
pub const API_PATH: &str = "/Report/GetPersonTrjn";

#[derive(Deserialize, Debug, Clone, Serialize)]
struct ApiResponse {
    rows: Vec<TransactionRow>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
struct TransactionRow {
    #[serde(rename = "OCCTIME")]
    time: String,
    #[serde(rename = "TRANAMT")]
    amount: f64,
    #[serde(rename = "MERCNAME")]
    merchant: String,
}

#[derive(Debug, Clone)]
pub enum MealFetcher {
    Real(RealMealFetcher),
    Mock(MockMealFetcher),
}

impl From<RealMealFetcher> for MealFetcher {
    fn from(fetcher: RealMealFetcher) -> Self {
        MealFetcher::Real(fetcher)
    }
}
impl From<MockMealFetcher> for MealFetcher {
    fn from(fetcher: MockMealFetcher) -> Self {
        MealFetcher::Mock(fetcher)
    }
}

impl Default for MealFetcher {
    fn default() -> Self {
        Self::Real(RealMealFetcher::default())
    }
}

#[derive(Debug, Clone)]
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

    fn fetch_transaction_one_page(&self, page: u32) -> Result<String> {
        let client = Client::new();

        let cookie = self.cookie.clone().ok_or(eyre!("Cookie not set"))?;
        let account = self.account.clone().ok_or(eyre!("Account not set"))?;

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
                                last_error = Some(eyre!("Failed to parse response: {}", e));
                            }
                        }
                    } else {
                        last_error =
                            Some(eyre!("Request failed with status: {}", response.status()));
                    }
                }
                Err(e) => {
                    last_error = Some(eyre!("Request error: {}", e));
                }
            }

            // Retry after delay 1000 (blocking)
            sleep(Duration::new(1, 0));
            attempts += 1;
        }

        // If we get here, all attempts failed
        bail!(last_error.unwrap_or_else(|| eyre!("Failed to fetch transactions")))
    }
}

fn api_response_to_transactions(s: &str) -> Result<Vec<Transaction>> {
    let api_response = serde_json::from_str::<ApiResponse>(s).map_err(|e| {
        if e.is_data() && format!("{}", e).contains("missing field `rows`") {
            eyre!("{}. This may indicate that your cookie has expired.", e).with_note(
                || "Consider re-logging in to card.xjtu.edu.cn and updating your cookie.",
            )
        } else {
            eyre!("Failed to parse API response: {}", e)
        }
        .with_section(|| s.to_string().header("Incorrect API response:"))
    })?;

    let row_map = |row: TransactionRow| {
        // Parse the date
        let time_str = &row.time.trim();
        let Ok(time) = parse_date(time_str).with_context(|| format!("Invalid date: {}", time_str))
        else {
            return None;
        };

        let amount = row.amount;
        let merchant = row.merchant.trim().to_string();

        // hash time-amount-merchant

        let hash = |s: &str| -> i64 {
            use std::hash::{DefaultHasher, Hash, Hasher};
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            i64::from_ne_bytes(hasher.finish().to_ne_bytes())
        };

        Some(Transaction {
            id: hash(&format!("{}&{}&{}", time.timestamp(), amount, &merchant)),
            time,
            amount,
            merchant,
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
    client: MealFetcher,
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
        let page_transactions = match &client {
            MealFetcher::Real(c) => c.fetch_transaction_one_page(page),
            MealFetcher::Mock(c) => c.fetch_transaction_one_page(page),
        }
        .with_context(|| format!("Error when fetching on page {}", page))?;

        let page_transactions =
            api_response_to_transactions(&page_transactions).with_context(|| {
                format!(
                    "Error when parsing data returned from XJTU server on page {}",
                    page
                )
            })?;
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

#[derive(Debug, Clone)]
pub struct MockMealFetcher {
    sim_delay: Option<Duration>,
    per_page: u32,
    data: Vec<TransactionRow>,
}

impl Default for MockMealFetcher {
    fn default() -> Self {
        let data = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test/mock-data/mock-transactions.json"
        ));
        let mut  data = serde_json::from_str::<Vec<TransactionRow>>(data).context(
            "Failed to parse mock data. This may indicate that the mock data file is missing or corrupted.",
        ).unwrap();
        data.sort_by(|a, b| {
            parse_date(&b.time)
                .unwrap()
                .cmp(&parse_date(&a.time).unwrap())
        });

        Self {
            sim_delay: None,
            per_page: 20,
            data,
        }
    }
}

impl MockMealFetcher {
    #[allow(dead_code)]
    pub fn set_sim_delay(self, duration: Duration) -> Self {
        Self {
            sim_delay: Some(duration),
            ..self
        }
    }

    pub fn per_page(mut self, size: u32) -> Self {
        self.per_page = size;
        self
    }

    fn fetch_transaction_one_page(&self, page: u32) -> Result<String> {
        if let Some(d) = self.sim_delay {
            sleep(d);
        }

        let start = (page - 1) * self.per_page;
        let end = start + self.per_page;

        let transactions = &self.data[start as usize..end as usize];

        serde_json::to_string(&ApiResponse {
            rows: transactions.to_vec(),
        })
        .context("Failed to serialize mock data")
    }
}

fn parse_date(time: &str) -> Result<DateTime<Local>> {
    let time = match chrono::NaiveDateTime::parse_from_str(time.trim(), "%Y-%m-%d %H:%M:%S") {
        Ok(dt) => match chrono::Local::now().timezone().from_local_datetime(&dt) {
            chrono::LocalResult::Single(t) => t,
            _ => bail!("Invalid date format"),
        },
        Err(_) => bail!("Invalid date format"),
    };
    Ok(time)
}

#[cfg(test)]
mod tests {

    use chrono::Duration as CDuration;
    use std::time::{Duration, Instant};

    use super::*;

    #[test]
    fn test_api_response_to_transactions() {
        let transactions = api_response_to_transactions(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test/mock-data/api-resp.json"
        )));
        println!("{:?}", transactions);
    }

    #[test]
    fn test_fetch_mock() {
        let fetcher = MockMealFetcher::default();
        let end_time = Local.with_ymd_and_hms(2025, 3, 1, 0, 0, 0).unwrap();

        let transactions = fetch(end_time, MealFetcher::Mock(fetcher), |_| ()).unwrap();
        assert!(!transactions.is_empty());
        transactions.iter().for_each(|t| {
            assert!(t.time.timestamp() > end_time.timestamp());
        });
    }

    #[tokio::test]
    async fn test_fetch_mock_progress() {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<FetchProgress>(1);
        let (c_tx, mut c_rx) = tokio::sync::mpsc::unbounded_channel::<u32>();
        tokio::task::spawn_blocking(move || {
            let fetcher = MockMealFetcher::default().set_sim_delay(Duration::from_millis(200));
            let end_time = Local.with_ymd_and_hms(2025, 3, 6, 0, 0, 0).unwrap();
            let result = fetch(end_time, MealFetcher::Mock(fetcher), |fp| {
                tx.blocking_send(fp).unwrap()
            })
            .unwrap();
            c_tx.send(result.len() as u32).unwrap();
            drop(tx);
            drop(c_tx);
        });

        let mut received = Vec::<(FetchProgress, Instant)>::new();

        loop {
            match tokio::time::timeout(Duration::from_secs(5), rx.recv()).await {
                Ok(Some(p)) => received.push((p, Instant::now())),
                Ok(None) => break,
                Err(_) => println!("timeout"),
            }
        }

        assert_ne!(received.len(), 0);

        loop {
            match tokio::time::timeout(Duration::from_secs(5), c_rx.recv()).await {
                Ok(Some(p)) => assert!(
                    // some fetched items are older than end_time, so they are filtered out
                    // thus, reported progress may be larger than the total fetched items
                    p <= received.last().unwrap().0.total_entries_fetched,
                    "fetched items count should be smaller as progress-reported total"
                ),
                Ok(None) => break,
                Err(_) => println!("timeout"),
            }
        }

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
                .all(|duration| duration.as_millis() > 50 && duration.as_millis() < 400),
            "Progress updates should be appropriate due to simulated delay (200ms), {:?}",
            gaps
        );

        println!("{:?}\n{:?}", received, gaps);
    }

    #[test]
    fn test_request() {
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
            .with_body(include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/test/mock-data/api-resp.json"
            )))
            .create();

        let fetch = RealMealFetcher::default()
            .account("Account")
            .cookie("Cookie")
            .origin(url);

        let t = fetch.fetch_transaction_one_page(1).unwrap();

        assert_eq!(api_response_to_transactions(&t).unwrap().is_empty(), false);

        // You can use `Mock::assert` to verify that your mock was called
        // TODO check if request is valid
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

        let transactions = super::fetch(end_time, MealFetcher::Real(fetch), |_| ()).unwrap();
        println!("{:?}", transactions);
        assert!(!transactions.is_empty());
    }
}
