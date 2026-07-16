//! Progressive anti-spam with score-based token bucket rate limiting.
//!
//! Design Decision #5: "Sprinkler gets sprinkled" — progressive load,
//! not exclusion. Abusive behavior becomes naturally irrational, not forbidden.
//!
//! Rate formula (smooth S-curve):
//!   effective_rate = min_rate + (max_rate - min_rate) * score / (score + midpoint)
//!
//! At score=0: 30 msg/sec (never blocked). At score=10: 65 msg/sec. At score=50: 88 msg/sec.

use std::num::NonZeroUsize;
use std::collections::HashMap;

use lru::LruCache;

use crate::types::NodeId;

// ── Configuration ──────────────────────────────────────────────────────

/// Maximum envelope size — garde anti-spam appliqué avant parsing.
/// Aligné sur le plafond de segmentation du transport (MAX_REASSEMBLED, 64 Mo) :
/// les gros messages sont chunkés au niveau transport et réassemblés, donc ce
/// garde ne doit plus rejeter à 256 Ko (ce qui cassait la livraison des messages
/// segmentés). Reste une borne dure anti-abus mémoire.
pub const MAX_ENVELOPE_SIZE: usize = 64 * 1024 * 1024;

/// Below this contribution score, a sender is a STRANGER for anti-spam purposes
/// — subject to the shared global stranger cap on top of its per-sender bucket
/// (red-team FINDING #8). Earning this score requires real contribution, which
/// a swarm of fresh forged identities cannot cheaply produce. Mirrors the
/// presence responder two-tier design (FINDING #5): known peers bypass the
/// global cap and cannot be starved by a stranger flood.
pub const STRANGER_SCORE_MAX: f64 = 1.0;

/// Aggregate ceiling (msg/sec) on messages accepted from ALL strangers combined.
/// The per-sender bucket alone is bypassable by identity rotation: N fresh
/// identities each get `min_rate` (30/s), and LRU eviction of their buckets
/// resets them — so aggregate ingress from a swarm is effectively unbounded, an
/// Ed25519-verify CPU DoS that scales with attacker identities. This shared
/// bucket bounds total stranger throughput regardless of identity count.
/// Generous enough that a real network of new peers never hits it; KNOWN peers
/// (score ≥ [`STRANGER_SCORE_MAX`]) are never subject to it.
pub const GLOBAL_STRANGER_RATE: f64 = 200.0;

/// Dedup window for SenderThrottled alerts (ms). At most one alert per peer
/// per window, to avoid spam of dozens/sec when a known peer reconnects and
/// absorbs throttle briefly. After the window, another alert is allowed.
pub const SENDER_THROTTLED_DEDUP_WINDOW: u64 = 10_000; // 10 seconds

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
            min_rate: 30.0,
            max_rate: 100.0,
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
    /// Create a new token bucket for a sender.
    ///
    /// If `is_known` (peer is in topology), grant a higher initial burst capacity
    /// to absorb reconnection traffic without throttle. A newly-reconnected known
    /// peer gets capacity = refill_rate * 4.0 instead of * 2.0, ensuring the
    /// brief burst of catch-up traffic (presence, pings, queued messages) passes.
    /// Strangers (fresh identities) always get the lower burst, bounded by the
    /// global cap.
    fn new(refill_rate: f64, now: u64, is_known: bool) -> Self {
        let capacity = if is_known {
            refill_rate * 4.0  // Reconnection grace: higher burst
        } else {
            refill_rate * 2.0  // Normal burst
        };
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
    /// Shared token bucket bounding aggregate throughput from STRANGERS
    /// (score < STRANGER_SCORE_MAX), so identity rotation can't scale ingress
    /// past a global ceiling (FINDING #8). Known peers never touch it.
    global_stranger: TokenBucket,
    /// Dedup tracker: last SenderThrottled emission time (ms) per peer.
    /// Prevents spam of dozens/sec alerts when a known peer reconnects briefly.
    sender_throttled_last_emit: HashMap<NodeId, u64>,
}

