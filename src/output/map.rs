use crate::core::attributes::{Attributes, MetricId, OnFlush, Prefixed, WithAttributes};
use crate::core::input::InputKind;
use crate::core::input::{Input, InputMetric, InputScope};
use crate::core::name::MetricName;
use crate::core::{Flush, MetricValue};

use std::collections::BTreeMap;
use std::error::Error;

use crate::{OutputMetric, OutputScope};
use std::sync::{Arc, RwLock};

/// A BTreeMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone, Default)]
pub struct StatsMap {
    attributes: Attributes,
}

impl WithAttributes for StatsMap {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Input for StatsMap {
    type SCOPE = StatsMapScope;

    fn metrics(&self) -> Self::SCOPE {
        StatsMapScope {
            attributes: self.attributes.clone(),
            inner: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }
}

/// A BTreeMap wrapper to receive metrics or stats values.
/// Every received value for a metric replaces the previous one (if any).
#[derive(Clone, Default)]
pub struct StatsMapScope {
    attributes: Attributes,
    inner: Arc<RwLock<BTreeMap<String, MetricValue>>>,
}

impl WithAttributes for StatsMapScope {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl InputScope for StatsMapScope {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        let write_to = self.inner.clone();
        let key: String = name.join(".");
        InputMetric::new(MetricId::forge("map", name), move |value, _labels| {
            let _previous = write_to.write().expect("Lock").insert(key.clone(), value);
        })
    }
}

impl OutputScope for StatsMapScope {
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> OutputMetric {
        let name = self.prefix_append(name);
        let write_to = self.inner.clone();
        let name: String = name.join(".");
        OutputMetric::new(move |value, _labels| {
            let _previous = write_to.write().expect("Lock").insert(name.clone(), value);
        })
    }
}

impl Flush for StatsMapScope {
    fn flush(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.notify_flush_listeners();
        Ok(())
    }
}

impl From<StatsMapScope> for BTreeMap<String, MetricValue> {
    fn from(map: StatsMapScope) -> Self {
        // FIXME this is is possibly a full map copy, for no reason.
        // into_inner() is what we'd really want here but would require some `unsafe`? don't know how to do this yet.
        map.inner.read().unwrap().clone()
    }
}

impl StatsMapScope {
    /// Extract the backing BTreeMap.
    pub fn into_map(self) -> BTreeMap<String, MetricValue> {
        self.into()
    }
}
