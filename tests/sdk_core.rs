use axum::{
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use kis_sdk::{
    apis::{
        bond::{
            self, BondQuotationOperation, BondRealtimeOperation, BondTradingAccountOperation,
            BOND_QUOTATION_OPERATIONS, BOND_REALTIME_TRYITOUT_OPERATIONS,
            BOND_TRADING_ACCOUNT_OPERATIONS,
        },
        domestic_futures_options::{
            operation_ids as domestic_futures_options_operation_ids,
            DomesticFuturesOptionsOperation, QUOTATION_OPERATION_IDS,
            REALTIME_QUOTATION_OPERATION_IDS, TRADING_ACCOUNT_OPERATION_IDS,
        },
        domestic_stock::{
            domestic_stock_rest_endpoints, CashOrderDivision, CashOrderRequest, CashOrderSide,
            DomesticStockMarketDivision, InquireBalanceRequest, InquirePriceRequest,
            DOMESTIC_STOCK_REST_COLLECTIONS, DOMESTIC_STOCK_REST_ENDPOINT_COUNT,
        },
        domestic_stock_realtime::{
            self, DomesticStockRealtimeOperation, DOMESTIC_STOCK_REALTIME_TRYITOUT_OPERATIONS,
        },
        overseas_futures_options::OverseasFuturesOptionsEndpoint,
        overseas_stock::{
            OverseasStockEndpoint, MARKET_ANALYSIS_ENDPOINTS, QUOTATION_ENDPOINTS,
            REALTIME_QUOTATION_ENDPOINTS, TRADING_ACCOUNT_ENDPOINTS,
        },
    },
    config::Environment,
    contract::EnvironmentSupport,
    credentials::{Account, AccountProductCode, AppCredentials, SecretString},
    endpoint::{InventoryCatalog, InventoryEndpointSpec, InventoryRequest, OperationKind},
    error::KisError,
    fallback::FallbackPolicy,
    mock::MockServer,
    retry::RetryPolicy,
    AccessTokenResponse, KisClient, RealtimeApprovalKeyResponse,
};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;
use tokio::{net::TcpListener, task::JoinHandle};

#[tokio::test]
async fn client_calls_mocked_domestic_stock_read_and_order_slice() {
    let server = MockServer::start().await.expect("mock server starts");
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect("quote succeeds");
    assert!(quote.is_success());
    assert!(quote.output.is_some());

    let balance = client
        .inquire_domestic_stock_balance(&InquireBalanceRequest::new(&account))
        .await
        .expect("balance succeeds");
    assert!(balance.is_success());

    let order = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 1, 70000),
        )
        .await
        .expect("mock cash order succeeds");
    assert!(order.is_success());

    server.shutdown().await;
}

#[test]
fn inventory_catalog_addresses_every_official_endpoint_with_unique_operation_ids() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    assert_eq!(catalog.endpoint_count(), 338);

    for endpoint in catalog.endpoints() {
        assert!(
            catalog.endpoint(&endpoint.operation_id).is_some(),
            "{} must be addressable by operation id",
            endpoint.operation_id
        );
        assert!(
            !endpoint.operation_id.contains("unknown_collection"),
            "{} must use a curated collection slug",
            endpoint.operation_id
        );
    }
}

#[test]
fn full_inventory_reconciliation_accounts_for_every_official_endpoint_once() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");
    let mut coverage = BTreeMap::<&str, HashSet<String>>::new();

    coverage.insert(
        "oauth_typed_methods",
        HashSet::from([
            "oauth_authentication.post_oauth2_tokenp".to_string(),
            "oauth_authentication.post_oauth2_revokep".to_string(),
            "oauth_authentication.post_oauth2_approval".to_string(),
        ]),
    );
    coverage.insert(
        "domestic_stock_rest_inventory_api",
        domestic_stock_rest_endpoints()
            .expect("domestic stock REST catalog builds")
            .iter()
            .map(|endpoint| endpoint.operation_id.clone())
            .collect(),
    );
    coverage.insert(
        "domestic_stock_realtime_tryitout_inventory_api",
        DOMESTIC_STOCK_REALTIME_TRYITOUT_OPERATIONS
            .iter()
            .map(|operation_id| (*operation_id).to_string())
            .collect(),
    );
    coverage.insert(
        "domestic_futures_options_inventory_api",
        domestic_futures_options_operation_ids()
            .map(str::to_string)
            .collect(),
    );
    coverage.insert(
        "overseas_stock_inventory_api",
        OverseasStockEndpoint::all()
            .iter()
            .map(|endpoint| endpoint.operation_id().to_string())
            .collect(),
    );
    coverage.insert(
        "overseas_futures_options_inventory_api",
        OverseasFuturesOptionsEndpoint::ALL
            .iter()
            .map(|endpoint| endpoint.operation_id().to_string())
            .collect(),
    );
    coverage.insert(
        "listed_bond_inventory_api",
        BOND_TRADING_ACCOUNT_OPERATIONS
            .iter()
            .chain(BOND_QUOTATION_OPERATIONS.iter())
            .chain(BOND_REALTIME_TRYITOUT_OPERATIONS.iter())
            .map(|operation_id| (*operation_id).to_string())
            .collect(),
    );

    let official = catalog
        .endpoints()
        .iter()
        .map(|endpoint| endpoint.operation_id.clone())
        .collect::<HashSet<_>>();
    let mut accounted = HashSet::new();

    for (surface, operation_ids) in &coverage {
        assert!(!operation_ids.is_empty(), "{surface} must not be empty");
        for operation_id in operation_ids {
            assert!(
                official.contains(operation_id),
                "{surface} includes unknown operation id {operation_id}"
            );
            assert!(
                accounted.insert(operation_id.clone()),
                "{operation_id} is assigned to more than one inventory coverage surface"
            );
        }
    }

    let missing = official.difference(&accounted).cloned().collect::<Vec<_>>();
    let unexpected = accounted.difference(&official).cloned().collect::<Vec<_>>();

    assert_eq!(accounted.len(), 338);
    assert!(
        missing.is_empty(),
        "unaccounted inventory endpoints: {missing:?}"
    );
    assert!(
        unexpected.is_empty(),
        "coverage references non-inventory endpoints: {unexpected:?}"
    );
    assert_eq!(coverage["oauth_typed_methods"].len(), 3);
    assert_eq!(coverage["domestic_stock_rest_inventory_api"].len(), 158);
    assert_eq!(
        coverage["domestic_stock_realtime_tryitout_inventory_api"].len(),
        29
    );
    assert_eq!(coverage["domestic_futures_options_inventory_api"].len(), 44);
    assert_eq!(coverage["overseas_stock_inventory_api"].len(), 51);
    assert_eq!(coverage["overseas_futures_options_inventory_api"].len(), 35);
    assert_eq!(coverage["listed_bond_inventory_api"].len(), 18);
}

