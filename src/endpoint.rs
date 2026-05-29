use std::collections::{HashMap, HashSet};

use http::{HeaderName, HeaderValue, Method};
use serde::Serialize;
use serde_json::{Map, Value};

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
    pub headers: Vec<(HeaderName, HeaderValue)>,
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
            headers: Vec::new(),
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

#[derive(Clone, Debug)]
pub struct InventoryEndpointSpec {
    pub operation_id: String,
    pub contract_id: String,
    pub collection_name: String,
    pub method: Method,
    pub path: String,
    pub default_real_tr_id: Option<String>,
    pub default_mock_tr_id: Option<String>,
    pub operation_kind: OperationKind,
    pub required_headers: Vec<String>,
    pub required_query: Vec<String>,
    pub required_body: Vec<String>,
    pub env_support: EnvironmentSupport,
}

#[derive(Clone, Debug, Default)]
pub struct InventoryRequest {
    query: Option<Value>,
    body: Option<Value>,
    headers: Vec<(String, String)>,
    tr_id_override: Option<String>,
}

#[derive(Clone, Debug)]
pub struct InventoryCatalog {
    endpoints: Vec<InventoryEndpointSpec>,
    by_operation_id: HashMap<String, usize>,
}

impl InventoryRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn query(mut self, query: impl Into<Value>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn body(mut self, body: impl Into<Value>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn tr_id_override(mut self, tr_id: impl Into<String>) -> Self {
        self.tr_id_override = Some(tr_id.into());
        self
    }
}

impl InventoryCatalog {
    pub fn bundled() -> Result<Self, KisError> {
        let inventory =
            ContractInventory::bundled().map_err(|error| KisError::Contract(error.to_string()))?;
        Self::from_contract_inventory(&inventory)
    }

    pub fn from_contract_inventory(inventory: &ContractInventory) -> Result<Self, KisError> {
        let mut endpoints = Vec::with_capacity(inventory.endpoints.len());
        let mut by_operation_id = HashMap::with_capacity(inventory.endpoints.len());

        for contract in &inventory.endpoints {
            let operation_id = operation_id(contract);
            let method = Method::from_bytes(contract.method.as_bytes()).map_err(|error| {
                KisError::Contract(format!(
                    "invalid method {} for {}: {error}",
                    contract.method, contract.path
                ))
            })?;
            let spec = InventoryEndpointSpec {
                operation_id: operation_id.clone(),
                contract_id: contract.id.clone(),
                collection_name: contract.collection_name.clone(),
                method,
                path: contract.path.clone(),
                default_real_tr_id: non_empty_tr_id(&contract.real_tr_id).map(str::to_string),
                default_mock_tr_id: non_empty_tr_id(&contract.virtual_tr_id).map(str::to_string),
                operation_kind: operation_kind(contract),
                required_headers: contract.required_headers.clone(),
                required_query: contract.required_query.clone(),
                required_body: contract.required_body.clone(),
                env_support: contract.env_support,
            };

            if by_operation_id
                .insert(operation_id.clone(), endpoints.len())
                .is_some()
            {
                return Err(KisError::Contract(format!(
                    "duplicate inventory operation id {operation_id}"
                )));
            }
            endpoints.push(spec);
        }

        Ok(Self {
            endpoints,
            by_operation_id,
        })
    }

    pub fn endpoint_count(&self) -> usize {
        self.endpoints.len()
    }

    pub fn endpoints(&self) -> &[InventoryEndpointSpec] {
        &self.endpoints
    }

    pub fn endpoint(&self, operation_id: &str) -> Option<&InventoryEndpointSpec> {
        self.by_operation_id
            .get(operation_id)
            .map(|index| &self.endpoints[*index])
    }

    pub(crate) fn prepare(
        &self,
        operation_id: &str,
        environment: Environment,
        request: &InventoryRequest,
    ) -> Result<(OperationKind, PreparedRequest), KisError> {
        let endpoint = self.endpoint(operation_id).ok_or_else(|| {
            KisError::Contract(format!("missing inventory operation id {operation_id}"))
        })?;
        endpoint.prepare(environment, request)
    }
}

impl InventoryEndpointSpec {
    pub(crate) fn prepare(
        &self,
        environment: Environment,
        request: &InventoryRequest,
    ) -> Result<(OperationKind, PreparedRequest), KisError> {
        validate_inventory_environment(self, environment)?;
        let query_required = self.required_query_fields();
        validate_required_fields("query", &query_required, request.query.as_ref())?;
        if self.method != Method::GET {
            validate_required_fields("body", &self.required_body, request.body.as_ref())?;
        }

        let tr_id = self.select_tr_id(environment, request.tr_id_override.as_deref())?;
        let headers = self.prepare_headers(request, tr_id.as_deref())?;
        let query = request
            .query
            .as_ref()
            .map(query_pairs)
            .transpose()?
            .unwrap_or_default();

        Ok((
            self.operation_kind,
            PreparedRequest {
                method: self.method.clone(),
                path: self.path.clone(),
                tr_id,
                body: request.body.clone(),
                query,
                headers,
            },
        ))
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
            Environment::Real => self.default_real_tr_id.as_deref(),
            Environment::Mock => self
                .default_mock_tr_id
                .as_deref()
                .or(self.default_real_tr_id.as_deref()),
        };

        match selected {
            Some(tr_id) if is_single_tr_id(tr_id) => Ok(Some(tr_id.to_string())),
            Some(tr_id) if self.requires_header("tr_id") => Err(KisError::AmbiguousTrId {
                endpoint: self.operation_id.clone(),
                tr_id: tr_id.to_string(),
            }),
            Some(_) | None => Ok(None),
        }
    }

    fn prepare_headers(
        &self,
        request: &InventoryRequest,
        tr_id: Option<&str>,
    ) -> Result<Vec<(HeaderName, HeaderValue)>, KisError> {
        let provided = request
            .headers
            .iter()
            .map(|(name, _)| name.to_ascii_lowercase())
            .collect::<HashSet<_>>();

        for header in &self.required_headers {
            let normalized = header.to_ascii_lowercase();
            if is_auto_header(&normalized) {
                if normalized == "tr_id" && tr_id.is_none() && !provided.contains("tr_id") {
                    return Err(KisError::Validation(format!(
                        "required header {header} is missing"
                    )));
                }
                continue;
            }

            if !provided.contains(&normalized) {
                return Err(KisError::Validation(format!(
                    "required header {header} is missing"
                )));
            }
        }

        request
            .headers
            .iter()
            .map(|(name, value)| {
                let name = HeaderName::from_bytes(name.as_bytes()).map_err(|error| {
                    KisError::Validation(format!("invalid header {name}: {error}"))
                })?;
                let value = HeaderValue::from_str(value).map_err(|error| {
                    KisError::Validation(format!("invalid value for header {name}: {error}"))
                })?;
                Ok((name, value))
            })
            .collect()
    }

    fn requires_header(&self, name: &str) -> bool {
        self.required_headers
            .iter()
            .any(|header| header.eq_ignore_ascii_case(name))
    }

    fn required_query_fields(&self) -> Vec<String> {
        let mut fields = self.required_query.clone();
        if self.method == Method::GET {
            fields.extend(self.required_body.iter().cloned());
        }
        fields
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

fn validate_inventory_environment(
    endpoint: &InventoryEndpointSpec,
    environment: Environment,
) -> Result<(), KisError> {
    if environment == Environment::Mock && endpoint.env_support == EnvironmentSupport::RealOnly {
        return Err(KisError::UnsupportedEnvironment {
            endpoint: endpoint.operation_id.clone(),
            environment,
        });
    }

    Ok(())
}

fn validate_required_fields(
    location: &str,
    required: &[String],
    value: Option<&Value>,
) -> Result<(), KisError> {
    if required.is_empty() {
        return Ok(());
    }

    let object = match value {
        Some(Value::Object(object)) => object,
        Some(_) | None => {
            return Err(KisError::Validation(format!(
                "{location} must contain required fields: {}",
                required.join(", ")
            )))
        }
    };

    for field in required {
        if !has_present_field(object, field) {
            return Err(KisError::Validation(format!(
                "required {location} field {field} is missing"
            )));
        }
    }

    Ok(())
}

fn has_present_field(object: &Map<String, Value>, field: &str) -> bool {
    object.get(field).is_some_and(|value| !value.is_null())
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

fn non_empty_tr_id(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains("미지원") {
        None
    } else {
        Some(trimmed)
    }
}

fn operation_kind(endpoint: &ContractEndpoint) -> OperationKind {
    if endpoint.is_auth() {
        OperationKind::Auth
    } else {
        match endpoint.kind.as_str() {
            "quotation/info" | "websocket" => OperationKind::Read,
            "trading/account"
                if endpoint.method == "POST" && is_trading_mutation_path(&endpoint.path) =>
            {
                OperationKind::TradingMutation
            }
            "trading/account" => OperationKind::Read,
            _ if endpoint.method == "GET" => OperationKind::Read,
            _ => OperationKind::TradingMutation,
        }
    }
}

fn is_trading_mutation_path(path: &str) -> bool {
    path.contains("/order") || path.ends_with("/buy") || path.ends_with("/sell")
}

fn operation_id(endpoint: &ContractEndpoint) -> String {
    format!(
        "{}.{}_{}",
        collection_slug(&endpoint.collection_name),
        endpoint.method.to_ascii_lowercase(),
        path_slug(&endpoint.path)
    )
}

fn collection_slug(collection_name: &str) -> &'static str {
    match collection_name {
        "OAuth인증" => "oauth_authentication",
        "[국내주식] 주문/계좌" => "domestic_stock_trading_account",
        "[국내주식] 기본시세" => "domestic_stock_quotation",
        "[국내주식] ELW 시세" => "domestic_stock_elw_quotation",
        "[국내주식] 업종/기타" => "domestic_stock_sector_misc",
        "[국내주식] 종목정보" => "domestic_stock_product_info",
        "[국내주식] 시세분석" => "domestic_stock_market_analysis",
        "[국내주식] 순위분석" => "domestic_stock_ranking_analysis",
        "[국내주식] 실시간시세" => "domestic_stock_realtime_quotation",
        "[국내선물옵션] 주문/계좌" => "domestic_futures_options_trading_account",
        "[국내선물옵션] 기본시세" => "domestic_futures_options_quotation",
        "[국내선물옵션] 실시간시세" => "domestic_futures_options_realtime_quotation",
        "[해외주식] 주문/계좌" => "overseas_stock_trading_account",
        "[해외주식] 기본시세" => "overseas_stock_quotation",
        "[해외주식] 시세분석" => "overseas_stock_market_analysis",
        "[해외주식] 실시간시세" => "overseas_stock_realtime_quotation",
        "[해외선물옵션] 주문/계좌" => "overseas_futures_options_trading_account",
        "[해외선물옵션] 기본시세" => "overseas_futures_options_quotation",
        "[해외선물옵션]실시간시세" => "overseas_futures_options_realtime_quotation",
        "[장내채권] 주문/계좌" => "bond_trading_account",
        "[장내채권] 기본시세" => "bond_quotation",
        "[장내채권] 실시간시세" => "bond_realtime_quotation",
        _ => "unknown_collection",
    }
}

fn path_slug(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !matches!(*segment, "uapi" | "v1"))
        .flat_map(|segment| segment.split('-'))
        .filter(|segment| !segment.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join("_")
}

fn is_auto_header(header: &str) -> bool {
    matches!(
        header,
        "authorization" | "appkey" | "appsecret" | "content-type" | "custtype" | "tr_id"
    )
}
