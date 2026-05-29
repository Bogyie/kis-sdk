use std::{sync::Arc, time::Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    config::{Environment, KisConfig},
    credentials::AppCredentials,
    endpoint::{EndpointSpec, InventoryCatalog, InventoryRequest, OperationKind, PreparedRequest},
    error::KisError,
    fallback::FallbackPolicy,
    retry::RetryPolicy,
};

#[derive(Clone)]
pub struct KisClient {
    http: reqwest::Client,
    config: KisConfig,
    credentials: Option<AppCredentials>,
    static_token: Option<String>,
    fallback_credentials: Option<AppCredentials>,
    fallback_static_token: Option<String>,
    token_cache: Arc<Mutex<Option<CachedToken>>>,
}

#[derive(Clone, Debug)]
struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Clone, Debug, Deserialize)]
pub struct KisEnvelope<T = Value> {
    pub rt_cd: String,
    pub msg_cd: Option<String>,
    pub msg1: Option<String>,
    pub output: Option<T>,
    #[serde(skip)]
    pub execution: ExecutionMetadata,
}

impl<T> KisEnvelope<T> {
    pub fn is_success(&self) -> bool {
        self.rt_cd == "0"
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ExecutionMetadata {
    pub attempts: usize,
    pub fallback: Option<FallbackDecision>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FallbackDecision {
    pub from_environment: Environment,
    pub to_environment: Environment,
    pub from_base_url: String,
    pub to_base_url: String,
}

#[derive(Debug, Deserialize)]
pub struct AccessTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub access_token_token_expired: Option<String>,
}

pub struct KisClientBuilder {
    config: KisConfig,
    credentials: Option<AppCredentials>,
    static_token: Option<String>,
    fallback_credentials: Option<AppCredentials>,
    fallback_static_token: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CredentialScope {
    Primary,
    Fallback,
}

impl KisClient {
    pub fn builder(environment: Environment) -> KisClientBuilder {
        KisClientBuilder {
            config: KisConfig::new(environment),
            credentials: None,
            static_token: None,
            fallback_credentials: None,
            fallback_static_token: None,
        }
    }

    pub fn environment(&self) -> Environment {
        self.config.environment
    }

    pub async fn issue_access_token(&self) -> Result<AccessTokenResponse, KisError> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or(KisError::MissingCredentials)?;

        #[derive(Serialize)]
        struct TokenRequest<'a> {
            grant_type: &'static str,
            appkey: &'a str,
            appsecret: &'a str,
        }

        let response = self
            .http
            .post(format!("{}/oauth2/tokenP", self.config.base_url))
            .json(&TokenRequest {
                grant_type: "client_credentials",
                appkey: credentials.app_key(),
                appsecret: credentials.app_secret(),
            })
            .send()
            .await
            .map_err(|error| KisError::Transport(error.to_string()))?;

        parse_response(response).await
    }

    pub(crate) async fn execute<Q, B, T>(
        &self,
        endpoint: &EndpointSpec,
        query: Option<&Q>,
        body: Option<&B>,
        tr_id_override: Option<&str>,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        Q: Serialize,
        B: Serialize,
        T: DeserializeOwned,
    {
        let request = endpoint.prepare(self.config.environment, query, body, tr_id_override)?;
        let fallback_request =
            if self.should_fallback_to_mock(endpoint.operation_kind, request.method.as_str()) {
                Some(endpoint.prepare(Environment::Mock, query, body, tr_id_override)?)
            } else {
                None
            };
        self.execute_prepared(
            endpoint.id,
            endpoint.operation_kind,
            request,
            fallback_request,
        )
        .await
    }

    pub async fn execute_inventory<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        let catalog = InventoryCatalog::bundled()?;
        let (operation_kind, prepared) =
            catalog.prepare(operation_id, self.config.environment, &request)?;
        let fallback_request =
            if self.should_fallback_to_mock(operation_kind, prepared.method.as_str()) {
                Some(
                    catalog
                        .prepare(operation_id, Environment::Mock, &request)?
                        .1,
                )
            } else {
                None
            };
        self.execute_prepared(operation_id, operation_kind, prepared, fallback_request)
            .await
    }

    async fn execute_prepared<T>(
        &self,
        endpoint_id: &str,
        operation_kind: OperationKind,
        request: PreparedRequest,
        fallback_request: Option<PreparedRequest>,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        if self.config.environment == Environment::Real
            && operation_kind == OperationKind::TradingMutation
        {
            return Err(KisError::LiveTradingDisabled {
                endpoint: endpoint_id.to_string(),
            });
        }

        let mut attempt = 1;
        loop {
            let result = self
                .send_prepared(
                    operation_kind,
                    &request,
                    &self.config.base_url,
                    CredentialScope::Primary,
                )
                .await;
            match result {
                Ok(mut response) => {
                    response.execution.attempts = attempt;
                    return Ok(response);
                }
                Err(error)
                    if self.config.retry_policy.should_retry(
                        request.method.as_str(),
                        operation_kind,
                        &error,
                        attempt,
                    ) =>
                {
                    attempt += 1;
                    if !self.config.retry_policy.backoff().is_zero() {
                        tokio::time::sleep(self.config.retry_policy.backoff()).await;
                    }
                }
                Err(error) if error.retryable() && fallback_request.is_some() => {
                    let fallback_request = fallback_request.as_ref().expect("checked is_some");
                    let mut response = self
                        .send_prepared(
                            operation_kind,
                            fallback_request,
                            &self.config.fallback_base_url,
                            CredentialScope::Fallback,
                        )
                        .await?;
                    response.execution.attempts = attempt;
                    response.execution.fallback = Some(FallbackDecision {
                        from_environment: Environment::Real,
                        to_environment: Environment::Mock,
                        from_base_url: self.config.base_url.clone(),
                        to_base_url: self.config.fallback_base_url.clone(),
                    });
                    return Ok(response);
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn should_fallback_to_mock(&self, operation_kind: OperationKind, method: &str) -> bool {
        self.config.environment == Environment::Real
            && operation_kind == OperationKind::Read
            && self
                .config
                .fallback_policy
                .allows_real_to_mock(method, operation_kind)
    }

    async fn send_prepared<T>(
        &self,
        operation_kind: OperationKind,
        request: &PreparedRequest,
        base_url: &str,
        credential_scope: CredentialScope,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        let mut builder = self
            .http
            .request(
                request.method.clone(),
                format!("{base_url}{}", request.path),
            )
            .timeout(self.config.request_timeout)
            .query(&request.query);

        if operation_kind != OperationKind::Auth {
            let credentials = self.credentials_for(credential_scope)?;
            builder = builder
                .header("appkey", credentials.app_key())
                .header("appsecret", credentials.app_secret())
                .header(
                    "authorization",
                    self.authorization_header(credential_scope).await?,
                )
                .header("custtype", "P");
        }

        if let Some(tr_id) = &request.tr_id {
            builder = builder.header("tr_id", tr_id);
        }

        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }

        if let Some(body) = &request.body {
            builder = builder.json(body);
        }

        let response = builder
            .send()
            .await
            .map_err(|error| KisError::Transport(error.to_string()))?;
        parse_response(response).await
    }

    fn credentials_for(
        &self,
        credential_scope: CredentialScope,
    ) -> Result<&AppCredentials, KisError> {
        match credential_scope {
            CredentialScope::Primary => self
                .credentials
                .as_ref()
                .ok_or(KisError::MissingCredentials),
            CredentialScope::Fallback => self
                .fallback_credentials
                .as_ref()
                .ok_or(KisError::MissingFallbackCredentials),
        }
    }

    async fn authorization_header(
        &self,
        credential_scope: CredentialScope,
    ) -> Result<String, KisError> {
        match credential_scope {
            CredentialScope::Primary => Ok(format!("Bearer {}", self.bearer_token().await?)),
            CredentialScope::Fallback => {
                let token = self
                    .fallback_static_token
                    .as_ref()
                    .ok_or(KisError::MissingFallbackCredentials)?;
                Ok(format!("Bearer {token}"))
            }
        }
    }

    async fn bearer_token(&self) -> Result<String, KisError> {
        if let Some(token) = &self.static_token {
            return Ok(token.clone());
        }

        let mut cache = self.token_cache.lock().await;
        if let Some(token) = cache.as_ref() {
            if token.expires_at > Instant::now() {
                return Ok(token.access_token.clone());
            }
        }

        let issued = self.issue_access_token().await?;
        let expires_at = Instant::now()
            + std::time::Duration::from_secs(issued.expires_in)
                .saturating_sub(self.config.token_refresh_skew);
        let cached = CachedToken {
            access_token: issued.access_token.clone(),
            expires_at,
        };
        *cache = Some(cached);
        Ok(issued.access_token)
    }
}

impl KisClientBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.config = self.config.with_base_url(base_url);
        self
    }

    pub fn app_credentials(mut self, credentials: AppCredentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    pub fn static_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.static_token = Some(token.into());
        self
    }

    pub fn retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.config.retry_policy = retry_policy;
        self
    }

    pub fn fallback_policy(mut self, fallback_policy: FallbackPolicy) -> Self {
        self.config.fallback_policy = fallback_policy;
        self
    }

    pub fn fallback_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.config = self.config.with_fallback_base_url(base_url);
        self
    }

