use std::collections::HashMap;

use serde::Deserialize;

pub const OFFICIAL_ENDPOINT_INVENTORY: &str =
    include_str!("../contracts/kis_official_endpoint_inventory.compact.json");

#[derive(Clone, Debug, Deserialize)]
pub struct ContractInventory {
    pub source: String,
    pub checked_at: String,
    pub endpoint_count: usize,
    pub note: String,
    pub collections: Vec<ContractCollection>,
    pub endpoints: Vec<ContractEndpoint>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ContractCollection {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ContractEndpoint {
    pub id: String,
    pub name: String,
    pub collection_name: String,
    pub method: String,
    pub path: String,
    pub kind: String,
    pub env_support: EnvironmentSupport,
    pub real_tr_id: String,
    pub virtual_tr_id: String,
    pub required_headers: Vec<String>,
    pub required_query: Vec<String>,
    pub required_body: Vec<String>,
    pub response_body_fields: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum EnvironmentSupport {
    #[serde(rename = "real+mock")]
    RealMock,
    #[serde(rename = "real_only")]
    RealOnly,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RouteKey {
    pub method: String,
    pub path: String,
}

impl ContractInventory {
    pub fn bundled() -> Result<Self, serde_json::Error> {
        serde_json::from_str(OFFICIAL_ENDPOINT_INVENTORY)
    }

    pub fn route_index(&self) -> HashMap<RouteKey, ContractEndpoint> {
        self.endpoints
            .iter()
            .cloned()
            .map(|endpoint| {
                (
                    RouteKey {
                        method: endpoint.method.clone(),
                        path: endpoint.path.clone(),
                    },
                    endpoint,
                )
            })
            .collect()
    }

    pub fn endpoint(&self, method: &str, path: &str) -> Option<&ContractEndpoint> {
        self.endpoints
            .iter()
            .find(|endpoint| endpoint.method == method && endpoint.path == path)
    }
}

impl ContractEndpoint {
    pub fn is_auth(&self) -> bool {
        self.kind == "auth" || self.path.starts_with("/oauth2/")
    }

    pub fn expected_mock_tr_id(&self) -> Option<&str> {
        non_empty_tr_id(&self.virtual_tr_id).or_else(|| non_empty_tr_id(&self.real_tr_id))
    }

    pub fn response_fields_for_output(&self) -> Vec<&str> {
        self.response_body_fields
            .iter()
            .map(String::as_str)
            .filter(|field| !matches!(*field, "rt_cd" | "msg_cd" | "msg1" | "output"))
            .collect()
    }
}

fn non_empty_tr_id(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains("미지원") {
        None
    } else {
        Some(trimmed)
    }
}
