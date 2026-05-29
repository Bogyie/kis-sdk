use std::time::Duration;

use crate::{fallback::FallbackPolicy, retry::RetryPolicy};

pub const REAL_REST_BASE_URL: &str = "https://openapi.koreainvestment.com:9443";
pub const MOCK_REST_BASE_URL: &str = "https://openapivts.koreainvestment.com:29443";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Environment {
    Real,
    Mock,
}

impl Environment {
    pub fn default_base_url(self) -> &'static str {
        match self {
            Self::Real => REAL_REST_BASE_URL,
            Self::Mock => MOCK_REST_BASE_URL,
        }
    }
}

#[derive(Clone, Debug)]
pub struct KisConfig {
    pub environment: Environment,
    pub base_url: String,
    pub request_timeout: Duration,
    pub token_refresh_skew: Duration,
    pub retry_policy: RetryPolicy,
    pub fallback_policy: FallbackPolicy,
}

impl KisConfig {
    pub fn new(environment: Environment) -> Self {
        Self {
            environment,
            base_url: environment.default_base_url().to_string(),
            request_timeout: Duration::from_secs(10),
            token_refresh_skew: Duration::from_secs(60),
            retry_policy: RetryPolicy::disabled(),
            fallback_policy: FallbackPolicy::disabled(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }
}
