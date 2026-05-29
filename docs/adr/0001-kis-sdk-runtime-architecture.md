# ADR/RFC 0001: KIS Rust SDK Runtime Architecture

Status: Proposed for implementation
Date: 2026-05-29
Issue: BOG-224
Parent branch: `bog-220-kis-sdk`
Child branch: `bog-220-sdk-architecture`

## Audience

This document is for SDK implementers, mock-server implementers, QA, security reviewers, and architecture reviewers who need a shared contract for the first Rust implementation of the Korea Investment & Securities Open API SDK.

## Source Material

Primary sources are the BOG-221 artifacts collected from the official KIS Developers portal on 2026-05-29:

- `kis_official_endpoint_inventory.json`
- `KIS_OFFICIAL_API_SPEC_RESEARCH.md`
- `kis_official_full_api_doc_20260529.xlsx`

The inventory is the implementation contract source. The Excel file is the official full API document download and should be used to resolve discrepancies before broad code generation.

Important source facts:

- Official portal: `https://apiportal.koreainvestment.com/apiservice`
- Official full Excel download path: `https://apiportal.koreainvestment.com/files/download/apiCollection/API_COLLECTION`
- Official sample repository used by BOG-221: `https://github.com/koreainvestment/open-trading-api`
- Collected public collections: 22
- Collected endpoint documents: 338
- Methods: 257 `GET`, 81 `POST`
- Types: 3 auth, 203 quotation/info, 73 trading/account, 59 websocket
- Environment support: 46 real+mock, 292 real-only
- REST real domain: `https://openapi.koreainvestment.com:9443`
- REST mock domain: `https://openapivts.koreainvestment.com:29443`
- WebSocket real endpoint: `ws://ops.koreainvestment.com:21000`
- WebSocket mock endpoint: `ws://ops.koreainvestment.com:31000`
- REST OAuth endpoints: `POST /oauth2/tokenP`, `POST /oauth2/revokeP`
- WebSocket approval endpoint: `POST /oauth2/Approval`
- Token behavior for ordinary customers: access tokens are valid for one day, and token requests within six hours can return the previous token.
- Public JSON mostly lacks populated `apiErrors`; error-code mapping needs later manual validation against FAQ/observed sandbox responses.

## Goals

- Provide a Rust SDK architecture that is fast enough for trading and quotation workloads without sacrificing testability.
- Keep REST connection reuse, token reuse, retry behavior, fallback behavior, logging, and error mapping explicit and configurable.
- Make the official endpoint inventory usable as the source for generated or manually curated typed request/response modules.
- Allow CI to validate contract behavior against mocks without live account credentials or real trading.
- Keep live real-account operations opt-in and outside ordinary automated tests.

## Non-Goals

- Do not execute real orders, mutate real accounts, or require production credentials in CI.
- Do not scrape private or login-gated documentation.
- Do not make automatic fallback or automatic retry the default for mutating operations.
- Do not guarantee full error-code taxonomy until KIS FAQ error pages or authorized runtime responses are mapped.
- Do not require WebSocket implementation in the first REST-only milestone, but keep module boundaries compatible with WebSocket support.

## Decision Summary

Build the SDK as a small layered Rust workspace with a reusable runtime client, typed endpoint modules generated or curated from the official inventory, a credential/token provider boundary, explicit retry/fallback policy objects, redacted structured tracing, and mock-first contract tests.

The SDK must default to conservative behavior:

- Reuse HTTP sessions and connection pools through one shared async client.
- Cache OAuth tokens by credential and environment, refreshing before expiry with a single-flight guard.
- Treat retries as opt-in and safe-by-class: network/timeouts/429/5xx for idempotent reads only by default; mutating calls require explicit idempotency policy or caller opt-in.
- Treat fallback as opt-in. If enabled, it must only move from real to mock or alternate endpoint families when the endpoint contract says that target is supported and when the caller's policy allows changed data semantics.
- Redact app keys, app secrets, tokens, approval keys, account numbers, personal secret keys, authorization headers, and full request/response bodies from logs.
- Validate every implemented endpoint against the BOG-221 contract source with mocks before marking implementation complete.

## Proposed Module Boundaries

