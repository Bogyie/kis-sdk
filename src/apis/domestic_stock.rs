use http::Method;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt, str::FromStr};

use crate::{
    client::{KisClient, KisEnvelope},
    credentials::Account,
    endpoint::{
        EndpointSpec, InventoryCatalog, InventoryEndpointSpec, InventoryRequest, OperationKind,
    },
    error::KisError,
};

pub const DOMESTIC_STOCK_REST_ENDPOINT_COUNT: usize = 158;

pub const DOMESTIC_STOCK_REST_COLLECTIONS: &[&str] = &[
    "[국내주식] 주문/계좌",
    "[국내주식] 기본시세",
    "[국내주식] ELW 시세",
    "[국내주식] 업종/기타",
    "[국내주식] 종목정보",
    "[국내주식] 시세분석",
    "[국내주식] 순위분석",
];

const INQUIRE_PRICE: EndpointSpec = EndpointSpec {
    id: "domestic_stock.inquire_price",
    method: Method::GET,
    path: "/uapi/domestic-stock/v1/quotations/inquire-price",
    default_real_tr_id: Some("FHKST01010100"),
    default_mock_tr_id: Some("FHKST01010100"),
    operation_kind: OperationKind::Read,
};

const INQUIRE_BALANCE: EndpointSpec = EndpointSpec {
    id: "domestic_stock.inquire_balance",
    method: Method::GET,
    path: "/uapi/domestic-stock/v1/trading/inquire-balance",
    default_real_tr_id: Some("TTTC8434R"),
    default_mock_tr_id: Some("VTTC8434R"),
    operation_kind: OperationKind::Read,
};

const ORDER_CASH: EndpointSpec = EndpointSpec {
    id: "domestic_stock.order_cash",
    method: Method::POST,
    path: "/uapi/domestic-stock/v1/trading/order-cash",
    default_real_tr_id: None,
    default_mock_tr_id: None,
    operation_kind: OperationKind::TradingMutation,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum DomesticStockMarketDivision {
    Stock,
}

impl DomesticStockMarketDivision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stock => "J",
        }
    }
}

impl fmt::Display for DomesticStockMarketDivision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DomesticStockMarketDivision {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "J" => Ok(Self::Stock),
            other => Err(KisError::Validation(format!(
                "{other} is not a supported domestic stock market division"
            ))),
        }
    }
}

impl Serialize for DomesticStockMarketDivision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CashOrderDivision {
    Limit,
}

impl CashOrderDivision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Limit => "00",
        }
    }
}

impl fmt::Display for CashOrderDivision {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for CashOrderDivision {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "00" => Ok(Self::Limit),
            other => Err(KisError::Validation(format!(
                "{other} is not a supported domestic cash order division"
            ))),
        }
    }
}

impl Serialize for CashOrderDivision {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct InquirePriceRequest {
    #[serde(rename = "FID_COND_MRKT_DIV_CODE")]
    pub market_division_code: String,
    #[serde(rename = "FID_INPUT_ISCD")]
    pub stock_code: String,
}

impl InquirePriceRequest {
    pub fn new(stock_code: impl Into<String>) -> Self {
        Self::with_market(DomesticStockMarketDivision::Stock, stock_code)
    }

