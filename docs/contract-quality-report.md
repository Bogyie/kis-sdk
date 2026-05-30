# KIS Contract Quality Report

Verification date: 2026-05-29 Asia/Seoul

## Source Contract

- Official source: Korea Investment & Securities Open API portal, as captured by BOG-221.
- Bundled contract: `contracts/kis_official_endpoint_inventory.compact.json`.
- Source timestamp in bundled contract: `2026-05-29 Asia/Seoul`.
- Total endpoint inventory: 338 endpoints across 22 collections.
- Environment split: 46 `real+mock` endpoints and 292 `real_only` endpoints.

## Collection Coverage

| Collection | Total | real+mock | real_only |
| --- | ---: | ---: | ---: |
| OAuth auth | 3 | 3 | 0 |
| Domestic stock trading/account | 23 | 5 | 18 |
| Domestic stock quotations | 22 | 11 | 11 |
| Domestic stock ELW quotations | 22 | 1 | 21 |
| Domestic stock industry/other | 14 | 1 | 13 |
| Domestic stock item information | 26 | 0 | 26 |
| Domestic stock chart/analysis | 29 | 0 | 29 |
| Domestic stock rank analysis | 22 | 0 | 22 |
| Domestic stock realtime | 29 | 3 | 26 |
| Domestic futures/options trading/account | 15 | 5 | 10 |
| Domestic futures/options quotations | 9 | 3 | 6 |
| Domestic futures/options realtime | 20 | 1 | 19 |
| Overseas stock trading/account | 18 | 8 | 10 |
| Overseas stock quotations | 14 | 4 | 10 |
| Overseas stock analysis | 15 | 0 | 15 |
| Overseas stock realtime | 4 | 1 | 3 |
| Overseas futures/options trading/account | 11 | 0 | 11 |
| Overseas futures/options quotations | 20 | 0 | 20 |
| Overseas futures/options realtime | 4 | 0 | 4 |
| Listed bond trading/account | 7 | 0 | 7 |
| Listed bond quotations | 8 | 0 | 8 |
| Listed bond realtime | 3 | 0 | 3 |

## SDK Surface

The initial typed SDK surface intentionally exposes a narrow domestic stock slice:

| SDK method | Method | Path | Contract status | Notes |
| --- | --- | --- | --- | --- |
| `issue_access_token` | POST | `/oauth2/tokenP` | Covered | Auth token issuance and in-memory token reuse. |
| `inquire_domestic_stock_price` | GET | `/uapi/domestic-stock/v1/quotations/inquire-price` | Covered | Uses `FHKST01010100` for real and mock. |
| `inquire_domestic_stock_balance` | GET | `/uapi/domestic-stock/v1/trading/inquire-balance` | Covered | Uses `TTTC8434R` real and `VTTC8434R` mock. |
| `place_domestic_stock_cash_order` | POST | `/uapi/domestic-stock/v1/trading/order-cash` | Covered | Buy/sell TR IDs are selected by side and environment. Real trading is locally blocked. |
| `execute_overseas_futures_options` | Mixed | 35 `[해외선물옵션]` order/account, quotation, and realtime endpoints | Covered | Uses `OverseasFuturesOptionsEndpoint` enum plus bundled inventory validation. The whole slice is real-only in the captured contract, mock mode rejects it locally, live trading mutations remain disabled, and ambiguous revision/cancel TR IDs require caller override. |

Domain-scoped inventory helpers also expose stable operation-id constants and
execution methods for these follow-on slices:

| Domain helper | Covered collection | Endpoint count | Notes |
| --- | --- | ---: | --- |
| `execute_domestic_stock_realtime_tryitout` | Domestic stock realtime | 29 | REST-style `/tryitout/*` inventory/mock-contract execution only; not live WebSocket subscription behavior. |
| `execute_bond_trading_account` | Listed bond trading/account | 7 | Real-only in bundled inventory; real trading mutations remain locally blocked. |
| `execute_bond_quotation` | Listed bond quotations | 8 | Real-only in bundled inventory. |
| `execute_bond_realtime_tryitout` | Listed bond realtime | 3 | REST-style `/tryitout/*` inventory/mock-contract execution only; not live WebSocket subscription behavior. |

The remaining official endpoints are represented in the bundled contract and
mock route inventory, but are not yet promoted to typed SDK request/response
methods.

## Mock Contract Evidence

The mock server loads the bundled contract through `ContractInventory::bundled()` and builds its route index from every `(method, path)` pair.

Executable coverage added in `tests/mock_server_contract.rs` and
`tests/sdk_core.rs`:

- Validates source metadata: official URL, checked date, 338 endpoints, 22 collections.
- Verifies route index cardinality equals the official endpoint count.
- Verifies domestic stock realtime and listed bond domain helper constants cover
  their 47 targeted inventory endpoints exactly.
- Starts the mock server and requests every bundled endpoint.
- Confirms 3 auth endpoints return success.
- Confirms 43 non-auth `real+mock` endpoints return KIS success envelopes when required headers/TR IDs are supplied.
- Confirms 292 `real_only` endpoints return explicit `501 KIS_MOCK_UNSUPPORTED_ENVIRONMENT` responses.
- Confirms mock error fixtures for unauthorized, rate limit, retryable server error, provider error, unsupported environment, and wrong method behavior.

## Error, Retry, Fallback, Security, And Performance Checks

- Retry is disabled by default. `RetryPolicy::conservative_reads()` retries only retryable GET/read failures and does not retry trading POST mutations.
- Real-to-mock fallback is opt-in and read-only. POST trading fallback is rejected by policy.
- Fallback requires separate fallback credentials and fallback bearer token, preventing primary real credentials from crossing into the mock fallback target.
- Real cash orders are blocked locally by `KisError::LiveTradingDisabled` before network I/O.
- Overseas futures/options order mutations are blocked locally by `KisError::LiveTradingDisabled` before network I/O.
- Overseas futures/options revision/cancel keeps the captured ambiguous TR ID boundary and requires the caller to choose the concrete TR ID.
- Account/order request validation rejects malformed account, product, quantity, and price fields before network I/O.
- Secret debug output uses redaction and does not expose raw secret values.
- The SDK reuses one `reqwest::Client` per `KisClient` and caches issued bearer tokens in memory until the configured refresh skew.
- The mock server binds to port `0` for isolated tests and supports graceful shutdown.

## Known Limitations

- BOG-221 source collection is a captured official inventory. This report does not perform a live re-scrape of the official portal.
- Request and response schemas are preserved as contract metadata and mock output field names, but broad typed Rust structs have only been implemented for the initial domestic stock slice above.
- Ambiguous TR ID endpoints remain represented in the contract. Generic mock routing accepts them without forcing one side-specific TR ID unless the contract exposes a single concrete TR ID.
- No production KIS credentials, account data, live trading, or live API calls were used.
