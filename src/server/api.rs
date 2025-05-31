use actix_web::{
    dev::ServiceRequest, // For test setup, if needed elsewhere
    error::{ErrorInternalServerError, ErrorNotFound},
    http::header::{ContentDisposition, DispositionParam, DispositionType, HeaderName, HeaderValue}, // Added for typed headers
    web,
    App, // For test setup
    HttpResponse,
    Responder,
    Result as ActixResult,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

// Assuming Transaction and FilterOptions are correctly defined and made public in libs::transactions
// and derive Serialize and Deserialize.
// Also, Transaction should be public for tests.
use crate::libs::{
    export_csv::{CsvExporter, ExportOptions},
    fetcher::{fetch, RealMealFetcher},
    transactions::{FilterOptions, TransactionManager}, // Assuming Transaction is also in here or imported separately for tests
};

// --- Helper for converting Result to ActixResult ---
fn to_actix_response<T: Serialize>(result: color_eyre::Result<T>) -> ActixResult<impl Responder> {
    match result {
        Ok(data) => Ok(web::Json(data)),
        Err(e) => {
            // Log the full error for debugging
            tracing::error!("Handler error: {:?}", e);
            // Provide a generic error to the client
            Err(ErrorInternalServerError(format!(
                "An internal error occurred: {}",
                e
            )))
        }
    }
}

fn to_actix_empty_response(result: color_eyre::Result<()>) -> ActixResult<impl Responder> {
    match result {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            tracing::error!("Handler error: {:?}", e);
            Err(ErrorInternalServerError(format!(
                "An internal error occurred: {}",
                e
            )))
        }
    }
}

// --- Handlers for TransactionManager methods ---

// GET /transactions
async fn handle_fetch_all_transactions(
    manager: web::Data<TransactionManager>,
) -> ActixResult<impl Responder> {
    to_actix_response(manager.fetch_all())
}

// POST /transactions/query (using POST to allow FilterOptions in body)
async fn handle_fetch_filtered_transactions(
    manager: web::Data<TransactionManager>,
    filter_opts: web::Json<FilterOptions>,
) -> ActixResult<impl Responder> {
    to_actix_response(manager.fetch_filtered(&filter_opts.into_inner()))
}

// GET /transactions/count
async fn handle_fetch_transaction_count(
    manager: web::Data<TransactionManager>,
) -> ActixResult<impl Responder> {
    to_actix_response(manager.fetch_count())
}

#[derive(Deserialize, Serialize)]
struct FetchTransactionsRequest {
    start_date: DateTime<FixedOffset>, // Ensure chrono's "serde" feature is enabled
}

// POST /transactions/fetch
async fn handle_fetch_transactions(
    manager: web::Data<TransactionManager>,
    req: web::Json<FetchTransactionsRequest>,
) -> ActixResult<impl Responder> {
    let (account, cookie) = manager.get_account_cookie().map_err(|e| {
        tracing::error!("Failed to get account/cookie: {:?}", e);
        ErrorInternalServerError(format!("Failed to get account/cookie: {}", e))
    })?;
    let client = RealMealFetcher::default().account(account).cookie(cookie);
    let results = tokio::task::spawn_blocking(move || {
        fetch(
            req.start_date,
            crate::libs::fetcher::MealFetcher::Real(client),
            |_t| Ok(()), // Assuming _t is a transaction type, progress callback
        )
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to spawn blocking task: {:?}", e);
        ErrorInternalServerError(format!("Failed to spawn blocking task: {}", e))
    })?;

    tracing::debug!("Fetched transactions: {:?}", results);
    match results {
        Ok(r) => { // Assuming r is Vec<Transaction> or compatible with manager.insert()
            manager.insert(&r).map_err(|e| {
                tracing::error!("Failed to insert transactions: {:?}", e);
                ErrorInternalServerError(format!("Failed to insert transactions: {}", e))
            })?;
            Ok(HttpResponse::Ok().finish())
        }
        Err(e) => {
            tracing::error!("Failed to fetch transactions: {:?}", e);
            Err(ErrorInternalServerError(format!(
                "Failed to fetch transactions: {}",
                e
            )))
        }
    }
}

#[derive(Deserialize, Serialize)] // Added Serialize for test usage
struct AccountUpdateRequest {
    account: String,
}

