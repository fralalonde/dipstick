//! Metric input scope caching.

use core::Flush;
use core::input::{InputKind, Input, InputScope, InputMetric, InputDyn};
use core::attributes::{Attributes, WithAttributes, Prefixed};
use core::name::MetricName;
use cache::lru_cache as lru;
use core::error;

use std::sync::{Arc, RwLock};

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
    target: Arc<InputDyn + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, InputMetric>>>,
}

impl InputCache {
    /// Wrap scopes with an asynchronous metric write & flush dispatcher.
    fn wrap<OUT: Input + Send + Sync + 'static>(target: OUT, max_size: usize) -> InputCache {
        InputCache {
            attributes: Attributes::default(),
            target: Arc::new(target),
            cache: Arc::new(RwLock::new(lru::LRUCache::with_capacity(max_size)))
        }
    }
}

impl WithAttributes for InputCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
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
    target: Arc<InputScope + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, InputMetric>>>,
}

impl WithAttributes for InputScopeCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl InputScope for InputScopeCache {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let name = self.prefix_append(name);
        let lookup = {
            self.cache.write().expect("Cache Lock").get(&name).cloned()
        };
        lookup.unwrap_or_else(|| {
            let new_metric = self.target.new_metric(name.clone(), kind);
            // FIXME (perf) having to take another write lock for a cache miss
            let mut cache_miss = self.cache.write().expect("Cache Lock");
            cache_miss.insert(name, new_metric.clone());
            new_metric
        })
    }
}

impl Flush for InputScopeCache {

    fn flush(&self) -> error::Result<()> {
        self.target.flush()
    }
}
