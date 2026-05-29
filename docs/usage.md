# KIS SDK Usage Guide

This guide shows the supported `kis-sdk` workflows for the current early Rust
implementation. It is written for application developers who want to test
against the bundled mock server first and then wire real KIS credentials through
their own secret-management path.

The typed SDK surface currently covers OAuth token issuance, domestic stock
price inquiry, domestic stock balance inquiry, domestic stock cash-order
requests, and domain-scoped domestic futures/options inventory calls. The
shared inventory-backed execution API can also call every endpoint captured in
`contracts/kis_official_endpoint_inventory.compact.json` by stable operation id
while follow-on work adds more ergonomic typed wrappers.

## Prerequisites

- Rust 2021 toolchain.
- Async runtime using `tokio`.
- KIS app credentials for real API calls. Do not store these in source control.
- A local mock server for deterministic development and tests.

## Add The Dependency

The crate is not published yet because `Cargo.toml` intentionally keeps
`publish = false`. Use the repository while the project is in integration:

```toml
[dependencies]
kis-sdk = { git = "https://github.com/bogyie/kis-sdk", branch = "bog-220-kis-sdk" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

After a future authorized crates.io release, switch to a versioned dependency:

```toml
[dependencies]
kis-sdk = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Run The Local Mock Server

```sh
cargo run --bin kis-mock-server -- 127.0.0.1:0
```

The server prints the selected local address:

```text
kis mock server listening on http://127.0.0.1:49152
```

Use the printed URL as the client `base_url`. Port `0` lets the operating system
choose a free port, which keeps parallel tests isolated.

## Create A Mock Client

Mock requests still require the same header shape as KIS requests, so provide
placeholder app credentials and a dummy bearer token:

```rust
use kis_sdk::{
    config::Environment,
    credentials::AppCredentials,
    KisClient,
};

fn mock_client(base_url: &str) -> Result<KisClient, kis_sdk::KisError> {
    KisClient::builder(Environment::Mock)
        .base_url(base_url)
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()
}
```

These placeholder values are for local development only. They are not real KIS
credentials and must not be copied into production configuration.

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

The inventory layer validates required query, body, and non-standard header
fields before network I/O. Standard KIS headers such as `appkey`, `appsecret`,
`authorization`, `custtype`, `content-type`, and unambiguous `tr_id` values are
filled by the client. Endpoints with ambiguous TR IDs require
`InventoryRequest::tr_id_override(...)`. Real-only endpoints are rejected in
`Environment::Mock`, and real trading mutations remain blocked locally by
`KisError::LiveTradingDisabled`.

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

## Submit A Mock Cash Order

```rust
use kis_sdk::{
    apis::domestic_stock::{CashOrderRequest, CashOrderSide},
    credentials::Account,
};

async fn submit_mock_order(client: &kis_sdk::KisClient) -> Result<(), kis_sdk::KisError> {
    let account = Account::new("12345678", "01");
    let request = CashOrderRequest::limit(&account, "005930", 1, 70_000);

    let response = client
        .place_domestic_stock_cash_order(CashOrderSide::Buy, &request)
        .await?;

    assert!(response.is_success());
    Ok(())
}
```

The current implementation blocks real-environment cash orders before network
I/O with `KisError::LiveTradingDisabled`. Mock cash-order examples validate SDK
request construction and mock contract handling; they do not place live orders.

## Use Real Credentials For Read Calls

For real read calls, load credentials outside the repository:

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

## Configure Retry

Retry is disabled by default:

```rust
use kis_sdk::{config::Environment, KisClient};

let client = KisClient::builder(Environment::Mock).build()?;
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

## Configure Real-To-Mock Fallback

Fallback is disabled by default. Real-to-mock fallback is opt-in, read-only, and
requires separate fallback credentials and a fallback bearer token:

```rust
use kis_sdk::{
    config::Environment,
    credentials::AppCredentials,
    fallback::FallbackPolicy,
    KisClient,
};

let client = KisClient::builder(Environment::Real)
    .app_credentials(AppCredentials::new("<real-app-key>", "<real-app-secret>"))
    .fallback_policy(FallbackPolicy::real_to_mock_reads())
    .fallback_base_url("http://127.0.0.1:49152")
    .fallback_credentials(AppCredentials::new("<mock-app-key>", "<mock-app-secret>"))
    .fallback_static_bearer_token("mock_access_token")
    .build()?;
```

When fallback is used, the response `execution.fallback` metadata records the
source and target environments and base URLs. Primary real credentials are not
sent to the fallback target.

## Mock Fixture Scenarios

The mock server supports deterministic scenario headers for error-path testing:

| Header | Result |
| --- | --- |
| `x-kis-mock-scenario: unauthorized` | Unauthorized provider envelope. |
| `x-kis-mock-scenario: rate-limit` | HTTP 429 with `retry-after: 1`. |
| `x-kis-mock-scenario: retryable-500` | HTTP 503. |
| `x-kis-mock-scenario: provider-error` | HTTP 200 with provider error envelope. |

Routes marked `real_only` in the bundled official inventory return
`KIS_MOCK_UNSUPPORTED_ENVIRONMENT` instead of simulating unsupported mock
behavior.

## Package Readiness Checklist

Current package evidence:

- `Cargo.toml` declares `name`, `version`, `edition`, `license`, `description`,
  `repository`, `readme`, and `keywords`.
- `publish = false` remains set, so crates.io publishing is intentionally
  disabled.
- README and this guide use placeholders only and do not contain app keys,
  access tokens, account numbers, customer data, or live order instructions.
- Contract and mock evidence is documented in
  [`contract-quality-report.md`](contract-quality-report.md).
- The expected local verification suite is:

```sh
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
cargo package
```

Before an authorized publish, review the generated package contents, confirm
license-file expectations, remove `publish = false`, tag a release only if the
release workflow allows it, and rerun the full verification suite.

## Related Documents

- [Repository README](../README.md)
- [Mock server guide](mock-server/README.md)
- [Contract quality report](contract-quality-report.md)
- [Runtime architecture ADR/RFC](adr/0001-kis-sdk-runtime-architecture.md)