impl AntiSpam {
    pub fn new(config: AntiSpamConfig) -> Self {
        let cap = NonZeroUsize::new(config.max_tracked_senders)
            .expect("max_tracked_senders must be > 0");
        Self {
            config,
            buckets: LruCache::new(cap),
            global_stranger: TokenBucket::new(GLOBAL_STRANGER_RATE, 0, false),
            sender_throttled_last_emit: HashMap::new(),
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
    ///
    /// `is_known`: true if the peer is in topology (recognized from before).
    /// If true, a newly-created bucket for this peer gets higher burst capacity
    /// to absorb reconnection traffic without throttle spam.
    pub fn check_rate(&mut self, sender: NodeId, score: f64, now: u64, is_known: bool) -> Result<(), String> {
        let rate = self.compute_rate(score);

        let bucket = if let Some(b) = self.buckets.get_mut(&sender) {
            b
        } else {
            self.buckets.push(sender, TokenBucket::new(rate, now, is_known));
            self.buckets.get_mut(&sender).unwrap()
        };

        // Update rate if score changed significantly
        if (bucket.refill_rate - rate).abs() > 0.01 {
            bucket.update_rate(rate, now);
        }

        if !bucket.try_consume(now) {
            return Err(format!(
                "Rate limited: score={score:.2}, rate={rate:.1} msg/s",
            ));
        }

        // FINDING #8: a stranger (low score, no real contribution) must also
        // draw from the shared global bucket. This bounds aggregate stranger
        // ingress regardless of how many identities the attacker rotates —
        // per-sender buckets alone are reset by LRU eviction at scale. Known
        // peers (score ≥ STRANGER_SCORE_MAX) skip this and can't be starved.
        if score < STRANGER_SCORE_MAX && !self.global_stranger.try_consume(now) {
            return Err(format!(
                "Aggregate stranger rate limit: global {GLOBAL_STRANGER_RATE:.0} msg/s reached",
            ));
        }

        Ok(())
    }

    /// Check if a SenderThrottled alert should be emitted for this peer.
    ///
    /// Returns true if this is the first throttle for the peer, or if the
    /// dedup window has expired (10s). Prevents spam of dozens of alerts/sec
    /// when a known peer reconnects and briefly exceeds rate.
    pub fn should_emit_sender_throttled(&mut self, sender: NodeId, now: u64) -> bool {
        match self.sender_throttled_last_emit.get(&sender) {
            None => {
                // First throttle for this peer; emit and record
                self.sender_throttled_last_emit.insert(sender, now);
                true
            }
            Some(&last_emit) => {
                if now.saturating_sub(last_emit) >= SENDER_THROTTLED_DEDUP_WINDOW {
                    // Dedup window expired; emit and update
                    self.sender_throttled_last_emit.insert(sender, now);
                    true
                } else {
                    // Within dedup window; suppress
                    false
                }
            }
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
        let mut bucket = TokenBucket::new(5.0, 0, false);
        // Capacity = 10 (2× rate). Should allow 10 immediate messages.
        for i in 0..10 {
            assert!(bucket.try_consume(0), "message {i} should pass");
        }
        assert!(!bucket.try_consume(0), "11th should be throttled");
    }

    #[test]
    fn token_bucket_refills() {
        let mut bucket = TokenBucket::new(5.0, 0, false);
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
        let mut bucket = TokenBucket::new(5.0, 0, false);
        bucket.update_rate(50.0, 0);
        assert!((bucket.refill_rate - 50.0).abs() < f64::EPSILON);
        assert!((bucket.capacity - 100.0).abs() < f64::EPSILON);
    }

    // ── Rate formula ───────────────────────────────────────────────

    #[test]
    fn compute_rate_scales() {
        let antispam = AntiSpam::new(AntiSpamConfig::default());

        let r0 = antispam.compute_rate(0.0);
        assert!((r0 - 30.0).abs() < f64::EPSILON, "score=0 → min_rate, got {r0}");

        let r10 = antispam.compute_rate(10.0);
        assert!((r10 - 65.0).abs() < f64::EPSILON, "score=10 → 65, got {r10}");

        let r100 = antispam.compute_rate(100.0);
        assert!(r100 > 90.0 && r100 <= 100.0, "score=100 → near max, got {r100}");
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
                antispam.check_rate(alice, 50.0, i * 10, false).is_ok(),
                "high-score message {i} should pass"
            );
        }
    }

    #[test]
    fn check_rate_throttles_low_score() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let spammer = test_node_id(2);
        // score=0 → rate=30, burst=60
        let mut allowed = 0u32;
        for _ in 0..80 {
            if antispam.check_rate(spammer, 0.0, 0, false).is_ok() {
                allowed += 1;
            }
        }
        assert_eq!(allowed, 60, "score=0 burst should be 60");
    }

