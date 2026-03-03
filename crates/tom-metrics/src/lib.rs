//! Minimal metrics primitives for the ToM protocol stack.
//!
//! Provides [`Counter`] — an atomic monotonic counter compatible with
//! serde serialization (postcard, JSON, etc.).

use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// A monotonically increasing counter backed by [`AtomicU64`].
///
/// All operations use [`Ordering::Relaxed`] — suitable for statistics
/// where exact inter-thread ordering is not required.
pub struct Counter(AtomicU64);

impl Counter {
    /// Create a counter starting at zero.
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Increment by one.
    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment by `n`.
    pub fn inc_by(&self, n: u64) {
        self.0.fetch_add(n, Ordering::Relaxed);
    }

    /// Read the current value.
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Counter {
    fn clone(&self) -> Self {
        let c = Self::new();
        c.inc_by(self.get());
        c
    }
}

impl fmt::Debug for Counter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Counter").field(&self.get()).finish()
    }
}

impl serde::Serialize for Counter {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Counter {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        let counter = Self::new();
        counter.inc_by(value);
        Ok(counter)
    }
}

/// An atomic gauge that can go up or down (e.g., active connections).
pub struct Gauge(AtomicU64);

impl Gauge {
    /// Create a gauge starting at zero.
    pub fn new() -> Self {
        Self(AtomicU64::new(0))
    }

    /// Set the gauge to an absolute value.
    pub fn set(&self, value: u64) {
        self.0.store(value, Ordering::Relaxed);
    }

    /// Increment by one.
    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement by one (saturating).
    pub fn dec(&self) {
        self.0.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
            Some(v.saturating_sub(1))
        })
        .ok();
    }

    /// Read the current value.
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for Gauge {
    fn clone(&self) -> Self {
        let g = Self::new();
        g.set(self.get());
        g
    }
}

impl fmt::Debug for Gauge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Gauge").field(&self.get()).finish()
    }
}

impl serde::Serialize for Gauge {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Gauge {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u64::deserialize(deserializer)?;
        let gauge = Self::new();
        gauge.set(value);
        Ok(gauge)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_operations() {
        let c = Counter::new();
        assert_eq!(c.get(), 0);
        c.inc();
        assert_eq!(c.get(), 1);
        c.inc_by(10);
        assert_eq!(c.get(), 11);
    }

    #[test]
    fn default_is_zero() {
        let c = Counter::default();
        assert_eq!(c.get(), 0);
    }

    #[test]
    fn clone_preserves_value() {
        let c = Counter::new();
        c.inc_by(42);
        let c2 = c.clone();
        assert_eq!(c2.get(), 42);
        // Independent after clone
        c.inc();
        assert_eq!(c.get(), 43);
        assert_eq!(c2.get(), 42);
    }

    #[test]
    fn serde_roundtrip() {
        let c = Counter::new();
        c.inc_by(99);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "99");
        let c2: Counter = serde_json::from_str(&json).unwrap();
        assert_eq!(c2.get(), 99);
    }

    // ── Gauge tests ──────────────────────────────────────────────────

    #[test]
    fn gauge_basic_operations() {
        let g = Gauge::new();
        assert_eq!(g.get(), 0);
        g.set(42);
        assert_eq!(g.get(), 42);
        g.inc();
        assert_eq!(g.get(), 43);
        g.dec();
        assert_eq!(g.get(), 42);
    }

    #[test]
    fn gauge_dec_saturates() {
        let g = Gauge::new();
        g.dec(); // 0 - 1 should saturate to 0
        assert_eq!(g.get(), 0);
    }

    #[test]
    fn gauge_serde_roundtrip() {
        let g = Gauge::new();
        g.set(77);
        let json = serde_json::to_string(&g).unwrap();
        assert_eq!(json, "77");
        let g2: Gauge = serde_json::from_str(&json).unwrap();
        assert_eq!(g2.get(), 77);
    }
}
