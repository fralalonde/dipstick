//! Internal Dipstick metrics.
//! Collect statistics about various metrics modules at runtime.
//! Stats can can be obtained for publication from `selfstats::SOURCE`.

pub use app::*;
pub use aggregate::*;

lazy_static! {

    /// Central metric storage
    static ref AGGREGATE: (AggregateSink, AggregateSource) = aggregate();

    /// Source of dipstick inner metrics, for eventual publication.
    pub static ref SOURCE: AggregateSource = AGGREGATE.1.clone();

    /// Application metrics are collected to the aggregator
    pub static ref SELF_METRICS: AppMetrics<Aggregate, AggregateSink> =
            metrics(AGGREGATE.0.clone()).with_prefix("dipstick.");

}