#[test]
fn inventory_operation_kind_uses_contract_kind_not_http_method_only() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    let realtime = catalog
        .endpoint("domestic_stock_realtime_quotation.post_tryitout_h0stcnt0")
        .expect("realtime operation exists");
    assert_eq!(realtime.operation_kind, OperationKind::Read);

    let cash_order = catalog
        .endpoint("domestic_stock_trading_account.post_domestic_stock_trading_order_cash")
        .expect("cash order operation exists");
    assert_eq!(cash_order.operation_kind, OperationKind::TradingMutation);

    let balance = catalog
        .endpoint("domestic_stock_trading_account.get_domestic_stock_trading_inquire_balance")
        .expect("balance operation exists");
    assert_eq!(balance.operation_kind, OperationKind::Read);
}

#[test]
fn overseas_stock_sdk_surface_covers_all_inventory_endpoints() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");
    let covered = OverseasStockEndpoint::all()
        .iter()
        .map(|endpoint| endpoint.operation_id())
        .collect::<HashSet<_>>();
    let inventory = catalog
        .endpoints()
        .iter()
        .filter(|endpoint| {
            matches!(
                endpoint.collection_name.as_str(),
                "[해외주식] 주문/계좌"
                    | "[해외주식] 기본시세"
                    | "[해외주식] 시세분석"
                    | "[해외주식] 실시간시세"
            )
        })
        .map(|endpoint| endpoint.operation_id.as_str())
        .collect::<HashSet<_>>();

    assert_eq!(TRADING_ACCOUNT_ENDPOINTS.len(), 18);
    assert_eq!(QUOTATION_ENDPOINTS.len(), 14);
    assert_eq!(MARKET_ANALYSIS_ENDPOINTS.len(), 15);
    assert_eq!(REALTIME_QUOTATION_ENDPOINTS.len(), 4);
    assert_eq!(OverseasStockEndpoint::all().len(), 51);
    assert_eq!(covered, inventory);

    for endpoint in OverseasStockEndpoint::all() {
        let collection = endpoint.collection();
        assert!(
            endpoint
                .operation_id()
                .starts_with(collection.inventory_slug()),
            "{} must stay in its inventory-backed collection",
            endpoint.operation_id()
        );
        assert!(
            collection.endpoints().contains(endpoint),
            "{} must be listed in its collection slice",
            endpoint.operation_id()
        );
    }
}

#[test]
fn domestic_realtime_and_bond_domain_operations_cover_target_inventory_collections() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    assert_domain_operations(
        &catalog,
        "[국내주식] 실시간시세",
        &DOMESTIC_STOCK_REALTIME_TRYITOUT_OPERATIONS,
        29,
    );
    assert_domain_operations(
        &catalog,
        "[장내채권] 주문/계좌",
        &BOND_TRADING_ACCOUNT_OPERATIONS,
        7,
    );
    assert_domain_operations(
        &catalog,
        "[장내채권] 기본시세",
        &BOND_QUOTATION_OPERATIONS,
        8,
    );
    assert_domain_operations(
        &catalog,
        "[장내채권] 실시간시세",
        &BOND_REALTIME_TRYITOUT_OPERATIONS,
        3,
    );
}