The repository can start as a single crate and grow into a workspace if needed. The implementation boundary should still be explicit:

```text
kis-sdk
  src/
    client.rs          // public SDK client builder and execution facade
    config.rs          // environment, timeout, retry, fallback, and tracing config
    credentials.rs     // credential traits and redacted secret wrappers
    auth.rs            // OAuth and WebSocket approval providers
    transport.rs       // reqwest/hyper transport adapter, connection reuse, timeouts
    endpoint.rs        // endpoint metadata, request assembly, response envelope parsing
    error.rs           // error taxonomy and retryability classification
    retry.rs           // retry/backoff/budget policy
    fallback.rs        // explicit fallback policy and decision trace
    pagination.rs      // tr_cont/CTX_AREA continuation support
    hashkey.rs         // optional request hash generation boundary
    logging.rs         // tracing spans and redaction helpers
    models/            // typed request/response structs grouped by API family
    apis/              // typed endpoint methods grouped by collection/domain
    websocket/         // approval-key and subscription runtime, later milestone-ready
  tests/
    contract/          // generated inventory contract checks
    fixtures/          // sanitized official-schema fixtures
```

Recommended API families:

- `auth`: token and approval-key calls.
- `domestic_stock`: domestic stock trading, account, quotation, ranking, ELW, ETF/ETN.
- `domestic_futures_options`
- `overseas_stock`
- `overseas_futures_options`
- `bond`
- `websocket`

The first implementation may expose fewer families, but every implemented endpoint must preserve its official `method`, `access_url`, `env_support`, `real_tr_id`, `virtual_tr_id`, request fields, response envelope, and continuation behavior.

## Runtime Flow

For a REST API call:

1. The user builds `KisClient` with environment, credential provider, timeout, retry, fallback, and tracing options.
2. The typed endpoint method constructs a request model and maps fields from inventory:
   - `req_h` to headers.
   - `req_q` to query parameters when present.
   - `req_b` to JSON body for `POST`; for current inventory, many `GET` request fields appear under `req_b`, so GET implementations must serialize those fields as query parameters unless an endpoint-specific source proves otherwise.
3. `auth.rs` supplies a bearer token unless the endpoint is an OAuth endpoint.
4. `endpoint.rs` selects the correct `tr_id` for real vs mock and for operation variants such as buy/sell.
5. `transport.rs` sends the request through the shared async HTTP client with bounded timeouts.
6. The response parser reads response headers, `rt_cd`, `msg_cd`, `msg1`, and `output*` fields.
7. `error.rs` classifies transport errors, HTTP status failures, provider envelope failures, decode errors, contract mismatches, and unsupported-environment errors.
8. `retry.rs` may resubmit only if the retry policy, endpoint class, method, and error classification allow it.
9. `fallback.rs` may try an alternate target only if the fallback policy and endpoint environment support allow it.
10. `logging.rs` emits redacted structured spans with request class, endpoint id, environment, `tr_id`, attempt number, latency, retry/fallback decision, and error category.

## HTTP Client And Network Performance

Decision:

- Use one shared async HTTP client per `KisClient` instance.
- Use connection pooling/session reuse instead of creating a client per request.
- Keep default timeouts finite and configurable.
- Keep request concurrency caller-controlled with optional SDK-side rate limiting.

Implementation guidance:

- Use `reqwest` on Tokio unless a later benchmark shows a need for lower-level `hyper`.
- Enable HTTP connection pooling and keep-alive via the shared client.
- Split timeout controls into connect timeout, total request timeout, and optional per-attempt timeout.
- Provide a `RateLimitPolicy` interface but do not hard-code unverified provider quotas as authoritative limits.
- Seed default pacing recommendations from BOG-221 only as optional examples: official samples use REST sleep values of roughly 0.05s for real and 0.5s for mock. These are not a formal per-endpoint quota.
- Preserve provider response headers such as `tr_cont` and any request/correlation identifiers in the response metadata when present.

Open implementation choices:

- If `reqwest` TLS and pooling defaults are enough, avoid custom connector complexity.
- If high-throughput quotation workloads later need stricter resource control, add builder knobs for pool idle timeout, max idle per host, TCP keepalive, and concurrency permits.

