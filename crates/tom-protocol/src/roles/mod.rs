/// Dynamic role management — contribution scoring and role promotion/demotion.
///
/// Nodes earn contribution scores by relaying messages for the network.
/// High scorers get promoted to Relay role; low scorers get demoted back to Peer.
/// Scores decay progressively (5%/hour) — no permanent bans (design decision #4).
pub mod manager;
pub mod scoring;

pub use manager::{RoleAction, RoleManager};
pub use scoring::ContributionMetrics;
