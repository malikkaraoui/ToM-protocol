//! tom-connect — ToM transport layer (forked from iroh 0.96)
//!
//! Peer-to-peer QUIC connections with hole punching and relay fallback.
//!
//! This crate provides the transport layer for the ToM Protocol, forked from
//! iroh v0.96.0 at the socket layer. It implements direct connectivity using
//! hole punching complemented by relay servers.
//!
//! # Phase R7.3 Fork
//!
//! This code is forked from iroh (MIT/Apache-2.0) at the socket/endpoint boundary only.
//! We keep Quinn, rustls, and other dependencies as upstream.
//!
//! # Core Components
//!
//! - [`Endpoint`]: Main API to establish connections
//! - [`protocol`]: Router and ProtocolHandler for accepting connections
//! - [`socket`]: UDP socket with path multiplexing (relay + direct)
//! - [`address_lookup`]: DNS/Pkarr address discovery
//! - [`net_report`]: Network diagnostics
//!
//! # Example
//!
//! ```no_run
//! use tom_connect::Endpoint;
//! # async fn example() -> anyhow::Result<()> {
//! // Bind endpoint
//! let endpoint = Endpoint::bind().await?;
//!
//! // Connect to peer (DHT lookup + hole punch)
//! // let conn = endpoint.connect(peer_addr, b"tom").await?;
//! # Ok(())
//! # }
//! ```

#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![recursion_limit = "256"]

mod socket;
mod tls;

pub(crate) mod util;

pub mod address_lookup;
pub mod defaults;
#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub mod dns;
pub mod endpoint;
pub mod metrics;
mod net_report;
pub mod protocol;

// Re-export main types
pub use endpoint::{Endpoint, RelayMode};

// Re-export iroh-base types (these will be our bridge to iroh ecosystem)
pub use iroh_base::{
    EndpointAddr, EndpointId, KeyParsingError, PublicKey, RelayUrl, RelayUrlParseError, SecretKey,
    Signature, SignatureError, TransportAddr,
};

// Re-export iroh-relay types
pub use iroh_relay::{RelayConfig, RelayMap, endpoint_info};

// Re-export net_report
pub use net_report::{Report as NetReport, TIMEOUT as NET_REPORT_TIMEOUT};

// Re-export n0 utilities
pub use n0_watcher::Watcher;

/// Node identity (Ed25519 public key).
/// This is a type alias for [`PublicKey`] to maintain naming consistency with ToM.
pub type NodeId = PublicKey;

/// Node address (ID + relay URLs + direct addresses).
/// This is a type alias for [`EndpointAddr`] to maintain naming consistency with ToM.
pub type NodeAddr = EndpointAddr;
