# KIS Mock Server

This mock server is built from the BOG-221 official endpoint inventory collected on
2026-05-29. The bundled compact fixture keeps the official source URL, collection
count, endpoint count, route method/path, environment support, TR IDs, required
fields, and response field names.

Run locally:

```sh
cargo run --bin kis-mock-server -- 127.0.0.1:0
```

The server dynamically registers all 338 official endpoint routes from
`contracts/kis_official_endpoint_inventory.compact.json`. Routes marked
`real_only` return `KIS_MOCK_UNSUPPORTED_ENVIRONMENT` instead of simulating mock
support that the official contract does not provide. Environment support is
checked before scenario fixtures, so scenario headers cannot make a `real_only`
route look mock-supported.

Supported deterministic fixture scenarios use the `x-kis-mock-scenario` header:

- `unauthorized`: returns an unauthorized provider envelope.
- `rate-limit`: returns HTTP 429 with `retry-after: 1`.
- `retryable-500`: returns HTTP 503.
- `provider-error`: returns HTTP 200 with a provider error envelope.

Non-auth routes validate required KIS headers from the contract, including bearer
authorization, app key headers, customer type, and simple TR ID matches. OAuth
routes return dummy local-only tokens and approval keys.