#[test]
fn typed_domestic_stock_values_serialize_and_parse_contract_codes() {
    let account = Account::domestic_stock("12345678");

    let price = InquirePriceRequest::with_market(DomesticStockMarketDivision::Stock, "005930");
    assert_eq!(
        serde_json::to_value(&price).expect("price request serializes"),
        json!({
            "FID_COND_MRKT_DIV_CODE": "J",
            "FID_INPUT_ISCD": "005930"
        })
    );

    let order = CashOrderRequest::with_order_division(
        &account,
        "005930",
        CashOrderDivision::Limit,
        1,
        70000,
    );
    assert_eq!(
        serde_json::to_value(&order).expect("cash order serializes"),
        json!({
            "CANO": "12345678",
            "ACNT_PRDT_CD": "01",
            "PDNO": "005930",
            "ORD_DVSN": "00",
            "ORD_QTY": "1",
            "ORD_UNPR": "70000"
        })
    );

    assert_eq!(
        AccountProductCode::from_str("01").expect("account product parses"),
        AccountProductCode::DomesticStock
    );
    assert_eq!(
        serde_json::to_value(AccountProductCode::DomesticStock)
            .expect("account product serializes"),
        json!("01")
    );
    assert_eq!(
        DomesticStockMarketDivision::from_str("J").expect("market division parses"),
        DomesticStockMarketDivision::Stock
    );
    assert_eq!(
        serde_json::to_value(DomesticStockMarketDivision::Stock)
            .expect("market division serializes"),
        json!("J")
    );
    assert_eq!(
        CashOrderDivision::from_str("00").expect("cash order division parses"),
        CashOrderDivision::Limit
    );
    assert_eq!(
        serde_json::to_value(CashOrderDivision::Limit).expect("order division serializes"),
        json!("00")
    );
    assert!(AccountProductCode::from_str("99").is_err());
    assert!(DomesticStockMarketDivision::from_str("K").is_err());
    assert!(CashOrderDivision::from_str("10").is_err());
}

#[test]
fn typed_operation_newtypes_parse_and_reject_unknown_operation_ids() {
    let bond_quotation = BondQuotationOperation::from_str(bond::INQUIRE_PRICE)
        .expect("bond quotation operation parses");
    assert_eq!(bond_quotation.operation_id(), bond::INQUIRE_PRICE);
    assert_eq!(
        BondTradingAccountOperation::BUY_ORDER.operation_id(),
        bond::BUY_ORDER
    );
    assert_eq!(
        BondRealtimeOperation::REALTIME_TRADE.operation_id(),
        bond::REALTIME_TRADE
    );

    let realtime =
        DomesticStockRealtimeOperation::from_str(domestic_stock_realtime::REALTIME_TRADE_KRX)
            .expect("domestic realtime operation parses");
    assert_eq!(
        realtime.operation_id(),
        domestic_stock_realtime::REALTIME_TRADE_KRX
    );

    let domestic_future =
        DomesticFuturesOptionsOperation::from_str(TRADING_ACCOUNT_OPERATION_IDS[0])
            .expect("domestic futures/options operation parses");
    assert_eq!(
        domestic_future.operation_id(),
        TRADING_ACCOUNT_OPERATION_IDS[0]
    );

    assert!(BondQuotationOperation::from_str(bond::BUY_ORDER).is_err());
    assert!(DomesticStockRealtimeOperation::from_str(bond::REALTIME_TRADE).is_err());
    assert!(DomesticFuturesOptionsOperation::from_str("unknown.operation").is_err());
}

#[tokio::test]
async fn overseas_stock_execute_calls_mock_supported_price_endpoint() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let response = client
        .execute_overseas_stock::<serde_json::Value>(
            OverseasStockEndpoint::GetOverseasPriceQuotationsPrice,
            InventoryRequest::new().query(json!({
                "AUTH": "",
                "EXCD": "NAS",
                "SYMB": "AAPL"
            })),
        )
        .await
        .expect("overseas price succeeds through mock");

    assert!(response.is_success());
    assert!(response.output.is_some());

    server.shutdown().await;
}

#[tokio::test]
async fn domestic_stock_realtime_tryitout_api_calls_mock_contract_endpoint() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let response = client
        .execute_domestic_stock_realtime_tryitout_operation::<serde_json::Value>(
            DomesticStockRealtimeOperation::REALTIME_TRADE_KRX,
            InventoryRequest::new()
                .header("approval_key", "test_approval_key")
                .header("tr_type", "1")
                .body(json!({
                    "tr_id": "H0STCNT0",
                    "tr_key": "005930"
                })),
        )
        .await
        .expect("domestic realtime tryitout succeeds against mock");

    assert!(response.is_success());

    server.shutdown().await;
}

#[tokio::test]
async fn overseas_stock_execute_rejects_missing_required_query_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_stock::<serde_json::Value>(
            OverseasStockEndpoint::GetOverseasPriceQuotationsPrice,
            InventoryRequest::new().query(json!({
                "AUTH": "",
                "EXCD": "NAS"
            })),
        )
        .await
        .expect_err("missing SYMB should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn domain_wrappers_reject_operations_from_other_collections_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_bond_quotation::<serde_json::Value>(
            domestic_stock_realtime::REALTIME_TRADE_KRX,
            InventoryRequest::new(),
        )
        .await
        .expect_err("wrong collection should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn overseas_stock_order_requires_tr_id_choice_for_ambiguous_inventory_ids() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_stock::<serde_json::Value>(
            OverseasStockEndpoint::PostOverseasStockTradingOrder,
            InventoryRequest::new().body(json!({
                "CANO": "12345678",
                "ACNT_PRDT_CD": "01",
                "OVRS_EXCG_CD": "NASD",
                "PDNO": "AAPL",
                "ORD_SVR_DVSN_CD": "0",
                "ORD_DVSN": "00",
                "ORD_QTY": "1",
                "OVRS_ORD_UNPR": "100.00"
            })),
        )
        .await
        .expect_err("ambiguous overseas order TR ID should require override");

    assert!(matches!(error, KisError::AmbiguousTrId { .. }));
}

