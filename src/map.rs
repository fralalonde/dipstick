use core::{Value, WriteFn, Kind, Namespace, MetricInput, Flush};
use std::sync::{Arc, RwLock};
use std::collections::BTreeMap;

/// A HashMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone)]
pub struct StatsMap {
    inner: Arc<RwLock<BTreeMap<String, Value>>>,
}

impl StatsMap {
    /// Create a new StatsMap.
    pub fn new() -> Self {
        StatsMap { inner: Arc::new(RwLock::new(BTreeMap::new())) }
    }
}

impl MetricInput for StatsMap {
    fn define_metric(&self, name: &Namespace, _kind: Kind) -> WriteFn {
        let write_to = self.inner.clone();
        let name: String = name.join(".");
        WriteFn::new(move |value| {
            let _previous = write_to.write().expect("StatsMap").insert(name.clone(), value);
        })
    }
}

impl Flush for StatsMap {}

impl From<StatsMap> for BTreeMap<String, Value> {
    fn from(map: StatsMap) -> Self {
        let z = Arc::try_unwrap(map.inner).expect("StatsMap");
        z.into_inner().expect("StatsMap")
    }
}