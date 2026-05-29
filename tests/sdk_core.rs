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
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_base_url(server.base_url())
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect("quote falls back to mock");

    let fallback = quote.execution.fallback.expect("fallback is visible");
    assert_eq!(fallback.from_environment, Environment::Real);
    assert_eq!(fallback.to_environment, Environment::Mock);
    assert_eq!(fallback.to_base_url, server.base_url());

    server.shutdown().await;
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
