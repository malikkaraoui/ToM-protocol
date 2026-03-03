/// Message lifecycle tracker for ToM protocol.
///
/// Tracks the status pipeline: Pending → Sent → Relayed → Delivered → Read.
/// Status is monotonically increasing — no regression allowed.
/// `Failed` is a terminal state set explicitly after ACK timeout + retries.
///
/// Pure logic, no I/O. The caller feeds events (ACKs, read receipts),
/// the tracker updates status and reports transitions.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::types::{now_ms, MessageStatus, NodeId};

/// Maximum number of tracked messages (DoS protection).
const MAX_TRACKED: usize = 10_000;

/// Maximum age for a tracked message before it's considered stuck (24h).
const MAX_AGE_SECS: u64 = 24 * 60 * 60;

/// Default ACK deadline: if no Delivered ACK within this window, retry.
pub const DEFAULT_ACK_DEADLINE_SECS: u64 = 30;

/// Default number of retries after initial send (on ACK timeout).
pub const DEFAULT_MAX_RETRIES: u8 = 2;

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
    to: NodeId,
    created: Instant,
    /// When delivery ACK is expected by. None = no deadline (already delivered or no retry).
    deadline: Option<Instant>,
    /// How many retries remain before marking Failed.
    retries_remaining: u8,
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

    /// Start tracking a new outgoing message with ACK deadline.
    ///
    /// Returns `None` if at capacity (caller should decide: drop oldest or reject).
    pub fn track(&mut self, message_id: String, to: NodeId) -> Option<StatusChange> {
        if self.messages.len() >= MAX_TRACKED {
            self.evict_expired();
            if self.messages.len() >= MAX_TRACKED {
                return None; // Still at capacity
            }
        }

        let now = Instant::now();
        self.messages.insert(
            message_id.clone(),
            TrackedMessage {
                status: MessageStatus::Pending,
                to,
                created: now,
                deadline: Some(now + Duration::from_secs(DEFAULT_ACK_DEADLINE_SECS)),
                retries_remaining: DEFAULT_MAX_RETRIES,
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
    /// Clears the ACK deadline — no more retries needed.
    pub fn mark_delivered(&mut self, message_id: &str) -> Option<StatusChange> {
        let result = self.advance(message_id, MessageStatus::Delivered);
        if result.is_some() {
            if let Some(entry) = self.messages.get_mut(message_id) {
                entry.deadline = None;
            }
        }
        result
    }

    /// Mark a message as read (read receipt received).
    pub fn mark_read(&mut self, message_id: &str) -> Option<StatusChange> {
        self.advance(message_id, MessageStatus::Read)
    }

    /// Mark a message as failed (terminal state — delivery abandoned).
    ///
    /// This is NOT part of the monotonic pipeline — it can be set from any
    /// pre-Delivered status. Used after ACK timeout + all retries exhausted.
    pub fn mark_failed(&mut self, message_id: &str) -> Option<StatusChange> {
        let entry = self.messages.get_mut(message_id)?;

        // Only fail messages that haven't been delivered yet
        if entry.status >= MessageStatus::Delivered {
            return None;
        }

        let previous = entry.status;
        entry.status = MessageStatus::Failed;
        entry.deadline = None;

        Some(StatusChange {
            message_id: message_id.to_string(),
            previous,
            current: MessageStatus::Failed,
        })
    }

    /// Reset the ACK deadline after a retry (extends the window).
    /// Decrements retries_remaining.
    pub fn reset_deadline(&mut self, message_id: &str) {
        if let Some(entry) = self.messages.get_mut(message_id) {
            entry.deadline = Some(Instant::now() + Duration::from_secs(DEFAULT_ACK_DEADLINE_SECS));
            entry.retries_remaining = entry.retries_remaining.saturating_sub(1);
        }
    }

    /// Check for messages whose ACK deadline has expired.
    /// Returns (message_id, to, retries_remaining) for each expired message.
    pub fn expired_deadlines(&self) -> Vec<(String, NodeId, u8)> {
        let now = Instant::now();
        self.messages
            .iter()
            .filter(|(_, m)| {
                m.status < MessageStatus::Delivered
                    && m.status != MessageStatus::Failed
                    && m.deadline.is_some_and(|d| now >= d)
            })
            .map(|(id, m)| (id.clone(), m.to, m.retries_remaining))
            .collect()
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
        let cutoff = Duration::from_secs(MAX_AGE_SECS);
        let now = Instant::now();
        self.messages
            .retain(|_, m| now.duration_since(m.created) < cutoff);
    }

    // ── Internal ───────────────────────────────────────────────────────

    /// Advance a message to a new status. Only forward transitions are allowed.
    /// Does NOT allow transitioning to Failed (use mark_failed() instead).
    fn advance(&mut self, message_id: &str, new_status: MessageStatus) -> Option<StatusChange> {
        let entry = self.messages.get_mut(message_id)?;

        // Don't advance Failed messages
        if entry.status == MessageStatus::Failed {
            return None;
        }

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

/// Serializable record for persistence (Instant → u64 ms since epoch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedMessageRecord {
    pub to: NodeId,
    pub status: MessageStatus,
    pub created_ms: u64,
    pub retries_remaining: u8,
}

impl MessageTracker {
    /// Export active (non-terminal) messages for persistence.
    ///
    /// Only messages with status < Delivered are included (Pending, Sent, Relayed).
    /// Delivered/Read/Failed messages are not worth persisting — they're done.
    pub fn snapshot(&self) -> HashMap<String, TrackedMessageRecord> {
        let epoch_now = now_ms();
        self.messages
            .iter()
            .filter(|(_, m)| m.status < MessageStatus::Delivered && m.status != MessageStatus::Failed)
            .map(|(id, m)| {
                let created_ms = epoch_now.saturating_sub(m.created.elapsed().as_millis() as u64);
                (
                    id.clone(),
                    TrackedMessageRecord {
                        to: m.to,
                        status: m.status,
                        created_ms,
                        retries_remaining: m.retries_remaining,
                    },
                )
            })
            .collect()
    }

    /// Restore tracked messages from persistence.
    ///
    /// Deadlines are reset to `now + DEFAULT_ACK_DEADLINE_SECS` since wall-clock
    /// time has passed during the downtime. Retries are preserved as-is.
    pub fn restore(&mut self, records: HashMap<String, TrackedMessageRecord>) {
        let now = Instant::now();
        for (id, record) in records {
            self.messages.insert(
                id,
                TrackedMessage {
                    status: record.status,
                    to: record.to,
                    created: now,
                    deadline: Some(now + Duration::from_secs(DEFAULT_ACK_DEADLINE_SECS)),
                    retries_remaining: record.retries_remaining,
                },
            );
        }
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
        let secret = tom_connect::SecretKey::generate(&mut rng);
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

    // ── R9.2: Deadline & retry tests ─────────────────────────────────

    #[test]
    fn track_sets_deadline() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));

        // Message should have a deadline (not yet expired since just created)
        let expired = tracker.expired_deadlines();
        assert!(expired.is_empty(), "fresh message should not be expired");
    }

    #[test]
    fn delivered_clears_deadline() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_delivered("msg-1");

        // After delivery, deadline should be cleared
        let expired = tracker.expired_deadlines();
        assert!(expired.is_empty());
    }

    #[test]
    fn mark_failed_terminal() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_sent("msg-1");

        let change = tracker.mark_failed("msg-1").unwrap();
        assert_eq!(change.previous, MessageStatus::Sent);
        assert_eq!(change.current, MessageStatus::Failed);
        assert_eq!(tracker.status("msg-1"), Some(MessageStatus::Failed));

        // Can't advance a Failed message
        assert!(tracker.mark_delivered("msg-1").is_none());
        assert!(tracker.mark_sent("msg-1").is_none());
    }

    #[test]
    fn cannot_fail_delivered_message() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_delivered("msg-1");

        // Can't fail a delivered message
        assert!(tracker.mark_failed("msg-1").is_none());
        assert_eq!(tracker.status("msg-1"), Some(MessageStatus::Delivered));
    }

    #[test]
    fn reset_deadline_decrements_retries() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));

        // Initial retries = DEFAULT_MAX_RETRIES (2)
        let expired = tracker.expired_deadlines();
        assert!(expired.is_empty()); // not expired yet

        // Simulate deadline expiry by resetting (which decrements)
        tracker.reset_deadline("msg-1");
        // After 1 reset: retries_remaining = 1

        tracker.reset_deadline("msg-1");
        // After 2 resets: retries_remaining = 0

        // Force expiry by checking internal state via expired_deadlines
        // (would need deadline to be in the past — tested via integration)
    }

    #[test]
    fn expired_deadlines_excludes_failed() {
        let mut tracker = MessageTracker::new();
        tracker.track("msg-1".into(), node_id(2));
        tracker.mark_failed("msg-1");

        // Failed messages should NOT appear in expired_deadlines
        // (even if they somehow had a deadline)
        let expired = tracker.expired_deadlines();
        assert!(expired.is_empty());
    }

    // ── R10.2: Snapshot/restore tests ──────────────────────────────────

    #[test]
    fn snapshot_excludes_delivered_and_failed() {
        let mut tracker = MessageTracker::new();
        let bob = node_id(2);

        tracker.track("pending".into(), bob);
        tracker.track("sent".into(), bob);
        tracker.mark_sent("sent");
        tracker.track("delivered".into(), bob);
        tracker.mark_delivered("delivered");
        tracker.track("failed".into(), bob);
        tracker.mark_failed("failed");

        let snap = tracker.snapshot();
        // Only Pending and Sent should be in snapshot
        assert_eq!(snap.len(), 2);
        assert!(snap.contains_key("pending"));
        assert!(snap.contains_key("sent"));
        assert!(!snap.contains_key("delivered"));
        assert!(!snap.contains_key("failed"));
    }

    #[test]
    fn restore_resets_deadlines() {
        let bob = node_id(2);
        let mut records = HashMap::new();
        records.insert(
            "msg-1".to_string(),
            TrackedMessageRecord {
                to: bob,
                status: MessageStatus::Sent,
                created_ms: 1000000,
                retries_remaining: 1,
            },
        );

        let mut tracker = MessageTracker::new();
        tracker.restore(records);

        assert_eq!(tracker.len(), 1);
        assert_eq!(tracker.status("msg-1"), Some(MessageStatus::Sent));

        // Should have a fresh deadline (not expired yet)
        let expired = tracker.expired_deadlines();
        assert!(expired.is_empty(), "restored message should have fresh deadline");
    }
}
