use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Json, Router,
};
use serde_json::{json, Map, Value};
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};

use crate::contract::{ContractEndpoint, ContractInventory, EnvironmentSupport, RouteKey};

#[derive(Clone)]
struct MockState {
    routes: Arc<HashMap<RouteKey, ContractEndpoint>>,
}

pub struct MockServer {
    addr: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl MockServer {
    pub async fn start() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let addr = listener.local_addr()?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let app = app(ContractInventory::bundled()?);

        let task = tokio::spawn(async move {
            let server = axum::serve(listener, app).with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            });

            if let Err(error) = server.await {
                eprintln!("kis mock server stopped with error: {error}");
            }
        });

        Ok(Self {
            addr,
            shutdown: Some(shutdown_tx),
            task,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub async fn shutdown(mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let _ = self.task.await;
    }
}

pub fn app(inventory: ContractInventory) -> Router {
    let state = MockState {
        routes: Arc::new(inventory.route_index()),
    };

    Router::new()
        .route("/*path", any(handle))
        .fallback(handle)
        .with_state(state)
}

pub fn route_count(inventory: &ContractInventory) -> usize {
    inventory.route_index().len()
}

async fn handle(State(state): State<MockState>, request: Request<Body>) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    let Some(endpoint) = state.routes.get(&RouteKey {
        method: method.to_string(),
        path: path.clone(),
    }) else {
        return error_response(
            StatusCode::NOT_FOUND,
            "KIS_MOCK_UNKNOWN_ENDPOINT",
            json!({ "method": method.as_str(), "path": path }),
        );
    };

    if method.as_str() != endpoint.method {
        return error_response(
            StatusCode::METHOD_NOT_ALLOWED,
            "KIS_MOCK_METHOD_NOT_ALLOWED",
            json!({ "expected": endpoint.method, "actual": method.as_str() }),
        );
    }

    match scenario(request.headers()) {
        Some(MockScenario::Unauthorized) => {
            return provider_envelope(StatusCode::UNAUTHORIZED, "1", "EGW00123", "unauthorized")
        }
        Some(MockScenario::RateLimit) => {
            return with_retry_after(provider_envelope(
                StatusCode::TOO_MANY_REQUESTS,
                "1",
                "EGW42900",
                "rate limit exceeded",
            ))
        }
        Some(MockScenario::RetryableServerError) => {
            return provider_envelope(
                StatusCode::SERVICE_UNAVAILABLE,
                "1",
                "EGW50300",
                "temporary upstream failure",
            )
        }
        Some(MockScenario::ProviderError) => {
            return provider_envelope(StatusCode::OK, "1", "KIS_MOCK_ERROR", "provider error")
        }
        None => {}
    }

    if endpoint.is_auth() {
        return auth_response(endpoint.path.as_str());
    }

    if endpoint.env_support == EnvironmentSupport::RealOnly {
        return error_response(
            StatusCode::NOT_IMPLEMENTED,
            "KIS_MOCK_UNSUPPORTED_ENVIRONMENT",
            json!({
                "endpoint_id": endpoint.id,
                "method": endpoint.method,
                "path": endpoint.path,
                "env_support": "real_only"
            }),
        );
    }

    if let Err(error) = validate_headers(endpoint, request.headers()) {
        return error_response(StatusCode::BAD_REQUEST, "KIS_MOCK_INVALID_HEADERS", error);
    }

    Json(success_body(endpoint)).into_response()
}

fn auth_response(path: &str) -> Response {
    match path {
        "/oauth2/tokenP" => Json(json!({
            "access_token": "kis_mock_access_token",
            "access_token_token_expired": "2099-12-31 23:59:59",
            "token_type": "Bearer",
            "expires_in": 86400
        }))
        .into_response(),
        "/oauth2/Approval" => {
            Json(json!({ "approval_key": "kis_mock_approval_key" })).into_response()
        }
        "/oauth2/revokeP" => Json(json!({ "code": 200, "message": "revoked" })).into_response(),
        _ => error_response(
            StatusCode::NOT_FOUND,
            "KIS_MOCK_UNKNOWN_AUTH_ENDPOINT",
            json!({}),
        ),
    }
}

fn validate_headers(endpoint: &ContractEndpoint, headers: &HeaderMap) -> Result<(), Value> {
    let mut missing = Vec::new();

    for required in &endpoint.required_headers {
        let name = required.to_ascii_lowercase();
        if matches!(name.as_str(), "content-type" | "content-length") {
            continue;
        }
        if !headers.contains_key(name.as_str()) {
            missing.push(required.clone());
        }
    }

    if !missing.is_empty() {
        return Err(json!({ "missing": missing }));
    }

    if endpoint
        .required_headers
        .iter()
        .any(|header| header.eq_ignore_ascii_case("authorization"))
    {
        let authorization = header_value(headers, "authorization").unwrap_or_default();
        if !authorization.starts_with("Bearer ") {
            return Err(json!({ "invalid": ["authorization"] }));
        }
    }

    if let Some(expected) = endpoint.expected_mock_tr_id() {
        if is_single_tr_id(expected) {
            let actual = header_value(headers, "tr_id").unwrap_or_default();
            if actual != expected {
                return Err(json!({ "invalid": ["tr_id"], "expected": expected }));
            }
        }
    }

    Ok(())
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string)
}

fn is_single_tr_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn success_body(endpoint: &ContractEndpoint) -> Value {
    let mut body = Map::new();
    body.insert("rt_cd".to_string(), json!("0"));
    body.insert("msg_cd".to_string(), json!("KIS_MOCK_OK"));
    body.insert("msg1".to_string(), json!("mock success"));
    body.insert(
        "kis_mock".to_string(),
        json!({
            "endpoint_id": endpoint.id,
            "method": endpoint.method,
            "path": endpoint.path,
            "kind": endpoint.kind,
            "env_support": "real+mock",
        }),
    );

    let output_fields = endpoint.response_fields_for_output();
    if !output_fields.is_empty()
        || endpoint
            .response_body_fields
            .iter()
            .any(|field| field == "output")
    {
        let mut output = Map::new();
        for field in output_fields.into_iter().take(64) {
            output.insert(field.to_string(), json!("kis_mock_value"));
        }
        body.insert("output".to_string(), Value::Object(output));
    }

    Value::Object(body)
}

fn provider_envelope(status: StatusCode, rt_cd: &str, msg_cd: &str, msg1: &str) -> Response {
    (
        status,
        Json(json!({ "rt_cd": rt_cd, "msg_cd": msg_cd, "msg1": msg1 })),
    )
        .into_response()
}

fn error_response(status: StatusCode, code: &str, detail: Value) -> Response {
    (
        status,
        Json(json!({
            "rt_cd": "1",
            "msg_cd": code,
            "msg1": code,
            "detail": detail
        })),
    )
        .into_response()
}

fn with_retry_after(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert("retry-after", http::HeaderValue::from_static("1"));
    response
}

enum MockScenario {
    Unauthorized,
    RateLimit,
    RetryableServerError,
    ProviderError,
}

fn scenario(headers: &HeaderMap) -> Option<MockScenario> {
    match header_value(headers, "x-kis-mock-scenario").as_deref() {
        Some("unauthorized") => Some(MockScenario::Unauthorized),
        Some("rate-limit") => Some(MockScenario::RateLimit),
        Some("retryable-500") => Some(MockScenario::RetryableServerError),
        Some("provider-error") => Some(MockScenario::ProviderError),
        _ => None,
    }
}
