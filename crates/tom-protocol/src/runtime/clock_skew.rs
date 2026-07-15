use std::collections::VecDeque;

/// Clock skew metrics — records the median difference between local time
/// and peer-sent timestamps (sampled from verified envelope signatures only).
/// Includes network latency in the signal (no NTP correction).
#[derive(Debug, Default, Clone)]
pub struct ClockSkewMetrics {
    /// Bounded sample window (max 64 entries).
    samples: VecDeque<i64>,
}

impl ClockSkewMetrics {
    const SKEW_WINDOW: usize = 64;
    const MAX_PLAUSIBLE_SKEW_MS: i64 = 86_400_000; // 24h
    const MIN_SAMPLES: usize = 5;

    /// Record a clock skew sample (now_ms - envelope.timestamp).
    /// Samples with |skew_ms| > 24h are silently dropped (outliers / attack probes).
    pub fn record(&mut self, skew_ms: i64) {
        if skew_ms.abs() <= Self::MAX_PLAUSIBLE_SKEW_MS {
            self.samples.push_back(skew_ms);
            if self.samples.len() > Self::SKEW_WINDOW {
                self.samples.pop_front();
            }
        }
    }

    /// Median clock skew in milliseconds.
    /// Returns None if fewer than 5 samples collected.
    /// For even-length samples, returns the lower-middle element.
    pub fn median_ms(&self) -> Option<i64> {
        if self.samples.len() < Self::MIN_SAMPLES {
            return None;
        }

        let mut sorted: Vec<i64> = self.samples.iter().copied().collect();
        sorted.sort_unstable();
        Some(sorted[sorted.len() / 2])
    }

    /// Minimum observed skew.
    #[allow(dead_code)]
    pub fn min_ms(&self) -> Option<i64> {
        self.samples.iter().copied().min()
    }

    /// Maximum observed skew.
    #[allow(dead_code)]
    pub fn max_ms(&self) -> Option<i64> {
        self.samples.iter().copied().max()
    }

    /// Number of samples in the window.
    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_correct_on_known_set() {
        let mut m = ClockSkewMetrics::default();
        m.record(100);
        m.record(200);
        m.record(150);
        m.record(50);
        m.record(300);
        // sorted: [50, 100, 150, 200, 300] → median = 150
        assert_eq!(m.median_ms(), Some(150));
    }

    #[test]
    fn bounds_window_at_64() {
        let mut m = ClockSkewMetrics::default();
        for i in 0..65 {
            m.record(i as i64);
        }
        // Should have evicted the first sample (0)
        assert_eq!(m.sample_count(), 64);
        assert_eq!(m.min_ms(), Some(1));
    }

    #[test]
    fn ignores_outlier_over_24h() {
        let mut m = ClockSkewMetrics::default();
        m.record(100);
        m.record(200);
        m.record(150);
        m.record(50);
        m.record(300);
        m.record(100_000_000); // Way over 24h, dropped
        assert_eq!(m.sample_count(), 5);
    }

    #[test]
    fn returns_none_under_min_samples() {
        let mut m = ClockSkewMetrics::default();
        m.record(100);
        m.record(200);
        m.record(150);
        m.record(50);
        // Only 4 samples, need 5
        assert_eq!(m.median_ms(), None);
        m.record(300);
        // Now 5, should return Some
        assert!(m.median_ms().is_some());
    }

    #[test]
    fn min_max_work() {
        let mut m = ClockSkewMetrics::default();
        m.record(-500);
        m.record(100);
        m.record(2000);
        assert_eq!(m.min_ms(), Some(-500));
        assert_eq!(m.max_ms(), Some(2000));
    }
}
