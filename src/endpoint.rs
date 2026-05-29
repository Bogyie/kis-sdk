use http::Method;
use serde::Serialize;
use serde_json::Value;

use crate::{
    config::Environment,
    contract::{ContractEndpoint, ContractInventory, EnvironmentSupport},
    error::KisError,
};

#[derive(Clone, Debug)]
pub struct EndpointSpec {
    pub id: &'static str,
    pub method: Method,
    pub path: &'static str,
    pub default_real_tr_id: Option<&'static str>,
    pub default_mock_tr_id: Option<&'static str>,
    pub operation_kind: OperationKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationKind {
    Read,
    Auth,
    TradingMutation,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedRequest {
    pub method: Method,
    pub path: String,
    pub tr_id: Option<String>,
    pub body: Option<Value>,
    pub query: Vec<(String, String)>,
}

impl EndpointSpec {
    pub(crate) fn prepare<Q, B>(
        &self,
        environment: Environment,
        query: Option<&Q>,
        body: Option<&B>,
        tr_id_override: Option<&str>,
    ) -> Result<PreparedRequest, KisError>
    where
        Q: Serialize,
        B: Serialize,
    {
        let inventory =
            ContractInventory::bundled().map_err(|error| KisError::Contract(error.to_string()))?;
        let contract = inventory
            .endpoint(self.method.as_str(), self.path)
            .ok_or_else(|| KisError::Contract(format!("missing endpoint {}", self.path)))?;

        validate_environment(contract, environment, self.id)?;
        let tr_id = self.select_tr_id(environment, tr_id_override)?;

        Ok(PreparedRequest {
            method: self.method.clone(),
            path: self.path.to_string(),
            tr_id,
            body: body
                .map(serde_json::to_value)
                .transpose()
                .map_err(|error| KisError::Decode(error.to_string()))?,
            query: query.map(query_pairs).transpose()?.unwrap_or_default(),
        })
    }

    fn select_tr_id(
        &self,
        environment: Environment,
        tr_id_override: Option<&str>,
    ) -> Result<Option<String>, KisError> {
        if let Some(tr_id) = tr_id_override {
            return Ok(Some(tr_id.to_string()));
        }

        let selected = match environment {
            Environment::Real => self.default_real_tr_id,
            Environment::Mock => self.default_mock_tr_id.or(self.default_real_tr_id),
        };

        match selected {
            Some(tr_id) if is_single_tr_id(tr_id) => Ok(Some(tr_id.to_string())),
            Some(tr_id) => Err(KisError::AmbiguousTrId {
                endpoint: self.id.to_string(),
                tr_id: tr_id.to_string(),
            }),
            None => Ok(None),
        }
    }
}

fn validate_environment(
    contract: &ContractEndpoint,
    environment: Environment,
    endpoint_id: &str,
) -> Result<(), KisError> {
    if environment == Environment::Mock && contract.env_support == EnvironmentSupport::RealOnly {
        return Err(KisError::UnsupportedEnvironment {
            endpoint: endpoint_id.to_string(),
            environment,
        });
    }

    Ok(())
}

fn query_pairs<T: Serialize>(query: &T) -> Result<Vec<(String, String)>, KisError> {
    let Value::Object(fields) =
        serde_json::to_value(query).map_err(|error| KisError::Decode(error.to_string()))?
    else {
        return Err(KisError::Decode(
            "query must serialize to an object".to_string(),
        ));
    };

    let mut pairs = Vec::new();
    for (key, value) in fields {
        match value {
            Value::Null => {}
            Value::String(value) => pairs.push((key, value)),
            Value::Bool(value) => pairs.push((key, value.to_string())),
            Value::Number(value) => pairs.push((key, value.to_string())),
            other => {
                return Err(KisError::Decode(format!(
                    "query field {key} must be scalar, got {other}"
                )))
            }
        }
    }
    Ok(pairs)
}

fn is_single_tr_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}