#[tokio::test]
async fn overseas_stock_real_order_is_blocked_even_with_explicit_tr_id() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_stock::<serde_json::Value>(
            OverseasStockEndpoint::PostOverseasStockTradingOrder,
            InventoryRequest::new()
                .tr_id_override("TTTT1002U")
                .body(json!({
                    "CANO": "12345678",
                    "ACNT_PRDT_CD": "01",
                    "OVRS_EXCG_CD": "NASD",
                    "PDNO": "AAPL",
                    "ORD_SVR_DVSN_CD": "0",
                    "ORD_DVSN": "00",
                    "ORD_QTY": "1",
                    "OVRS_ORD_UNPR": "100.00"
                })),
        )
        .await
        .expect_err("real overseas order should be locally blocked before network");

    assert!(matches!(error, KisError::LiveTradingDisabled { .. }));
}

#[tokio::test]
async fn bond_domain_apis_preserve_inventory_validation_and_safety_guards() {
    let read_client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let missing_query_error = read_client
        .execute_bond_quotation_operation::<serde_json::Value>(
            BondQuotationOperation::INQUIRE_PRICE,
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "B"
            })),
        )
        .await
        .expect_err("missing required bond query field should fail locally");
    assert!(matches!(missing_query_error, KisError::Validation(_)));

    let mock_client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let unsupported_mock_error = mock_client
        .execute_bond_realtime_tryitout_operation::<serde_json::Value>(
            BondRealtimeOperation::REALTIME_TRADE,
            InventoryRequest::new()
                .header("approval_key", "test_approval_key")
                .header("tr_type", "1")
                .body(json!({
                    "tr_id": "H0BJCNT0",
                    "tr_key": "KR103502GA34"
                })),
        )
        .await
        .expect_err("real-only bond realtime endpoint should not run against mock");
    assert!(matches!(
        unsupported_mock_error,
        KisError::UnsupportedEnvironment { .. }
    ));

    let live_trading_error = read_client
        .execute_bond_trading_account_operation::<serde_json::Value>(
            BondTradingAccountOperation::BUY_ORDER,
            InventoryRequest::new().body(json!({
                "ACNT_PRDT_CD": "01",
                "BOND_ORD_UNPR": "10000",
                "BOND_RTL_MKET_YN": "N",
                "CANO": "12345678",
                "CTAC_TLNO": "01000000000",
                "IDCR_STFNO": "",
                "MGCO_APTM_ODNO": "",
                "ORD_QTY2": "1",
                "ORD_SVR_DVSN_CD": "0",
                "PDNO": "KR103502GA34",
                "SAMT_MKET_PTCI_YN": "N"
            })),
        )
        .await
        .expect_err("real bond order should be blocked before network");
    assert!(matches!(
        live_trading_error,
        KisError::LiveTradingDisabled { .. }
    ));
}

#[test]
fn overseas_futures_options_sdk_surface_covers_inventory_slice() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");

    assert_eq!(OverseasFuturesOptionsEndpoint::ALL.len(), 35);

    let mut trading_account = 0;
    let mut quotations = 0;
    let mut realtime = 0;

    for endpoint in OverseasFuturesOptionsEndpoint::ALL {
        let spec = catalog
            .endpoint(endpoint.operation_id())
            .unwrap_or_else(|| panic!("{} exists in inventory", endpoint.operation_id()));

        assert_eq!(spec.env_support, EnvironmentSupport::RealOnly);

        match spec.collection_name.as_str() {
            "[해외선물옵션] 주문/계좌" => trading_account += 1,
            "[해외선물옵션] 기본시세" => quotations += 1,
            "[해외선물옵션]실시간시세" => realtime += 1,
            other => panic!("unexpected overseas futures/options collection {other}"),
        }
    }

    assert_eq!(trading_account, 11);
    assert_eq!(quotations, 20);
    assert_eq!(realtime, 4);
}

#[tokio::test]
async fn inventory_execute_calls_mocked_endpoint_by_operation_id() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let response = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await
        .expect("inventory-backed quote succeeds");

    assert!(response.is_success());
    assert!(response.output.is_some());

    server.shutdown().await;
}

#[test]
fn domestic_stock_rest_catalog_covers_listed_inventory_collections() {
    let endpoints = domestic_stock_rest_endpoints().expect("domestic stock REST catalog builds");

    assert_eq!(endpoints.len(), DOMESTIC_STOCK_REST_ENDPOINT_COUNT);

    let mut by_collection = BTreeMap::new();
    for endpoint in &endpoints {
        assert!(
            DOMESTIC_STOCK_REST_COLLECTIONS.contains(&endpoint.collection_name.as_str()),
            "{} must stay inside listed domestic stock REST collections",
            endpoint.operation_id
        );
        assert!(
            !endpoint.collection_name.contains("실시간시세"),
            "{} must not include realtime websocket coverage",
            endpoint.operation_id
        );
        *by_collection
            .entry(endpoint.collection_name.as_str())
            .or_insert(0usize) += 1;
    }

    assert_eq!(by_collection["[국내주식] 주문/계좌"], 23);
    assert_eq!(by_collection["[국내주식] 기본시세"], 22);
    assert_eq!(by_collection["[국내주식] ELW 시세"], 22);
    assert_eq!(by_collection["[국내주식] 업종/기타"], 14);
    assert_eq!(by_collection["[국내주식] 종목정보"], 26);
    assert_eq!(by_collection["[국내주식] 시세분석"], 29);
    assert_eq!(by_collection["[국내주식] 순위분석"], 22);
}

