//! Metric input scope caching.

use crate::Flush;
use crate::attributes::{Attributes, OnFlush, Prefixed, WithAttributes};
use crate::input::{Input, InputDyn, InputKind, InputMetric, InputScope};
use crate::lru_cache as lru;
use crate::name::MetricName;

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;
use std::io;

/// Wrap an input with a metric definition cache.
/// This can provide performance benefits for metrics that are dynamically defined at runtime on each access.
/// Caching is useless if all metrics are statically declared
/// or instantiated programmatically in advance and referenced by a long living variable.
pub trait CachedInput: Input + Send + Sync + 'static + Sized {
    /// Wrap an input with a metric definition cache.
    /// This can provide performance benefits for metrics that are dynamically defined at runtime on each access.
    /// Caching is useless if all metrics are statically declared
    /// or instantiated programmatically in advance and referenced by a long living variable.
    fn cached(self, max_size: usize) -> InputCache {
        InputCache::wrap(self, max_size)
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct InputCache {
    attributes: Attributes,
    target: Arc<dyn InputDyn + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, InputMetric>>>,
}

impl InputCache {
    /// Wrap scopes with an asynchronous metric write & flush dispatcher.
    fn wrap<OUT: Input + Send + Sync + 'static>(target: OUT, max_size: usize) -> InputCache {
        InputCache {
            attributes: Attributes::default(),
            target: Arc::new(target),
            cache: Arc::new(RwLock::new(lru::LRUCache::with_capacity(max_size))),
        }
    }
}

impl WithAttributes for InputCache {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Input for InputCache {
    type SCOPE = InputScopeCache;

    fn metrics(&self) -> Self::SCOPE {
        let target = self.target.input_dyn();
        InputScopeCache {
            attributes: self.attributes.clone(),
            target,
            cache: self.cache.clone(),
        }
    }
}

/// Input wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct InputScopeCache {
    attributes: Attributes,
    target: Arc<dyn InputScope + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, InputMetric>>>,
}

impl WithAttributes for InputScopeCache {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl InputScope for InputScopeCache {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        let lookup = { write_lock!(self.cache).get(&name).cloned() };
        lookup.unwrap_or_else(|| {
            let new_metric = self.target.new_metric(name.clone(), kind);
            // FIXME (perf) having to take another write lock for a cache miss
            write_lock!(self.cache).insert(name, new_metric.clone());
            new_metric
        })
    }
}

impl Flush for InputScopeCache {
    fn flush(&self) -> io::Result<()> {
        self.notify_flush_listeners();
        self.target.flush()
    }
}
