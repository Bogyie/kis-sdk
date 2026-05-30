# kis-sdk

Rust SDK core for Korea Investment & Securities Open API.

`kis-sdk` is an early Rust client for KIS Open API integrations. The narrow
typed SDK surface focuses on OAuth token issuance and a small domestic stock
slice, while inventory-backed SDK APIs account for all 338 endpoints in the
bundled official inventory by stable operation id. Follow-on work can add more
ergonomic typed wrappers without changing the current coverage boundary.

## Current Status

- Package name: `kis-sdk`.
- Current package version: `0.2.1`.
- Crates.io publishing: package metadata is publishable, but the crate has not
  been published yet. Actual upload still requires an authorized `v*.*.*` tag,
  the `crates-io` environment gate, and `CARGO_REGISTRY_TOKEN`.
- License metadata: `MIT OR Apache-2.0`.
- Supported runtime: async Rust with `tokio`, `reqwest`, and rustls TLS.
- Official contract snapshot: `contracts/kis_official_endpoint_inventory.compact.json`,
  captured on 2026-05-29 Asia/Seoul.
- Inventory reconciliation: 338/338 official endpoints are accounted for by
  typed methods, scoped inventory APIs, or lower-level `execute_inventory`.

## Features

- `KisClient` builder with explicit environment selection and shared `reqwest`
  client reuse.
- Redacted `AppCredentials`, `Account`, `AccountProductCode`, and
  `SecretString` helpers.
- OAuth token issuance, token revoke, and WebSocket approval-key issuance, with
  in-memory token reuse and static bearer token injection for tests.
- Typed domestic stock methods for quotation price, balance inquiry, and cash
  order calls.
- Inventory-backed overseas stock API surface for 51 endpoints across
  trading/account, quotation, market-analysis, and realtime-quotation
  collections.
- Domain-scoped domestic futures/options inventory methods for 44
  order/account, quotation, and realtime quotation endpoints, with typed
  operation-id newtypes for safer call sites.
- Domain-scoped inventory helpers for 29 domestic stock realtime tryitout
  endpoints and 18 listed bond endpoints, with typed operation-id newtypes
  available alongside the legacy string constants.
- Collection-specific overseas futures/options inventory wrapper covering all
  35 order/account, quotation, and realtime endpoints from the bundled
  official inventory.
- Inventory-backed `execute_inventory` support for the bundled official
  endpoint inventory, including required input/header validation and TR ID
  selection rules from the captured metadata.
- Domestic stock REST `execute_domestic_stock_rest` support for the 158 listed
  endpoints across the domestic stock trading/account, quotation, ELW,
  sector/misc, product info, market analysis, and ranking analysis collections.
- Explicit `RetryPolicy` and `FallbackPolicy` options. Retry is disabled by
  default. `RetryPolicy::conservative_reads()` retries retryable GET/read
  failures only and does not retry trading POST mutations.

## Installation

If the crate is not available on crates.io yet, use the repository directly:

```toml
[dependencies]
kis-sdk = { git = "https://github.com/bogyie/kis-sdk", branch = "main" }
```

After the first authorized crates.io publish completes, consumers should be able
to switch to a versioned dependency:

```toml
[dependencies]
kis-sdk = "0.2"
```

## Quick Start

```rust
use kis_sdk::{
    apis::domestic_stock::{DomesticStockMarketDivision, InquirePriceRequest},
    config::Environment,
    credentials::AppCredentials,
    KisClient,
};

#[tokio::main]
async fn main() -> Result<(), kis_sdk::KisError> {
    let client = KisClient::builder(Environment::Real)
        .app_credentials(AppCredentials::new(
            std::env::var("KIS_APP_KEY").expect("KIS_APP_KEY is required"),
            std::env::var("KIS_APP_SECRET").expect("KIS_APP_SECRET is required"),
        ))
        .build()?;

    let quote = client
        .inquire_domestic_stock_price(&InquirePriceRequest::with_market(
            DomesticStockMarketDivision::Stock,
            "005930",
        ))
        .await?;

    assert!(quote.is_success());
    Ok(())
}
```

## Supported API Scope

