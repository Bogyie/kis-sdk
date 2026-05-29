use std::{env, net::SocketAddr};

use kis_sdk::{contract::ContractInventory, mock};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:0".to_string())
        .parse::<SocketAddr>()?;
    let inventory = ContractInventory::bundled()?;
    let app = mock::app(inventory);
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;

    println!("kis mock server listening on http://{local_addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
