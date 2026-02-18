/// Message lifecycle tracker for ToM protocol.
///
/// Tracks the status pipeline: Pending → Sent → Relayed → Delivered → Read.
/// Status is monotonically increasing — no regression allowed.
///
/// Pure logic, no I/O. The caller feeds events (ACKs, read receipts),
/// the tracker updates status and reports transitions.
use std::collections::HashMap;
use std::time::Instant;

use crate::types::{MessageStatus, NodeId};

/// Maximum number of tracked messages (DoS protection).
const MAX_TRACKED: usize = 10_000;

/// Maximum age for a tracked message before it's considered stuck (24h).
const MAX_AGE_SECS: u64 = 24 * 60 * 60;

/// A status transition event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    pub message_id: String,
    pub previous: MessageStatus,
    pub current: MessageStatus,
}

/// Internal tracking entry for a single message.
#[derive(Debug, Clone)]
struct TrackedMessage {
    status: MessageStatus,
    #[allow(dead_code)] // Used for future monitoring/queries
    to: NodeId,
    created: Instant,
}

/// Tracks message lifecycle from send to read receipt.
///
/// Feed it events from the Router (ACKs, read receipts) and it
/// returns status transitions that the application can display.
pub struct MessageTracker {
    messages: HashMap<String, TrackedMessage>,
}

impl MessageTracker {
    pub fn new() -> Self {
        Self {
            messages: HashMap::new(),
        }
    }

    /// Start tracking a new outgoing message.
    ///
    /// Returns `None` if at capacity (caller should decide: drop oldest or reject).
    pub fn track(&mut self, message_id: String, to: NodeId) -> Option<StatusChange> {
        if self.messages.len() >= MAX_TRACKED {
            self.evict_expired();
            if self.messages.len() >= MAX_TRACKED {
                return None; // Still at capacity
            }
        }

        self.messages.insert(
            message_id.clone(),
            TrackedMessage {
                status: MessageStatus::Pending,
                to,
                created: Instant::now(),
            },
        );

        Some(StatusChange {
            message_id,
            previous: MessageStatus::Pending,
            current: MessageStatus::Pending,
        })
    }

    /// Mark a message as sent (transport confirmed it left this node).
    pub fn mark_sent(&mut self, message_id: &str) -> Option<StatusChange> {
        self.advance(message_id, MessageStatus::Sent)
    }

    /// Mark a message as relayed (relay ACK received).
    pub fn mark_relayed(&mut self, message_id: &str) -> Option<StatusChange> {
        self.advance(message_id, MessageStatus::Relayed)
    }

    /// Mark a message as delivered (recipient ACK received).
    pub fn mark_delivered(&mut self, message_id: &str) -> Option<StatusChange> {
        self.advance(message_id, MessageStatus::Delivered)
    }

    /// Mark a message as read (read receipt received).
    pub fn mark_read(&mut self, message_id: &str) -> Option<StatusChange> {
        self.advance(message_id, MessageStatus::Read)
    }

    /// Get the current status of a tracked message.
    pub fn status(&self, message_id: &str) -> Option<MessageStatus> {
        self.messages.get(message_id).map(|m| m.status)
    }

    /// Number of currently tracked messages.
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Remove a message from tracking (e.g., after Read or after TTL expiry).
    pub fn remove(&mut self, message_id: &str) -> bool {
        self.messages.remove(message_id).is_some()
    }

    /// Evict messages older than MAX_AGE_SECS.
    pub fn evict_expired(&mut self) {
        let cutoff = std::time::Duration::from_secs(MAX_AGE_SECS);
        let now = Instant::now();
        self.messages
            .retain(|_, m| now.duration_since(m.created) < cutoff);
    }

    // ── Internal ───────────────────────────────────────────────────────

    /// Advance a message to a new status. Only forward transitions are allowed.
    fn advance(&mut self, message_id: &str, new_status: MessageStatus) -> Option<StatusChange> {
        let entry = self.messages.get_mut(message_id)?;

        // Monotonic: only advance, never regress
        if new_status <= entry.status {
            return None;
        }

        let previous = entry.status;
        entry.status = new_status;

        Some(StatusChange {
            message_id: message_id.to_string(),
            previous,
            current: new_status,
        })
    }
}

impl Default for MessageTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = iroh::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn track_new_message() {
        let mut tracker = MessageTracker::new();
        let change = tracker.track("msg-1".into(), node_id(2));
        assert!(change.is_some());
        assert_eq!(tracker.status("msg-1"), Some(MessageStatus::Pending));
    }

    #[test]
    fn full_lifecycle() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));

        let c1 = tracker.mark_sent("msg-1").unwrap();
        assert_eq!(c1.previous, MessageStatus::Pending);
        assert_eq!(c1.current, MessageStatus::Sent);

        let c2 = tracker.mark_relayed("msg-1").unwrap();
        assert_eq!(c2.previous, MessageStatus::Sent);
        assert_eq!(c2.current, MessageStatus::Relayed);

        let c3 = tracker.mark_delivered("msg-1").unwrap();
        assert_eq!(c3.previous, MessageStatus::Relayed);
        assert_eq!(c3.current, MessageStatus::Delivered);

        let c4 = tracker.mark_read("msg-1").unwrap();
        assert_eq!(c4.previous, MessageStatus::Delivered);
        assert_eq!(c4.current, MessageStatus::Read);
    }

    #[test]
    fn no_regression() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_delivered("msg-1");

        // Trying to go back to Sent — should return None
        assert!(tracker.mark_sent("msg-1").is_none());
        assert!(tracker.mark_relayed("msg-1").is_none());

        // Status unchanged
        assert_eq!(tracker.status("msg-1"), Some(MessageStatus::Delivered));
    }

    #[test]
    fn skip_intermediate_states() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));

        // Jump from Pending straight to Delivered (e.g., direct connection, no relay)
        let change = tracker.mark_delivered("msg-1").unwrap();
        assert_eq!(change.previous, MessageStatus::Pending);
        assert_eq!(change.current, MessageStatus::Delivered);
    }

    #[test]
    fn duplicate_advance_returns_none() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_sent("msg-1");

        // Second mark_sent should be a no-op
        assert!(tracker.mark_sent("msg-1").is_none());
    }

    #[test]
    fn unknown_message_returns_none() {
        let mut tracker = MessageTracker::new();
        assert!(tracker.mark_sent("nonexistent").is_none());
        assert!(tracker.status("nonexistent").is_none());
    }

    #[test]
    fn remove_message() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        assert!(!tracker.is_empty());

        assert!(tracker.remove("msg-1"));
        assert!(tracker.is_empty());
        assert!(!tracker.remove("msg-1")); // Already removed
    }

    #[test]
    fn capacity_limit() {
        let mut tracker = MessageTracker::new();
        let target = node_id(2);

        // Fill to capacity
        for i in 0..MAX_TRACKED {
            tracker.track(format!("msg-{i}"), target);
        }
        assert_eq!(tracker.len(), MAX_TRACKED);

        // Next track should trigger eviction, but since all are fresh, still at capacity
        let result = tracker.track("overflow".into(), target);
        assert!(result.is_none());
    }

    #[test]
    fn len_and_is_empty() {
        let mut tracker = MessageTracker::new();
        assert!(tracker.is_empty());
        assert_eq!(tracker.len(), 0);

        tracker.track("msg-1".into(), node_id(2));
        assert!(!tracker.is_empty());
        assert_eq!(tracker.len(), 1);
    }
}
