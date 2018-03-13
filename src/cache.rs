//! Cache metric definitions.

use core::*;
use std::sync::{Arc, RwLock};
use lru_cache::LRUCache;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub trait WithCache
where
    Self: Sized,
{
    /// Cache metrics to prevent them from being re-defined on every use.
    fn with_cache(&self, cache_size: usize) -> Self;
}

// TODO add selfmetrics cache stats

/// Add a caching decorator to a metric definition function.
pub fn add_cache<M>(cache_size: usize, next: DefineMetricFn<M>) -> DefineMetricFn<M>
where
    M: Clone + Send + Sync + 'static,
{
    let cache: RwLock<LRUCache<String, M>> = RwLock::new(LRUCache::with_capacity(cache_size));
    Arc::new(move |kind, name, rate| {
        let mut cache = cache.write().expect("Locking metric cache");
        let name_str = String::from(name);

        // FIXME lookup should use straight &str
        if let Some(value) = cache.get(&name_str) {
            return value.clone();
        }

        let new_value = (next)(kind, name, rate).clone();
        cache.insert(name_str, new_value.clone());
        new_value
    })
}
