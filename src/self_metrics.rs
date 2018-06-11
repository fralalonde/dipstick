//! Internal Dipstick runtime metrics.
//! Because the possibly high volume of data, this is pre-set to use aggregation.
//! This is also kept in a separate module because it is not to be exposed outside of the crate.

use dispatch::MetricDispatch;
use aggregate::MetricAggregator;

//lazy_static! {
//    pub static ref DIPSTICK_METRICS: MetricAggregator = "dipstick".into();
//}

metrics!{
    /// Aggregator of dipstick's own internal metrics.
    <MetricAggregator> pub DIPSTICK_METRICS = "dipstick";
}
