use std::{sync::Arc, time::Instant};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::{
    config::{Environment, KisConfig},
    credentials::AppCredentials,
    endpoint::{EndpointSpec, OperationKind, PreparedRequest},
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
}

impl<T> KisEnvelope<T> {
    pub fn is_success(&self) -> bool {
        self.rt_cd == "0"
    }
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
}

impl KisClient {
    pub fn builder(environment: Environment) -> KisClientBuilder {
        KisClientBuilder {
            config: KisConfig::new(environment),
            credentials: None,
            static_token: None,
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

        let mut attempt = 1;
        loop {
            let result = self.send_prepared(endpoint.operation_kind, &request).await;
            match result {
                Ok(response) => return Ok(response),
                Err(error)
                    if self.config.retry_policy.should_retry(
                        request.method.as_str(),
                        &error,
                        attempt,
                    ) =>
                {
                    attempt += 1;
                    if !self.config.retry_policy.backoff().is_zero() {
                        tokio::time::sleep(self.config.retry_policy.backoff()).await;
                    }
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn send_prepared<T>(
        &self,
        operation_kind: OperationKind,
        request: &PreparedRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        let mut builder = self
            .http
            .request(
                request.method.clone(),
                format!("{}{}", self.config.base_url, request.path),
            )
            .timeout(self.config.request_timeout)
            .query(&request.query);

        if operation_kind != OperationKind::Auth {
            let credentials = self
                .credentials
                .as_ref()
                .ok_or(KisError::MissingCredentials)?;
            builder = builder
                .header("appkey", credentials.app_key())
                .header("appsecret", credentials.app_secret())
                .header(
                    "authorization",
                    format!("Bearer {}", self.bearer_token().await?),
                )
                .header("custtype", "P");
        }

        if let Some(tr_id) = &request.tr_id {
            builder = builder.header("tr_id", tr_id);
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
