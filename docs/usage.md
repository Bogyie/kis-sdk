# KIS SDK Usage Guide

This guide shows the supported `kis-sdk` workflows for the current early Rust
implementation. It is written for application developers who want to wire real
KIS credentials through their own secret-management path.

The typed SDK surface currently covers OAuth token issuance and revoke,
WebSocket approval-key issuance, domestic stock price inquiry, domestic stock
balance inquiry, and domestic stock cash-order requests. It also exposes
inventory-backed overseas stock endpoint handles for 51 official endpoints.
Domain-scoped inventory helpers cover domestic futures/options, 29 domestic
stock realtime tryitout endpoints, and 18 listed bond endpoints. The shared
inventory-backed execution API can also call every endpoint captured in
`contracts/kis_official_endpoint_inventory.compact.json` by stable operation id
while follow-on work adds more ergonomic typed wrappers.

## Prerequisites

- Rust 2021 toolchain.
- Async runtime using `tokio`.
- KIS app credentials for real API calls. Do not store these in source control.

## Add The Dependency

If the crate is not available on crates.io yet, use the repository while the
project is in integration:

```toml
[dependencies]
kis-sdk = { git = "https://github.com/bogyie/kis-sdk", branch = "main" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

After the first authorized crates.io release, switch to a versioned dependency:

```toml
[dependencies]
kis-sdk = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Create A Real Read Client

Load credentials outside the repository and pass them to the SDK at runtime:

```rust
use kis_sdk::{
    config::Environment,
    credentials::AppCredentials,
    KisClient,
};

fn real_read_client_from_env() -> Result<KisClient, Box<dyn std::error::Error>> {
    let app_key = std::env::var("KIS_APP_KEY")?;
    let app_secret = std::env::var("KIS_APP_SECRET")?;

    let client = KisClient::builder(Environment::Real)
        .app_credentials(AppCredentials::new(app_key, app_secret))
        .build()?;

    Ok(client)
}
```

Do not print or persist the loaded values. Do not use production credentials in
tests that can run on shared developer machines or public CI.

## Manage OAuth Tokens And WebSocket Approval Keys

Use `issue_access_token` when the application needs to fetch an OAuth bearer
token directly:

```rust
async fn issue_token(client: &kis_sdk::KisClient) -> Result<String, kis_sdk::KisError> {
    let token = client.issue_access_token().await?;
    Ok(token.access_token)
}
```

Use `revoke_access_token` to explicitly revoke a token. The SDK validates that
the token is not blank before network I/O, and it never revokes tokens
implicitly when a client is dropped:

```rust
async fn revoke_token(
    client: &kis_sdk::KisClient,
    token: &str,
) -> Result<(), kis_sdk::KisError> {
    let response = client.revoke_access_token(token).await?;
    assert_eq!(response.code, 200);
    Ok(())
}
```

Use `issue_realtime_approval_key` to issue the `/oauth2/Approval` access key
needed by KIS WebSocket clients:

```rust
async fn websocket_approval_key(
    client: &kis_sdk::KisClient,
) -> Result<String, kis_sdk::KisError> {
    let response = client.issue_realtime_approval_key().await?;
    Ok(response.approval_key)
}
```

This method only issues the WebSocket approval key. The current typed SDK API
does not manage live WebSocket sessions, subscriptions, reconnect behavior, or
message decoding.

## Inquire A Domestic Stock Price

```rust
use kis_sdk::apis::domestic_stock::InquirePriceRequest;

async fn inquire_price(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await?;

    if response.is_success() {
        println!("quote output: {:?}", response.output);
    }

    Ok(())
}
```

The current output type preserves provider fields as `serde_json::Value` so the
SDK can expose the endpoint before broad typed response structs are finalized.

## Call An Inventory Endpoint

Use `InventoryCatalog` to inspect generated operation ids and
`execute_inventory` to call a captured endpoint directly:

```rust
use kis_sdk::endpoint::InventoryRequest;
use serde_json::json;

async fn inventory_quote(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_inventory::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

For the listed domestic stock REST collections, `execute_domestic_stock_rest`
adds a scope guard around the same inventory execution path. It covers 158
endpoints across domestic stock trading/account, quotation, ELW, sector/misc,
product info, market analysis, and ranking analysis collections. Realtime
domestic stock endpoints remain outside this REST helper.

```rust
use kis_sdk::endpoint::InventoryRequest;
use serde_json::json;

async fn domestic_stock_rest_quote(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_domestic_stock_rest::<serde_json::Value>(
            "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "J",
                "FID_INPUT_ISCD": "005930"
            })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

The inventory layer validates required query, body, and non-standard header
fields before network I/O. Standard KIS headers such as `appkey`, `appsecret`,
`authorization`, `custtype`, `content-type`, and unambiguous `tr_id` values are
filled by the client. Endpoints with ambiguous TR IDs require
`InventoryRequest::tr_id_override(...)`, and real trading mutations remain
blocked locally by `KisError::LiveTradingDisabled`.

## Call An Overseas Stock Endpoint

The overseas stock module pins all inventory-backed endpoints from the official
overseas stock collections: 18 trading/account endpoints, 14 quotation
endpoints, 15 market-analysis endpoints, and 4 realtime-quotation endpoints.
Use the enum when you want a stable SDK handle instead of a raw operation-id
string:

```rust
use kis_sdk::{
    apis::overseas_stock::OverseasStockEndpoint,
    endpoint::InventoryRequest,
};
use serde_json::json;

async fn overseas_price(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_overseas_stock::<serde_json::Value>(
            OverseasStockEndpoint::GetOverseasPriceQuotationsPrice,
            InventoryRequest::new().query(json!({
                "AUTH": "",
                "EXCD": "NAS",
                "SYMB": "AAPL"
            })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

Order endpoints keep the same safety boundary as domestic orders: live
environment trading mutations return `KisError::LiveTradingDisabled` before
network I/O. Overseas order TR IDs vary by country, exchange, and order side, so
ambiguous inventory values require caller-supplied `tr_id_override(...)`.

## Call A Domestic Futures/Options Endpoint

Domestic futures/options coverage is exposed as a scoped inventory API for 44
official endpoints: 15 trading/account endpoints, 9 quotation endpoints, and 20
realtime quotation endpoints. The operation id constants are available from
`kis_sdk::apis::domestic_futures_options`.

```rust
use kis_sdk::{
    apis::domestic_futures_options::QUOTATION_OPERATION_IDS,
    endpoint::InventoryRequest,
};
use serde_json::json;

async fn domestic_futures_options_quote(
    client: &kis_sdk::KisClient,
) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_domestic_futures_options_quotation::<serde_json::Value>(
            QUOTATION_OPERATION_IDS[0],
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "F",
                "FID_INPUT_ISCD": "101W09"
            })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

Order-changing domestic futures/options endpoints keep the same SDK safety
rules as other trading mutations. Inventory metadata with side/session-specific
TR ID text requires `InventoryRequest::tr_id_override(...)`, and real
environment trading mutations are blocked locally by
`KisError::LiveTradingDisabled`.

## Call A Realtime Tryitout Endpoint

Realtime domain helpers use the REST-style `/tryitout/*` shape preserved in the
official inventory. They validate the request shape for inventory-backed
tryitout endpoints, but they are not live WebSocket subscription APIs.

```rust
use kis_sdk::{
    apis::domestic_stock_realtime,
    endpoint::InventoryRequest,
};
use serde_json::json;

async fn realtime_tryitout(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_domestic_stock_realtime_tryitout::<serde_json::Value>(
            domestic_stock_realtime::REALTIME_TRADE_KRX,
            InventoryRequest::new()
                .header("approval_key", "test_approval_key")
                .header("tr_type", "1")
                .body(json!({
                    "tr_id": "H0STCNT0",
                    "tr_key": "005930"
                })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

## Call A Listed Bond Endpoint

Listed bond helpers scope inventory execution to bond trading/account,
quotation, or realtime tryitout operation id constants. Most listed bond
endpoints in the bundled inventory are read-only quotation calls or guarded
trading/account calls; real trading mutations are still blocked before network
I/O.

```rust
use kis_sdk::{
    apis::bond,
    endpoint::InventoryRequest,
};
use serde_json::json;

async fn bond_price(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let response = client
        .execute_bond_quotation::<serde_json::Value>(
            bond::INQUIRE_PRICE,
            InventoryRequest::new().query(json!({
                "FID_COND_MRKT_DIV_CODE": "B",
                "FID_INPUT_ISCD": "KR103502GA34"
            })),
        )
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

## Inquire A Domestic Stock Balance

```rust
use kis_sdk::{
    apis::domestic_stock::InquireBalanceRequest,
    credentials::Account,
};

async fn inquire_balance(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let account = Account::new("12345678", "01");
    let response = client
        .inquire_domestic_stock_balance(&InquireBalanceRequest::new(&account))
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

Use placeholders in examples and tests. Real account identifiers are sensitive
and should be supplied only by an approved runtime secret path.

The current implementation blocks real-environment cash orders before network
I/O with `KisError::LiveTradingDisabled`.

## Configure Retry

Retry is disabled by default:

```rust
use kis_sdk::{config::Environment, KisClient};

let client = KisClient::builder(Environment::Real).build()?;
```

Enable conservative read retries explicitly:

```rust
use kis_sdk::{config::Environment, retry::RetryPolicy, KisClient};

let client = KisClient::builder(Environment::Real)
    .retry_policy(RetryPolicy::conservative_reads())
    .build()?;
```

`RetryPolicy::conservative_reads()` retries retryable GET/read failures. It does
not retry trading POST mutations, which avoids hidden duplicate-write behavior.

## Package Readiness Checklist

Current package evidence:

- `Cargo.toml` declares `name`, `version`, `edition`, `license`, `description`,
  `repository`, `readme`, and `keywords`.
- The package metadata is publishable to crates.io, while actual upload remains
  controlled by the release workflow tag, environment, and secret gates.
- README and this guide use placeholders only and do not contain app keys,
  access tokens, account numbers, customer data, or live order instructions.
- Contract and test evidence is documented in
  [`contract-quality-report.md`](contract-quality-report.md).
- The expected local verification suite is:

```sh
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
python3 scripts/verify-crates-publishable.py
cargo package --locked
```

Before an authorized publish, review the generated package contents, confirm
license-file expectations, tag a release only if the release workflow allows it,
and rerun the full verification suite.

## Related Documents

- [Repository README](../README.md)
- [Contract quality report](contract-quality-report.md)
- [Runtime architecture ADR/RFC](adr/0001-kis-sdk-runtime-architecture.md)
