//! Cache defined metrics.

// TODO one cache per metric kind (why?)

use core::*;
use cached::{SizedCache, Cached};
use std::sync::{Arc,RwLock};

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn cache<M, S>(size: usize, sink: S) -> MetricCache<M, S>
    where S: Sink<M>, M: Send + Sync
{
    let cache = RwLock::new(SizedCache::with_capacity(size));
    MetricCache { next_sink: sink, cache }
}

/// The cache key copies the target key.
pub type CachedKey<M> = Arc<M>;

/// A cache to help with ad-hoc defined metrics
/// Does not alter the values of the metrics
pub struct MetricCache<M, S> {
    next_sink: S,
    cache: RwLock<SizedCache<String, CachedKey<M>>>,
}

impl<M, S> Sink<Arc<M>> for MetricCache<M, S>
    where S: Sink<M>, M: 'static + Send + Sync
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Arc<M> {
        // TODO use ref for key, not owned
        let key = name.to_string();
        {
            let mut cache = self.cache.write().unwrap();
            let cached_metric = cache.cache_get(&key);
            if let Some(cached_metric) = cached_metric {
                return cached_metric.clone();
            }
        }

        let target_metric = self.next_sink.new_metric(kind, name, sampling);
        let new_metric = Arc::new(target_metric);
        let mut cache = self.cache.write().unwrap();
        cache.cache_set(key, new_metric.clone());
        new_metric
    }

    fn new_scope(&self) -> ScopeFn<Arc<M>> {
        let next_scope = self.next_sink.new_scope();
        Arc::new(move |cmd| match cmd {
            Some((metric, value)) => next_scope(Some((metric, value))),
            None => next_scope(None)
        })
    }
}