// PUT /config/account
async fn handle_update_account(
    manager: web::Data<TransactionManager>,
    req: web::Json<AccountUpdateRequest>,
) -> ActixResult<impl Responder> {
    to_actix_empty_response(manager.update_account(&req.account))
}

#[derive(Deserialize, Serialize)] // Added Serialize for test usage
struct CookieUpdateRequest { // This struct seems unused by any route
    cookie: String,
}

#[derive(Deserialize, Serialize)] // Added Serialize for test usage
struct HallticketUpdateRequest {
    hallticket: String,
}

// PUT /config/hallticket
async fn handle_update_hallticket(
    manager: web::Data<TransactionManager>,
    req: web::Json<HallticketUpdateRequest>,
) -> ActixResult<impl Responder> {
    to_actix_empty_response(manager.update_hallticket(&req.hallticket))
}

#[derive(Serialize, Deserialize)] // Added Deserialize for test usage
struct AccountCookieResponse {
    account: String,
    cookie: String,
}

// GET /config/account-cookie
async fn handle_get_account_cookie(
    manager: web::Data<TransactionManager>,
) -> ActixResult<impl Responder> {
    match manager.get_account_cookie() {
        Ok((account, cookie)) => Ok(web::Json(AccountCookieResponse { account, cookie })),
        Err(e) => {
            tracing::error!("Failed to get account/cookie: {:?}", e);
            let err_str = e.to_string();
            if err_str.contains("No account and cookie found")
                || err_str.contains("Account or cookie is empty")
            {
                Err(ErrorNotFound(err_str))
            } else {
                Err(ErrorInternalServerError(format!(
                    "Failed to get account/cookie: {}",
                    e
                )))
            }
        }
    }
}

/// CSV 导出请求参数
#[derive(Debug, Deserialize)]
struct CsvExportQuery {
    /// 商户名称筛选
    merchant: Option<String>,
    /// 最小金额筛选（正数）
    min_amount: Option<f64>,
    /// 最大金额筛选（正数）
    max_amount: Option<f64>,
    /// 开始日期筛选 YYYY-MM-DD
    time_start: Option<String>,
    /// 结束日期筛选 YYYY-MM-DD
    time_end: Option<String>,

    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "csv".to_string()
}

/// CSV 导出响应（JSON 格式）
#[derive(Serialize, Deserialize)]
struct CsvExportResponse {
    success: bool,
    count: usize,
    content: Option<String>,
    error: Option<String>,
}

// GET /export/csv
async fn handle_export_csv(
    manager: web::Data<TransactionManager>,
    query: web::Query<CsvExportQuery>,
) -> ActixResult<HttpResponse> {
    let params = query.into_inner();

    // 构建导出选项
    let options = ExportOptions {
        output: None,
        merchant: params.merchant,
        min_amount: params.min_amount,
        max_amount: params.max_amount,
        time_start: params.time_start,
        time_end: params.time_end,
    };

    // 执行导出
    match CsvExporter::export_to_string(&manager, &options) {
        Ok((csv_content, count)) => {
            if params.format == "json" {
                Ok(HttpResponse::Ok().json(CsvExportResponse {
                    success: true,
                    count,
                    content: Some(csv_content),
                    error: None,
                }))
            } else {
                let filename_str = generate_csv_filename(&options);
                let disposition = ContentDisposition {
                    disposition: DispositionType::Attachment,
                    parameters: vec![DispositionParam::Filename(filename_str)],
                };

                Ok(HttpResponse::Ok()
                    .content_type("text/csv; charset=utf-8")
                    .insert_header(disposition) // Use typed header
                    .body(csv_content))
            }
        }
        Err(e) => {
            tracing::error!("CSV export failed: {:?}", e);
            // It's often better to return an HTTP error status for API errors,
            // but returning 200 with error in JSON is also a valid choice.
            Ok(HttpResponse::Ok().json(CsvExportResponse {
                success: false,
                count: 0,
                content: None,
                error: Some(format!("Export failed: {}", e)),
            }))
        }
    }
}

