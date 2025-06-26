//! # 数据获取模块
//!
//! 提供从 XJTU 校园卡系统获取交易记录的功能。支持真实网络请求和模拟数据获取两种模式。
//!
//! ## 架构设计
//!
//! ```text
//! MealFetcher (Enum)
//! ├── Real(RealMealFetcher)    - 生产环境：真实网络请求
//! └── Mock(MockMealFetcher)    - 测试环境：模拟数据生成
//! ```
//!
//! ## API 交互流程
//!
//! ```text
//! 1. 配置认证信息 (Cookie + Account)
//!    ↓
//! 2. 构建 HTTP 请求
//!    ↓
//! 3. 发送分页请求到 XJTU 服务器
//!    ↓
//! 4. 解析 JSON 响应
//!    ↓
//! 5. 转换为标准 Transaction 对象
//! ```
//!
//! ## 网络请求规格
//!
//! - **请求 URL**: `http://card.xjtu.edu.cn/Report/GetPersonTrjn`
//! - **请求方法**: POST
//! - **内容类型**: `application/x-www-form-urlencoded`
//! - **认证方式**: Cookie-based session authentication
//! - **分页机制**: 通过 `page` 和 `rows` 参数控制
//!
//! ## 基本用法
//!
//! ### 生产环境（真实数据获取）
//!
//! ```rust
//! use chrono::{DateTime, FixedOffset};
//! use crate::libs::fetcher::{MealFetcher, RealMealFetcher, fetch};
//! use crate::page::fetch::FetchProgress;
//!
//! // 配置获取器
//! let fetcher = MealFetcher::Real(
//!     RealMealFetcher::default()
//!         .cookie("your_session_cookie")
//!         .account("your_student_id")
//! );
//!
//! // 获取交易记录
//! let end_time = chrono::Utc::now().with_timezone(&FixedOffset::east(8 * 3600));
//! let transactions = fetch(end_time, fetcher, |progress| {
//!     println!("Progress: {:?}", progress);
//!     Ok(())
//! })?;
//! ```
//!
//! ### 测试环境（模拟数据）
//!
//! ```rust
//! use std::time::Duration;
//! use crate::libs::fetcher::{MealFetcher, MockMealFetcher};
//!
//! // 配置模拟获取器
//! let fetcher = MealFetcher::Mock(
//!     MockMealFetcher::default()
//!         .set_sim_delay(Duration::from_millis(100))  // 模拟网络延迟
//!         .per_page(50)                               // 每页记录数
//! );
//! ```
//!
//! ## 错误处理
//!
//! - **认证失败**: Cookie 过期或无效时会返回解析错误，建议重新登录
//! - **网络错误**: 自动重试机制（最多3次），每次间隔1秒
//! - **数据解析**: 严格的 JSON 格式验证，字段缺失时提供详细错误信息
//!
//! ## 进度回调
//!
//! 获取过程支持进度回调，用于 UI 更新：
//!
//! ```rust
//! let progress_callback = |progress: FetchProgress| -> Result<()> {
//!     match progress {
//!         FetchProgress::Started => println!("开始获取..."),
//!         FetchProgress::Fetching { current_page, total_estimated } => {
//!             println!("获取第 {} 页，预计总共 {} 页", current_page, total_estimated);
//!         }
//!         FetchProgress::Finished { total_count } => {
//!             println!("获取完成，共 {} 条记录", total_count);
//!         }
//!     }
//!     Ok(())
//! };
//! ```

use chrono::{DateTime, FixedOffset};
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

/// XJTU 校园卡系统 API 基础地址
pub const API_ORIGIN: &str = "http://card.xjtu.edu.cn";

/// 获取个人交易记录的 API 路径
pub const API_PATH: &str = "/Report/GetPersonTrjn";

/// XJTU 校园卡 API 响应数据结构
///
/// 包含交易记录列表的 JSON 响应格式
#[derive(Deserialize, Debug, Clone, Serialize)]
struct ApiResponse {
    /// 交易记录数组
    rows: Vec<TransactionRow>,
}

