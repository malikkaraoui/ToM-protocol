fn main() {
    println!("ToM Protocol - iroh PoC");
    println!();
    println!("Available binaries:");
    println!("  cargo run --bin echo-server   # Start the echo server");
    println!("  cargo run --bin node -- <ID>  # Connect to echo server");
    println!();
    println!("Run echo-server first, then use the printed EndpointId");
    println!("to connect with the node binary from another terminal.");
}
