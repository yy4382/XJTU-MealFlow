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
    transactions::{FilterOptions, TransactionManager},
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
    let results = tokio::task::spawn_blocking(move || {
        fetch(
            (&req.start_date).clone(),
            crate::libs::fetcher::MealFetcher::Real(client),
            |_t| Ok(()),
        )
    })
    .await
    .map_err(|e| {
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
        );
    cfg.service(scope);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::libs::{
        fetcher,
        transactions::{FilterOptions, Transaction, TransactionManager},
    };
    use actix_web::{App, http::StatusCode, test, web::Data};

    // Helper to initialize TransactionManager for tests (in-memory DB)
    async fn setup_test_app() -> impl actix_web::dev::Service<
        actix_http::Request,
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
    > {
        let manager =
            TransactionManager::new(None).expect("Failed to create test TransactionManager");
        let mock_data = fetcher::test_utils::get_mock_data(50);
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
        assert_eq!(fetched_transactions.len(), 46);
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
        assert_eq!(result.len(), 8);
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
        assert_eq!(count, 46);
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
        assert_eq!(ac_response2.cookie, "hallticket=test_hallticket_val");
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
    }
}