/// 单条交易记录的原始数据结构
///
/// 直接映射 XJTU 校园卡 API 返回的字段格式
#[derive(Deserialize, Debug, Clone, Serialize)]
struct TransactionRow {
    /// 交易发生时间（字符串格式）
    /// 
    /// API 字段名: `OCCTIME`
    /// 格式: "YYYY-MM-DD HH:MM:SS"
    #[serde(rename = "OCCTIME")]
    time: String,
    
    /// 交易金额（浮点数）
    ///
    /// API 字段名: `TRANAMT`
    /// 负数表示消费，正数表示充值
    #[serde(rename = "TRANAMT")]
    amount: f64,
    
    /// 商家名称
    ///
    /// API 字段名: `MERCNAME`
    /// 例如: "梧桐苑餐厅", "文治书院超市"
    #[serde(rename = "MERCNAME")]
    merchant: String,
}

/// 校园卡数据获取器的统一接口
///
/// 提供生产环境和测试环境两种数据获取模式：
/// - `Real`: 从 XJTU 校园卡系统获取真实数据
/// - `Mock`: 生成模拟数据用于测试和开发
///
/// ## 设计模式
///
/// 使用枚举包装不同的获取器实现，便于在运行时切换数据源，
/// 同时保持相同的 API 接口。
#[derive(Debug, Clone)]
pub enum MealFetcher {
    /// 真实数据获取器
    ///
    /// 连接到 XJTU 校园卡系统，获取用户的实际交易记录
    Real(RealMealFetcher),
    
    /// 模拟数据获取器
    ///
    /// 生成随机的模拟交易数据，用于测试和演示
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

/// 真实数据获取器
///
/// 负责与 XJTU 校园卡系统进行 HTTP 通信，获取用户的真实交易记录。
/// 
/// ## 认证机制
///
/// 使用基于 Cookie 的会话认证：
/// 1. 用户需要先在浏览器中登录 XJTU 校园卡系统
/// 2. 提取登录后的 Cookie 和学号
/// 3. 配置到 `RealMealFetcher` 中进行 API 调用
///
/// ## 分页策略
///
/// - 默认每页获取 50 条记录
/// - 自动处理分页，直到获取所有数据
/// - 支持自定义每页记录数（测试用）
#[derive(Debug, Clone)]
pub struct RealMealFetcher {
    /// 用户会话 Cookie
    ///
    /// 从浏览器中提取的完整 Cookie 字符串，
    /// 用于维护登录状态
    cookie: Option<String>,
    
    /// 用户学号/账号
    ///
    /// 用于 API 请求中的 account 参数
    account: Option<String>,
    
    /// API 服务器地址
    ///
    /// 默认为 XJTU 校园卡系统地址，
    /// 测试时可以指向 Mock 服务器
    origin: String,
    
    /// 每页记录数
    ///
    /// 控制单次 API 请求获取的交易记录数量，
    /// 影响请求频率和内存使用
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
    /// 设置用户会话 Cookie
    ///
    /// # 参数
    ///
    /// * `cookie` - 完整的 Cookie 字符串，从浏览器的开发者工具中获取
    ///
    /// # 返回值
    ///
    /// 返回配置了 Cookie 的新实例（Builder 模式）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let fetcher = RealMealFetcher::default()
    ///     .cookie("JSESSIONID=ABCD1234; Path=/; HttpOnly");
    /// ```
    pub fn cookie<T: Into<String>>(self, cookie: T) -> Self {
        Self {
            cookie: Some(cookie.into()),
            ..self
        }
    }
    
    /// 设置用户学号/账号
    ///
    /// # 参数
    ///
    /// * `account` - 用户的学号或账号，通常是数字字符串
    ///
    /// # 返回值
    ///
    /// 返回配置了账号的新实例（Builder 模式）
    ///
    /// # 示例
    ///
    /// ```rust
    /// let fetcher = RealMealFetcher::default()
    ///     .account("2021123456");
    /// ```
    pub fn account<T: Into<String>>(self, account: T) -> Self {
        Self {
            account: Some(account.into()),
            ..self
        }
    }

