use serde::de::DeserializeOwned;

use crate::{
    client::{KisClient, KisEnvelope},
    endpoint::InventoryRequest,
    error::KisError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverseasFuturesOptionsEndpoint {
    Order,
    OrderRevisionCancellation,
    InquireCcld,
    InquireUnpd,
    InquirePsamount,
    InquirePeriodCcld,
    InquireDailyCcld,
    InquireDeposit,
    InquireDailyOrder,
    InquirePeriodTrans,
    MarginDetail,
    InquirePrice,
    StockDetail,
    InquireAskingPrice,
    InquireTimeFutureChartPrice,
    TickCcnl,
    WeeklyCcnl,
    DailyCcnl,
    MonthlyCcnl,
    SearchContractDetail,
    InvestorUnpdTrend,
    OptPrice,
    OptDetail,
    OptAskingPrice,
    InquireTimeOptChartPrice,
    OptTickCcnl,
    OptDailyCcnl,
    OptWeeklyCcnl,
    OptMonthlyCcnl,
    SearchOptDetail,
    MarketTime,
    RealtimeExecution,
    RealtimeQuote,
    RealtimeExecutionNotice,
    RealtimeOrderNotice,
}

impl OverseasFuturesOptionsEndpoint {
    pub const ALL: [Self; 35] = [
        Self::Order,
        Self::OrderRevisionCancellation,
        Self::InquireCcld,
        Self::InquireUnpd,
        Self::InquirePsamount,
        Self::InquirePeriodCcld,
        Self::InquireDailyCcld,
        Self::InquireDeposit,
        Self::InquireDailyOrder,
        Self::InquirePeriodTrans,
        Self::MarginDetail,
        Self::InquirePrice,
        Self::StockDetail,
        Self::InquireAskingPrice,
        Self::InquireTimeFutureChartPrice,
        Self::TickCcnl,
        Self::WeeklyCcnl,
        Self::DailyCcnl,
        Self::MonthlyCcnl,
        Self::SearchContractDetail,
        Self::InvestorUnpdTrend,
        Self::OptPrice,
        Self::OptDetail,
        Self::OptAskingPrice,
        Self::InquireTimeOptChartPrice,
        Self::OptTickCcnl,
        Self::OptDailyCcnl,
        Self::OptWeeklyCcnl,
        Self::OptMonthlyCcnl,
        Self::SearchOptDetail,
        Self::MarketTime,
        Self::RealtimeExecution,
        Self::RealtimeQuote,
        Self::RealtimeExecutionNotice,
        Self::RealtimeOrderNotice,
    ];

    pub fn operation_id(self) -> &'static str {
        match self {
            Self::Order => "overseas_futures_options_trading_account.post_overseas_futureoption_trading_order",
            Self::OrderRevisionCancellation => "overseas_futures_options_trading_account.post_overseas_futureoption_trading_order_rvsecncl",
            Self::InquireCcld => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_ccld",
            Self::InquireUnpd => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_unpd",
            Self::InquirePsamount => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_psamount",
            Self::InquirePeriodCcld => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_period_ccld",
            Self::InquireDailyCcld => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_daily_ccld",
            Self::InquireDeposit => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_deposit",
            Self::InquireDailyOrder => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_daily_order",
            Self::InquirePeriodTrans => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_inquire_period_trans",
            Self::MarginDetail => "overseas_futures_options_trading_account.get_overseas_futureoption_trading_margin_detail",
            Self::InquirePrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_inquire_price",
            Self::StockDetail => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_stock_detail",
            Self::InquireAskingPrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_inquire_asking_price",
            Self::InquireTimeFutureChartPrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_inquire_time_futurechartprice",
            Self::TickCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_tick_ccnl",
            Self::WeeklyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_weekly_ccnl",
            Self::DailyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_daily_ccnl",
            Self::MonthlyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_monthly_ccnl",
            Self::SearchContractDetail => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_search_contract_detail",
            Self::InvestorUnpdTrend => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_investor_unpd_trend",
            Self::OptPrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_price",
            Self::OptDetail => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_detail",
            Self::OptAskingPrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_asking_price",
            Self::InquireTimeOptChartPrice => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_inquire_time_optchartprice",
            Self::OptTickCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_tick_ccnl",
            Self::OptDailyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_daily_ccnl",
            Self::OptWeeklyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_weekly_ccnl",
            Self::OptMonthlyCcnl => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_opt_monthly_ccnl",
            Self::SearchOptDetail => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_search_opt_detail",
            Self::MarketTime => "overseas_futures_options_quotation.get_overseas_futureoption_quotations_market_time",
            Self::RealtimeExecution => "overseas_futures_options_realtime_quotation.post_tryitout_hdfff020",
            Self::RealtimeQuote => "overseas_futures_options_realtime_quotation.post_tryitout_hdfff010",
            Self::RealtimeExecutionNotice => "overseas_futures_options_realtime_quotation.post_tryitout_hdfff1c0",
            Self::RealtimeOrderNotice => "overseas_futures_options_realtime_quotation.post_tryitout_hdfff2c0",
        }
    }
}

impl KisClient {
    pub async fn execute_overseas_futures_options<T>(
        &self,
        endpoint: OverseasFuturesOptionsEndpoint,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        self.execute_inventory(endpoint.operation_id(), request)
            .await
    }
}