    pub fn with_market(market: DomesticStockMarketDivision, stock_code: impl Into<String>) -> Self {
        Self {
            market_division_code: market.as_str().to_string(),
            stock_code: stock_code.into(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct InquirePriceOutput {
    #[serde(flatten)]
    pub fields: Value,
}

#[derive(Clone, Debug, Serialize)]
pub struct InquireBalanceRequest {
    #[serde(rename = "CANO")]
    pub cano: String,
    #[serde(rename = "ACNT_PRDT_CD")]
    pub account_product_code: String,
    #[serde(rename = "AFHR_FLPR_YN")]
    pub after_hours_price: String,
    #[serde(rename = "OFL_YN")]
    pub offline: String,
    #[serde(rename = "INQR_DVSN")]
    pub inquiry_division: String,
    #[serde(rename = "UNPR_DVSN")]
    pub price_division: String,
    #[serde(rename = "FUND_STTL_ICLD_YN")]
    pub include_fund_settlement: String,
    #[serde(rename = "FNCG_AMT_AUTO_RDPT_YN")]
    pub auto_redeem_financing: String,
    #[serde(rename = "PRCS_DVSN")]
    pub processing_division: String,
    #[serde(rename = "CTX_AREA_FK100", skip_serializing_if = "Option::is_none")]
    pub context_fk100: Option<String>,
    #[serde(rename = "CTX_AREA_NK100", skip_serializing_if = "Option::is_none")]
    pub context_nk100: Option<String>,
}

impl InquireBalanceRequest {
    pub fn new(account: &Account) -> Self {
        Self {
            cano: account.cano().to_string(),
            account_product_code: account.product_code().to_string(),
            after_hours_price: "N".to_string(),
            offline: "N".to_string(),
            inquiry_division: "01".to_string(),
            price_division: "01".to_string(),
            include_fund_settlement: "N".to_string(),
            auto_redeem_financing: "N".to_string(),
            processing_division: "00".to_string(),
            context_fk100: None,
            context_nk100: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CashOrderSide {
    Buy,
    Sell,
}

impl CashOrderSide {
    fn tr_id(self, mock: bool) -> &'static str {
        match (self, mock) {
            (Self::Buy, true) => "VTTC0012U",
            (Self::Sell, true) => "VTTC0011U",
            (Self::Buy, false) => "TTTC0012U",
            (Self::Sell, false) => "TTTC0011U",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CashOrderRequest {
    #[serde(rename = "CANO")]
    pub cano: String,
    #[serde(rename = "ACNT_PRDT_CD")]
    pub account_product_code: String,
    #[serde(rename = "PDNO")]
    pub product_number: String,
    #[serde(rename = "ORD_DVSN")]
    pub order_division: String,
    #[serde(rename = "ORD_QTY")]
    pub order_quantity: String,
    #[serde(rename = "ORD_UNPR")]
    pub order_unit_price: String,
}

impl CashOrderRequest {
    pub fn limit(
        account: &Account,
        stock_code: impl Into<String>,
        quantity: u64,
        price: u64,
    ) -> Self {
        Self {
            cano: account.cano().to_string(),
            account_product_code: account.product_code().to_string(),
            product_number: stock_code.into(),
            order_division: CashOrderDivision::Limit.as_str().to_string(),
            order_quantity: quantity.to_string(),
            order_unit_price: price.to_string(),
        }
    }

    pub fn with_order_division(
        account: &Account,
        stock_code: impl Into<String>,
        order_division: CashOrderDivision,
        quantity: u64,
        price: u64,
    ) -> Self {
        Self {
            cano: account.cano().to_string(),
            account_product_code: account.product_code().to_string(),
            product_number: stock_code.into(),
            order_division: order_division.as_str().to_string(),
            order_quantity: quantity.to_string(),
            order_unit_price: price.to_string(),
        }
    }

    fn validate(&self) -> Result<(), KisError> {
        require_digits("CANO", &self.cano, 8)?;
        require_digits("ACNT_PRDT_CD", &self.account_product_code, 2)?;
        require_digits("PDNO", &self.product_number, 6)?;
        require_positive_u64("ORD_QTY", &self.order_quantity)?;
        require_positive_u64("ORD_UNPR", &self.order_unit_price)?;
        Ok(())
    }
}

impl KisClient {
    pub async fn inquire_domestic_stock_price(
        &self,
        request: &InquirePriceRequest,
    ) -> Result<KisEnvelope<InquirePriceOutput>, KisError> {
        self.execute(&INQUIRE_PRICE, Some(request), Option::<&()>::None, None)
            .await
    }

    pub async fn inquire_domestic_stock_balance(
        &self,
        request: &InquireBalanceRequest,
    ) -> Result<KisEnvelope<Value>, KisError> {
        self.execute(&INQUIRE_BALANCE, Some(request), Option::<&()>::None, None)
            .await
    }

    pub async fn place_domestic_stock_cash_order(
        &self,
        side: CashOrderSide,
        request: &CashOrderRequest,
    ) -> Result<KisEnvelope<Value>, KisError> {
        request.validate()?;
        let tr_id = side.tr_id(self.environment() == crate::config::Environment::Mock);
        self.execute(&ORDER_CASH, Option::<&()>::None, Some(request), Some(tr_id))
            .await
    }

    pub async fn execute_domestic_stock_rest<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        let catalog = InventoryCatalog::bundled()?;
        let endpoint = catalog.endpoint(operation_id).ok_or_else(|| {
            KisError::Contract(format!("missing inventory operation id {operation_id}"))
        })?;

        if !is_domestic_stock_rest_collection(&endpoint.collection_name) {
            return Err(KisError::Contract(format!(
                "operation {operation_id} is not in domestic stock REST coverage"
            )));
        }

        self.execute_inventory(operation_id, request).await
    }
}

pub fn domestic_stock_rest_endpoints() -> Result<Vec<InventoryEndpointSpec>, KisError> {
    let catalog = InventoryCatalog::bundled()?;
    Ok(catalog
        .endpoints()
        .iter()
        .filter(|endpoint| is_domestic_stock_rest_collection(&endpoint.collection_name))
        .cloned()
        .collect())
}

pub fn is_domestic_stock_rest_collection(collection_name: &str) -> bool {
    DOMESTIC_STOCK_REST_COLLECTIONS.contains(&collection_name)
}

fn require_digits(name: &str, value: &str, len: usize) -> Result<(), KisError> {
    if value.len() == len && value.chars().all(|ch| ch.is_ascii_digit()) {
        Ok(())
    } else {
        Err(KisError::Validation(format!(
            "{name} must be exactly {len} ASCII digits"
        )))
    }
}

fn require_positive_u64(name: &str, value: &str) -> Result<(), KisError> {
    match value.parse::<u64>() {
        Ok(number) if number > 0 => Ok(()),
        _ => Err(KisError::Validation(format!(
            "{name} must be a positive integer"
        ))),
    }
}
