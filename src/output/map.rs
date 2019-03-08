use core::{Flush, MetricValue};
use core::input::InputKind;
use core::name::MetricName;
use core::output::{OutputMetric, OutputScope};

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;

/// A HashMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone, Default)]
pub struct StatsMap {
    inner: Rc<RefCell<BTreeMap<String, MetricValue>>>,
}

impl OutputScope for StatsMap {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> OutputMetric {
        let write_to = self.inner.clone();
        let name: String = name.join(".");
        OutputMetric::new(move |value, _labels| {
            let _previous = write_to.borrow_mut().insert(name.clone(), value);
        })
    }
}

impl Flush for StatsMap {
    fn flush(&self) -> Result<(), Box<Error + Send + Sync>> {
        Ok(())
    }
}

impl From<StatsMap> for BTreeMap<String, MetricValue> {
    fn from(map: StatsMap) -> Self {
        // FIXME this is is possibly a full map copy, for nothing.
        // into_inner() is what we'd really want here but would require some `unsafe`? don't know how to do this yet.
        map.inner.borrow().clone()
    }
}
