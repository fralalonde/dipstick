//! Internal Dipstick metrics.
//! Collect statistics about various metrics modules at runtime.
//! Stats can can be obtained for publication from `selfstats::SOURCE`.

pub use app_metrics::*;
pub use aggregate::*;
pub use publish::*;
pub use scores::*;
pub use core::*;
pub use namespace::*;

use output::to_void;

// TODO send to_dispatch()
fn build_aggregator() -> Chain<Aggregate> {
    aggregate(summary, to_void())
}

/// Capture a snapshot of Dipstick's internal metrics since the last snapshot.
pub fn snapshot() -> Vec<ScoreSnapshot> {
    vec![]
}

fn build_self_metrics() -> AppMetrics<Aggregate> {
    app_metrics(AGGREGATOR.clone()).with_prefix("dipstick")
}

lazy_static! { static ref AGGREGATOR: Chain<Aggregate> = build_aggregator(); }

/// Application metrics are collected to the aggregator
app_metric!(Aggregate, DIPSTICK_METRICS = build_self_metrics());
