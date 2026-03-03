//! Progressive anti-spam with score-based token bucket rate limiting.
//!
//! Design Decision #5: "Sprinkler gets sprinkled" — progressive load,
//! not exclusion. Abusive behavior becomes naturally irrational, not forbidden.
//!
//! Rate formula (smooth S-curve):
//!   effective_rate = min_rate + (max_rate - min_rate) * score / (score + midpoint)
//!
//! At score=0: 2 msg/sec (never blocked). At score=10: 26 msg/sec. At score=50: 43 msg/sec.

use std::num::NonZeroUsize;

use lru::LruCache;

use crate::types::NodeId;

// ── Configuration ──────────────────────────────────────────────────────

/// Maximum envelope size (256 KB) — enforced before parsing.
pub const MAX_ENVELOPE_SIZE: usize = 256 * 1024;

/// Configuration for progressive anti-spam rate limiting.
#[derive(Debug, Clone)]
pub struct AntiSpamConfig {
    /// Minimum rate (msg/sec) — NEVER 0 (Design Decision #5).
    pub min_rate: f64,
    /// Maximum rate (msg/sec) for high-scoring senders.
    pub max_rate: f64,
    /// Score at which rate is halfway between min and max.
    pub midpoint_score: f64,
    /// Maximum envelope size (bytes).
    pub max_envelope_size: usize,
    /// Max tracked senders before LRU eviction.
    pub max_tracked_senders: usize,
}

impl Default for AntiSpamConfig {
    fn default() -> Self {
        Self {
            min_rate: 2.0,
            max_rate: 50.0,
            midpoint_score: 10.0,
            max_envelope_size: MAX_ENVELOPE_SIZE,
            max_tracked_senders: 10_000,
        }
    }
}

// ── Token Bucket ───────────────────────────────────────────────────────

/// Token bucket for rate limiting a single sender.
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Maximum tokens (burst capacity = 2 × refill_rate).
    capacity: f64,
    /// Current token count.
    tokens: f64,
    /// Tokens added per second.
    refill_rate: f64,
    /// Last refill timestamp (Unix ms).
    last_refill: u64,
}

impl TokenBucket {
    fn new(refill_rate: f64, now: u64) -> Self {
        let capacity = refill_rate * 2.0;
        Self {
            capacity,
            tokens: capacity, // start full
            refill_rate,
            last_refill: now,
        }
    }

    /// Try to consume 1 token. Returns true if allowed.
    fn try_consume(&mut self, now: u64) -> bool {
        self.refill(now);
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self, now: u64) {
        let elapsed_ms = now.saturating_sub(self.last_refill);
        let elapsed_sec = elapsed_ms as f64 / 1000.0;
        self.tokens = (self.tokens + self.refill_rate * elapsed_sec).min(self.capacity);
        self.last_refill = now;
    }

    /// Update rate when sender's score changes. Preserves existing tokens.
    fn update_rate(&mut self, new_rate: f64, now: u64) {
        self.refill(now);
        self.refill_rate = new_rate;
        self.capacity = new_rate * 2.0;
        if self.tokens > self.capacity {
            self.tokens = self.capacity;
        }
    }
}

// ── AntiSpam Engine ────────────────────────────────────────────────────

/// Progressive anti-spam engine with score-based token bucket rate limiting.
pub struct AntiSpam {
    config: AntiSpamConfig,
    /// Per-sender token buckets (LRU-bounded).
    buckets: LruCache<NodeId, TokenBucket>,
}

impl AntiSpam {
    pub fn new(config: AntiSpamConfig) -> Self {
        let cap = NonZeroUsize::new(config.max_tracked_senders)
            .expect("max_tracked_senders must be > 0");
        Self {
            config,
            buckets: LruCache::new(cap),
        }
    }

    /// Validate envelope size (call before parsing).
    pub fn validate_size(raw_bytes: &[u8], max_size: usize) -> Result<(), String> {
        if raw_bytes.len() > max_size {
            Err(format!(
                "Envelope size {} exceeds limit {} bytes",
                raw_bytes.len(),
                max_size,
            ))
        } else {
            Ok(())
        }
    }

    /// Check if a message from `sender` is allowed.
    pub fn check_rate(&mut self, sender: NodeId, score: f64, now: u64) -> Result<(), String> {
        let rate = self.compute_rate(score);

        let bucket = if let Some(b) = self.buckets.get_mut(&sender) {
            b
        } else {
            self.buckets.push(sender, TokenBucket::new(rate, now));
            self.buckets.get_mut(&sender).unwrap()
        };

        // Update rate if score changed significantly
        if (bucket.refill_rate - rate).abs() > 0.01 {
            bucket.update_rate(rate, now);
        }

        if bucket.try_consume(now) {
            Ok(())
        } else {
            Err(format!(
                "Rate limited: score={score:.2}, rate={rate:.1} msg/s",
            ))
        }
    }

