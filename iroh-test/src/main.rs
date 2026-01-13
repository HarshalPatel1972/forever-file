use anyhow::Result;
use iroh::{protocol::Router, Endpoint, Watcher};
use iroh_ping::Ping;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Iroh P2P Connectivity Test ===");
    
    // Create a receiver endpoint
    let recv_endpoint = Endpoint::bind().await?;
    recv_endpoint.online().await;
    
    let ping_protocol = Ping::new();
    
    // Set up the receiver to accept ping requests
    let _router = Router::builder(recv_endpoint.clone())
        .accept(iroh_ping::ALPN, ping_protocol.clone())
        .spawn();
    
    // Get the address to share
    let addr = recv_endpoint.addr();
    println!("Receiver address: {:?}", addr);
    
    // Create a sender endpoint and ping the receiver
    let send_endpoint = Endpoint::bind().await?;
    let sender_ping = Ping::new();
    
    println!("Sending ping...");
    let rtt = sender_ping.ping(&send_endpoint, addr).await?;
    
    println!("✅ SUCCESS! Ping completed in {:?}", rtt);
    println!("P2P connection established and verified!");
    
    Ok(())
}
