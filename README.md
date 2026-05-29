# kis-sdk

Rust SDK core for Korea Investment & Securities Open API.

The first implementation exposes:

- `KisClient` builder with explicit real/mock environment selection and shared
  `reqwest` client reuse.
- Redacted `AppCredentials`, `Account`, and `SecretString` helpers.
- OAuth token issuance and in-memory token reuse, with static bearer token
  injection for tests.
- Typed domestic stock examples for quotation price, balance inquiry, and
  cash order calls against the bundled mock server contract.
- Explicit `RetryPolicy` and `FallbackPolicy` options. Retry is disabled by
  default; trading POST retries are not enabled by the conservative read policy.

## Architecture

- [ADR/RFC 0001: KIS Rust SDK Runtime Architecture](docs/adr/0001-kis-sdk-runtime-architecture.md)

## Mock Server

- [KIS Mock Server](docs/mock-server/README.md)
