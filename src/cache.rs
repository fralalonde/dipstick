//! Cache metric definitions.

use core::*;
use core::Scope::*;

use std::sync::{Arc, RwLock};
use lru_cache::LRUCache;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn cache<M, S>(size: usize, sink: S) -> MetricCache<M, S>
where
    S: Sink<M>,
    M: Clone + Send + Sync,
{
    let cache = RwLock::new(LRUCache::with_capacity(size));
    MetricCache {
        next_sink: sink,
        cache,
    }
}

/// A cache to help with ad-hoc defined metrics
/// Does not alter the values of the metrics
#[derive(Derivative)]
#[derivative(Debug)]
pub struct MetricCache<M, S> {
    next_sink: S,
    #[derivative(Debug = "ignore")]
    cache: RwLock<LRUCache<String, M>>,
}

impl<M, S> Sink<M> for MetricCache<M, S>
where
    S: Sink<M>,
    M: 'static + Clone + Send + Sync,
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        let mut cache = self.cache.write().expect("Failed to acquire metric cache");
        let name_str = String::from(name);

        // FIXME lookup should use straight &str
        if let Some(value) = cache.get(&name_str) {
            return value.clone()
        }
            let new_value = self.next_sink.new_metric(kind, name, sampling).clone();
            cache.insert(name_str, new_value.clone());
            new_value

    }

    fn new_scope(&self, auto_flush: bool) -> ScopeFn<M> {
        let next_scope = self.next_sink.new_scope(auto_flush);
        Arc::new(move |cmd| match cmd {
            Write(metric, value) => next_scope(Write(metric, value)),
            Flush => next_scope(Flush),
        })
    }
}