The current SDK surface has two layers:

- Typed methods for OAuth and selected domestic stock workflows.
- Inventory-backed methods for the full bundled official endpoint inventory.
  These methods validate required inventory fields and safety rules before
  network I/O, but they do not yet provide narrow Rust request/response structs
  for every endpoint.

The typed SDK currently exposes:

| Method | KIS path | Notes |
| --- | --- | --- |
| `issue_access_token` | `/oauth2/tokenP` | OAuth token issuance and in-memory token reuse. |
| `revoke_access_token` | `/oauth2/revokeP` | Explicit OAuth access-token revoke; never called implicitly on drop. |
| `issue_realtime_approval_key` | `/oauth2/Approval` | Issues a WebSocket access approval key only; live WebSocket subscription management is outside the current typed API. |
| `inquire_domestic_stock_price` | `/uapi/domestic-stock/v1/quotations/inquire-price` | Domestic stock quote read. |
| `inquire_domestic_stock_balance` | `/uapi/domestic-stock/v1/trading/inquire-balance` | Domestic stock balance read. |
| `place_domestic_stock_cash_order` | `/uapi/domestic-stock/v1/trading/order-cash` | Real cash orders are locally blocked by `KisError::LiveTradingDisabled`. |
| `execute_domestic_stock_realtime_tryitout` | `/tryitout/*` | Domain-scoped inventory execution for 29 domestic stock realtime tryitout endpoints. This is not a live WebSocket subscription API. |
| `execute_bond_trading_account` | `/uapi/domestic-bond/v1/trading/*` | Domain-scoped inventory execution for 7 listed bond trading/account endpoints. Real trading mutations remain locally blocked. |
| `execute_bond_quotation` | `/uapi/domestic-bond/v1/quotations/*` | Domain-scoped inventory execution for 8 listed bond quotation endpoints. |
| `execute_bond_realtime_tryitout` | `/tryitout/*` | Domain-scoped inventory execution for 3 listed bond realtime tryitout endpoints. This is not a live WebSocket subscription API. |
| `execute_overseas_futures_options` | 35 overseas futures/options inventory endpoints | Collection-specific wrapper keyed by `OverseasFuturesOptionsEndpoint`; all bundled endpoints are real-only, required fields are validated from inventory, and real trading mutations are locally blocked. |

For new call sites, prefer the typed variants where they exist:
`Account::domestic_stock`, `InquirePriceRequest::with_market`,
`CashOrderRequest::with_order_division`,
`execute_domestic_stock_realtime_tryitout_operation`,
`execute_bond_*_operation`, and
`execute_domestic_futures_options_operation`. The older `String` fields,
string constants, and `&str` operation-id methods remain available for
compatibility.

Typed helpers still serialize to the exact KIS wire values. For example,
`AccountProductCode::DomesticStock` serializes to `01`,
`DomesticStockMarketDivision::Stock` serializes to `J`, and
`CashOrderDivision::Limit` serializes to `00`. Operation newtypes expose their
stable inventory operation id through `operation_id()` and reject out-of-scope
strings through `FromStr`.

The domestic futures/options SDK surface exposes inventory-backed domain
methods for all 44 endpoints in these bundled official collections:

| Collection | Endpoint count | SDK entry point |
| --- | ---: | --- |
| Domestic futures/options trading/account | 15 | `execute_domestic_futures_options_trading_account` |
| Domestic futures/options quotations | 9 | `execute_domestic_futures_options_quotation` |
| Domestic futures/options realtime quotations | 20 | `execute_domestic_futures_options_realtime_quotation` |

The same endpoints are also available through the combined
`execute_domestic_futures_options` method. Operation ids are exposed through
`kis_sdk::apis::domestic_futures_options::{TRADING_ACCOUNT_OPERATION_IDS,
QUOTATION_OPERATION_IDS, REALTIME_QUOTATION_OPERATION_IDS}`.

The bundled inventory covers 338 official endpoints across 22 collections.
Endpoints outside the typed domestic stock methods and domain-scoped inventory
surfaces do not yet have ergonomic typed Rust request methods, but they can be
addressed and called through the lower-level inventory execution API with
stable operation ids:

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

