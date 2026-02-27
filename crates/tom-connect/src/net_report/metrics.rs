use serde::{Deserialize, Serialize};
use tom_metrics::Counter;

/// Metrics for the net_report module
#[derive(Debug, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Metrics {
    /// Number of reports executed by net_report, including full reports.
    pub reports: Counter,
    /// Number of full reports executed by net_report
    pub reports_full: Counter,
}
