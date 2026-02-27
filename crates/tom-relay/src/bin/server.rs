//! ToM relay server binary
//!
//! Phase R7.2: Placeholder. Will run actual relay in R7.3.

use tom_relay::RelayServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("tom-relay server (skeleton)");
    println!("Phase R7.2: Binary placeholder only");

    let _server = RelayServer::new();

    Ok(())
}