/// 根据筛选条件生成文件名
fn generate_csv_filename(options: &ExportOptions) -> String {
    let mut parts = vec!["transactions".to_string()];

    if let Some(merchant) = &options.merchant {
        parts.push(format!("merchant_{}", merchant.replace([' ', '/'], "_"))); // Replaced space and slash for safety
    }

    if options.min_amount.is_some() || options.max_amount.is_some() {
        let min = options.min_amount.unwrap_or(0.0);
        let max = options.max_amount.unwrap_or(std::f64::INFINITY); // Use f64::INFINITY for a more logical upper bound
        parts.push(format!("amount_{:.2}_{:.2}", min, max)); // Format floats
    }

    if let Some(start) = &options.time_start {
        parts.push(format!("from_{}", start.replace("-", "")));
    }

    if let Some(end) = &options.time_end {
        parts.push(format!("to_{}", end.replace("-", "")));
    }

    format!("{}.csv", parts.join("_"))
}

// --- Actix App Configuration ---
pub fn config_routes(cfg: &mut web::ServiceConfig) {
    let scope = web::scope("/api")
        .service(
            web::scope("/transactions")
                .route("", web::get().to(handle_fetch_all_transactions))
                .route("/query", web::post().to(handle_fetch_filtered_transactions))
                .route("/count", web::get().to(handle_fetch_transaction_count))
                .route("/fetch", web::post().to(handle_fetch_transactions)),
        )
        .service(
            web::scope("/config")
                .route("/account", web::put().to(handle_update_account))
                .route("/hallticket", web::put().to(handle_update_hallticket))
                .route("/account-cookie", web::get().to(handle_get_account_cookie)),
        )
        // for csv export:
        .service(web::scope("/export").route("/csv", web::get().to(handle_export_csv)));
    cfg.service(scope);
}

#[cfg(test)]
mod tests {
    use super::*;
    // Assuming Transaction is accessible for deserialization, e.g. from crate::libs::transactions::Transaction
    use crate::libs::{
        fetcher, /* typically you'd mock fetcher or have test_utils for it */
        transactions::Transaction, // Ensure this is public and derives Deserialize/Serialize
    };
    use actix_web::{http::StatusCode, test, web::Data};
    // Note: `actix_http::Request` and `actix_web::dev::ServiceResponse/Service` might be needed if you directly use the service type
    // For `test::init_service` and `call_service` these are usually inferred or handled by Actix.

    // Helper to initialize TransactionManager for tests (in-memory DB)
    async fn setup_test_app() -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    > {
        let manager =
            TransactionManager::new(None).expect("Failed to create test TransactionManager");
        // Ensure get_mock_data returns Vec<Transaction> or a compatible type for insert
        let mock_data: Vec<Transaction> = fetcher::test_utils::get_mock_data(50);
        manager
            .insert(&mock_data)
            .expect("Failed to insert mock data");
        test::init_service(
            App::new()
                .app_data(Data::new(manager)) // Use app_data for shared state
                .configure(config_routes),
        )
        .await
    }

    #[actix_web::test]
    async fn test_insert_and_fetch_all_transactions() {
        let app = setup_test_app().await;

        // Fetch All
        let req_fetch = test::TestRequest::get()
            .uri("/api/transactions")
            .to_request();
        let resp_fetch = test::call_service(&app, req_fetch).await;
        assert_eq!(resp_fetch.status(), StatusCode::OK);

        let fetched_transactions: Vec<Transaction> = test::read_body_json(resp_fetch).await;
        assert_eq!(fetched_transactions.len(), 46); // This count depends on mock data and insert logic
    }

    #[actix_web::test]
    async fn test_fetch_filtered_transactions() {
        let app = setup_test_app().await;

        let filter = FilterOptions {
            merchant: Some("西14西15东12浴室".to_string()),
            ..Default::default()
        };
        let req = test::TestRequest::post()
            .uri("/api/transactions/query")
            .set_json(&filter)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let result: Vec<Transaction> = test::read_body_json(resp).await;
        assert_eq!(result.len(), 8); // This count depends on mock data and filter logic
    }

    #[actix_web::test]
    async fn test_fetch_transaction_count() {
        let app = setup_test_app().await;

        let req = test::TestRequest::get()
            .uri("/api/transactions/count")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let count: u64 = test::read_body_json(resp).await;
        assert_eq!(count, 46); // This count depends on mock data
    }

