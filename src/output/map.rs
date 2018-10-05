use core::{Flush, Value};
use core::input::Kind;
use core::name::Name;
use core::output::{OutputMetric, OutputScope};

use std::rc::Rc;
use std::cell::RefCell;
use std::collections::BTreeMap;

/// A HashMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone, Default)]
pub struct StatsMap {
    inner: Rc<RefCell<BTreeMap<String, Value>>>,
}

impl OutputScope for StatsMap {
    fn new_metric(&self, name: Name, _kind: Kind) -> OutputMetric {
        let write_to = self.inner.clone();
        let name: String = name.join(".");
        OutputMetric::new(move |value| {
            let _previous = write_to.borrow_mut().insert(name.clone(), value);
        })
    }
}

impl Flush for StatsMap {}

impl From<StatsMap> for BTreeMap<String, Value> {
    fn from(map: StatsMap) -> Self {
        // FIXME this is is possibly a full map copy, for nothing.
        // into_inner() is what we'd really want here but would require some `unsafe`? don't know how to do this yet.
        map.inner.borrow().clone()
    }
}