    pub fn fallback_credentials(mut self, credentials: AppCredentials) -> Self {
        self.fallback_credentials = Some(credentials);
        self
    }

    pub fn fallback_static_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.fallback_static_token = Some(token.into());
        self
    }

    pub fn build(self) -> Result<KisClient, KisError> {
        let http = reqwest::Client::builder()
            .timeout(self.config.request_timeout)
            .build()
            .map_err(|error| KisError::Config(error.to_string()))?;

        Ok(KisClient {
            http,
            config: self.config,
            credentials: self.credentials,
            static_token: self.static_token,
            fallback_credentials: self.fallback_credentials,
            fallback_static_token: self.fallback_static_token,
            token_cache: Arc::new(Mutex::new(None)),
        })
    }
}

async fn parse_response<T>(response: reqwest::Response) -> Result<T, KisError>
where
    T: DeserializeOwned,
{
    let status = response.status();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);

    let value: Value = response
        .json()
        .await
        .map_err(|error| KisError::Decode(error.to_string()))?;

    if !status.is_success() {
        return Err(KisError::HttpStatus {
            status: status.as_u16(),
            provider_code: value
                .get("msg_cd")
                .and_then(Value::as_str)
                .map(str::to_string),
            retry_after,
        });
    }

    if value
        .get("rt_cd")
        .and_then(Value::as_str)
        .is_some_and(|rt_cd| rt_cd != "0")
    {
        return Err(KisError::Provider {
            rt_cd: value
                .get("rt_cd")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            msg_cd: value
                .get("msg_cd")
                .and_then(Value::as_str)
                .map(str::to_string),
            msg1: value
                .get("msg1")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }

    serde_json::from_value(value).map_err(|error| KisError::Decode(error.to_string()))
}