    #[actix_web::test]
    async fn test_config_routes() {
        let app = setup_test_app().await;

        // Update Account
        let acc_payload = AccountUpdateRequest {
            account: "test_user".to_string(),
        };
        let req_acc = test::TestRequest::put()
            .uri("/api/config/account")
            .set_json(&acc_payload)
            .to_request();
        let resp_acc = test::call_service(&app, req_acc).await;
        assert_eq!(resp_acc.status(), StatusCode::OK);

        // Update Hallticket
        let hallticket_payload = HallticketUpdateRequest {
            hallticket: "test_hallticket_val".to_string(),
        };
        let req_hallticket = test::TestRequest::put()
            .uri("/api/config/hallticket")
            .set_json(&hallticket_payload)
            .to_request();
        let resp_hallticket = test::call_service(&app, req_hallticket).await;
        assert_eq!(resp_hallticket.status(), StatusCode::OK);

        // Get Account Cookie again to see hallticket update
        let req_get_ac2 = test::TestRequest::get()
            .uri("/api/config/account-cookie")
            .to_request();
        let resp_get_ac2 = test::call_service(&app, req_get_ac2).await;
        assert_eq!(resp_get_ac2.status(), StatusCode::OK);
        let ac_response2: AccountCookieResponse = test::read_body_json(resp_get_ac2).await;
        assert_eq!(ac_response2.account, "test_user"); // Account should persist
        assert_eq!(ac_response2.cookie, "hallticket=test_hallticket_val"); // Depends on TransactionManager logic
    }

    #[actix_web::test]
    async fn test_get_account_cookie_not_found() {
        // Setup a new app with a fresh TransactionManager to ensure no pre-existing cookie data
        let manager =
            TransactionManager::new(None).expect("Failed to create test TransactionManager");
        // We don't call update_account or update_hallticket here
        let app = test::init_service(
            App::new()
                .app_data(Data::new(manager))
                .configure(config_routes),
        )
        .await;

        let req_get_ac = test::TestRequest::get()
            .uri("/api/config/account-cookie") // Corrected URI
            .to_request();
        let resp_get_ac = test::call_service(&app, req_get_ac).await;
        assert_eq!(resp_get_ac.status(), StatusCode::NOT_FOUND);
    }

    #[actix_web::test]
    async fn test_csv_export_json_format() {
        let app = setup_test_app().await;

        let req = test::TestRequest::get()
            .uri("/api/export/csv?format=json&merchant=西14西15东12浴室")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let result: CsvExportResponse = test::read_body_json(resp).await;
        assert!(result.success);
        assert_eq!(result.count, 8); // Depends on mock data
        assert!(result.content.is_some());
        assert!(result.error.is_none());
    }

    #[actix_web::test]
    async fn test_csv_export_file_download() {
        let app = setup_test_app().await;

        let req = test::TestRequest::get()
            .uri("/api/export/csv?format=csv&min_amount=10&max_amount=50")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let content_type = resp.headers().get("content-type").unwrap();
        assert!(content_type.to_str().unwrap().contains("text/csv"));

        let disposition = resp.headers().get("content-disposition").unwrap();
        let disposition_str = disposition.to_str().unwrap();
        assert!(disposition_str.contains("attachment"));
        // Expected filename based on generate_csv_filename and params: transactions_amount_10.00_50.00.csv
        assert!(disposition_str.contains("filename=\"transactions_amount_10.00_50.00.csv\""));


        let body = test::read_body(resp).await;
        let csv_content = String::from_utf8(body.to_vec()).unwrap();
        assert!(csv_content.contains("ID,Time,Amount,Merchant")); // Basic check for CSV header
    }

    #[actix_web::test]
    async fn test_csv_export_with_date_filter() {
        let app = setup_test_app().await;

        // Example dates, ensure mock data can satisfy this if count > 0 is critical
        let req = test::TestRequest::get()
            .uri("/api/export/csv?format=json&time_start=2022-01-01&time_end=2025-12-31")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);

        let result: CsvExportResponse = test::read_body_json(resp).await;
        assert!(result.success);
        // This assertion depends heavily on the mock data's date ranges.
        // If mock data is consistently within this range, count > 0 is fine.
        // For the default mock data, this should yield all 46 items.
        assert_eq!(result.count, 46, "Expected count for wide date range");
    }
}