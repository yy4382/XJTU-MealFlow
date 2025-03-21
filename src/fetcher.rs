use anyhow::{Context, Result, bail};
use chrono::{DateTime, Local, TimeZone};
use reqwest::{Client, header};
use serde::Deserialize;
use std::str;

use crate::transactions::Transaction;

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

fn api_response_to_transactions(api_response: ApiResponse) -> Vec<Transaction> {
    api_response
        .rows
        .into_iter()
        .filter_map(|row| {
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
        })
        .filter(|t| t.amount < 0.0)
        .collect()
}

/// Fetches a single page of transactions
async fn fetch_transaction_one_page(cookie: &str, page: u32) -> Result<Vec<Transaction>> {
    let client = Client::new();

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
    headers.insert(header::ORIGIN, "http://card.xjtu.edu.cn".parse().unwrap());
    headers.insert(header::CONNECTION, "keep-alive".parse().unwrap());
    headers.insert(
        header::REFERER,
        "http://card.xjtu.edu.cn/PPage/ComePage?flowID=15"
            .parse()
            .unwrap(),
    );
    headers.insert(header::USER_AGENT, "".parse().unwrap());
    headers.insert(header::COOKIE, cookie.parse().context("Invalid cookie")?);

    let body = format!("account=253079&page={}&json=true", page);

    // Attempt request with retry logic
    let mut attempts = 0;
    let max_attempts = 3;
    let mut last_error = None;

    while attempts < max_attempts {
        match client
            .post("http://card.xjtu.edu.cn/Report/GetPersonTrjn")
            .headers(headers.clone())
            .body(body.clone())
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<ApiResponse>().await {
                        Ok(api_response) => {
                            // Transform the response into our Transaction type
                            let transactions: Vec<Transaction> =
                                api_response_to_transactions(api_response);
                            return Ok(transactions);
                        }
                        Err(e) => {
                            last_error = Some(anyhow::anyhow!("Failed to parse response: {}", e));
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

        // Retry after delay
        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
        attempts += 1;
    }

    // If we get here, all attempts failed
    bail!(last_error.unwrap_or_else(|| anyhow::anyhow!("Failed to fetch transactions")))
}

/// Fetches all transactions until we reach transactions older than the provided timestamp
pub async fn fetch_transactions(cookie: &str, end_timestamp: i64) -> Result<Vec<Transaction>> {
    let mut all_transactions: Vec<Transaction> = Vec::new();
    let max_pages = 5;

    for page in 1..=max_pages {
        let page_transactions = fetch_transaction_one_page(cookie, page).await?;

        if page_transactions.is_empty() {
            break;
        }

        all_transactions.extend(page_transactions);

        // Check if we've reached transactions older than the end timestamp
        if let Some(last_transaction) = all_transactions.last() {
            let last_timestamp = last_transaction.time.timestamp();
            if last_timestamp <= end_timestamp {
                // Filter out transactions older than the end timestamp
                all_transactions.retain(|t| (t.time.timestamp()) > end_timestamp);
                break;
            }
        } else {
            bail!("No transactions fetched");
        }
    }

    Ok(all_transactions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_to_transactions() {
        let response = include_str!("../test/mock-data/api-resp.json");
        let api_response = serde_json::from_str::<ApiResponse>(response).unwrap();
        let transactions = api_response_to_transactions(api_response);
        println!("{:?}", transactions);
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_transaction_one_page() {
        dotenv::dotenv().ok();

        let cookie = std::env::var("COOKIE").unwrap();
        let page = 1;
        let transactions = fetch_transaction_one_page(cookie.as_str(), page)
            .await
            .unwrap();
        println!("{:?}", transactions);
        assert!(!transactions.is_empty());
    }

    #[tokio::test]
    #[ignore]
    async fn test_fetch_transactions() {
        dotenv::dotenv().ok();

        let cookie = std::env::var("COOKIE").unwrap();
        let end_timestamp = DateTime::parse_from_rfc3339("2025-03-01T00:00:00+08:00")
            .unwrap()
            .timestamp();
        let transactions = fetch_transactions(cookie.as_str(), end_timestamp)
            .await
            .unwrap();
        println!("{:?}", transactions);
        assert!(!transactions.is_empty());
    }
}
