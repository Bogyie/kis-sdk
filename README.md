# kis-sdk

Rust SDK core for Korea Investment & Securities Open API.

`kis-sdk` is an early Rust client and local mock contract harness for KIS Open
API integrations. The current typed SDK surface is intentionally narrow: it
focuses on OAuth token issuance and a small domestic stock slice. A lower-level
inventory-backed execution API can address and call the broader bundled
official endpoint inventory by stable operation id while follow-on work adds
more ergonomic typed wrappers.

## Current Status

- Package name: `kis-sdk`.
- Crates.io publishing: not enabled yet. `Cargo.toml` still has
  `publish = false` until an explicit release decision is made.
- License metadata: `MIT OR Apache-2.0`.
- Supported runtime: async Rust with `tokio`, `reqwest`, and rustls TLS.
- Official contract snapshot: `contracts/kis_official_endpoint_inventory.compact.json`,
  captured on 2026-05-29 Asia/Seoul.

## Features

- `KisClient` builder with explicit real/mock environment selection and shared
  `reqwest` client reuse.
- Redacted `AppCredentials`, `Account`, and `SecretString` helpers.
- OAuth token issuance and in-memory token reuse, with static bearer token
  injection for tests and mock workflows.
- Typed domestic stock methods for quotation price, balance inquiry, and cash
  order calls.
- Domain-scoped inventory helpers for 29 domestic stock realtime tryitout
  endpoints and 18 listed bond endpoints.
- Collection-specific overseas futures/options inventory wrapper covering all
  35 order/account, quotation, and realtime endpoints from the bundled
  official inventory.
- Inventory-backed `execute_inventory` support for the bundled official
  endpoint inventory, including required input/header validation and TR ID
  selection rules from the captured metadata.
- Local mock server generated from the bundled official endpoint inventory.
- Explicit `RetryPolicy` and `FallbackPolicy` options. Retry is disabled by
  default. `RetryPolicy::conservative_reads()` retries retryable GET/read
  failures only and does not retry trading POST mutations.
- Real-to-mock fallback is opt-in, read-only, and recorded in response execution
  metadata. Fallback requests require separate fallback credentials and a
  fallback bearer token, so primary real credentials are not reused across the
  fallback trust boundary.

## Installation

The crate is not published to crates.io yet. Until publishing is authorized,
use the repository directly:

```toml
[dependencies]
kis-sdk = { git = "https://github.com/bogyie/kis-sdk", branch = "bog-220-kis-sdk" }
```

After crates.io publishing is explicitly enabled, consumers should be able to
switch to a versioned dependency:

```toml
[dependencies]
kis-sdk = "0.1"
```

## Quick Start With The Mock Server

Start the local mock server:

```sh
cargo run --bin kis-mock-server -- 127.0.0.1:0
```

The server prints the bound URL, for example
`kis mock server listening on http://127.0.0.1:49152`.

Use that URL with static local-only credentials and a dummy bearer token:

```rust
use kis_sdk::{
    apis::domestic_stock::InquirePriceRequest,
    config::Environment,
    credentials::AppCredentials,
    KisClient,
};

#[tokio::main]
async fn main() -> Result<(), kis_sdk::KisError> {
    let client = KisClient::builder(Environment::Mock)
        .base_url("http://127.0.0.1:49152")
        .app_credentials(AppCredentials::new("test_app_key", "test_app_secret"))
        .static_bearer_token("test_access_token")
        .build()?;

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::new("005930"))
        .await?;

    assert!(quote.is_success());
    Ok(())
}
```

## Supported API Scope

The typed SDK currently exposes:

