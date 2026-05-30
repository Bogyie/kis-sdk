use serde::de::DeserializeOwned;
use std::{fmt, str::FromStr};

use crate::{
    client::{KisClient, KisEnvelope},
    endpoint::InventoryRequest,
    error::KisError,
};

pub const BUY_ORDER: &str = "bond_trading_account.post_domestic_bond_trading_buy";
pub const SELL_ORDER: &str = "bond_trading_account.post_domestic_bond_trading_sell";
pub const REVISE_CANCEL_ORDER: &str =
    "bond_trading_account.post_domestic_bond_trading_order_rvsecncl";
pub const INQUIRE_REVERSIBLE_CANCELABLE_ORDERS: &str =
    "bond_trading_account.get_domestic_bond_trading_inquire_psbl_rvsecncl";
pub const INQUIRE_DAILY_EXECUTIONS: &str =
    "bond_trading_account.get_domestic_bond_trading_inquire_daily_ccld";
pub const INQUIRE_BALANCE: &str = "bond_trading_account.get_domestic_bond_trading_inquire_balance";
pub const INQUIRE_BUYABLE_ORDER: &str =
    "bond_trading_account.get_domestic_bond_trading_inquire_psbl_order";

pub const INQUIRE_ASKING_PRICE: &str =
    "bond_quotation.get_domestic_bond_quotations_inquire_asking_price";
pub const INQUIRE_PRICE: &str = "bond_quotation.get_domestic_bond_quotations_inquire_price";
pub const INQUIRE_EXECUTIONS: &str = "bond_quotation.get_domestic_bond_quotations_inquire_ccnl";
pub const INQUIRE_DAILY_PRICE: &str =
    "bond_quotation.get_domestic_bond_quotations_inquire_daily_price";
pub const INQUIRE_DAILY_ITEM_CHART_PRICE: &str =
    "bond_quotation.get_domestic_bond_quotations_inquire_daily_itemchartprice";
pub const INQUIRE_AVG_UNIT: &str = "bond_quotation.get_domestic_bond_quotations_avg_unit";
pub const INQUIRE_ISSUE_INFO: &str = "bond_quotation.get_domestic_bond_quotations_issue_info";
pub const SEARCH_BOND_INFO: &str = "bond_quotation.get_domestic_bond_quotations_search_bond_info";

pub const REALTIME_TRADE: &str = "bond_realtime_quotation.post_tryitout_h0bjcnt0";
pub const REALTIME_ASKING_PRICE: &str = "bond_realtime_quotation.post_tryitout_h0bjasp0";
pub const INDEX_REALTIME_TRADE: &str = "bond_realtime_quotation.post_tryitout_h0bicnt0";

pub const BOND_TRADING_ACCOUNT_OPERATIONS: [&str; 7] = [
    BUY_ORDER,
    SELL_ORDER,
    REVISE_CANCEL_ORDER,
    INQUIRE_REVERSIBLE_CANCELABLE_ORDERS,
    INQUIRE_DAILY_EXECUTIONS,
    INQUIRE_BALANCE,
    INQUIRE_BUYABLE_ORDER,
];

pub const BOND_QUOTATION_OPERATIONS: [&str; 8] = [
    INQUIRE_ASKING_PRICE,
    INQUIRE_PRICE,
    INQUIRE_EXECUTIONS,
    INQUIRE_DAILY_PRICE,
    INQUIRE_DAILY_ITEM_CHART_PRICE,
    INQUIRE_AVG_UNIT,
    INQUIRE_ISSUE_INFO,
    SEARCH_BOND_INFO,
];

pub const BOND_REALTIME_TRYITOUT_OPERATIONS: [&str; 3] =
    [REALTIME_TRADE, REALTIME_ASKING_PRICE, INDEX_REALTIME_TRADE];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BondTradingAccountOperation(&'static str);

impl BondTradingAccountOperation {
    pub const BUY_ORDER: Self = Self(BUY_ORDER);
    pub const SELL_ORDER: Self = Self(SELL_ORDER);
    pub const REVISE_CANCEL_ORDER: Self = Self(REVISE_CANCEL_ORDER);
    pub const INQUIRE_REVERSIBLE_CANCELABLE_ORDERS: Self =
        Self(INQUIRE_REVERSIBLE_CANCELABLE_ORDERS);
    pub const INQUIRE_DAILY_EXECUTIONS: Self = Self(INQUIRE_DAILY_EXECUTIONS);
    pub const INQUIRE_BALANCE: Self = Self(INQUIRE_BALANCE);
    pub const INQUIRE_BUYABLE_ORDER: Self = Self(INQUIRE_BUYABLE_ORDER);

    pub fn operation_id(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for BondTradingAccountOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.operation_id())
    }
}