#[tokio::test]
async fn domestic_stock_rest_execute_covers_listed_inventory_against_mock_contract() {
    let endpoints = domestic_stock_rest_endpoints().expect("domestic stock REST catalog builds");
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let mut real_mock_successes = 0;
    let mut real_only_rejections = 0;

    for endpoint in &endpoints {
        let request = inventory_request_for_endpoint(endpoint);
        let result = client
            .execute_domestic_stock_rest::<serde_json::Value>(&endpoint.operation_id, request)
            .await;

        match endpoint.env_support {
            EnvironmentSupport::RealOnly => {
                assert!(
                    matches!(result, Err(KisError::UnsupportedEnvironment { .. })),
                    "{} must reject real-only endpoints in mock before network",
                    endpoint.operation_id
                );
                real_only_rejections += 1;
            }
            EnvironmentSupport::RealMock => {
                let response = result.unwrap_or_else(|error| {
                    panic!(
                        "{} should execute against mock: {error:?}",
                        endpoint.operation_id
                    )
                });
                assert!(response.is_success(), "{}", endpoint.operation_id);
                real_mock_successes += 1;
            }
        }
    }

    assert_eq!(real_mock_successes, 18);
    assert_eq!(real_only_rejections, 140);

    server.shutdown().await;
}

#[tokio::test]
async fn domestic_stock_rest_execute_rejects_out_of_scope_operation_ids() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_domestic_stock_rest::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new(),
        )
        .await
        .expect_err("realtime operation should stay out of REST coverage");

    assert!(matches!(error, KisError::Contract(_)));
}

fn inventory_request_for_endpoint(endpoint: &InventoryEndpointSpec) -> InventoryRequest {
    let mut request = InventoryRequest::new();

    let mut query = Map::new();
    for field in &endpoint.required_query {
        query.insert(field.clone(), Value::String(value_for_field(field)));
    }
    if endpoint.method == http::Method::GET {
        for field in &endpoint.required_body {
            query.insert(field.clone(), Value::String(value_for_field(field)));
        }
    }
    if !query.is_empty() {
        request = request.query(Value::Object(query));
    }

    if endpoint.method != http::Method::GET {
        let mut body = Map::new();
        for field in &endpoint.required_body {
            body.insert(field.clone(), Value::String(value_for_field(field)));
        }
        request = request.body(Value::Object(body));
    }

    for header in &endpoint.required_headers {
        if !is_auto_header(header) {
            request = request.header(header, "kis_mock_value");
        }
    }

    if endpoint
        .default_mock_tr_id
        .as_deref()
        .or(endpoint.default_real_tr_id.as_deref())
        .is_some_and(|tr_id| !is_single_tr_id(tr_id))
    {
        request = request.tr_id_override(first_tr_id(endpoint).unwrap_or("KISMOCK0000"));
    }

    request
}

fn value_for_field(field: &str) -> String {
    match field.to_ascii_uppercase().as_str() {
        "CANO" => "12345678".to_string(),
        "ACNT_PRDT_CD" => "01".to_string(),
        "PDNO" | "FID_INPUT_ISCD" | "MKSC_SHRN_ISCD" => "005930".to_string(),
        "ORD_QTY" => "1".to_string(),
        "ORD_UNPR" => "70000".to_string(),
        _ if field.to_ascii_uppercase().contains("DATE") || field.ends_with("_DT") => {
            "20260529".to_string()
        }
        _ => "0".to_string(),
    }
}

fn first_tr_id(endpoint: &InventoryEndpointSpec) -> Option<&str> {
    endpoint
        .default_mock_tr_id
        .as_deref()
        .or(endpoint.default_real_tr_id.as_deref())
        .and_then(|value| {
            value
                .split(|ch: char| !(ch.is_ascii_uppercase() || ch.is_ascii_digit()))
                .find(|candidate| !candidate.is_empty())
        })
}

fn is_auto_header(header: &str) -> bool {
    matches!(
        header.to_ascii_lowercase().as_str(),
        "authorization" | "appkey" | "appsecret" | "content-type" | "custtype" | "tr_id"
    )
}

fn is_single_tr_id(value: &str) -> bool {
    value
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
}

fn assert_domain_operations(
    catalog: &InventoryCatalog,
    collection_name: &str,
    operations: &[&str],
    expected_count: usize,
) {
    assert_eq!(operations.len(), expected_count);

    let inventory_count = catalog
        .endpoints()
        .iter()
        .filter(|endpoint| endpoint.collection_name == collection_name)
        .count();
    assert_eq!(inventory_count, expected_count);

    for operation_id in operations {
        let endpoint = catalog
            .endpoint(operation_id)
            .unwrap_or_else(|| panic!("{operation_id} must exist in inventory catalog"));
        assert_eq!(endpoint.collection_name, collection_name);
    }
}