    /// 设置 API 服务器地址（仅测试用）
    ///
    /// 允许在测试环境中指向 Mock 服务器
    ///
    /// # 参数
    ///
    /// * `origin` - 服务器地址，如 "http://localhost:3000"
    #[cfg(test)]
    pub fn origin<T: Into<String>>(self, origin: T) -> Self {
        Self {
            origin: origin.into(),
            ..self
        }
    }

    /// 设置每页记录数
    ///
    /// 控制单次 API 请求获取的交易记录数量。
    /// 较大的值可以减少请求次数，但会增加单次请求的响应时间。
    ///
    /// # 参数
    ///
    /// * `size` - 每页记录数，建议范围：10-100
    ///
    /// # 返回值
    ///
    /// 返回配置了页面大小的新实例
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
        let Ok(time) = Transaction::parse_to_fixed_utc_plus8(time_str, "%Y-%m-%d %H:%M:%S") else {
            return None;
        };

        let amount = row.amount;
        let merchant = row.merchant.trim().to_string();

        Some(Transaction::new(amount, merchant, time))
    };

    Ok(api_response
        .rows
        .into_iter()
        .filter_map(row_map)
        .filter(|t| t.amount < 0.0)
        .collect())
}

pub fn fetch<F>(
    end_time: DateTime<FixedOffset>,
    client: MealFetcher,
    progress_cb: F,
) -> Result<Vec<Transaction>>
where
    F: Fn(FetchProgress) -> Result<()>,
{
    let mut all_transactions: Vec<Transaction> = Vec::new();
    let max_pages = 200;

    progress_cb(FetchProgress {
        current_page: 0,
        total_entries_fetched: 0,
        oldest_date: None,
    })?;

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
            })?;

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

        let parse_date = |date_str: &str| {
            Transaction::parse_to_fixed_utc_plus8(date_str, "%Y-%m-%d %H:%M:%S").unwrap()
        };

        data.sort_by(|a, b| {
            let a_time = parse_date(&a.time);
            let b_time = parse_date(&b.time);
            b_time.cmp(&a_time)
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

        let start = std::cmp::min((page - 1) * self.per_page, self.data.len() as u32);
        let end = std::cmp::min(start + self.per_page, self.data.len() as u32);

        let transactions = &self.data[start as usize..end as usize];

        serde_json::to_string(&ApiResponse {
            rows: transactions.to_vec(),
        })
        .context("Failed to serialize mock data")
    }
}

#[cfg(test)]
pub mod test_utils {
    use crate::libs::transactions::Transaction;

    use super::{MockMealFetcher, api_response_to_transactions};

    pub fn get_mock_data(count: u32) -> Vec<Transaction> {
        let fetcher = MockMealFetcher::default().per_page(count);
        let data = fetcher.fetch_transaction_one_page(1).unwrap();
        api_response_to_transactions(&data).unwrap()
    }

    mod test {
        use insta::assert_debug_snapshot;

        use super::*;

        #[test]
        fn test_get_mock_data() {
            let data = get_mock_data(5);
            assert_debug_snapshot!(data);
        }
    }
}

#[cfg(test)]
mod tests {

    use chrono::Duration as CDuration;
    use chrono::Local;
    use chrono::TimeZone as _;
    use std::time::{Duration, Instant};

    use crate::libs::transactions::OFFSET_UTC_PLUS8;

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
        let end_time = OFFSET_UTC_PLUS8
            .with_ymd_and_hms(2025, 3, 1, 0, 0, 0)
            .unwrap();

        let transactions = fetch(end_time, MealFetcher::Mock(fetcher), |_| Ok(())).unwrap();
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
            let end_time = OFFSET_UTC_PLUS8
                .with_ymd_and_hms(2025, 3, 6, 0, 0, 0)
                .unwrap();
            let result = fetch(end_time, MealFetcher::Mock(fetcher), |fp| {
                tx.blocking_send(fp)?;
                Ok(())
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
        let end_time = Local::now().fixed_offset() - CDuration::days(7);
        let fetch = RealMealFetcher::default().account(account).cookie(cookie);

        let transactions = super::fetch(end_time, MealFetcher::Real(fetch), |_| Ok(())).unwrap();
        println!("{:?}", transactions);
        assert!(!transactions.is_empty());
    }
}
