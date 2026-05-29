use http::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    client::{KisClient, KisEnvelope},
    credentials::Account,
    endpoint::{EndpointSpec, OperationKind},
    error::KisError,
};

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

#[derive(Clone, Debug, Serialize)]
pub struct InquirePriceRequest {
    #[serde(rename = "FID_COND_MRKT_DIV_CODE")]
    pub market_division_code: String,
    #[serde(rename = "FID_INPUT_ISCD")]
    pub stock_code: String,
}

impl InquirePriceRequest {
    pub fn new(stock_code: impl Into<String>) -> Self {
        Self {
            market_division_code: "J".to_string(),
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
            order_division: "00".to_string(),
            order_quantity: quantity.to_string(),
            order_unit_price: price.to_string(),
        }
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
        let tr_id = side.tr_id(self.environment() == crate::config::Environment::Mock);
        self.execute(&ORDER_CASH, Option::<&()>::None, Some(request), Some(tr_id))
            .await
    }
}
