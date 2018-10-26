//! Cache metric definitions.

use core::Flush;
use core::attributes::{Attributes, WithAttributes, Prefixed};
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope, OutputDyn};
use core::input::InputKind;
use cache::lru_cache as lru;
use core::error;

use std::sync::{Arc, RwLock};
use std::rc::Rc;

/// Wrap an output with a metric definition cache.
/// This is useless if all metrics are statically declared but can provide performance
/// benefits if some metrics are dynamically defined at runtime.
pub trait CachedOutput: Output + Send + Sync + 'static + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn cached(self, max_size: usize) -> OutputCache {
        OutputCache::wrap(self, max_size)
    }
}

/// Output wrapper caching frequently defined metrics
#[derive(Clone)]
pub struct OutputCache {
    attributes: Attributes,
    target: Arc<OutputDyn + Send + Sync + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, OutputMetric>>>,
}

impl OutputCache {
    /// Wrap scopes with an asynchronous metric write & flush dispatcher.
    pub fn wrap<OUT: Output + Send + Sync + 'static>(target: OUT, max_size: usize) -> OutputCache {
        OutputCache {
            attributes: Attributes::default(),
            target: Arc::new(target),
            cache: Arc::new(RwLock::new(lru::LRUCache::with_capacity(max_size)))
        }
    }
}

impl WithAttributes for OutputCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Output for OutputCache {
    type SCOPE = OutputScopeCache;

    fn output(&self) -> Self::SCOPE {
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
    target: Rc<OutputScope + 'static>,
    cache: Arc<RwLock<lru::LRUCache<MetricName, OutputMetric>>>,
}

impl WithAttributes for OutputScopeCache {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl OutputScope for OutputScopeCache {
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let name = self.prefix_append(name);
        let lookup = {
            self.cache.write().expect("Cache Lock").get(&name).cloned()
        };
        lookup.unwrap_or_else(|| {
            let new_metric: OutputMetric = self.target.new_metric(name.clone(), kind);
            // FIXME (perf) having to take another write lock for a cache miss
            let mut cache_miss = self.cache.write().expect("Cache Lock");
            cache_miss.insert(name, new_metric.clone());
            new_metric
        })
    }
}

impl Flush for OutputScopeCache {

    fn flush(&self) -> error::Result<()> {
        self.target.flush()
    }
}