| Method | KIS path | Notes |
| --- | --- | --- |
| `issue_access_token` | `/oauth2/tokenP` | OAuth token issuance and in-memory token reuse. |
| `inquire_domestic_stock_price` | `/uapi/domestic-stock/v1/quotations/inquire-price` | Domestic stock quote read. |
| `inquire_domestic_stock_balance` | `/uapi/domestic-stock/v1/trading/inquire-balance` | Domestic stock balance read. |
| `place_domestic_stock_cash_order` | `/uapi/domestic-stock/v1/trading/order-cash` | Mock cash orders are supported; real cash orders are locally blocked by `KisError::LiveTradingDisabled`. |
| `execute_domestic_stock_realtime_tryitout` | `/tryitout/*` | Domain-scoped inventory execution for 29 domestic stock realtime tryitout/mock-contract endpoints. This is not a live WebSocket subscription API. |
| `execute_bond_trading_account` | `/uapi/domestic-bond/v1/trading/*` | Domain-scoped inventory execution for 7 listed bond trading/account endpoints. Real trading mutations remain locally blocked. |
| `execute_bond_quotation` | `/uapi/domestic-bond/v1/quotations/*` | Domain-scoped inventory execution for 8 listed bond quotation endpoints. |
| `execute_bond_realtime_tryitout` | `/tryitout/*` | Domain-scoped inventory execution for 3 listed bond realtime tryitout/mock-contract endpoints. This is not a live WebSocket subscription API. |
| `execute_overseas_futures_options` | 35 overseas futures/options inventory endpoints | Collection-specific wrapper keyed by `OverseasFuturesOptionsEndpoint`; all bundled endpoints are real-only, required fields are validated from inventory, and real trading mutations are locally blocked. |

The bundled inventory covers 338 official endpoints across 22 collections.
Endpoints outside the typed SDK surface do not yet have ergonomic typed Rust
request methods, but they can be addressed and called through the lower-level
inventory execution API with stable operation ids:

```rust
use kis_sdk::endpoint::InventoryRequest;
use serde_json::json;

let response = client
    .execute_inventory::<serde_json::Value>(
        "domestic_stock_quotation.get_domestic_stock_quotations_inquire_price",
        InventoryRequest::new().query(json!({
            "FID_COND_MRKT_DIV_CODE": "J",
            "FID_INPUT_ISCD": "005930"
        })),
    )
    .await?;
```

The inventory execution API follows the same safety boundary as the typed
methods: required query/body/non-standard header fields are validated before
network I/O, standard KIS headers are filled by the client, ambiguous TR IDs
require an explicit override, real-only endpoints are rejected in mock mode, and
real trading mutations are locally blocked.

The realtime helpers intentionally execute the REST-style inventory tryitout
shape used by the bundled mock contract. Future live WebSocket subscription
support should use a separate API so callers do not confuse mock/tryitout
coverage with streaming behavior.

## Credentials And Safety

- Do not hard-code real app keys, app secrets, access tokens, approval keys,
  account numbers, or customer data.
- Prefer loading real credentials from a secret manager or process environment
  outside source control.
- Use `AppCredentials::new("<app-key>", "<app-secret>")` and
  `Account::new("<8-digit-cano>", "<2-digit-product-code>")` placeholders in
  examples and tests.
- `SecretString` redacts debug output, but callers must still avoid logging raw
  values before constructing SDK types.
- Real trading mutations are blocked locally in the current implementation.
  Mock cash-order examples are for integration testing only and do not execute
  live orders.

## Testing And Verification

Repository checks used for the current SDK and documentation baseline:

```sh
cargo fmt --check
cargo check
cargo clippy --all-targets -- -D warnings
cargo test
```

Contract evidence is recorded in
[`docs/contract-quality-report.md`](docs/contract-quality-report.md). The
mock-server test suite requests every bundled endpoint and verifies expected
mock support or explicit `KIS_MOCK_UNSUPPORTED_ENVIRONMENT` rejection.

## Architecture

- [ADR/RFC 0001: KIS Rust SDK Runtime Architecture](docs/adr/0001-kis-sdk-runtime-architecture.md)

## Usage Guide

- [KIS SDK usage guide](docs/usage.md)

## Release

- [Crates.io publish workflow](docs/release/crates-publish.md)

## Mock Server

- [KIS Mock Server](docs/mock-server/README.md)

## Package Readiness

This repository is prepared for a future crates.io decision, but publishing has
not been performed.

- `Cargo.toml` includes package name, version, edition, license, description,
  repository, README, keywords, and `publish = false`.
- README and usage documentation avoid secrets and use local/mock placeholders.
- The mock server and contract-quality report provide package validation
  evidence without live KIS credentials.
- Before publishing, remove `publish = false` only with explicit authorization,
  confirm license-file expectations, run `cargo package`, and review the
  generated package contents before any upload step.