For domestic stock REST coverage, prefer the scoped helper when the operation
must stay inside the listed domestic stock REST collections:

```rust
use kis_sdk::endpoint::InventoryRequest;
use serde_json::json;

let response = client
    .execute_domestic_stock_rest::<serde_json::Value>(
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

## Endpoint Inventory Coverage

The machine-checkable reconciliation test
`full_inventory_reconciliation_accounts_for_every_official_endpoint_once`
proves that every endpoint in the bundled official inventory is assigned to
exactly one SDK-callable coverage surface:

| Coverage surface | Endpoint count |
| --- | ---: |
| OAuth typed methods | 3 |
| Domestic stock REST inventory API | 158 |
| Domestic stock realtime tryitout inventory API | 29 |
| Domestic futures/options inventory API | 44 |
| Overseas stock inventory API | 51 |
| Overseas futures/options inventory API | 35 |
| Listed bond inventory API | 18 |
| **Total accounted official inventory** | **338/338** |

See [`docs/contract-quality-report.md`](docs/contract-quality-report.md) for
the collection split, mock-contract evidence, and known limitations. The
coverage count is based on the captured BOG-221 inventory snapshot, not a live
portal re-scrape.

The overseas stock SDK surface pins inventory-backed endpoint handles for these
collections:

| Collection | Endpoint count | Access |
| --- | ---: | --- |
| `[해외주식] 주문/계좌` | 18 | `OverseasStockCollection::TradingAccount` / `TRADING_ACCOUNT_ENDPOINTS` |
| `[해외주식] 기본시세` | 14 | `OverseasStockCollection::Quotation` / `QUOTATION_ENDPOINTS` |
| `[해외주식] 시세분석` | 15 | `OverseasStockCollection::MarketAnalysis` / `MARKET_ANALYSIS_ENDPOINTS` |
| `[해외주식] 실시간시세` | 4 | `OverseasStockCollection::RealtimeQuotation` / `REALTIME_QUOTATION_ENDPOINTS` |

```rust
use kis_sdk::{
    apis::overseas_stock::OverseasStockEndpoint,
    endpoint::InventoryRequest,
};
use serde_json::json;

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
```

The realtime helpers intentionally execute the REST-style inventory tryitout
shape captured in the bundled official inventory. Future live WebSocket
subscription support should use a separate API so callers do not confuse
tryitout coverage with streaming behavior.

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

## Testing And Verification

Repository checks used for the current SDK and documentation baseline:

```sh
cargo fmt --check
cargo test --locked
cargo test --locked --test mock_server_contract
cargo doc --locked --no-deps
git diff --check
```

Contract evidence is recorded in
[`docs/contract-quality-report.md`](docs/contract-quality-report.md). The
mock-server test suite requests every bundled endpoint and verifies expected
mock support or explicit `KIS_MOCK_UNSUPPORTED_ENVIRONMENT` rejection.
The developer-only mock server guide is available at
[`docs/mock-server/README.md`](docs/mock-server/README.md) for contract and
test-harness validation.

## Architecture

- [ADR/RFC 0001: KIS Rust SDK Runtime Architecture](docs/adr/0001-kis-sdk-runtime-architecture.md)

## Usage Guide

- [KIS SDK usage guide](docs/usage.md)
- [KIS SDK Korean usage guide](docs/usage-ko.md)

## Release

- [Crates.io publish workflow](docs/release/crates-publish.md)

## Package Readiness

This repository is prepared for an authorized crates.io publish, but publishing
has not been performed from this branch.

- `Cargo.toml` includes package name, version, edition, license, description,
  repository, README, and keywords.
- README and usage documentation avoid secrets and use placeholder-only
  examples.
- The developer mock-server harness and contract-quality report provide package
  validation evidence without live KIS credentials.
- The publish workflow runs `scripts/verify-crates-publishable.py` before the
  third-party publish action so `publish = false` or registry restrictions
  cannot produce a false-positive empty publish.
- Before publishing, confirm license-file expectations, run
  `cargo package --locked`, review the generated package contents, and use only
  the authorized tag/environment workflow for any upload step.
