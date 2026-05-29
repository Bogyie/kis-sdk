use axum::{
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use kis_sdk::{
    apis::domestic_stock::{
        domestic_stock_rest_endpoints, CashOrderRequest, CashOrderSide, InquireBalanceRequest,
        InquirePriceRequest, DOMESTIC_STOCK_REST_COLLECTIONS, DOMESTIC_STOCK_REST_ENDPOINT_COUNT,
    },
    config::Environment,
    contract::EnvironmentSupport,
    credentials::{Account, AppCredentials, SecretString},
    endpoint::{InventoryCatalog, InventoryEndpointSpec, InventoryRequest, OperationKind},
    error::KisError,
    fallback::FallbackPolicy,
    mock::MockServer,
    retry::RetryPolicy,
    KisClient,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
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

#[test]
fn inventory_catalog_addresses_every_official_endpoint_with_unique_operation_ids() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    assert_eq!(catalog.endpoint_count(), 338);

    for endpoint in catalog.endpoints() {
        assert!(
            catalog.endpoint(&endpoint.operation_id).is_some(),
            "{} must be addressable by operation id",
            endpoint.operation_id
        );
        assert!(
            !endpoint.operation_id.contains("unknown_collection"),
            "{} must use a curated collection slug",
            endpoint.operation_id
        );
    }
}

#[test]
fn inventory_operation_kind_uses_contract_kind_not_http_method_only() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    let realtime = catalog
        .endpoint("domestic_stock_realtime_quotation.post_tryitout_h0stcnt0")
        .expect("realtime operation exists");
    assert_eq!(realtime.operation_kind, OperationKind::Read);

    let cash_order = catalog
        .endpoint("domestic_stock_trading_account.post_domestic_stock_trading_order_cash")
        .expect("cash order operation exists");
    assert_eq!(cash_order.operation_kind, OperationKind::TradingMutation);

    let balance = catalog
        .endpoint("domestic_stock_trading_account.get_domestic_stock_trading_inquire_balance")
        .expect("balance operation exists");
    assert_eq!(balance.operation_kind, OperationKind::Read);
}

#[tokio::test]
async fn inventory_execute_calls_mocked_endpoint_by_operation_id() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let response = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await
        .expect("inventory-backed quote succeeds");

    assert!(response.is_success());
    assert!(response.output.is_some());

    server.shutdown().await;
}

#[test]
fn domestic_stock_rest_catalog_covers_listed_inventory_collections() {
    let endpoints = domestic_stock_rest_endpoints().expect("domestic stock REST catalog builds");

    assert_eq!(endpoints.len(), DOMESTIC_STOCK_REST_ENDPOINT_COUNT);

    let mut by_collection = BTreeMap::new();
    for endpoint in &endpoints {
        assert!(
            DOMESTIC_STOCK_REST_COLLECTIONS.contains(&endpoint.collection_name.as_str()),
            "{} must stay inside listed domestic stock REST collections",
            endpoint.operation_id
        );
        assert!(
            !endpoint.collection_name.contains("실시간시세"),
            "{} must not include realtime websocket coverage",
            endpoint.operation_id
        );
        *by_collection
            .entry(endpoint.collection_name.as_str())
            .or_insert(0usize) += 1;
    }

    assert_eq!(by_collection["[국내주식] 주문/계좌"], 23);
    assert_eq!(by_collection["[국내주식] 기본시세"], 22);
    assert_eq!(by_collection["[국내주식] ELW 시세"], 22);
    assert_eq!(by_collection["[국내주식] 업종/기타"], 14);
    assert_eq!(by_collection["[국내주식] 종목정보"], 26);
    assert_eq!(by_collection["[국내주식] 시세분석"], 29);
    assert_eq!(by_collection["[국내주식] 순위분석"], 22);
}

#[tokio::test]
async fn domestic_stock_rest_execute_covers_listed_inventory_against_mock_contract() {
    let endpoints = domestic_stock_rest_endpoints().expect("domestic stock REST catalog builds");
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let mut real_mock_successes = 0;
    let mut real_only_rejections = 0;

    for endpoint in &endpoints {
        let request = inventory_request_for_endpoint(endpoint);
        let result = client
            .execute_domestic_stock_rest::<serde_json::Value>(&endpoint.operation_id, request)
            .await;

        match endpoint.env_support {
            EnvironmentSupport::RealOnly => {
                assert!(
                    matches!(result, Err(KisError::UnsupportedEnvironment { .. })),
                    "{} must reject real-only endpoints in mock before network",
                    endpoint.operation_id
                );
                real_only_rejections += 1;
            }
            EnvironmentSupport::RealMock => {
                let response = result.unwrap_or_else(|error| {
                    panic!(
                        "{} should execute against mock: {error:?}",
                        endpoint.operation_id
                    )
                });
                assert!(response.is_success(), "{}", endpoint.operation_id);
                real_mock_successes += 1;
            }
        }
    }

    assert_eq!(real_mock_successes, 18);
    assert_eq!(real_only_rejections, 140);

    server.shutdown().await;
}

