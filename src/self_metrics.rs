//! Internal Dipstick runtime metrics.
//! Because the possibly high volume of data, this is pre-set to use aggregation.
//! This is also kept in a separate module because it is not to be exposed outside of the crate.

pub use core::*;

pub use input::*;
pub use aggregate::*;

metrics!(
    /// Aggregator of dipstick's own internal metrics.
    <Aggregate> pub DIPSTICK_METRICS = "dipstick"
);
