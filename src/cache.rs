//! Cache defined metrics.

// TODO one cache per metric kind (why?)

use core::*;
use cached::{Cached as CCC, self};
use std::sync::{Arc,RwLock};

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn cache<M, S>(size: usize, sink: S) -> MetricCache<M, S>
    where S: Sink<M>,
          M: Clone + Send + Sync
{
    let cache = RwLock::new(cached::SizedCache::with_capacity(size));
    MetricCache { next_sink: sink, cache }
}

/// A cache to help with ad-hoc defined metrics
/// Does not alter the values of the metrics
#[derive(Derivative)]
#[derivative(Debug)]
pub struct MetricCache<M, S> {
    next_sink: S,
    #[derivative(Debug="ignore")]
    cache: RwLock<cached::SizedCache<String, Cached<M>>>,
}

impl<M, S> Sink<M> for MetricCache<M, S>
    where S: Sink<M>,
          M: 'static + Clone + Send + Sync
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Cached<M> {
        let mut cache = self.cache.write().expect("Failed to acquire metric cache");
        let cached_metric = cache.cache_get(&name);
        if let Some(cached_metric) = cached_metric {
            return cached_metric.clone();
        }

        let new_metric = self.next_sink.new_metric(kind, name, sampling);
        cache.cache_set(name.to_string(), new_metric.clone());
        new_metric
    }

    fn new_scope(&self, auto_flush: bool) -> ScopeFn<M> {
        let next_scope = self.next_sink.new_scope(auto_flush);
        Arc::new(move |cmd| match cmd {
            Scope::Write(metric, value) => next_scope(Scope::Write(metric, value)),
            Scope::Flush => next_scope(Scope::Flush)
        })
    }
}
