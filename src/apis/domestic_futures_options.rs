use serde::de::DeserializeOwned;

use crate::{
    client::{KisClient, KisEnvelope},
    endpoint::InventoryRequest,
    error::KisError,
};

pub const TRADING_ACCOUNT_OPERATION_IDS: [&str; 15] = [
    "domestic_futures_options_trading_account.post_domestic_futureoption_trading_order",
    "domestic_futures_options_trading_account.post_domestic_futureoption_trading_order_rvsecncl",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_ccnl",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_balance",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_psbl_order",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_ngt_ccnl",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_ngt_balance",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_psbl_ngt_order",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_ngt_margin_detail",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_balance_settlement_pl",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_deposit",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_balance_valuation_pl",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_ccnl_bstime",
    "domestic_futures_options_trading_account.get_domestic_futureoption_trading_inquire_daily_amount_fee",
    "domestic_futures_options_trading_account.get_domestic_futureoption_quotations_margin_rate",
];

pub const QUOTATION_OPERATION_IDS: [&str; 9] = [
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_inquire_price",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_inquire_asking_price",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_inquire_daily_fuopchartprice",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_inquire_time_fuopchartprice",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_display_board_option_list",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_display_board_top",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_display_board_callput",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_display_board_futures",
    "domestic_futures_options_quotation.get_domestic_futureoption_quotations_exp_price_trend",
];

pub const REALTIME_QUOTATION_OPERATION_IDS: [&str; 20] = [
    "domestic_futures_options_realtime_quotation.post_tryitout_h0ifasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0ifcnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0ioasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0iocnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0ifcni0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0cfasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0cfcnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zfasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zfcnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zfanc0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zoasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zocnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0zoanc0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0euasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0eucnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0euanc0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0eucni0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0mfasp0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0mfcnt0",
    "domestic_futures_options_realtime_quotation.post_tryitout_h0mfcni0",
];

pub fn operation_ids() -> impl Iterator<Item = &'static str> {
    TRADING_ACCOUNT_OPERATION_IDS
        .into_iter()
        .chain(QUOTATION_OPERATION_IDS)
        .chain(REALTIME_QUOTATION_OPERATION_IDS)
}

impl KisClient {
    pub async fn execute_domestic_futures_options<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        validate_domestic_futures_options_operation(operation_id)?;
        self.execute_inventory(operation_id, request).await
    }

    pub async fn execute_domestic_futures_options_trading_account<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        validate_collection_operation(
            "domestic futures/options trading/account",
            operation_id,
            &TRADING_ACCOUNT_OPERATION_IDS,
        )?;
        self.execute_inventory(operation_id, request).await
    }

    pub async fn execute_domestic_futures_options_quotation<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        validate_collection_operation(
            "domestic futures/options quotation",
            operation_id,
            &QUOTATION_OPERATION_IDS,
        )?;
        self.execute_inventory(operation_id, request).await
    }

    pub async fn execute_domestic_futures_options_realtime_quotation<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        validate_collection_operation(
            "domestic futures/options realtime quotation",
            operation_id,
            &REALTIME_QUOTATION_OPERATION_IDS,
        )?;
        self.execute_inventory(operation_id, request).await
    }
}

fn validate_domestic_futures_options_operation(operation_id: &str) -> Result<(), KisError> {
    if operation_ids().any(|candidate| candidate == operation_id) {
        Ok(())
    } else {
        Err(KisError::Validation(format!(
            "operation id {operation_id} is not a domestic futures/options endpoint"
        )))
    }
}

fn validate_collection_operation(
    collection: &str,
    operation_id: &str,
    allowed: &[&str],
) -> Result<(), KisError> {
    if allowed.contains(&operation_id) {
        Ok(())
    } else {
        Err(KisError::Validation(format!(
            "operation id {operation_id} is not a {collection} endpoint"
        )))
    }
}
