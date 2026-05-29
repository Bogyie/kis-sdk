use axum::{
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use kis_sdk::{
    apis::domestic_stock::{
        CashOrderRequest, CashOrderSide, InquireBalanceRequest, InquirePriceRequest,
    },
    config::Environment,
    credentials::{Account, AppCredentials, SecretString},
    endpoint::OperationKind,
    error::KisError,
    fallback::FallbackPolicy,
    mock::MockServer,
    retry::RetryPolicy,
    KisClient,
};
use serde_json::json;
use tokio::{net::TcpListener, task::JoinHandle};

#[tokio::test]
async fn client_calls_mocked_domestic_stock_read_and_order_slice() {
    let server = MockServer::start().await.expect("mock server starts");
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect("quote succeeds");
    assert!(quote.is_success());
    assert!(quote.output.is_some());

    let balance = client
        .inquire_domestic_stock_balance(&InquireBalanceRequest::new(&account))
        .await
        .expect("balance succeeds");
    assert!(balance.is_success());

    let order = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 1, 70000),
        )
        .await
        .expect("mock cash order succeeds");
    assert!(order.is_success());

    server.shutdown().await;
}

#[tokio::test]
async fn client_uses_static_token_and_redacts_secret_values() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let result = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await;
    assert!(result.is_ok());

    let secret = SecretString::new("very_sensitive_value");
    let debug = format!("{secret:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("very_sensitive_value"));

    server.shutdown().await;
}

#[test]
fn retry_and_fallback_policies_are_explicit_options() {
    let disabled = RetryPolicy::disabled();
    assert_eq!(disabled.max_attempts(), 1);
    assert!(!disabled.should_retry(
        "GET",
        OperationKind::Read,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));

    let reads = RetryPolicy::conservative_reads();
    assert!(reads.should_retry(
        "GET",
        OperationKind::Read,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));
    assert!(!reads.should_retry(
        "POST",
        OperationKind::TradingMutation,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));
    assert!(!reads.should_retry(
        "POST",
        OperationKind::TradingMutation,
        &KisError::Transport("ambiguous write failure".to_string()),
        1,
    ));

    let fallback = FallbackPolicy::real_to_mock_reads();
    assert!(fallback.allows_real_to_mock_read("GET"));
    assert!(!fallback.allows_real_to_mock_read("POST"));
    assert!(!fallback.allows_real_to_mock("POST", OperationKind::TradingMutation));
}

#[tokio::test]
async fn real_to_mock_fallback_is_opt_in_and_read_only() {
    let (base_url, server_task) = start_fallback_header_asserting_server()
        .await
        .expect("fallback assertion server starts");
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_base_url(base_url.clone())
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("primary_app_key", "primary_app_secret"))
        .static_bearer_token("primary_access_token")
        .fallback_credentials(AppCredentials::new(
            "fallback_app_key",
            "fallback_app_secret",
        ))
        .fallback_static_bearer_token("fallback_access_token")
        .build()
        .expect("client builds");

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect("quote falls back to mock");

    let fallback = quote.execution.fallback.expect("fallback is visible");
    assert_eq!(fallback.from_environment, Environment::Real);
    assert_eq!(fallback.to_environment, Environment::Mock);
    assert_eq!(fallback.to_base_url, base_url);

    server_task.abort();
}

#[tokio::test]
async fn real_to_mock_fallback_requires_separate_fallback_credentials() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_base_url("http://127.0.0.1:9")
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("primary_app_key", "primary_app_secret"))
        .static_bearer_token("primary_access_token")
        .build()
        .expect("client builds");

    let error = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect_err("fallback must not reuse primary credentials");

    assert!(matches!(error, KisError::MissingFallbackCredentials));
}

#[tokio::test]
async fn real_cash_order_is_blocked_before_network_without_live_trading_guard() {
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 1, 70000),
        )
        .await
        .expect_err("real order should be blocked locally");

    assert!(matches!(error, KisError::LiveTradingDisabled { .. }));
}

#[tokio::test]
async fn cash_order_rejects_invalid_quantity_before_network() {
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 0, 70000),
        )
        .await
        .expect_err("zero quantity should be rejected locally");

    assert!(matches!(error, KisError::Validation(_)));
}

async fn start_fallback_header_asserting_server() -> std::io::Result<(String, JoinHandle<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let app = Router::new().route(
        "/uapi/domestic-stock/v1/quotations/inquire-price",
        get(|headers: HeaderMap| async move {
            let app_key = header(&headers, "appkey");
            let app_secret = header(&headers, "appsecret");
            let authorization = header(&headers, "authorization");

            if app_key == Some("primary_app_key")
                || app_secret == Some("primary_app_secret")
                || authorization == Some("Bearer primary_access_token")
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "rt_cd": "1",
                        "msg_cd": "PRIMARY_CREDENTIAL_LEAK",
                        "msg1": "primary credentials reached fallback target"
                    })),
                );
            }

            if app_key != Some("fallback_app_key")
                || app_secret != Some("fallback_app_secret")
                || authorization != Some("Bearer fallback_access_token")
            {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "rt_cd": "1",
                        "msg_cd": "MISSING_FALLBACK_CREDENTIAL",
                        "msg1": "fallback credentials were not used"
                    })),
                );
            }

            (
                StatusCode::OK,
                Json(json!({
                    "rt_cd": "0",
                    "msg_cd": "KIS_MOCK_OK",
                    "msg1": "mock success",
                    "output": {
                        "stck_prpr": "kis_mock_value"
                    }
                })),
            )
        }),
    );

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok((format!("http://{addr}"), task))
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}
