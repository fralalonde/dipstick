//! Internal Dipstick metrics.
//! Collect statistics about various metrics modules at runtime.
//! Stats can can be obtained for publication from `selfstats::SOURCE`.

pub use global_metrics::*;
pub use aggregate::*;
pub use publish::*;
pub use scores::*;
pub use core::*;
use output::to_void;

fn build_aggregator() -> Chain<Aggregate> {
    aggregate(32, summary, to_void())
}

/// Capture a snapshot of Dipstick's internal metrics since the last snapshot.
///
pub fn snapshot() -> Vec<ScoreSnapshot> {
    vec![]
}

fn build_self_metrics() -> GlobalMetrics<Aggregate> {
    // TODO send to_map() when snapshot() is called
//    let agg = aggregate(summary, to_void());
    global_metrics(AGGREGATOR.clone()).with_prefix("dipstick.")
}

lazy_static! {

    static ref AGGREGATOR: Chain<Aggregate> = build_aggregator();

    /// Application metrics are collected to the aggregator
    pub static ref SELF_METRICS: GlobalMetrics<Aggregate> = build_self_metrics();

}