#[tokio::test]
async fn client_exposes_oauth_revoke_and_realtime_approval_key() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .build()
        .expect("client builds");

    let revoked = client
        .revoke_access_token("test_access_token")
        .await
        .expect("token revoke succeeds");
    assert_eq!(revoked.code, 200);
    assert_eq!(revoked.message, "revoked");

    let approval = client
        .issue_realtime_approval_key()
        .await
        .expect("approval key issuance succeeds");
    assert_eq!(approval.approval_key, "kis_mock_approval_key");

    server.shutdown().await;
}

#[test]
fn auth_response_debug_redacts_sensitive_values() {
    let access_token = AccessTokenResponse {
        access_token: "sensitive_access_token".to_string(),
        token_type: "Bearer".to_string(),
        expires_in: 86400,
        access_token_token_expired: Some("2099-12-31 23:59:59".to_string()),
    };
    let access_token_debug = format!("{access_token:?}");
    assert!(access_token_debug.contains("[REDACTED]"));
    assert!(!access_token_debug.contains("sensitive_access_token"));

    let approval_key = RealtimeApprovalKeyResponse {
        approval_key: "sensitive_approval_key".to_string(),
    };
    let approval_key_debug = format!("{approval_key:?}");
    assert!(approval_key_debug.contains("[REDACTED]"));
    assert!(!approval_key_debug.contains("sensitive_approval_key"));
}

#[tokio::test]
async fn oauth_revoke_rejects_empty_token_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .build()
        .expect("client builds");

    let error = client
        .revoke_access_token("   ")
        .await
        .expect_err("empty revoke token should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn oauth_revoke_and_approval_require_app_credentials_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .build()
        .expect("client builds");

    let revoke_error = client
        .revoke_access_token("test_access_token")
        .await
        .expect_err("revoke should require credentials");
    assert!(matches!(revoke_error, KisError::MissingCredentials));

    let approval_error = client
        .issue_realtime_approval_key()
        .await
        .expect_err("approval key should require credentials");
    assert!(matches!(approval_error, KisError::MissingCredentials));
}

#[tokio::test]
async fn overseas_futures_options_wrapper_rejects_mock_for_real_only_endpoint() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_futures_options::<serde_json::Value>(
            OverseasFuturesOptionsEndpoint::InquirePrice,
            InventoryRequest::new().query(json!({
                "SRS_CD": "ESM26"
            })),
        )
        .await
        .expect_err("real-only overseas futures/options endpoint should not run in mock");

    assert!(matches!(error, KisError::UnsupportedEnvironment { .. }));
}

#[tokio::test]
async fn overseas_futures_options_read_wrapper_validates_then_reaches_transport_in_real_mode() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_futures_options::<serde_json::Value>(
            OverseasFuturesOptionsEndpoint::InquirePrice,
            InventoryRequest::new().query(json!({
                "SRS_CD": "ESM26"
            })),
        )
        .await
        .expect_err("unreachable local URL should fail after local validation");

    assert!(
        matches!(error, KisError::Transport(_)),
        "expected transport error after passing local read guards, got {error:?}"
    );
}

#[tokio::test]
async fn overseas_futures_options_order_wrapper_keeps_real_trading_disabled() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_futures_options::<serde_json::Value>(
            OverseasFuturesOptionsEndpoint::Order,
            InventoryRequest::new().body(json!({
                "ACNT_PRDT_CD": "01",
                "CANO": "12345678",
                "CCLD_CNDT_CD": "2",
                "CPLX_ORD_DVSN_CD": "0",
                "ECIS_RSVN_ORD_YN": "N",
                "FM_HDGE_ORD_SCRN_YN": "N",
                "FM_LIMIT_ORD_PRIC": "5000",
                "FM_ORD_QTY": "1",
                "FM_STOP_ORD_PRIC": "0",
                "OVRS_FUTR_FX_PDNO": "ESM26",
                "PRIC_DVSN_CD": "1",
                "SLL_BUY_DVSN_CD": "02"
            })),
        )
        .await
        .expect_err("real order should be locally blocked before transport");

    assert!(matches!(error, KisError::LiveTradingDisabled { .. }));
}

#[tokio::test]
async fn overseas_futures_options_ambiguous_order_tr_id_requires_caller_choice() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_overseas_futures_options::<serde_json::Value>(
            OverseasFuturesOptionsEndpoint::OrderRevisionCancellation,
            InventoryRequest::new().body(json!({
                "ACNT_PRDT_CD": "01",
                "CANO": "12345678",
                "FM_HDGE_ORD_SCRN_YN": "N",
                "ORGN_ODNO": "0000000001",
                "ORGN_ORD_DT": "20260529"
            })),
        )
        .await
        .expect_err("revision/cancel endpoint should require explicit TR ID");

    assert!(matches!(error, KisError::AmbiguousTrId { .. }));
}

#[tokio::test]
async fn inventory_real_non_trading_post_is_not_blocked_by_live_trading_guard() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new()
                .header("approval_key", "test_approval_key")
                .header("tr_type", "1")
                .body(json!({
                    "tr_id": "H0STCNT0",
                    "tr_key": "005930"
                })),
        )
        .await
        .expect_err("unreachable local URL should fail at transport, not live trading guard");

    assert!(
        matches!(error, KisError::Transport(_)),
        "expected transport error after passing live trading guard, got {error:?}"
    );
}

