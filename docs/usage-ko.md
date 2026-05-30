# KIS SDK 한국어 사용 가이드

이 문서는 현재 `kis-sdk` Rust 구현을 사용하는 애플리케이션 개발자를
위한 한국어 가이드입니다. 승인된 비밀 관리 경로를 통해 실거래 환경의
읽기 요청을 연결하는 흐름을 기준으로 설명합니다.

현재 SDK는 OAuth 토큰 발급/폐기, WebSocket approval key 발급, 일부
국내주식 typed 메서드, 그리고 공식 inventory 기반 실행 API를 제공합니다.
`contracts/kis_official_endpoint_inventory.compact.json`에 포함된 공식
endpoint 338개는 typed 메서드, domain-scoped inventory API, 또는
lower-level `execute_inventory` 경로 중 하나로 SDK에서 호출 가능한 상태로
account됩니다. 모든 endpoint가 개별 Rust request/response struct로 승격된
상태는 아닙니다.

## 사전 준비

- Rust 2021 toolchain
- `tokio` 기반 async runtime
- 실환경 읽기 호출에 사용할 KIS app key/app secret

실제 app key, app secret, access token, approval key, 계좌번호, 고객 데이터는
소스 코드, 테스트 fixture, 로그, 문서 예제에 넣지 마세요.

## 의존성 추가

현재 crate가 아직 crates.io에 publish되지 않았다면 repository를 직접
참조하세요.

```toml
[dependencies]
kis-sdk = { git = "https://github.com/bogyie/kis-sdk", branch = "main" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
```

나중에 별도 승인으로 crates.io publish가 활성화되면 versioned dependency로
전환할 수 있습니다.

```toml
[dependencies]
kis-sdk = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
serde_json = "1"
```

## 실환경 읽기 Client 구성

실환경 읽기 호출은 credential을 repository 밖에서 로드하세요.

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

로드한 값을 출력하거나 저장하지 마세요. 공유 개발 머신 또는 public CI에서
실제 credential을 사용하는 테스트를 실행하지 마세요.

## OAuth와 Approval Key

OAuth bearer token을 직접 발급해야 하면 `issue_access_token`을 사용합니다.

```rust
async fn issue_token(client: &kis_sdk::KisClient) -> Result<String, kis_sdk::KisError> {
    let token = client.issue_access_token().await?;
    Ok(token.access_token)
}
```

토큰 폐기는 명시적으로 `revoke_access_token`을 호출해야 합니다. `KisClient`가
drop될 때 자동 폐기하지 않습니다.

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

KIS WebSocket client가 필요한 approval key는 `issue_realtime_approval_key`로
발급합니다.

```rust
async fn websocket_approval_key(
    client: &kis_sdk::KisClient,
) -> Result<String, kis_sdk::KisError> {
    let response = client.issue_realtime_approval_key().await?;
    Ok(response.approval_key)
}
```

이 메서드는 approval key 발급까지만 담당합니다. 현재 typed SDK는 live
WebSocket session, subscription, reconnect, message decoding을 관리하지
않습니다.

## 국내주식 현재가 조회

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

현재 응답 `output`은 provider field를 `serde_json::Value`로 보존합니다. 넓은
범위의 typed response struct는 후속 작업에서 점진적으로 추가될 수 있습니다.

## Inventory Endpoint 호출

공식 inventory에 포함된 endpoint는 stable operation id와
`execute_inventory`로 호출할 수 있습니다.

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

국내주식 REST collection으로 scope를 제한하려면
`execute_domestic_stock_rest`를 사용합니다. 이 helper는 국내주식
주문/계좌, 기본시세, ELW, 업종/기타, 종목정보, 차트/분석, 순위분석
collection의 158개 endpoint를 다룹니다.

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

Inventory layer는 network I/O 전에 필수 query/body/non-standard header를
검증합니다. `appkey`, `appsecret`, `authorization`, `custtype`,
`content-type`, 명확한 `tr_id` 같은 표준 KIS header는 client가 채웁니다.
TR ID가 여러 후보로 표현된 endpoint는 `InventoryRequest::tr_id_override(...)`
를 통해 caller가 명시적으로 선택해야 하며, 실환경 trading mutation은
`KisError::LiveTradingDisabled`로 차단됩니다.