## Environment And Endpoint Support

Decision:

- Represent environment explicitly as `Environment::Real` or `Environment::Mock`.
- Reject unsupported environment calls before network execution.
- Use `env_support` from the official inventory to prevent accidental mock calls to real-only endpoints.

Rules:

- `real+mock`: real and mock are allowed; choose `real_tr_id` or `virtual_tr_id` by environment.
- `real_only`: mock environment must return `Error::UnsupportedEnvironment` unless the caller explicitly supplies a manual endpoint override for local mock-server testing.
- Empty or `None` TR IDs on OAuth endpoints are allowed.
- TR IDs containing multiple variants, such as domestic stock cash order buy/sell variants, require a typed operation enum so callers cannot pass ambiguous strings.

## Authentication And Token Cache

Decision:

- Implement OAuth as a provider trait so tests can inject static tokens and production can use KIS token endpoints.
- Cache access tokens in memory by credential fingerprint and environment.
- Refresh tokens before expiry and avoid thundering herds with a single-flight refresh guard.

Required behavior:

- `POST /oauth2/tokenP` request body is `{grant_type, appkey, appsecret}`.
- Use returned `access_token`, `token_type`, `expires_in`, and `access_token_token_expired` to build cache entries.
- Prefer the earlier of parsed absolute expiry and `now + expires_in`, then subtract a configurable skew.
- Do not request new tokens on every API call because the source states that tokens are valid for one day and token requests within six hours may return the previous token.
- `POST /oauth2/revokeP` should be exposed as an explicit operation; do not call it automatically on drop.
- `POST /oauth2/Approval` should live under the WebSocket auth provider and cache approval keys only according to documented WebSocket session semantics.

Security requirements:

- Secret-bearing types must implement redacted `Debug`.
- Do not clone raw secrets into errors, logs, metrics, traces, panic messages, or fixtures.
- Read credentials from caller-provided values or environment variables through explicit APIs; do not read arbitrary global environment variables implicitly.
- Tests must use dummy values only.

## Request Hash Boundary

The BOG-221 research summary states that some body-sending order/amend APIs require `hashkey`, while the normalized inventory sample examined for this ADR did not expose `hashkey` as a `req_h` field. Treat hash generation as a first-class extension boundary rather than ignoring it.

Decision:

- Add a `HashKeyProvider` trait used only for endpoints that require a request hash.
- Do not guess hash requirements from HTTP method alone.
- Maintain an endpoint metadata flag, initially curated from the official research/Excel, for `requires_hashkey`.
- Contract QA must verify the hash-required endpoint list against the official Excel before enabling live trading calls.

Safe default:

- If an endpoint is marked `requires_hashkey` and no provider is configured, return a local configuration error before sending a request.

## Error Taxonomy

Decision:

Represent errors in a typed enum with provider details preserved but sanitized:

```rust
pub enum KisError {
    Config(ConfigError),
    UnsupportedEnvironment { endpoint: EndpointId, environment: Environment },
    Auth(AuthError),
    HashKey(HashKeyError),
    Transport(TransportError),
    HttpStatus { status: u16, category: HttpStatusCategory, request_id: Option<String> },
    Provider { rt_cd: String, msg_cd: Option<String>, msg1: Option<String> },
    Decode(DecodeError),
    Contract(ContractError),
    RateLimited { retry_after: Option<Duration>, provider_code: Option<String> },
    Timeout { phase: TimeoutPhase },
}
```

Classification rules:

- Network connect/read timeout: retryable only if the retry policy allows it.
- HTTP 401/403: not retryable except a single token refresh retry for 401 when token expiry is plausible.
- HTTP 404/422 or local validation failures: not retryable.
- HTTP 429: retryable only within budget and respecting `Retry-After` when present.
- HTTP 5xx: retryable for idempotent reads by default; mutating calls require explicit opt-in.
- Provider envelope with non-success `rt_cd`: not retryable by default until `msg_cd` mapping is validated.
- Decode and contract errors: not retryable; these indicate SDK/spec drift or mock mismatch.

Because official public JSON mostly lacks populated `apiErrors`, the first implementation must not overfit a fake provider-code taxonomy.

