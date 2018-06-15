//! Internal Dipstick runtime metrics.
//! Because the possibly high volume of data, this is pre-set to use aggregation.
//! This is also kept in a separate module because it is not to be exposed outside of the crate.

pub use bucket::Bucket;

//lazy_static! {
//    pub static ref DIPSTICK_METRICS: Bucket = "dipstick".into();
//}

metrics!{
    /// Aggregator of dipstick's own internal metrics.
    <Bucket> pub DIPSTICK_METRICS = "dipstick";
}
