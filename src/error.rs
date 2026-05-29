use std::{error::Error, fmt};

use crate::config::Environment;

#[derive(Debug)]
pub enum KisError {
    Config(String),
    UnsupportedEnvironment {
        endpoint: String,
        environment: Environment,
    },
    MissingCredentials,
    AmbiguousTrId {
        endpoint: String,
        tr_id: String,
    },
    Transport(String),
    HttpStatus {
        status: u16,
        provider_code: Option<String>,
        retry_after: Option<String>,
    },
    Provider {
        rt_cd: String,
        msg_cd: Option<String>,
        msg1: Option<String>,
    },
    Decode(String),
    Contract(String),
}

impl KisError {
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::Transport(_)
                | Self::HttpStatus {
                    status: 429 | 500..=599,
                    ..
                }
        )
    }
}

impl fmt::Display for KisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(message) => write!(formatter, "configuration error: {message}"),
            Self::UnsupportedEnvironment {
                endpoint,
                environment,
            } => write!(
                formatter,
                "endpoint {endpoint} does not support {environment:?}"
            ),
            Self::MissingCredentials => formatter.write_str("KIS app credentials are required"),
            Self::AmbiguousTrId { endpoint, tr_id } => {
                write!(
                    formatter,
                    "endpoint {endpoint} has ambiguous TR ID variants: {tr_id}"
                )
            }
            Self::Transport(message) => write!(formatter, "transport error: {message}"),
            Self::HttpStatus { status, .. } => write!(formatter, "HTTP status error: {status}"),
            Self::Provider {
                rt_cd,
                msg_cd,
                msg1,
            } => write!(
                formatter,
                "provider error rt_cd={rt_cd} msg_cd={} msg1={}",
                msg_cd.as_deref().unwrap_or(""),
                msg1.as_deref().unwrap_or("")
            ),
            Self::Decode(message) => write!(formatter, "decode error: {message}"),
            Self::Contract(message) => write!(formatter, "contract error: {message}"),
        }
    }
}

impl Error for KisError {}