impl FromStr for BondTradingAccountOperation {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        typed_operation_id(
            value,
            &BOND_TRADING_ACCOUNT_OPERATIONS,
            "bond trading/account",
        )
        .map(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BondQuotationOperation(&'static str);

impl BondQuotationOperation {
    pub const INQUIRE_ASKING_PRICE: Self = Self(INQUIRE_ASKING_PRICE);
    pub const INQUIRE_PRICE: Self = Self(INQUIRE_PRICE);
    pub const INQUIRE_EXECUTIONS: Self = Self(INQUIRE_EXECUTIONS);
    pub const INQUIRE_DAILY_PRICE: Self = Self(INQUIRE_DAILY_PRICE);
    pub const INQUIRE_DAILY_ITEM_CHART_PRICE: Self = Self(INQUIRE_DAILY_ITEM_CHART_PRICE);
    pub const INQUIRE_AVG_UNIT: Self = Self(INQUIRE_AVG_UNIT);
    pub const INQUIRE_ISSUE_INFO: Self = Self(INQUIRE_ISSUE_INFO);
    pub const SEARCH_BOND_INFO: Self = Self(SEARCH_BOND_INFO);

    pub fn operation_id(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for BondQuotationOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.operation_id())
    }
}

impl FromStr for BondQuotationOperation {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        typed_operation_id(value, &BOND_QUOTATION_OPERATIONS, "bond quotation").map(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct BondRealtimeOperation(&'static str);

impl BondRealtimeOperation {
    pub const REALTIME_TRADE: Self = Self(REALTIME_TRADE);
    pub const REALTIME_ASKING_PRICE: Self = Self(REALTIME_ASKING_PRICE);
    pub const INDEX_REALTIME_TRADE: Self = Self(INDEX_REALTIME_TRADE);

    pub fn operation_id(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for BondRealtimeOperation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.operation_id())
    }
}

impl FromStr for BondRealtimeOperation {
    type Err = KisError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        typed_operation_id(
            value,
            &BOND_REALTIME_TRYITOUT_OPERATIONS,
            "bond realtime tryitout",
        )
        .map(Self)
    }
}

impl KisClient {
    pub async fn execute_bond_trading_account_operation<T>(
        &self,
        operation: BondTradingAccountOperation,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        self.execute_bond_trading_account(operation.operation_id(), request)
            .await
    }

    pub async fn execute_bond_trading_account<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        ensure_operation(
            operation_id,
            &BOND_TRADING_ACCOUNT_OPERATIONS,
            "bond trading/account",
        )?;
        self.execute_inventory(operation_id, request).await
    }

    pub async fn execute_bond_quotation_operation<T>(
        &self,
        operation: BondQuotationOperation,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        self.execute_bond_quotation(operation.operation_id(), request)
            .await
    }

    pub async fn execute_bond_quotation<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        ensure_operation(operation_id, &BOND_QUOTATION_OPERATIONS, "bond quotation")?;
        self.execute_inventory(operation_id, request).await
    }

    pub async fn execute_bond_realtime_tryitout_operation<T>(
        &self,
        operation: BondRealtimeOperation,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        self.execute_bond_realtime_tryitout(operation.operation_id(), request)
            .await
    }

    pub async fn execute_bond_realtime_tryitout<T>(
        &self,
        operation_id: &str,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        ensure_operation(
            operation_id,
            &BOND_REALTIME_TRYITOUT_OPERATIONS,
            "bond realtime tryitout",
        )?;
        self.execute_inventory(operation_id, request).await
    }
}

fn typed_operation_id(
    operation_id: &str,
    allowed: &[&'static str],
    label: &str,
) -> Result<&'static str, KisError> {
    allowed
        .iter()
        .copied()
        .find(|candidate| *candidate == operation_id)
        .ok_or_else(|| KisError::Validation(format!("{operation_id} is not a {label} operation")))
}

fn ensure_operation(operation_id: &str, allowed: &[&str], label: &str) -> Result<(), KisError> {
    if allowed.contains(&operation_id) {
        Ok(())
    } else {
        Err(KisError::Validation(format!(
            "{operation_id} is not a {label} operation"
        )))
    }
}