#[tokio::test]
async fn inventory_execute_rejects_missing_required_query_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J"
            })),
        )
        .await
        .expect_err("missing query field should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn inventory_execute_requires_override_for_ambiguous_tr_id() {
    let body = json!({
        "CANO": "12345678",
        "ACNT_PRDT_CD": "01",
        "PDNO": "005930",
        "ORD_DVSN": "00",
        "ORD_QTY": "1",
        "ORD_UNPR": "70000"
    });
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_trading_account.post_domestic_stock_trading_order_cash",
            InventoryRequest::new().body(body),
        )
        .await
        .expect_err("ambiguous order TR ID should require override");

    assert!(matches!(error, KisError::AmbiguousTrId { .. }));
}

#[tokio::test]
async fn inventory_execute_rejects_missing_required_header_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_realtime_quotation.post_tryitout_h0stcnt0",
            InventoryRequest::new().body(json!({
                "tr_id": "H0STCNT0",
                "tr_key": "005930"
            })),
        )
        .await
        .expect_err("missing approval_key and tr_type should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn inventory_execute_rejects_real_only_endpoint_in_mock_before_network() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price_2",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await
        .expect_err("real-only endpoint should not run against mock");

    assert!(matches!(error, KisError::UnsupportedEnvironment { .. }));
}

#[test]
fn domestic_futures_options_operation_ids_cover_inventory_collections() {
    let catalog = InventoryCatalog::bundled().expect("inventory catalog builds");
    let actual = catalog
        .endpoints()
        .iter()
        .filter(|endpoint| endpoint.collection_name.starts_with("[국내선물옵션]"))
        .map(|endpoint| endpoint.operation_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let expected =
        domestic_futures_options_operation_ids().collect::<std::collections::HashSet<_>>();

    assert_eq!(TRADING_ACCOUNT_OPERATION_IDS.len(), 15);
    assert_eq!(QUOTATION_OPERATION_IDS.len(), 9);
    assert_eq!(REALTIME_QUOTATION_OPERATION_IDS.len(), 20);
    assert_eq!(expected.len(), 44);
    assert_eq!(actual, expected);
}

#[tokio::test]
async fn domestic_futures_options_quotation_executes_real_mock_endpoint() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let response = client
        .execute_domestic_futures_options_quotation::<serde_json::Value>(
            "domestic_futures_options_quotation.get_domestic_futureoption_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "F",
                "FID_INPUT_ISCD": "101W09"
            })),
        )
        .await
        .expect("domestic futures/options quote succeeds");

    assert!(response.is_success());
    assert!(response.output.is_some());

    server.shutdown().await;
}

#[tokio::test]
async fn domestic_futures_options_scope_rejects_non_domain_operation_id() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_domestic_futures_options::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new(),
        )
        .await
        .expect_err("non-domain operation should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn domestic_futures_options_order_requires_explicit_tr_id_override() {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let body = json!({
        "ACNT_PRDT_CD": "03",
        "CANO": "12345678",
        "ORD_DVSN_CD": "01",
        "ORD_PRCS_DVSN_CD": "02",
        "ORD_QTY": "1",
        "SHTN_PDNO": "101W09",
        "SLL_BUY_DVSN_CD": "02",
        "UNIT_PRICE": "350.00"
    });

    let error = client
        .execute_domestic_futures_options_trading_account::<serde_json::Value>(
            "domestic_futures_options_trading_account.post_domestic_futureoption_trading_order",
            InventoryRequest::new().body(body),
        )
        .await
        .expect_err("ambiguous domestic futures/options order TR ID should require override");

    assert!(matches!(error, KisError::AmbiguousTrId { .. }));
}

#[tokio::test]
async fn domestic_futures_options_order_with_override_hits_live_trading_guard_in_real() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let body = json!({
        "ACNT_PRDT_CD": "03",
        "CANO": "12345678",
        "ORD_DVSN_CD": "01",
        "ORD_PRCS_DVSN_CD": "02",
        "ORD_QTY": "1",
        "SHTN_PDNO": "101W09",
        "SLL_BUY_DVSN_CD": "02",
        "UNIT_PRICE": "350.00"
    });

    let error = client
        .execute_domestic_futures_options_trading_account::<serde_json::Value>(
            "domestic_futures_options_trading_account.post_domestic_futureoption_trading_order",
            InventoryRequest::new()
                .body(body)
                .tr_id_override("TTTO1101U"),
        )
        .await
        .expect_err("real trading mutation should be locally blocked");

    assert!(matches!(error, KisError::LiveTradingDisabled { .. }));
}

#[tokio::test]
async fn domestic_futures_options_realtime_validates_required_headers() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .execute_domestic_futures_options_realtime_quotation::<serde_json::Value>(
            "domestic_futures_options_realtime_quotation.post_tryitout_h0ifasp0",
            InventoryRequest::new().body(json!({
                "tr_id": "H0IFASP0",
                "tr_key": "101W09"
            })),
        )
        .await
        .expect_err("missing approval_key and tr_type should fail locally");

    assert!(matches!(error, KisError::Validation(_)));
}