    /// Compute effective rate (msg/sec) for a given contribution score.
    pub fn compute_rate(&self, score: f64) -> f64 {
        let score = score.max(0.0); // never negative
        let range = self.config.max_rate - self.config.min_rate;
        let denom = score + self.config.midpoint_score;
        let normalized = if denom > 0.0 { score / denom } else { 0.0 };
        self.config.min_rate + range * normalized
    }

    /// Number of tracked senders (for observability).
    pub fn tracked_senders(&self) -> usize {
        self.buckets.len()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(seed: u8) -> NodeId {
        use rand::SeedableRng;
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed as u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    // ── TokenBucket ────────────────────────────────────────────────

    #[test]
    fn token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(5.0, 0);
        // Capacity = 10 (2× rate). Should allow 10 immediate messages.
        for i in 0..10 {
            assert!(bucket.try_consume(0), "message {i} should pass");
        }
        assert!(!bucket.try_consume(0), "11th should be throttled");
    }

    #[test]
    fn token_bucket_refills() {
        let mut bucket = TokenBucket::new(5.0, 0);
        // Drain all tokens
        for _ in 0..10 {
            bucket.try_consume(0);
        }
        assert!(!bucket.try_consume(0));

        // 1 second later → 5 new tokens
        assert!(bucket.try_consume(1000));
        assert!(bucket.try_consume(1000));
    }

    #[test]
    fn token_bucket_update_rate() {
        let mut bucket = TokenBucket::new(5.0, 0);
        bucket.update_rate(50.0, 0);
        assert!((bucket.refill_rate - 50.0).abs() < f64::EPSILON);
        assert!((bucket.capacity - 100.0).abs() < f64::EPSILON);
    }

    // ── Rate formula ───────────────────────────────────────────────

    #[test]
    fn compute_rate_scales() {
        let antispam = AntiSpam::new(AntiSpamConfig::default());

        let r0 = antispam.compute_rate(0.0);
        assert!((r0 - 2.0).abs() < f64::EPSILON, "score=0 → min_rate, got {r0}");

        let r10 = antispam.compute_rate(10.0);
        assert!(r10 > 25.0 && r10 < 27.0, "score=10 → ~26, got {r10}");

        let r100 = antispam.compute_rate(100.0);
        assert!(r100 > 45.0 && r100 <= 50.0, "score=100 → near max, got {r100}");
    }

    #[test]
    fn never_blocks_completely() {
        // Design Decision #5: min_rate > 0 always
        let antispam = AntiSpam::new(AntiSpamConfig::default());
        assert!(antispam.compute_rate(0.0) > 0.0);
        assert!(antispam.compute_rate(-1.0) > 0.0); // even negative score
    }

    // ── AntiSpam engine ────────────────────────────────────────────

    #[test]
    fn check_rate_allows_high_score() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let alice = test_node_id(1);
        // score=50 → rate ~43, burst ~86 — should allow many rapid messages
        for i in 0..80 {
            assert!(
                antispam.check_rate(alice, 50.0, i * 10).is_ok(),
                "high-score message {i} should pass"
            );
        }
    }

    #[test]
    fn check_rate_throttles_low_score() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let spammer = test_node_id(2);
        // score=0 → rate=2, burst=4
        let mut allowed = 0u32;
        for _ in 0..10 {
            if antispam.check_rate(spammer, 0.0, 0).is_ok() {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 4, "score=0 burst should be 4");
    }

    #[test]
    fn check_rate_updates_on_score_change() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let alice = test_node_id(1);

        // Start with low score
        antispam.check_rate(alice, 0.0, 0).ok();
        let rate_low = antispam.buckets.peek(&alice).unwrap().refill_rate;

        // Score improves
        antispam.check_rate(alice, 30.0, 1000).ok();
        let rate_high = antispam.buckets.peek(&alice).unwrap().refill_rate;

        assert!(rate_high > rate_low, "rate should increase: {rate_low} → {rate_high}");
    }

    #[test]
    fn validate_size_rejects_oversized() {
        let small = vec![0u8; 1024];
        let huge = vec![0u8; 512 * 1024];
        let max = MAX_ENVELOPE_SIZE;

        assert!(AntiSpam::validate_size(&small, max).is_ok());
        assert!(AntiSpam::validate_size(&huge, max).is_err());
    }

    #[test]
    fn lru_eviction() {
        let mut config = AntiSpamConfig::default();
        config.max_tracked_senders = 3;
        let mut antispam = AntiSpam::new(config);

        let ids: Vec<_> = (1..=4).map(test_node_id).collect();

        // Fill cache
        for id in &ids[..3] {
            antispam.check_rate(*id, 5.0, 0).ok();
        }
        assert_eq!(antispam.tracked_senders(), 3);

        // 4th sender evicts oldest
        antispam.check_rate(ids[3], 5.0, 0).ok();
        assert_eq!(antispam.tracked_senders(), 3);
        assert!(antispam.buckets.peek(&ids[0]).is_none(), "oldest evicted");
    }
}
