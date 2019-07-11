//! Metric output scope caching.

use crate::cache::lru_cache as lru;
use crate::core::attributes::{Attributes, OnFlush, Prefixed, WithAttributes};
use crate::core::error;
use crate::core::input::InputKind;
use crate::core::name::MetricName;
use crate::core::output::{Output, OutputDyn, OutputMetric, OutputScope};
use crate::core::Flush;

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::RwLock;

#[cfg(feature = "parking_lot")]
use parking_lot::RwLock;

use std::rc::Rc;

/// Wrap an output with a metric definition cache.
/// This can provide performance benefits for metrics that are dynamically defined at runtime on each access.
/// Caching is useless if all metrics are statically declared
/// or instantiated programmatically in advance and referenced by a long living variable.
pub trait CachedOutput: Output + Send + Sync + 'static + Sized {
    /// Wrap an output with a metric definition cache.
    /// This can provide performance benefits for metrics that are dynamically defined at runtime on each access.
    /// Caching is useless if all metrics are statically declared
    /// or instantiated programmatically in advance and referenced by a long living variable.
    fn cached(self, max_size: usize) -> OutputCache {
        OutputCache::wrap(self, max_size)
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct OutputCache {
    attributes: Attributes,
    target: Arc<dyn OutputDyn + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, OutputMetric>>>,
}

impl OutputCache {
    /// Wrap scopes with an asynchronous metric write & flush dispatcher.
    fn wrap<OUT: Output + Send + Sync + 'static>(target: OUT, max_size: usize) -> OutputCache {
        OutputCache {
            attributes: Attributes::default(),
            target: Arc::new(target),
            cache: Arc::new(RwLock::new(lru::LRUCache::with_capacity(max_size))),
        }
    }
}

impl WithAttributes for OutputCache {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Output for OutputCache {
    type SCOPE = OutputScopeCache;

    fn new_scope(&self) -> Self::SCOPE {
        let target = self.target.output_dyn();
        OutputScopeCache {
            attributes: self.attributes.clone(),
            target,
            cache: self.cache.clone(),
        }
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct OutputScopeCache {
    attributes: Attributes,
    target: Rc<dyn OutputScope + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, OutputMetric>>>,
}

impl WithAttributes for OutputScopeCache {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl OutputScope for OutputScopeCache {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
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

impl Flush for OutputScopeCache {
    fn flush(&self) -> error::Result<()> {
        self.notify_flush_listeners();
        self.target.flush()
    }
}
