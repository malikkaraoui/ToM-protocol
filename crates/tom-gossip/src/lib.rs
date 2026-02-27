//! tom-gossip — Gossip broadcast over HyParView/PlumTree
//!
//! Forked from iroh-gossip v0.96.0. Broadcast messages to peers subscribed to a topic.
//!
//! This crate is designed to be used with [tom-connect] for transport,
//! but the protocol layer (`proto`) can also be used standalone.
#![deny(missing_docs, rustdoc::broken_intra_doc_links)]
#![allow(unexpected_cfgs)]

#[cfg(feature = "net")]
pub use net::Gossip;
#[cfg(feature = "net")]
#[doc(inline)]
pub use net::GOSSIP_ALPN as ALPN;

#[cfg(feature = "net")]
pub mod api;
pub mod metrics;
#[cfg(feature = "net")]
pub mod net;
pub mod proto;

pub use proto::TopicId;
