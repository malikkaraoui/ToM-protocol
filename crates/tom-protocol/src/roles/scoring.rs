//! Contribution metrics — tracks a node's relay activity and computes a score.
//!
//! Score formula: weighted sum of relay count, success rate, and uptime,
//! with progressive decay (5%/hour since last activity). Scores are always
//! recoverable — no permanent bans (design decision #4).

/// Decay rate: 5% per hour (expressed as fraction per ms).
const DECAY_RATE_PER_MS: f64 = 0.05 / 3_600_000.0;

// Scoring weight constants (tunable based on beta testing)

/// Weight for relay count in score calculation.
pub const RELAY_COUNT_WEIGHT: f64 = 1.0;

/// Weight for success rate (0.0–1.0) in score calculation.
pub const SUCCESS_RATE_WEIGHT: f64 = 5.0;

/// Weight for uptime hours in score calculation.
pub const UPTIME_WEIGHT: f64 = 0.5;

/// Weight for total bandwidth relayed (in MB) in score calculation.
pub const BANDWIDTH_MB_WEIGHT: f64 = 0.2;

/// Weight for give/take bandwidth ratio in score calculation.
pub const BANDWIDTH_RATIO_WEIGHT: f64 = 1.5;

/// Contribution metrics for a single node.
#[derive(Debug, Clone)]
pub struct ContributionMetrics {
    /// Total messages successfully relayed.
    pub messages_relayed: u64,
    /// Total relay failures.
    pub relay_failures: u64,
    /// Unix ms timestamp when this node was first observed.
    pub first_seen: u64,
    /// Unix ms timestamp of last activity.
    pub last_activity: u64,
    /// Cumulative uptime in milliseconds.
    pub total_uptime_ms: u64,
    /// Total bytes relayed for other peers.
    pub bytes_relayed: u64,
    /// Total bytes received from the network.
    pub bytes_received: u64,
}

impl ContributionMetrics {
    /// Create new metrics for a freshly observed node.
    pub fn new(now: u64) -> Self {
        Self {
            messages_relayed: 0,
            relay_failures: 0,
            first_seen: now,
            last_activity: now,
            total_uptime_ms: 0,
            bytes_relayed: 0,
            bytes_received: 0,
        }
    }

    /// Record a successful relay.
    pub fn record_relay(&mut self, now: u64) {
        self.messages_relayed += 1;
        let elapsed = now.saturating_sub(self.last_activity);
        self.total_uptime_ms += elapsed;
        self.last_activity = now;
    }

    /// Record a relay failure.
    pub fn record_relay_failure(&mut self, now: u64) {
        self.relay_failures += 1;
        let elapsed = now.saturating_sub(self.last_activity);
        self.total_uptime_ms += elapsed;
        self.last_activity = now;
    }

    /// Compute the contribution score at the given timestamp.
    ///
    /// The raw score is: relay_count * W_relay + success_rate * W_success + uptime_hours * W_uptime
    /// Then decayed by 5%/hour since last_activity.
    pub fn score(&self, now: u64) -> f64 {
        let total_attempts = self.messages_relayed + self.relay_failures;
        let success_rate = if total_attempts == 0 {
            0.0
        } else {
            self.messages_relayed as f64 / total_attempts as f64
        };

        let uptime_hours = self.total_uptime_ms as f64 / 3_600_000.0;

        let raw = (self.messages_relayed as f64) * RELAY_COUNT_WEIGHT
            + success_rate * SUCCESS_RATE_WEIGHT
            + uptime_hours * UPTIME_WEIGHT;

        // Progressive decay since last activity
        let idle_ms = now.saturating_sub(self.last_activity) as f64;
        let decay = (-DECAY_RATE_PER_MS * idle_ms).exp();

        raw * decay
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_node_score_is_zero() {
        let m = ContributionMetrics::new(1000);
        assert_eq!(m.score(1000), 0.0);
    }

    #[test]
    fn relay_increases_score() {
        let mut m = ContributionMetrics::new(1000);
        m.record_relay(2000);
        let s1 = m.score(2000);
        m.record_relay(3000);
        let s2 = m.score(3000);
        assert!(s2 > s1, "score should increase with more relays");
    }

    #[test]
    fn failure_reduces_success_rate() {
        let mut good = ContributionMetrics::new(1000);
        good.record_relay(2000);
        good.record_relay(3000);

        let mut mixed = ContributionMetrics::new(1000);
        mixed.record_relay(2000);
        mixed.record_relay_failure(3000);

        let now = 3000;
        assert!(
            good.score(now) > mixed.score(now),
            "failures should lower score"
        );
    }

    #[test]
    fn decay_reduces_score_over_time() {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);
        m.record_relay(2000);

        let at_active = m.score(2000);
        let one_hour_later = m.score(2000 + 3_600_000);
        let ten_hours_later = m.score(2000 + 36_000_000);

        assert!(
            one_hour_later < at_active,
            "score should decay after 1 hour"
        );
        assert!(
            ten_hours_later < one_hour_later,
            "score should decay more after 10 hours"
        );
        assert!(ten_hours_later > 0.0, "score never reaches zero (no permanent ban)");
    }

    #[test]
    fn score_recovers_after_decay() {
        let mut m = ContributionMetrics::new(0);
        m.record_relay(1000);

        // Let it decay for 50 hours (heavy decay)
        let decayed = m.score(50 * 3_600_000);
        assert!(decayed < 1.0, "heavily decayed: {decayed}");

        // Resume activity
        let base = 50 * 3_600_000;
        m.record_relay(base + 1000);
        m.record_relay(base + 2000);
        m.record_relay(base + 3000);

        let recovered = m.score(base + 3000);
        assert!(recovered > decayed, "score recovers with new activity");
    }

    #[test]
    fn uptime_contributes_to_score() {
        let mut m = ContributionMetrics::new(0);
        // Relay once per hour for 10 hours
        for i in 1..=10 {
            m.record_relay(i * 3_600_000);
        }
        let score = m.score(10 * 3_600_000);
        // Should have relay count contribution + uptime contribution
        assert!(score > 10.0, "10 relays + uptime should give decent score, got {score}");
    }
}
