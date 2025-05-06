//! # XJTU MealFlow API Documentation
//!
//! This document outlines the available API endpoints for managing meal transactions and configurations.
//!
//! ## Transaction Endpoints
//!
//! Base path: `/transactions`
//!
//! ### `POST /transactions`
//!
//! Insert a list of new transactions into the database.
//!
//! **Request Body:** `Vec<Transaction>`
//!
//! ```json
//! [
//!   {
//!     "amount": 10.5,
//!     "merchant": "Campus Cafeteria",
//!     "time": "2024-01-15T10:30:00+08:00"
//!   }
//! ]
//! ```
//!
//! **Responses:**
//! - `200 OK`: Transactions inserted successfully.
//! - `500 Internal Server Error`: If an error occurs during insertion.
//!
//! ### `GET /transactions`
//!
//! Fetch all stored transactions.
//!
//! **Responses:**
//! - `200 OK`: Returns `Vec<Transaction>`.
//! - `500 Internal Server Error`: If an error occurs during fetching.
//!
//! ### `POST /transactions/query`
//!
//! Fetch transactions based on filter criteria.
//!
//! **Request Body:** `FilterOptions`
//!
//! ```json
//! {
//!   "merchant": "Campus Cafeteria",
//!   "start_date": "2024-01-01T00:00:00+08:00",
//!   "end_date": "2024-01-31T23:59:59+08:00",
//!   "min_amount": 5.0,
//!   "max_amount": 20.0
//! }
//! ```
//!
//! **Responses:**
//! - `200 OK`: Returns `Vec<Transaction>` matching the filters.
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `GET /transactions/count`
//!
//! Get the total number of stored transactions.
//!
//! **Responses:**
//! - `200 OK`: Returns a `u64` count.
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `DELETE /transactions`
//!
//! Clear all transactions from the database.
//!
//! **Responses:**
//! - `200 OK`: Transactions cleared successfully.
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `POST /transactions/fetch`
//!
//! Fetch new transactions from the external meal card system starting from a given date and store them.
//!
//! **Request Body:** `FetchTransactionsRequest`
//!
//! ```json
//! {
//!   "start_date": "2024-03-01T00:00:00+08:00"
//! }
//! ```
//!
//! **Responses:**
//! - `200 OK`: Transactions fetched and stored successfully.
//! - `500 Internal Server Error`: If an error occurs during fetching or storing, or if account/cookie are not set.
//!
//! ## Configuration Endpoints
//!
//! Base path: `/config`
//!
//! ### `PUT /config/account`
//!
//! Update the account username.
//!
//! **Request Body:** `AccountUpdateRequest`
//!
//! ```json
//! {
//!   "account": "new_username"
//! }
//! ```
//!
//! **Responses:**
//! - `200 OK`: Account updated successfully.
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `PUT /config/cookie`
//!
//! Update the authentication cookie.
//!
//! **Request Body:** `CookieUpdateRequest`
//!
//! ```json
//! {
//!   "cookie": "new_cookie_value"
//! }
//! ```
//!
//! **Responses:**
//! - `200 OK`: Cookie updated successfully.
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `PUT /config/hallticket`
//!
//! Update the hallticket value (which typically forms part of the cookie).
//!
//! **Request Body:** `HallticketUpdateRequest`
//!
//! ```json
//! {
//!   "hallticket": "new_hallticket_value"
//! }
//! ```
//!
//! **Responses:**
//! - `200 OK`: Hallticket updated successfully (and reflected in the stored cookie).
//! - `500 Internal Server Error`: If an error occurs.
//!
//! ### `GET /config/account-cookie`
//!
//! Get the currently stored account and cookie.
//!
//! **Responses:**
//! - `200 OK`: Returns `AccountCookieResponse`.
//!   ```json
//!   {
//!     "account": "current_username",
//!     "cookie": "current_cookie_value"
//!   }
//!   ```
//! - `404 Not Found`: If account or cookie is not found or is empty.
//! - `500 Internal Server Error`: If a general error occurs.
//!
//! ### `GET /config/account-cookie/allow-empty`
//!
//! Get the currently stored account and cookie. This endpoint is intended to allow retrieval even if values might be empty,
//! but current implementation might still return `404 Not Found` if the database record itself is missing.
//!
//! **Responses:**
//! - `200 OK`: Returns `AccountCookieResponse` (values might be empty strings if set as such).
//!   ```json
//!   {
//!     "account": "current_username_or_empty",
//!     "cookie": "current_cookie_value_or_empty"
//!   }
//!   ```
//! - `404 Not Found`: If the underlying database record for account/cookie is not found.
//! - `500 Internal Server Error`: If a general error occurs.

