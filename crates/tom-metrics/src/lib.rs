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
}