## Retry Policy

Decision:

- Retry is controlled by `RetryPolicy`, disabled or conservative by default.
- Retries are bounded by attempts, elapsed time, and endpoint safety.
- Backoff must include jitter.

Recommended default:

- Auth token refresh retry: one retry after refreshing token on eligible 401.
- Idempotent REST GET quotation/account reads: max 2 retries for timeout, 429, and 5xx.
- POST auth calls: max 1 retry for transport failure before response body is received.
- Trading/order/amend/cancel POST calls: no automatic retry unless caller explicitly sets an idempotency-aware policy for that endpoint.
- WebSocket reconnect: separate policy with subscription replay only after caller-approved semantics.

Implementation requirements:

- Every retry attempt must be visible in tracing.
- Retry budget exhaustion must return the last error plus attempt metadata.
- Do not retry after a request body was sent for mutating calls unless the policy explicitly allows the duplicate-risk tradeoff.

## Fallback Policy

Decision:

- Fallback is opt-in and separate from retry.
- Fallback must never be silent because real and mock environments can differ materially.
- Fallback must not be used to execute real trading after a mock failure.

Allowed fallback examples:

- Real quotation read to mock quotation read only when the endpoint supports `real+mock`, caller opts in, and the caller accepts non-real data semantics.
- Primary REST endpoint to a local mock server during tests when configured through `EndpointOverride`.
- WebSocket reconnect to the same environment when the reconnect policy allows it.

Disallowed fallback examples:

- Mock to real order execution.
- Real-only endpoint to mock when inventory says `real_only`, except local test overrides that never use real credentials.
- Provider envelope business errors to alternate operation variants.
- Retrying or falling back an order after ambiguous transport failure without explicit caller policy.

## Logging, Metrics, And Tracing

Decision:

- Use `tracing` instrumentation and redacted structured fields.
- Logs are operational metadata, not payload dumps.

Required span fields:

- endpoint id or stable SDK method name.
- API family and kind.
- environment.
- method and path template, not full URL with query values.
- selected `tr_id`.
- attempt number.
- retry/fallback decision.
- elapsed latency.
- error category and provider `msg_cd` when available.

Never log:

- `authorization`
- access tokens
- approval keys
- app keys or app secrets
- `personalseckey`
- account numbers such as `CANO`
- raw request/response bodies for trading or account endpoints
- full WebSocket payloads containing account/order events

Metrics should be optional and feature-gated if the crate wants to avoid forcing a metrics ecosystem. If present, metrics must use low-cardinality labels.

## Contract And Model Generation

Decision:

- Treat `kis_official_endpoint_inventory.json` as the canonical generated-contract source for the first implementation.
- Generate or validate endpoint metadata from inventory, but keep public Rust APIs curated enough to be usable.

Implementation rules:

- Preserve official field names in serialized form.
- Rust model fields may use snake_case with serde renames.
- Unknown response fields should be ignored unless the endpoint model is in strict contract-test mode.
- Required fields in inventory become required Rust request fields unless an endpoint-specific implementation note proves the field is conditionally required.
- Use newtypes or enums for operation choices that select different TR IDs.
- Keep response envelope available even when typed `output*` data is parsed.

## Testing Strategy

Use a risk-based test pyramid:

### Unit tests

- Environment support checks.
- TR ID selection, including buy/sell and real/mock variants.
- Token cache expiry, skew, and single-flight refresh behavior.
- Retry classifier decisions for timeout, 401, 403, 429, 5xx, provider envelope errors, and decode errors.
- Fallback allow/deny matrix.
- Redaction behavior for all secret-bearing types.
- GET query serialization from inventory fields.

### Contract tests with mock server

- For every implemented endpoint, verify method, path, headers, query/body fields, required field handling, and response envelope shape against inventory.
- Verify mock responses include `rt_cd`, `msg_cd`, `msg1`, and documented `output*` keys.
- Verify `real_only` endpoints fail in SDK mock environment unless a local mock override is configured.
- Verify endpoints with continuation headers or `CTX_AREA_*` fields expose continuation metadata.

### Integration tests with local mocks

