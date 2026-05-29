use kis_sdk::{
    contract::{ContractEndpoint, ContractInventory, EnvironmentSupport},
    mock::{route_count, MockServer},
};
use reqwest::{Method, RequestBuilder, StatusCode};
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

    for scenario in [
        "unauthorized",
        "rate-limit",
        "retryable-500",
        "provider-error",
    ] {
        let response = client
            .get(format!(
                "{}/uapi/domestic-stock/v1/quotations/inquire-price-2",
                server.base_url()
            ))
            .header("x-kis-mock-scenario", scenario)
            .send()
            .await
            .expect("unsupported scenario response");

        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        let unsupported: Value = response.json().await.expect("unsupported scenario json");
        assert_eq!(
            unsupported["msg_cd"], "KIS_MOCK_UNSUPPORTED_ENVIRONMENT",
            "scenario {scenario} must not override real_only support"
        );
        assert_eq!(unsupported["detail"]["env_support"], "real_only");
    }

    let wrong_method: Value = client
        .post(format!(
            "{}/uapi/domestic-stock/v1/quotations/inquire-price",
            server.base_url()
        ))
        .send()
        .await
        .expect("method not allowed response")
        .json()
        .await
        .expect("method not allowed json");

    assert_eq!(wrong_method["msg_cd"], "KIS_MOCK_METHOD_NOT_ALLOWED");
    assert_eq!(wrong_method["detail"]["actual"], "POST");

    server.shutdown().await;
}

#[tokio::test]
async fn mock_server_routes_every_bundled_contract_endpoint() {
    let inventory = ContractInventory::bundled().expect("bundled contract should parse");
    let server = MockServer::start().await.expect("mock server starts");
    let client = reqwest::Client::new();

    let mut real_mock_successes = 0;
    let mut real_only_rejections = 0;
    let mut auth_successes = 0;

    for endpoint in &inventory.endpoints {
        let method = Method::from_bytes(endpoint.method.as_bytes()).expect("valid method");
        let mut request = client.request(method, format!("{}{}", server.base_url(), endpoint.path));
        request = attach_contract_headers(request, endpoint);

        if endpoint.method == "POST" {
            request = request.json(&serde_json::json!({}));
        }

        let response = request.send().await.expect("contract endpoint response");
        let status = response.status();
        let body: Value = response.json().await.expect("contract endpoint json");

        if endpoint.is_auth() {
            assert!(
                status.is_success(),
                "{} {} auth endpoint should succeed: {body}",
                endpoint.method,
                endpoint.path
            );
            auth_successes += 1;
        } else if endpoint.env_support == EnvironmentSupport::RealOnly {
            assert_eq!(
                status,
                StatusCode::NOT_IMPLEMENTED,
                "{} {} real-only endpoint should be explicitly unsupported",
                endpoint.method,
                endpoint.path
            );
            assert_eq!(body["msg_cd"], "KIS_MOCK_UNSUPPORTED_ENVIRONMENT");
            real_only_rejections += 1;
        } else {
            assert!(
                status.is_success(),
                "{} {} real+mock endpoint should succeed: {body}",
                endpoint.method,
                endpoint.path
            );
            assert_eq!(body["rt_cd"], "0");
            assert_eq!(body["kis_mock"]["path"], endpoint.path);
            real_mock_successes += 1;
        }
    }

    assert_eq!(auth_successes, 3);
    assert_eq!(real_mock_successes, 43);
    assert_eq!(real_only_rejections, 292);

    server.shutdown().await;
}

fn attach_contract_headers(request: RequestBuilder, endpoint: &ContractEndpoint) -> RequestBuilder {
    if endpoint.is_auth() || endpoint.env_support == EnvironmentSupport::RealOnly {
        return request;
    }

    let mut request = request
        .header("authorization", "Bearer test_access_token")
        .header("appkey", "test_app_key")
        .header("appsecret", "test_app_secret")
        .header("custtype", "P");

    if let Some(tr_id) = endpoint
        .expected_mock_tr_id()
        .filter(|tr_id| is_single_tr_id(tr_id))
    {
        request = request.header("tr_id", tr_id);
    } else if endpoint
        .required_headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("tr_id"))
    {
        request = request.header("tr_id", "KISMOCK0000");
    }

    for header in &endpoint.required_headers {
        let lower = header.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "authorization"
                | "appkey"
                | "appsecret"
                | "custtype"
                | "tr_id"
                | "content-type"
                | "content-length"
        ) {
            continue;
        }
        request = request.header(header, "kis_mock_value");
    }

    request
}

fn is_single_tr_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}
