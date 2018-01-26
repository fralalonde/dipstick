//! Internal Dipstick metrics.
//! Collect statistics about various metrics modules at runtime.
//! Stats can can be obtained for publication from `selfstats::SOURCE`.

pub use core::*;

pub use app_metrics::*;
pub use aggregate::*;
pub use publish::*;
pub use scores::*;
pub use namespace::*;

use output::to_void;

lazy_static! {
    static ref DIPSTICK_AGGREGATOR: Aggregator = build_aggregator();
}

/// Application metrics are collected to the aggregator

app_metrics!(Aggregate, DIPSTICK_METRICS = build_self_metrics());

fn build_aggregator() -> Aggregator {
    // TODO make publishable
    aggregate(summary, to_void())
}

/// Capture a snapshot of Dipstick's internal metrics since the last snapshot.
pub fn snapshot() -> Vec<ScoreSnapshot> {
    vec![]
}

fn build_self_metrics() -> AppMetrics<Aggregate> {
    let mug: &Aggregator = &DIPSTICK_AGGREGATOR;
    let am: AppMetrics<Aggregate> = mug.clone().into();
    am.with_prefix("dipstick")
}