## 해외주식 Endpoint 호출

해외주식 SDK surface는 공식 inventory의 51개 endpoint를 enum handle로
고정합니다. 원문 operation id 문자열 대신 안정적인 SDK handle을 쓰고 싶을 때
`OverseasStockEndpoint`를 사용하세요.

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

해외주식 주문 endpoint도 국내 주문과 같은 안전 경계를 따릅니다. 실환경의
trading mutation은 network I/O 전에 `KisError::LiveTradingDisabled`를
반환합니다.

## 국내선물옵션 Endpoint 호출

국내선물옵션 coverage는 44개 공식 endpoint를 scoped inventory API로
제공합니다. 주문/계좌 15개, 시세 9개, 실시간시세 20개이며 operation id
상수는 `kis_sdk::apis::domestic_futures_options`에서 제공합니다.

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

주문 변경 성격의 국내선물옵션 endpoint는 다른 trading mutation과 동일하게
실환경에서 local block됩니다.

## 실시간 Tryitout Endpoint 호출

국내주식 실시간과 채권 실시간 helper는 공식 inventory에 보존된 REST-style
`/tryitout/*` 형태를 실행합니다. 이는 inventory 기반 요청 형식 검증을 위한
API이며 live WebSocket subscription API가 아닙니다.

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

## 채권 Endpoint 호출

채권 helper는 주문/계좌 7개, 시세 8개, realtime tryitout 3개 endpoint로
scope가 나뉩니다. 실환경 trading mutation은 network I/O 전에 차단됩니다.

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

## 잔고 조회

잔고 조회에는 placeholder 계좌 값을 사용할 수 있습니다. 실제 계좌 식별자는
민감 정보이며 승인된 secret path에서만 주입해야 합니다.

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

현재 구현은 실환경 cash order를 network I/O 전에
`KisError::LiveTradingDisabled`로 차단합니다.

## Live Trading 안전 경계

- 예제는 placeholder credential과 placeholder 계좌번호만 사용합니다.
- 문서 예제는 live credential, production account data, live order execution을
  소스 코드에 넣지 않습니다.
- `Environment::Real`의 trading mutation은 현재 local guard로 차단되며
  `KisError::LiveTradingDisabled`를 반환합니다.
- Realtime helper의 `/tryitout/*` 호출은 inventory REST shape 검증용이며
  live WebSocket subscription이 아닙니다.
- Retry는 기본적으로 꺼져 있습니다. `RetryPolicy::conservative_reads()`는
  retry 가능한 GET/read 실패만 재시도하며 trading POST mutation은 재시도하지
  않습니다.

## Endpoint Coverage 상태

`tests/sdk_core.rs::full_inventory_reconciliation_accounts_for_every_official_endpoint_once`
는 bundled `InventoryCatalog`의 모든 공식 operation id를 정확히 하나의
SDK-callable coverage surface에 배정하는 machine-checkable gate입니다.

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

이 상태는 BOG-221에서 captured된 bundled official inventory 기준입니다. 공식
포털을 live re-scrape한 결과가 아니며, 모든 endpoint가 narrow typed
request/response struct로 제공된다는 의미도 아닙니다. 현재 보장되는 것은 각
endpoint가 typed method, scoped inventory API, 또는 lower-level
`execute_inventory` 경로로 SDK-callable하게 account된다는 점입니다.

자세한 collection split, contract evidence, known limitation은
[`contract-quality-report.md`](contract-quality-report.md)를 참고하세요.

## 검증 명령

현재 문서와 SDK baseline에서 사용한 주요 검증 명령은 다음과 같습니다.

```sh
cargo fmt --check
cargo test --locked
cargo test --locked --test mock_server_contract
cargo doc --locked --no-deps
git diff --check
```

문서만 변경한 경우에도 code block과 intra-doc link 확인을 위해
`cargo doc --locked --no-deps`와 `git diff --check`를 다시 실행하는 것이
좋습니다.

## 관련 문서

- [Repository README](../README.md)
- [English usage guide](usage.md)
- [Contract quality report](contract-quality-report.md)
- [Runtime architecture ADR/RFC](adr/0001-kis-sdk-runtime-architecture.md)