    #[test]
    fn stranger_swarm_bounded_by_global_cap() {
        // FINDING #8: many fresh STRANGER identities, each within its own
        // per-sender burst, must not scale aggregate ingress past the global
        // stranger ceiling. At a frozen instant the global burst is 2×rate.
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let now = 1_000_000;
        let mut allowed = 0u32;
        for id in 0..100u8 {
            let sybil = test_node_id(id);
            for _ in 0..60 {
                if antispam.check_rate(sybil, 0.0, now, false).is_ok() {
                    allowed += 1;
                }
            }
        }
        let global_burst = (GLOBAL_STRANGER_RATE * 2.0) as u32;
        assert_eq!(
            allowed, global_burst,
            "stranger swarm must be capped at the global burst {global_burst}, not scale with identities"
        );
    }

    #[test]
    fn known_sender_not_starved_by_stranger_swarm() {
        // FINDING #8 (mirror of presence #5): a stranger flood that exhausts
        // the global bucket must NOT deny service to a KNOWN, high-score peer.
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let now = 1_000_000;
        for id in 0..100u8 {
            let sybil = test_node_id(id);
            for _ in 0..60 {
                let _ = antispam.check_rate(sybil, 0.0, now, false);
            }
        }
        // Global stranger bucket is now empty; a known peer still passes.
        let known = test_node_id(200);
        assert!(
            antispam.check_rate(known, 50.0, now, false).is_ok(),
            "known peer (score ≥ threshold) must bypass the global stranger cap"
        );
    }

    #[test]
    fn check_rate_updates_on_score_change() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let alice = test_node_id(1);

        // Start with low score
        antispam.check_rate(alice, 0.0, 0, false).ok();
        let rate_low = antispam.buckets.peek(&alice).unwrap().refill_rate;

        // Score improves
        antispam.check_rate(alice, 30.0, 1000, false).ok();
        let rate_high = antispam.buckets.peek(&alice).unwrap().refill_rate;