- Auth flow with a fake token server.
- Token refresh and one-time 401 recovery.
- Rate-limited response with `Retry-After`.
- Server 5xx retry with jitter controlled by a deterministic test clock.
- Fallback enabled/disabled behavior.
- WebSocket approval-key flow with fake approval response and fake WebSocket server when the module is implemented.

### Negative and security tests

- Missing credentials.
- Expired token refresh failure.
- Unsupported environment.
- Provider envelope failure.
- Malformed JSON.
- Unexpected `output*` type.
- Redacted logs do not contain dummy token/appsecret/account values.
- Trading POST endpoints do not auto-retry by default.

### Performance smoke tests

- Shared client reuse under repeated quotation calls against a local mock.
- Token cache prevents token endpoint calls on every SDK request.
- Optional concurrency smoke with bounded request futures and no connection-client churn.

### Live/sandbox tests

- Must be opt-in, ignored by default, and require explicit environment variables.
- Must avoid real order execution and real account mutation unless separately authorized.
- Prefer read-only quotation endpoints or mock environment endpoints.
- Must rate-limit themselves and redact all output.

## QA Acceptance Checklist

QA should verify this ADR/RFC and later implementation against these gates:

- The document cites the BOG-221 official artifacts and 2026-05-29 source date.
- Network design uses a shared async client and finite configurable timeouts.
- Token design caches access tokens and accounts for one-day validity and six-hour same-token behavior.
- Retry and fallback are explicit options, not unconditional behavior.
- Trading/order POST calls do not auto-retry by default.
- Environment support blocks mock calls for `real_only` endpoints unless local mock override is configured.
- Logging/tracing redacts credentials, tokens, approval keys, account numbers, and payloads.
- Error taxonomy does not invent unverified provider-code mappings.
- Test strategy covers unit, contract, integration-with-mock, negative/security, and performance smoke layers.
- Live tests are opt-in and non-destructive by default.

## Alternatives Considered

### A. Generate a full SDK directly from inventory

Pros:

- Fast broad endpoint coverage.
- Consistent field mapping.

Cons:

- Poor public API ergonomics for endpoints with variant TR IDs.
- Risk of encoding inventory quirks directly, such as GET fields appearing under `req_b`.
- Harder to apply careful retry, redaction, hashkey, and trading-safety policies.

Use generation for metadata and internal models, but curate public endpoint methods and safety policy boundaries.

### B. Hand-write only the highest-value endpoints

Pros:

- Better ergonomics and reviewability.
- Lower initial implementation complexity.

Cons:

- Slow coverage of 338 official endpoints.
- Higher drift risk against official specs.

Use hand-written wrappers for critical endpoints, backed by generated contract metadata.

### C. Default automatic fallback and retry for resilience

Pros:

- Appears more resilient for read paths.

Cons:

- Dangerous for trading and account mutation.
- Can hide provider outages or semantic changes between real and mock.
- Violates parent requirement that fallback and retry be SDK options.

Rejected. Retry and fallback must be explicit and visible.

## Risks And Open Questions

- `apiErrors` are mostly empty in public JSON; provider error-code mapping requires separate validation.
- `hashkey` is called out by research but was not present as a header field in the inspected normalized inventory; verify against Excel before enabling live trading.
- Array vs object shape for some `output*` fields may require official examples or mock-server contract refinement.
- Formal rate-limit values are not available as structured per-endpoint fields; avoid hard-coded quotas until verified.
- WebSocket frame schemas and reconnect semantics need a dedicated implementation RFC if WebSocket support becomes part of the first release.
- The repository currently only contains a README, so crate layout and dependency choices will be finalized by the first implementation PR.

## Implementation Handoff

The first implementation issue should:

1. Create the Rust crate layout and `KisClient` builder.
2. Implement REST transport, environment selection, redacted credentials, OAuth token provider, and the typed error enum.
3. Add inventory-backed metadata for a small endpoint slice: OAuth token/revoke, one read-only quotation endpoint, one account read endpoint, and one order endpoint behind a no-live-test safety boundary.
4. Add mock-server contract tests for that slice.
5. Add retry/fallback policy tests before exposing retry/fallback defaults.
6. Defer live credentials, live account calls, and real order execution until a separately authorized test plan exists.
