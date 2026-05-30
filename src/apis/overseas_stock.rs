use serde::de::DeserializeOwned;

use crate::{
    client::{KisClient, KisEnvelope},
    endpoint::InventoryRequest,
    error::KisError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OverseasStockEndpoint {
    PostOverseasStockTradingOrder,
    PostOverseasStockTradingOrderRvsecncl,
    PostOverseasStockTradingOrderResv,
    PostOverseasStockTradingOrderResvCcnl,
    GetOverseasStockTradingInquirePsamount,
    GetOverseasStockTradingInquireNccs,
    GetOverseasStockTradingInquireBalance,
    GetOverseasStockTradingInquireCcnl,
    GetOverseasStockTradingInquirePresentBalance,
    GetOverseasStockTradingOrderResvList,
    GetOverseasStockTradingInquirePaymtStdrBalance,
    GetOverseasStockTradingInquirePeriodTrans,
    GetOverseasStockTradingInquirePeriodProfit,
    GetOverseasStockTradingForeignMargin,
    PostOverseasStockTradingDaytimeOrder,
    PostOverseasStockTradingDaytimeOrderRvsecncl,
    GetOverseasStockTradingAlgoOrdno,
    GetOverseasStockTradingInquireAlgoCcnl,
    GetOverseasPriceQuotationsPriceDetail,
    GetOverseasPriceQuotationsInquireAskingPrice,
    GetOverseasPriceQuotationsPrice,
    GetOverseasPriceQuotationsInquireCcnl,
    GetOverseasPriceQuotationsInquireTimeItemchartprice,
    GetOverseasPriceQuotationsInquireTimeIndexchartprice,
    GetOverseasPriceQuotationsDailyprice,
    GetOverseasPriceQuotationsInquireDailyChartprice,
    GetOverseasPriceQuotationsInquireSearch,
    GetOverseasStockQuotationsCountriesHoliday,
    GetOverseasPriceQuotationsSearchInfo,
    GetOverseasPriceQuotationsIndustryTheme,
    GetOverseasPriceQuotationsIndustryPrice,
    GetOverseasPriceQuotationsMultprice,
    GetOverseasStockRankingPriceFluct,
    GetOverseasStockRankingVolumeSurge,
    GetOverseasStockRankingVolumePower,
    GetOverseasStockRankingUpdownRate,
    GetOverseasStockRankingNewHighlow,
    GetOverseasStockRankingTradeVol,
    GetOverseasStockRankingTradePbmn,
    GetOverseasStockRankingTradeGrowth,
    GetOverseasStockRankingTradeTurnover,
    GetOverseasStockRankingMarketCap,
    GetOverseasPriceQuotationsPeriodRights,
    GetOverseasPriceQuotationsNewsTitle,
    GetOverseasPriceQuotationsRightsByIce,
    GetOverseasPriceQuotationsColableByCompany,
    GetOverseasPriceQuotationsBrknewsTitle,
    PostTryitoutHdfsasp0,
    PostTryitoutHdfsasp1,
    PostTryitoutHdfscnt0,
    PostTryitoutH0gscni0,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum OverseasStockCollection {
    TradingAccount,
    Quotation,
    MarketAnalysis,
    RealtimeQuotation,
}

pub const TRADING_ACCOUNT_ENDPOINTS: &[OverseasStockEndpoint] = &[
    OverseasStockEndpoint::PostOverseasStockTradingOrder,
    OverseasStockEndpoint::PostOverseasStockTradingOrderRvsecncl,
    OverseasStockEndpoint::PostOverseasStockTradingOrderResv,
    OverseasStockEndpoint::PostOverseasStockTradingOrderResvCcnl,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePsamount,
    OverseasStockEndpoint::GetOverseasStockTradingInquireNccs,
    OverseasStockEndpoint::GetOverseasStockTradingInquireBalance,
    OverseasStockEndpoint::GetOverseasStockTradingInquireCcnl,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePresentBalance,
    OverseasStockEndpoint::GetOverseasStockTradingOrderResvList,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePaymtStdrBalance,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePeriodTrans,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePeriodProfit,
    OverseasStockEndpoint::GetOverseasStockTradingForeignMargin,
    OverseasStockEndpoint::PostOverseasStockTradingDaytimeOrder,
    OverseasStockEndpoint::PostOverseasStockTradingDaytimeOrderRvsecncl,
    OverseasStockEndpoint::GetOverseasStockTradingAlgoOrdno,
    OverseasStockEndpoint::GetOverseasStockTradingInquireAlgoCcnl,
];

pub const QUOTATION_ENDPOINTS: &[OverseasStockEndpoint] = &[
    OverseasStockEndpoint::GetOverseasPriceQuotationsPriceDetail,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireAskingPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireCcnl,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireTimeItemchartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireTimeIndexchartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsDailyprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireDailyChartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireSearch,
    OverseasStockEndpoint::GetOverseasStockQuotationsCountriesHoliday,
    OverseasStockEndpoint::GetOverseasPriceQuotationsSearchInfo,
    OverseasStockEndpoint::GetOverseasPriceQuotationsIndustryTheme,
    OverseasStockEndpoint::GetOverseasPriceQuotationsIndustryPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsMultprice,
];

pub const MARKET_ANALYSIS_ENDPOINTS: &[OverseasStockEndpoint] = &[
    OverseasStockEndpoint::GetOverseasStockRankingPriceFluct,
    OverseasStockEndpoint::GetOverseasStockRankingVolumeSurge,
    OverseasStockEndpoint::GetOverseasStockRankingVolumePower,
    OverseasStockEndpoint::GetOverseasStockRankingUpdownRate,
    OverseasStockEndpoint::GetOverseasStockRankingNewHighlow,
    OverseasStockEndpoint::GetOverseasStockRankingTradeVol,
    OverseasStockEndpoint::GetOverseasStockRankingTradePbmn,
    OverseasStockEndpoint::GetOverseasStockRankingTradeGrowth,
    OverseasStockEndpoint::GetOverseasStockRankingTradeTurnover,
    OverseasStockEndpoint::GetOverseasStockRankingMarketCap,
    OverseasStockEndpoint::GetOverseasPriceQuotationsPeriodRights,
    OverseasStockEndpoint::GetOverseasPriceQuotationsNewsTitle,
    OverseasStockEndpoint::GetOverseasPriceQuotationsRightsByIce,
    OverseasStockEndpoint::GetOverseasPriceQuotationsColableByCompany,
    OverseasStockEndpoint::GetOverseasPriceQuotationsBrknewsTitle,
];

pub const REALTIME_QUOTATION_ENDPOINTS: &[OverseasStockEndpoint] = &[
    OverseasStockEndpoint::PostTryitoutHdfsasp0,
    OverseasStockEndpoint::PostTryitoutHdfsasp1,
    OverseasStockEndpoint::PostTryitoutHdfscnt0,
    OverseasStockEndpoint::PostTryitoutH0gscni0,
];

pub const ALL_ENDPOINTS: &[OverseasStockEndpoint] = &[
    OverseasStockEndpoint::PostOverseasStockTradingOrder,
    OverseasStockEndpoint::PostOverseasStockTradingOrderRvsecncl,
    OverseasStockEndpoint::PostOverseasStockTradingOrderResv,
    OverseasStockEndpoint::PostOverseasStockTradingOrderResvCcnl,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePsamount,
    OverseasStockEndpoint::GetOverseasStockTradingInquireNccs,
    OverseasStockEndpoint::GetOverseasStockTradingInquireBalance,
    OverseasStockEndpoint::GetOverseasStockTradingInquireCcnl,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePresentBalance,
    OverseasStockEndpoint::GetOverseasStockTradingOrderResvList,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePaymtStdrBalance,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePeriodTrans,
    OverseasStockEndpoint::GetOverseasStockTradingInquirePeriodProfit,
    OverseasStockEndpoint::GetOverseasStockTradingForeignMargin,
    OverseasStockEndpoint::PostOverseasStockTradingDaytimeOrder,
    OverseasStockEndpoint::PostOverseasStockTradingDaytimeOrderRvsecncl,
    OverseasStockEndpoint::GetOverseasStockTradingAlgoOrdno,
    OverseasStockEndpoint::GetOverseasStockTradingInquireAlgoCcnl,
    OverseasStockEndpoint::GetOverseasPriceQuotationsPriceDetail,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireAskingPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireCcnl,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireTimeItemchartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireTimeIndexchartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsDailyprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireDailyChartprice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsInquireSearch,
    OverseasStockEndpoint::GetOverseasStockQuotationsCountriesHoliday,
    OverseasStockEndpoint::GetOverseasPriceQuotationsSearchInfo,
    OverseasStockEndpoint::GetOverseasPriceQuotationsIndustryTheme,
    OverseasStockEndpoint::GetOverseasPriceQuotationsIndustryPrice,
    OverseasStockEndpoint::GetOverseasPriceQuotationsMultprice,
    OverseasStockEndpoint::GetOverseasStockRankingPriceFluct,
    OverseasStockEndpoint::GetOverseasStockRankingVolumeSurge,
    OverseasStockEndpoint::GetOverseasStockRankingVolumePower,
    OverseasStockEndpoint::GetOverseasStockRankingUpdownRate,
    OverseasStockEndpoint::GetOverseasStockRankingNewHighlow,
    OverseasStockEndpoint::GetOverseasStockRankingTradeVol,
    OverseasStockEndpoint::GetOverseasStockRankingTradePbmn,
    OverseasStockEndpoint::GetOverseasStockRankingTradeGrowth,
    OverseasStockEndpoint::GetOverseasStockRankingTradeTurnover,
    OverseasStockEndpoint::GetOverseasStockRankingMarketCap,
    OverseasStockEndpoint::GetOverseasPriceQuotationsPeriodRights,
    OverseasStockEndpoint::GetOverseasPriceQuotationsNewsTitle,
    OverseasStockEndpoint::GetOverseasPriceQuotationsRightsByIce,
    OverseasStockEndpoint::GetOverseasPriceQuotationsColableByCompany,
    OverseasStockEndpoint::GetOverseasPriceQuotationsBrknewsTitle,
    OverseasStockEndpoint::PostTryitoutHdfsasp0,
    OverseasStockEndpoint::PostTryitoutHdfsasp1,
    OverseasStockEndpoint::PostTryitoutHdfscnt0,
    OverseasStockEndpoint::PostTryitoutH0gscni0,
];

impl OverseasStockEndpoint {
    pub fn all() -> &'static [Self] {
        ALL_ENDPOINTS
    }

    pub fn collection(self) -> OverseasStockCollection {
        match self {
            Self::PostOverseasStockTradingOrder
            | Self::PostOverseasStockTradingOrderRvsecncl
            | Self::PostOverseasStockTradingOrderResv
            | Self::PostOverseasStockTradingOrderResvCcnl
            | Self::GetOverseasStockTradingInquirePsamount
            | Self::GetOverseasStockTradingInquireNccs
            | Self::GetOverseasStockTradingInquireBalance
            | Self::GetOverseasStockTradingInquireCcnl
            | Self::GetOverseasStockTradingInquirePresentBalance
            | Self::GetOverseasStockTradingOrderResvList
            | Self::GetOverseasStockTradingInquirePaymtStdrBalance
            | Self::GetOverseasStockTradingInquirePeriodTrans
            | Self::GetOverseasStockTradingInquirePeriodProfit
            | Self::GetOverseasStockTradingForeignMargin
            | Self::PostOverseasStockTradingDaytimeOrder
            | Self::PostOverseasStockTradingDaytimeOrderRvsecncl
            | Self::GetOverseasStockTradingAlgoOrdno
            | Self::GetOverseasStockTradingInquireAlgoCcnl => {
                OverseasStockCollection::TradingAccount
            }
            Self::GetOverseasPriceQuotationsPriceDetail
            | Self::GetOverseasPriceQuotationsInquireAskingPrice
            | Self::GetOverseasPriceQuotationsPrice
            | Self::GetOverseasPriceQuotationsInquireCcnl
            | Self::GetOverseasPriceQuotationsInquireTimeItemchartprice
            | Self::GetOverseasPriceQuotationsInquireTimeIndexchartprice
            | Self::GetOverseasPriceQuotationsDailyprice
            | Self::GetOverseasPriceQuotationsInquireDailyChartprice
            | Self::GetOverseasPriceQuotationsInquireSearch
            | Self::GetOverseasStockQuotationsCountriesHoliday
            | Self::GetOverseasPriceQuotationsSearchInfo
            | Self::GetOverseasPriceQuotationsIndustryTheme
            | Self::GetOverseasPriceQuotationsIndustryPrice
            | Self::GetOverseasPriceQuotationsMultprice => OverseasStockCollection::Quotation,
            Self::GetOverseasStockRankingPriceFluct
            | Self::GetOverseasStockRankingVolumeSurge
            | Self::GetOverseasStockRankingVolumePower
            | Self::GetOverseasStockRankingUpdownRate
            | Self::GetOverseasStockRankingNewHighlow
            | Self::GetOverseasStockRankingTradeVol
            | Self::GetOverseasStockRankingTradePbmn
            | Self::GetOverseasStockRankingTradeGrowth
            | Self::GetOverseasStockRankingTradeTurnover
            | Self::GetOverseasStockRankingMarketCap
            | Self::GetOverseasPriceQuotationsPeriodRights
            | Self::GetOverseasPriceQuotationsNewsTitle
            | Self::GetOverseasPriceQuotationsRightsByIce
            | Self::GetOverseasPriceQuotationsColableByCompany
            | Self::GetOverseasPriceQuotationsBrknewsTitle => {
                OverseasStockCollection::MarketAnalysis
            }
            Self::PostTryitoutHdfsasp0
            | Self::PostTryitoutHdfsasp1
            | Self::PostTryitoutHdfscnt0
            | Self::PostTryitoutH0gscni0 => OverseasStockCollection::RealtimeQuotation,
        }
    }

    pub fn operation_id(self) -> &'static str {
        match self {
            Self::PostOverseasStockTradingOrder => {
                "overseas_stock_trading_account.post_overseas_stock_trading_order"
            }
            Self::PostOverseasStockTradingOrderRvsecncl => {
                "overseas_stock_trading_account.post_overseas_stock_trading_order_rvsecncl"
            }
            Self::PostOverseasStockTradingOrderResv => {
                "overseas_stock_trading_account.post_overseas_stock_trading_order_resv"
            }
            Self::PostOverseasStockTradingOrderResvCcnl => {
                "overseas_stock_trading_account.post_overseas_stock_trading_order_resv_ccnl"
            }
            Self::GetOverseasStockTradingInquirePsamount => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_psamount"
            }
            Self::GetOverseasStockTradingInquireNccs => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_nccs"
            }
            Self::GetOverseasStockTradingInquireBalance => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_balance"
            }
            Self::GetOverseasStockTradingInquireCcnl => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_ccnl"
            }
            Self::GetOverseasStockTradingInquirePresentBalance => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_present_balance"
            }
            Self::GetOverseasStockTradingOrderResvList => {
                "overseas_stock_trading_account.get_overseas_stock_trading_order_resv_list"
            }
            Self::GetOverseasStockTradingInquirePaymtStdrBalance => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_paymt_stdr_balance"
            }
            Self::GetOverseasStockTradingInquirePeriodTrans => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_period_trans"
            }
            Self::GetOverseasStockTradingInquirePeriodProfit => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_period_profit"
            }
            Self::GetOverseasStockTradingForeignMargin => {
                "overseas_stock_trading_account.get_overseas_stock_trading_foreign_margin"
            }
            Self::PostOverseasStockTradingDaytimeOrder => {
                "overseas_stock_trading_account.post_overseas_stock_trading_daytime_order"
            }
            Self::PostOverseasStockTradingDaytimeOrderRvsecncl => {
                "overseas_stock_trading_account.post_overseas_stock_trading_daytime_order_rvsecncl"
            }
            Self::GetOverseasStockTradingAlgoOrdno => {
                "overseas_stock_trading_account.get_overseas_stock_trading_algo_ordno"
            }
            Self::GetOverseasStockTradingInquireAlgoCcnl => {
                "overseas_stock_trading_account.get_overseas_stock_trading_inquire_algo_ccnl"
            }
            Self::GetOverseasPriceQuotationsPriceDetail => {
                "overseas_stock_quotation.get_overseas_price_quotations_price_detail"
            }
            Self::GetOverseasPriceQuotationsInquireAskingPrice => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_asking_price"
            }
            Self::GetOverseasPriceQuotationsPrice => {
                "overseas_stock_quotation.get_overseas_price_quotations_price"
            }
            Self::GetOverseasPriceQuotationsInquireCcnl => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_ccnl"
            }
            Self::GetOverseasPriceQuotationsInquireTimeItemchartprice => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_time_itemchartprice"
            }
            Self::GetOverseasPriceQuotationsInquireTimeIndexchartprice => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_time_indexchartprice"
            }
            Self::GetOverseasPriceQuotationsDailyprice => {
                "overseas_stock_quotation.get_overseas_price_quotations_dailyprice"
            }
            Self::GetOverseasPriceQuotationsInquireDailyChartprice => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_daily_chartprice"
            }
            Self::GetOverseasPriceQuotationsInquireSearch => {
                "overseas_stock_quotation.get_overseas_price_quotations_inquire_search"
            }
            Self::GetOverseasStockQuotationsCountriesHoliday => {
                "overseas_stock_quotation.get_overseas_stock_quotations_countries_holiday"
            }
            Self::GetOverseasPriceQuotationsSearchInfo => {
                "overseas_stock_quotation.get_overseas_price_quotations_search_info"
            }
            Self::GetOverseasPriceQuotationsIndustryTheme => {
                "overseas_stock_quotation.get_overseas_price_quotations_industry_theme"
            }
            Self::GetOverseasPriceQuotationsIndustryPrice => {
                "overseas_stock_quotation.get_overseas_price_quotations_industry_price"
            }
            Self::GetOverseasPriceQuotationsMultprice => {
                "overseas_stock_quotation.get_overseas_price_quotations_multprice"
            }
            Self::GetOverseasStockRankingPriceFluct => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_price_fluct"
            }
            Self::GetOverseasStockRankingVolumeSurge => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_volume_surge"
            }
            Self::GetOverseasStockRankingVolumePower => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_volume_power"
            }
            Self::GetOverseasStockRankingUpdownRate => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_updown_rate"
            }
            Self::GetOverseasStockRankingNewHighlow => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_new_highlow"
            }
            Self::GetOverseasStockRankingTradeVol => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_trade_vol"
            }
            Self::GetOverseasStockRankingTradePbmn => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_trade_pbmn"
            }
            Self::GetOverseasStockRankingTradeGrowth => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_trade_growth"
            }
            Self::GetOverseasStockRankingTradeTurnover => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_trade_turnover"
            }
            Self::GetOverseasStockRankingMarketCap => {
                "overseas_stock_market_analysis.get_overseas_stock_ranking_market_cap"
            }
            Self::GetOverseasPriceQuotationsPeriodRights => {
                "overseas_stock_market_analysis.get_overseas_price_quotations_period_rights"
            }
            Self::GetOverseasPriceQuotationsNewsTitle => {
                "overseas_stock_market_analysis.get_overseas_price_quotations_news_title"
            }
            Self::GetOverseasPriceQuotationsRightsByIce => {
                "overseas_stock_market_analysis.get_overseas_price_quotations_rights_by_ice"
            }
            Self::GetOverseasPriceQuotationsColableByCompany => {
                "overseas_stock_market_analysis.get_overseas_price_quotations_colable_by_company"
            }
            Self::GetOverseasPriceQuotationsBrknewsTitle => {
                "overseas_stock_market_analysis.get_overseas_price_quotations_brknews_title"
            }
            Self::PostTryitoutHdfsasp0 => "overseas_stock_realtime_quotation.post_tryitout_hdfsasp0",
            Self::PostTryitoutHdfsasp1 => "overseas_stock_realtime_quotation.post_tryitout_hdfsasp1",
            Self::PostTryitoutHdfscnt0 => "overseas_stock_realtime_quotation.post_tryitout_hdfscnt0",
            Self::PostTryitoutH0gscni0 => "overseas_stock_realtime_quotation.post_tryitout_h0gscni0",
        }
    }
}

impl OverseasStockCollection {
    pub fn endpoints(self) -> &'static [OverseasStockEndpoint] {
        match self {
            Self::TradingAccount => TRADING_ACCOUNT_ENDPOINTS,
            Self::Quotation => QUOTATION_ENDPOINTS,
            Self::MarketAnalysis => MARKET_ANALYSIS_ENDPOINTS,
            Self::RealtimeQuotation => REALTIME_QUOTATION_ENDPOINTS,
        }
    }

    pub fn inventory_slug(self) -> &'static str {
        match self {
            Self::TradingAccount => "overseas_stock_trading_account",
            Self::Quotation => "overseas_stock_quotation",
            Self::MarketAnalysis => "overseas_stock_market_analysis",
            Self::RealtimeQuotation => "overseas_stock_realtime_quotation",
        }
    }
}

impl KisClient {
    pub async fn execute_overseas_stock<T>(
        &self,
        endpoint: OverseasStockEndpoint,
        request: InventoryRequest,
    ) -> Result<KisEnvelope<T>, KisError>
    where
        T: DeserializeOwned,
    {
        self.execute_inventory(endpoint.operation_id(), request)
            .await
    }
}