#[tokio::test]
async fn domestic_stock_rest_execute_rejects_out_of_scope_operation_ids() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_domestic_stock_rest::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new(),
        )
        .await
        .expect_err("realtime operation should stay out of REST coverage");

    assert!(matches!(error, KisError::Contract(_)));
}

fn inventory_request_for_endpoint(endpoint: &InventoryEndpointSpec) -> InventoryRequest {
    let mut request = InventoryRequest::new();

    let mut query = Map::new();
    for field in &endpoint.required_query {
        query.insert(field.clone(), Value::String(value_for_field(field)));
    }
    if endpoint.method == http::Method::GET {
        for field in &endpoint.required_body {
            query.insert(field.clone(), Value::String(value_for_field(field)));
        }
    }
    if !query.is_empty() {
        request = request.query(Value::Object(query));
    }

    if endpoint.method != http::Method::GET {
        let mut body = Map::new();
        for field in &endpoint.required_body {
            body.insert(field.clone(), Value::String(value_for_field(field)));
        }
        request = request.body(Value::Object(body));
    }

    for header in &endpoint.required_headers {
        if !is_auto_header(header) {
            request = request.header(header, "kis_mock_value");
        }
    }

    if endpoint
        .default_mock_tr_id
        .as_deref()
        .or(endpoint.default_real_tr_id.as_deref())
        .is_some_and(|tr_id| !is_single_tr_id(tr_id))
    {
        request = request.tr_id_override(first_tr_id(endpoint).unwrap_or("KISMOCK0000"));
    }

    request
}

fn value_for_field(field: &str) -> String {
    match field.to_ascii_uppercase().as_str() {
        "CANO" => "12345678".to_string(),
        "ACNT_PRDT_CD" => "01".to_string(),
        "PDNO" | "FID_INPUT_ISCD" | "MKSC_SHRN_ISCD" => "005930".to_string(),
        "ORD_QTY" => "1".to_string(),
        "ORD_UNPR" => "70000".to_string(),
        _ if field.to_ascii_uppercase().contains("DATE") || field.ends_with("_DT") => {
            "20260529".to_string()
        }
        _ => "0".to_string(),
    }
}

fn first_tr_id(endpoint: &InventoryEndpointSpec) -> Option<&str> {
    endpoint
        .default_mock_tr_id
        .as_deref()
        .or(endpoint.default_real_tr_id.as_deref())
        .and_then(|value| {
            value
                .split(|ch: char| !(ch.is_ascii_uppercase() || ch.is_ascii_digit()))
                .find(|candidate| !candidate.is_empty())
        })
}

fn is_auto_header(header: &str) -> bool {
    matches!(
        header.to_ascii_lowercase().as_str(),
        "authorization" | "appkey" | "appsecret" | "content-type" | "custtype" | "tr_id"
    )
}

fn is_single_tr_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

#[tokio::test]
async fn inventory_real_non_trading_post_is_not_blocked_by_live_trading_guard() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new()
                .header("approval_key", "test_approval_key")
                .header("tr_type", "1")
                .body(json!({
                    "tr_id": "H0STCNT0",
                    "tr_key": "005930"
                })),
        )
        .await
        .expect_err("unreachable local URL should fail at transport, not live trading guard");

    assert!(
        matches!(error, KisError::Transport(_)),
        "expected transport error after passing live trading guard, got {error:?}"
    );
}

#[tokio::test]
async fn inventory_execute_rejects_missing_required_query_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J"
            })),
        )
        .await
        .expect_err("missing query field should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn inventory_execute_requires_override_for_ambiguous_tr_id() {
    let body = json!({
        "CANO": "12345678",
        "ACNT_PRDT_CD": "01",
        "PDNO": "005930",
        "ORD_DVSN": "00",
        "ORD_QTY": "1",
        "ORD_UNPR": "70000"
    });
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_trading_account.post_domestic_stock_trading_order_cash",
            InventoryRequest::new().body(body),
        )
        .await
        .expect_err("ambiguous order TR ID should require override");

    assert!(matches!(error, KisError::AmbiguousTrId { .. }));
}

#[tokio::test]
async fn inventory_execute_rejects_missing_required_header_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new().body(json!({
                "tr_id": "H0STCNT0",
                "tr_key": "005930"
            })),
        )
        .await
        .expect_err("missing approval_key and tr_type should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn inventory_execute_rejects_real_only_endpoint_in_mock_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price_2",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await
        .expect_err("real-only endpoint should not run against mock");

    assert!(matches!(error, KisError::UnsupportedEnvironment { .. }));
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