use actix_web::{
    HttpResponse, Responder, Result as ActixResult,
    error::{ErrorInternalServerError, ErrorNotFound},
    web,
};
use chrono::{DateTime, FixedOffset};
use serde::{Deserialize, Serialize};

// Assuming Transaction and FilterOptions are correctly defined and made public in libs::transactions
// and derive Serialize and Deserialize.
use crate::libs::{
    fetcher::{RealMealFetcher, fetch},
    transactions::{FilterOptions, Transaction, TransactionManager},
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

// POST /transactions
async fn handle_insert_transactions(
    manager: web::Data<TransactionManager>,
    transactions: web::Json<Vec<Transaction>>,
) -> ActixResult<impl Responder> {
    to_actix_empty_response(manager.insert(&transactions.into_inner()))
}

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

// DELETE /transactions
async fn handle_clear_transactions(
    manager: web::Data<TransactionManager>,
) -> ActixResult<impl Responder> {
    match manager.clear_db() {
        Ok(_) => Ok(HttpResponse::Ok().finish()),
        Err(e) => {
            tracing::error!("Failed to clear database: {:?}", e);
            Err(ErrorInternalServerError(format!(
                "Failed to clear database: {}",
                e
            )))
        }
    }
}

#[derive(Deserialize, Serialize)]
struct FetchTransactionsRequest {
    start_date: DateTime<FixedOffset>,
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
    let results = tokio::task::spawn_blocking(move||fetch(
        (&req.start_date).clone(),
        crate::libs::fetcher::MealFetcher::Real(client),
        |_t| Ok(()),
    )).await.map_err(|e| {
        tracing::error!("Failed to spawn blocking task: {:?}", e);
        ErrorInternalServerError(format!("Failed to spawn blocking task: {}", e))
    })?;
    tracing::debug!("Fetched transactions: {:?}", results);
    match results {
        Ok(r) => {
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
struct CookieUpdateRequest {
    cookie: String,
}

// PUT /config/cookie
async fn handle_update_cookie(
    manager: web::Data<TransactionManager>,
    req: web::Json<CookieUpdateRequest>,
) -> ActixResult<impl Responder> {
    to_actix_empty_response(manager.update_cookie(&req.cookie))
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
            // Check if the error indicates "No account and cookie found" or "Account or cookie is empty"
            // This requires inspecting the error message, which is fragile.
            // A more robust way would be to have specific error types from TransactionManager.
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

// GET /config/account-cookie/allow-empty
async fn handle_get_account_cookie_may_empty(
    manager: web::Data<TransactionManager>,
) -> ActixResult<impl Responder> {
    match manager.get_account_cookie_may_empty() {
        Ok((account, cookie)) => Ok(web::Json(AccountCookieResponse { account, cookie })),
        Err(e) => {
            tracing::error!("Failed to get account/cookie (allow empty): {:?}", e);
            let err_str = e.to_string();
            if err_str.contains("No account and cookie found") {
                // This specific error from bail!
                // If "No account and cookie found", it implies the DB table might be empty or record missing.
                // Depending on desired API behavior, this could be an Ok with empty strings, or still an error.
                // For now, let's treat "No account and cookie found" as a specific not found case.
                Err(ErrorNotFound(err_str))
            } else {
                Err(ErrorInternalServerError(format!(
                    "Failed to get account/cookie (allow empty): {}",
                    e
                )))
            }
        }
    }
}

// --- Actix App Configuration ---
pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/transactions")
            .route("", web::post().to(handle_insert_transactions))
            .route("", web::get().to(handle_fetch_all_transactions))
            .route("/query", web::post().to(handle_fetch_filtered_transactions))
            .route("/count", web::get().to(handle_fetch_transaction_count))
            .route("", web::delete().to(handle_clear_transactions))
            .route("/fetch", web::post().to(handle_fetch_transactions)),
    )
    .service(
        web::scope("/config")
            .route("/account", web::put().to(handle_update_account))
            .route("/cookie", web::put().to(handle_update_cookie))
            .route("/hallticket", web::put().to(handle_update_hallticket))
            .route("/account-cookie", web::get().to(handle_get_account_cookie))
            .route(
                "/account-cookie/allow-empty",
                web::get().to(handle_get_account_cookie_may_empty),
            ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libs::transactions::{
        FilterOptions, OFFSET_UTC_PLUS8, Transaction, TransactionManager,
    };
    use actix_web::{App, http::StatusCode, test, web::Data};
    use chrono::TimeZone;

    // Helper to initialize TransactionManager for tests (in-memory DB)
    async fn setup_test_app() -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    > {
        let manager =
            TransactionManager::new(None).expect("Failed to create test TransactionManager");
        test::init_service(
            App::new()
                .app_data(Data::new(manager)) // Use app_data for shared state
                .configure(config_routes),
        )
        .await
    }

    fn create_sample_transactions() -> Vec<Transaction> {
        vec![
            Transaction::new(
                10.0,
                "Merchant A".to_string(),
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2024, 1, 1, 10, 0, 0)
                    .unwrap(),
            ),
            Transaction::new(
                20.0,
                "Merchant B".to_string(),
                OFFSET_UTC_PLUS8
                    .with_ymd_and_hms(2024, 1, 2, 11, 0, 0)
                    .unwrap(),
            ),
        ]
    }

    #[actix_web::test]
    async fn test_insert_and_fetch_all_transactions() {
        let app = setup_test_app().await;
        let transactions_payload = create_sample_transactions();

        // Insert
        let req_insert = test::TestRequest::post()
            .uri("/transactions")
            .set_json(&transactions_payload)
            .to_request();
        let resp_insert = test::call_service(&app, req_insert).await;
        assert_eq!(resp_insert.status(), StatusCode::OK);

        // Fetch All
        let req_fetch = test::TestRequest::get().uri("/transactions").to_request();
        let resp_fetch = test::call_service(&app, req_fetch).await;
        assert_eq!(resp_fetch.status(), StatusCode::OK);

        let fetched_transactions: Vec<Transaction> = test::read_body_json(resp_fetch).await;
        assert_eq!(fetched_transactions.len(), 2);
        // Note: Order might not be guaranteed by fetch_all, so check presence or sort before deep comparison.
        // For simplicity, assuming order based on insertion for this test.
        assert_eq!(fetched_transactions[0].merchant, "Merchant A");
        assert_eq!(fetched_transactions[1].merchant, "Merchant B");
    }

    #[actix_web::test]
    async fn test_fetch_filtered_transactions() {
        let app = setup_test_app().await;
        let transactions_payload = create_sample_transactions();
        let _ = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/transactions")
                .set_json(&transactions_payload)
                .to_request(),
        )
        .await; // Insert data

        let filter = FilterOptions {
            merchant: Some("Merchant A".to_string()),
            ..Default::default()
        };
        let req = test::TestRequest::post()
            .uri("/transactions/query")
            .set_json(&filter)
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let result: Vec<Transaction> = test::read_body_json(resp).await;
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].merchant, "Merchant A");
    }

    #[actix_web::test]
    async fn test_fetch_transaction_count() {
        let app = setup_test_app().await;
        let transactions_payload = create_sample_transactions();
        let _ = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/transactions")
                .set_json(&transactions_payload)
                .to_request(),
        )
        .await; // Insert data

        let req = test::TestRequest::get()
            .uri("/transactions/count")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let count: u64 = test::read_body_json(resp).await;
        assert_eq!(count, 2);
    }

    #[actix_web::test]
    async fn test_clear_transactions() {
        let app = setup_test_app().await;
        let transactions_payload = create_sample_transactions();
        let _ = test::call_service(
            &app,
            test::TestRequest::post()
                .uri("/transactions")
                .set_json(&transactions_payload)
                .to_request(),
        )
        .await; // Insert data

        // Verify count before delete
        let resp_count_before: u64 = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/transactions/count")
                    .to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(resp_count_before, 2);

        // Delete
        let req_delete = test::TestRequest::delete()
            .uri("/transactions")
            .to_request();
        let resp_delete = test::call_service(&app, req_delete).await;
        assert_eq!(resp_delete.status(), StatusCode::OK);

        // Verify count after delete
        let resp_count_after: u64 = test::read_body_json(
            test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/transactions/count")
                    .to_request(),
            )
            .await,
        )
        .await;
        assert_eq!(resp_count_after, 0);
    }

    #[actix_web::test]
    async fn test_config_routes() {
        let app = setup_test_app().await;

        // Update Account
        let acc_payload = AccountUpdateRequest {
            account: "test_user".to_string(),
        };
        let req_acc = test::TestRequest::put()
            .uri("/config/account")
            .set_json(&acc_payload)
            .to_request();
        let resp_acc = test::call_service(&app, req_acc).await;
        assert_eq!(resp_acc.status(), StatusCode::OK);

        // Update Cookie
        let cookie_payload = CookieUpdateRequest {
            cookie: "test_cookie_val".to_string(),
        };
        let req_cookie = test::TestRequest::put()
            .uri("/config/cookie")
            .set_json(&cookie_payload)
            .to_request();
        let resp_cookie = test::call_service(&app, req_cookie).await;
        assert_eq!(resp_cookie.status(), StatusCode::OK);

        // Get Account Cookie
        let req_get_ac = test::TestRequest::get()
            .uri("/config/account-cookie")
            .to_request();
        let resp_get_ac = test::call_service(&app, req_get_ac).await;
        assert_eq!(resp_get_ac.status(), StatusCode::OK);
        let ac_response: AccountCookieResponse = test::read_body_json(resp_get_ac).await;
        assert_eq!(ac_response.account, "test_user");
        assert_eq!(ac_response.cookie, "test_cookie_val");

        // Update Hallticket
        let hallticket_payload = HallticketUpdateRequest {
            hallticket: "test_hallticket_val".to_string(),
        };
        let req_hallticket = test::TestRequest::put()
            .uri("/config/hallticket")
            .set_json(&hallticket_payload)
            .to_request();
        let resp_hallticket = test::call_service(&app, req_hallticket).await;
        assert_eq!(resp_hallticket.status(), StatusCode::OK);

        // Get Account Cookie again to see hallticket update
        let req_get_ac2 = test::TestRequest::get()
            .uri("/config/account-cookie")
            .to_request();
        let resp_get_ac2 = test::call_service(&app, req_get_ac2).await;
        assert_eq!(resp_get_ac2.status(), StatusCode::OK);
        let ac_response2: AccountCookieResponse = test::read_body_json(resp_get_ac2).await;
        assert_eq!(ac_response2.account, "test_user"); // Account should persist
        assert_eq!(ac_response2.cookie, "hallticket=test_hallticket_val");

        // Test get_account_cookie_may_empty
        // First, let's simulate a state where no cookie/account is set by re-initializing manager or clearing cookies table.
        // For simplicity in this test, we'll assume the previous state.
        // If we had a clear_cookies method in TransactionManager, we'd call it.
        // The current get_account_cookie_may_empty will return the values set above.
        let req_get_ac_empty_ok = test::TestRequest::get()
            .uri("/config/account-cookie/allow-empty")
            .to_request();
        let resp_get_ac_empty_ok = test::call_service(&app, req_get_ac_empty_ok).await;
        assert_eq!(resp_get_ac_empty_ok.status(), StatusCode::OK);
        let ac_response_empty_ok: AccountCookieResponse =
            test::read_body_json(resp_get_ac_empty_ok).await;
        assert_eq!(ac_response_empty_ok.account, "test_user");
        assert_eq!(
            ac_response_empty_ok.cookie,
            "hallticket=test_hallticket_val"
        );

        // To properly test the "empty" case leading to ErrorNotFound for get_account_cookie,
        // we would need to ensure the DB starts truly empty for that specific call.
        // The current TransactionManager::new(None) initializes an empty DB, but cookies table might get populated by other tests.
        // A dedicated test with a fresh manager instance for this scenario would be more robust.
    }

    #[actix_web::test]
    async fn test_get_account_cookie_not_found() {
        // Setup a new app with a fresh TransactionManager to ensure no pre-existing cookie data
        let manager =
            TransactionManager::new(None).expect("Failed to create test TransactionManager");
        // We don't call update_account or update_cookie here
        let app = test::init_service(
            App::new()
                .app_data(Data::new(manager))
                .configure(config_routes),
        )
        .await;

        let req_get_ac = test::TestRequest::get()
            .uri("/config/account-cookie")
            .to_request();
        let resp_get_ac = test::call_service(&app, req_get_ac).await;
        // Expecting Not Found because get_account_cookie bails if empty
        assert_eq!(resp_get_ac.status(), StatusCode::NOT_FOUND);

        let req_get_ac_empty = test::TestRequest::get()
            .uri("/config/account-cookie/allow-empty")
            .to_request();
        let resp_get_ac_empty = test::call_service(&app, req_get_ac_empty).await;
        // Expecting Not Found because get_account_cookie_may_empty bails if "No account and cookie found"
        assert_eq!(resp_get_ac_empty.status(), StatusCode::NOT_FOUND);
    }
}