#[tokio::test]
async fn client_uses_static_token_and_redacts_secret_values() {
    let server = MockServer::start().await.expect("mock server starts");
    let client = KisClient::builder(Environment::Mock)
        .base_url(server.base_url())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let result = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await;
    assert!(result.is_ok());

    let secret = SecretString::new("very_sensitive_value");
    let debug = format!("{secret:?}");
    assert!(debug.contains("[REDACTED]"));
    assert!(!debug.contains("very_sensitive_value"));

    server.shutdown().await;
}

#[test]
fn retry_and_fallback_policies_are_explicit_options() {
    let disabled = RetryPolicy::disabled();
    assert_eq!(disabled.max_attempts(), 1);
    assert!(!disabled.should_retry(
        "GET",
        OperationKind::Read,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));

    let reads = RetryPolicy::conservative_reads();
    assert!(reads.should_retry(
        "GET",
        OperationKind::Read,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));
    assert!(!reads.should_retry(
        "POST",
        OperationKind::TradingMutation,
        &KisError::HttpStatus {
            status: 503,
            provider_code: None,
            retry_after: None,
        },
        1,
    ));
    assert!(!reads.should_retry(
        "POST",
        OperationKind::TradingMutation,
        &KisError::Transport("ambiguous write failure".to_string()),
        1,
    ));

    let fallback = FallbackPolicy::real_to_mock_reads();
    assert!(fallback.allows_real_to_mock_read("GET"));
    assert!(!fallback.allows_real_to_mock_read("POST"));
    assert!(!fallback.allows_real_to_mock("POST", OperationKind::TradingMutation));
}

#[tokio::test]
async fn real_to_mock_fallback_is_opt_in_and_read_only() {
    let (base_url, server_task) = start_fallback_header_asserting_server()
        .await
        .expect("fallback assertion server starts");
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_base_url(base_url.clone())
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("primary_app_key", "primary_app_secret"))
        .static_bearer_token("primary_access_token")
        .fallback_credentials(AppCredentials::new(
            "fallback_app_key",
            "fallback_app_secret",
        ))
        .fallback_static_bearer_token("fallback_access_token")
        .build()
        .expect("client builds");

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect("quote falls back to mock");

    let fallback = quote.execution.fallback.expect("fallback is visible");
    assert_eq!(fallback.from_environment, Environment::Real);
    assert_eq!(fallback.to_environment, Environment::Mock);
    assert_eq!(fallback.to_base_url, base_url);

    server_task.abort();
}

#[tokio::test]
async fn real_to_mock_fallback_requires_separate_fallback_credentials() {
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_base_url("http://127.0.0.1:9")
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("primary_app_key", "primary_app_secret"))
        .static_bearer_token("primary_access_token")
        .build()
        .expect("client builds");

    let error = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await
        .expect_err("fallback must not reuse primary credentials");

    assert!(matches!(error, KisError::MissingFallbackCredentials));
}

#[tokio::test]
async fn real_cash_order_is_blocked_before_network_without_live_trading_guard() {
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Real)
        .base_url("http://127.0.0.1:9")
        .fallback_policy(FallbackPolicy::real_to_mock_reads())
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 1, 70000),
        )
        .await
        .expect_err("real order should be blocked locally");

    assert!(matches!(error, KisError::LiveTradingDisabled { .. }));
}

#[tokio::test]
async fn cash_order_rejects_invalid_quantity_before_network() {
    let account = Account::new("12345678", "01");
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:9")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
        .expect("client builds");

    let error = client
        .place_domestic_stock_cash_order(
            CashOrderSide::Buy,
            &CashOrderRequest::limit(&account, "005930", 0, 70000),
        )
        .await
        .expect_err("zero quantity should be rejected locally");

    assert!(matches!(error, KisError::Validation(_)));
}

async fn start_fallback_header_asserting_server() -> std::io::Result<(String, JoinHandle<()>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let addr = listener.local_addr()?;
    let app = Router::new().route(
        "/uapi/domestic-stock/v1/quotations/inquire-price",
        get(|headers: HeaderMap| async move {
            let app_key = header(&headers, "appkey");
            let app_secret = header(&headers, "appsecret");
            let authorization = header(&headers, "authorization");

            if app_key == Some("primary_app_key")
                || app_secret == Some("primary_app_secret")
                || authorization == Some("Bearer primary_access_token")
            {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "rt_cd": "1",
                        "msg_cd": "PRIMARY_CREDENTIAL_LEAK",
                        "msg1": "primary credentials reached fallback target"
                    })),
                );
            }

            if app_key != Some("fallback_app_key")
                || app_secret != Some("fallback_app_secret")
                || authorization != Some("Bearer fallback_access_token")
            {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({
                        "rt_cd": "1",
                        "msg_cd": "MISSING_FALLBACK_CREDENTIAL",
                        "msg1": "fallback credentials were not used"
                    })),
                );
            }

            (
                StatusCode::OK,
                Json(json!({
                    "rt_cd": "0",
                    "msg_cd": "KIS_MOCK_OK",
                    "msg1": "mock success",
                    "output": {
                        "stck_prpr": "kis_mock_value"
                    }
                })),
            )
        }),
    );

    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok((format!("http://{addr}"), task))
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}