        assert!(rate_high > rate_low, "rate should increase: {rate_low} → {rate_high}");
    }

    #[test]
    fn validate_size_rejects_oversized() {
        let max = MAX_ENVELOPE_SIZE;
        // Sous le plafond (les messages segmentés vont jusqu'à 64 Mo) : accepté.
        let ok = vec![0u8; 1024];
        assert!(AntiSpam::validate_size(&ok, max).is_ok());
        // Au-dessus du plafond anti-abus : rejeté.
        let huge = vec![0u8; max + 1];
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
            antispam.check_rate(*id, 5.0, 0, false).ok();
        }
        assert_eq!(antispam.tracked_senders(), 3);

        // 4th sender evicts oldest
        antispam.check_rate(ids[3], 5.0, 0, false).ok();
        assert_eq!(antispam.tracked_senders(), 3);
        assert!(antispam.buckets.peek(&ids[0]).is_none(), "oldest evicted");
    }

    // ── Grace for known peers ──────────────────────────────────────

    #[test]
    fn known_peer_gets_higher_burst() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let known = test_node_id(10);
        let stranger = test_node_id(11);

        // Score 0 → rate=30, normal burst=60, grace burst=120
        let now = 0;

        // Known peer: should allow ~120 messages (4x rate)
        let mut known_allowed = 0;
        for _ in 0..120 {
            if antispam.check_rate(known, 0.0, now, true).is_ok() {
                known_allowed += 1;
            } else {
                break;
            }
        }

        // Stranger: should allow ~60 messages (2x rate)
        let mut stranger_allowed = 0;
        for _ in 0..120 {
            if antispam.check_rate(stranger, 0.0, now, false).is_ok() {
                stranger_allowed += 1;
            } else {
                break;
            }
        }

        assert!(
            known_allowed > stranger_allowed,
            "known peer burst {known_allowed} should be > stranger burst {stranger_allowed}"
        );
        assert!(
            known_allowed >= 100,
            "known peer should allow >=100 messages, got {known_allowed}"
        );
        assert_eq!(
            stranger_allowed, 60,
            "stranger should allow 60 (2x rate of 30), got {stranger_allowed}"
        );
    }

    #[test]
    fn known_peer_still_rate_limited() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let known = test_node_id(12);
        let now = 0;

        // Burst should allow ~120 (4x), but after that still throttle
        let mut allowed = 0;
        for _ in 0..150 {
            if antispam.check_rate(known, 0.0, now, true).is_ok() {
                allowed += 1;
            }
        }

        // Should NOT be allowed all 150 — after initial burst, rate-limited
        assert!(
            allowed < 150,
            "known peer should still be rate-limited, allowed {allowed}/150"
        );
    }

    #[test]
    fn sybil_identities_dont_benefit_from_grace() {
        // Multiple fresh strangers each get normal burst,
        // but aggregate is still capped by GLOBAL_STRANGER_RATE
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let now = 1_000_000;
        let mut allowed = 0;

        // Sybil swarm: 100 identities, each marked as NOT known (is_known=false)
        for id in 0..100u8 {
            let sybil = test_node_id(id);
            for _ in 0..60 {
                if antispam.check_rate(sybil, 0.0, now, false).is_ok() {
                    allowed += 1;
                }
            }
        }

        let global_burst = (GLOBAL_STRANGER_RATE * 2.0) as u32;
        assert_eq!(
            allowed, global_burst,
            "sybil swarm (all is_known=false) must still be capped at global burst {global_burst}"
        );
    }

    #[test]
    fn sender_throttled_dedup() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let peer = test_node_id(20);
        let now = 0u64;

        // First throttle → should emit
        assert!(
            antispam.should_emit_sender_throttled(peer, now),
            "first throttle should emit"
        );

        // Immediate second throttle (same timestamp) → should NOT emit
        assert!(
            !antispam.should_emit_sender_throttled(peer, now),
            "second throttle within window should NOT emit"
        );

        // After dedup window (10s) → should emit again
        let later = now + SENDER_THROTTLED_DEDUP_WINDOW;
        assert!(
            antispam.should_emit_sender_throttled(peer, later),
            "throttle after dedup window should emit"
        );

        // Within new window → suppress again
        assert!(
            !antispam.should_emit_sender_throttled(peer, later + 1000),
            "second throttle in new window should NOT emit"
        );
    }

    #[test]
    fn sender_throttled_dedup_multiple_peers() {
        let mut antispam = AntiSpam::new(AntiSpamConfig::default());
        let peer1 = test_node_id(21);
        let peer2 = test_node_id(22);
        let now = 0u64;

        // peer1: first throttle → emit
        assert!(antispam.should_emit_sender_throttled(peer1, now));
        // peer2: first throttle → separate, should emit
        assert!(antispam.should_emit_sender_throttled(peer2, now));
        // peer1: second → suppress
        assert!(!antispam.should_emit_sender_throttled(peer1, now));
        // peer2: second → suppress independently
        assert!(!antispam.should_emit_sender_throttled(peer2, now));
    }
}
