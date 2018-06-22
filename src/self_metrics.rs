//! Internal Dipstick runtime metrics.
//! Because the possibly high volume of data, this is pre-set to use aggregation.
//! This is also kept in a separate module because it is not to be exposed outside of the crate.

use proxy::ProxyInput;

metrics!{
    /// Dipstick's own internal metrics.
    pub DIPSTICK_METRICS = "dipstick";
}
