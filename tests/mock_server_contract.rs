use kis_sdk::{
    contract::{ContractInventory, EnvironmentSupport},
    mock::{route_count, MockServer},
};
use reqwest::StatusCode;
use serde_json::Value;

#[test]
fn bundled_contract_matches_bog_221_inventory_counts() {
    let inventory = ContractInventory::bundled().expect("bundled contract should parse");

    assert_eq!(
        inventory.source,
        "https://apiportal.koreainvestment.com/apiservice"
    );
    assert_eq!(inventory.checked_at, "2026-05-29 Asia/Seoul");
    assert_eq!(inventory.endpoint_count, 338);
    assert_eq!(inventory.collections.len(), 22);
    assert_eq!(inventory.endpoints.len(), inventory.endpoint_count);
    assert_eq!(route_count(&inventory), inventory.endpoint_count);

    let real_mock = inventory
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.env_support == EnvironmentSupport::RealMock)
        .count();
    let real_only = inventory
        .endpoints
        .iter()
        .filter(|endpoint| endpoint.env_support == EnvironmentSupport::RealOnly)
        .count();

    assert_eq!(real_mock, 46);
    assert_eq!(real_only, 292);
    assert!(inventory.endpoint("POST", "/oauth2/tokenP").is_some());
    assert!(inventory
        .endpoint("GET", "/uapi/domestic-stock/v1/quotations/inquire-price")
        .is_some());
}

#[tokio::test]
async fn mock_server_starts_responds_and_shutdowns() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = reqwest::Client::new();

    let token: Value = client
        .post(format!("{}/oauth2/tokenP", server.base_url()))
        .json(&serde_json::json!({
            "grant_type": "client_credentials",
            "appkey": "test_app_key",
            "appsecret": "test_app_secret"
        }))
        .send()
        .await
        .expect("token response")
        .json()
        .await
        .expect("token json");

    assert_eq!(token["token_type"], "Bearer");
    assert_eq!(token["expires_in"], 86400);

    let quote: Value = client
        .get(format!(
            "{}/uapi/domestic-stock/v1/quotations/inquire-price",
            server.base_url()
        ))
        .header(
            "authorization",
            format!("Bearer {}", token["access_token"].as_str().unwrap()),
        )
        .header("appkey", "test_app_key")
        .header("appsecret", "test_app_secret")
        .header("tr_id", "FHKST01010100")
        .header("custtype", "P")
        .send()
        .await
        .expect("quote response")
        .json()
        .await
        .expect("quote json");

    assert_eq!(quote["rt_cd"], "0");
    assert_eq!(
        quote["kis_mock"]["path"],
        "/uapi/domestic-stock/v1/quotations/inquire-price"
    );

    server.shutdown().await;
}

#[tokio::test]
async fn mock_server_exposes_error_rate_limit_and_unsupported_fixtures() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = reqwest::Client::new();

    let rate_limited = client
        .get(format!(
            "{}/uapi/domestic-stock/v1/quotations/inquire-price",
            server.base_url()
        ))
        .header("x-kis-mock-scenario", "rate-limit")
        .send()
        .await
        .expect("rate limit response");
    assert_eq!(rate_limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(rate_limited.headers().get("retry-after").unwrap(), "1");

    let unsupported: Value = client
        .get(format!(
            "{}/uapi/domestic-stock/v1/quotations/inquire-price-2",
            server.base_url()
        ))
        .send()
        .await
        .expect("unsupported response")
        .json()
        .await
        .expect("unsupported json");
    assert_eq!(unsupported["msg_cd"], "KIS_MOCK_UNSUPPORTED_ENVIRONMENT");
    assert_eq!(unsupported["detail"]["env_support"], "real_only");

    server.shutdown().await;
}
