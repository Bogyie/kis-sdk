use serde::de::DeserializeOwned;

use crate::{
    client::{KisClient, KisEnvelope},
    endpoint::InventoryRequest,
    error::KisError,
};

pub const REALTIME_TRADE_KRX: &str = "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0";
pub const REALTIME_ASKING_PRICE_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stasp0";
pub const REALTIME_EXECUTION_NOTICE: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stcni0";
pub const REALTIME_EXPECTED_EXECUTION_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stanc0";
pub const REALTIME_MEMBER_KRX: &str = "domestic_stock_realtime_quotation.post_tryitout_h0stmbc0";
pub const REALTIME_PROGRAM_TRADE_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stpgm0";
pub const REALTIME_MARKET_OPERATION_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stmko0";
pub const AFTER_HOURS_REALTIME_ASKING_PRICE_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stoaa0";
pub const AFTER_HOURS_REALTIME_TRADE_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stoup0";
pub const AFTER_HOURS_EXPECTED_EXECUTION_KRX: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0stoac0";
pub const INDEX_REALTIME_TRADE: &str = "domestic_stock_realtime_quotation.post_tryitout_h0upcnt0";
pub const INDEX_EXPECTED_EXECUTION: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0upanc0";
pub const INDEX_PROGRAM_TRADE: &str = "domestic_stock_realtime_quotation.post_tryitout_h0uppgm0";
pub const ELW_REALTIME_ASKING_PRICE: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0ewasp0";
pub const ELW_REALTIME_TRADE: &str = "domestic_stock_realtime_quotation.post_tryitout_h0ewcnt0";
pub const ELW_EXPECTED_EXECUTION: &str = "domestic_stock_realtime_quotation.post_tryitout_h0ewanc0";
pub const ETF_NAV_TREND: &str = "domestic_stock_realtime_quotation.post_tryitout_h0stnav0";
pub const REALTIME_TRADE_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0uncnt0";
pub const REALTIME_ASKING_PRICE_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0unasp0";
pub const REALTIME_EXPECTED_EXECUTION_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0unanc0";
pub const REALTIME_MEMBER_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0unmbc0";
pub const REALTIME_PROGRAM_TRADE_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0unpgm0";
pub const REALTIME_MARKET_OPERATION_INTEGRATED: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0unmko0";
pub const REALTIME_TRADE_NXT: &str = "domestic_stock_realtime_quotation.post_tryitout_h0nxcnt0";
pub const REALTIME_ASKING_PRICE_NXT: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0nxasp0";
pub const REALTIME_EXPECTED_EXECUTION_NXT: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0nxanc0";
pub const REALTIME_MEMBER_NXT: &str = "domestic_stock_realtime_quotation.post_tryitout_h0nxmbc0";
pub const REALTIME_PROGRAM_TRADE_NXT: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0nxpgm0";
pub const REALTIME_MARKET_OPERATION_NXT: &str =
    "domestic_stock_realtime_quotation.post_tryitout_h0nxmko0";

pub const DOMESTIC_STOCK_REALTIME_TRYITOUT_OPERATIONS: [&str; 29] = [
    REALTIME_TRADE_KRX,
    REALTIME_ASKING_PRICE_KRX,
    REALTIME_EXECUTION_NOTICE,
    REALTIME_EXPECTED_EXECUTION_KRX,
    REALTIME_MEMBER_KRX,
    REALTIME_PROGRAM_TRADE_KRX,
    REALTIME_MARKET_OPERATION_KRX,
    AFTER_HOURS_REALTIME_ASKING_PRICE_KRX,
    AFTER_HOURS_REALTIME_TRADE_KRX,
    AFTER_HOURS_EXPECTED_EXECUTION_KRX,
    INDEX_REALTIME_TRADE,
    INDEX_EXPECTED_EXECUTION,
    INDEX_PROGRAM_TRADE,
    ELW_REALTIME_ASKING_PRICE,
    ELW_REALTIME_TRADE,
    ELW_EXPECTED_EXECUTION,
    ETF_NAV_TREND,
    REALTIME_TRADE_INTEGRATED,
    REALTIME_ASKING_PRICE_INTEGRATED,
    REALTIME_EXPECTED_EXECUTION_INTEGRATED,
    REALTIME_MEMBER_INTEGRATED,
    REALTIME_PROGRAM_TRADE_INTEGRATED,
    REALTIME_MARKET_OPERATION_INTEGRATED,
    REALTIME_TRADE_NXT,
    REALTIME_ASKING_PRICE_NXT,
    REALTIME_EXPECTED_EXECUTION_NXT,
    REALTIME_MEMBER_NXT,
    REALTIME_PROGRAM_TRADE_NXT,
    REALTIME_MARKET_OPERATION_NXT,
];

impl KisClient {
    pub async fn execute_domestic_stock_realtime_tryitout<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        ensure_domestic_stock_realtime_operation(operation_id)?;
        self.execute_inventory(operation_id, request).await
    }
}

fn ensure_domestic_stock_realtime_operation(operation_id: &str) -> Result<(), KisError> {
    if DOMESTIC_STOCK_REALTIME_TRYITOUT_OPERATIONS.contains(&operation_id) {
        Ok(())
    } else {
        Err(KisError::Validation(format!(
            "{operation_id} is not a domestic stock realtime tryitout operation"
        )))
    }
}
