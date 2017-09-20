//! Cache defined metrics.

// TODO one cache per metric kind (why?)

use ::*;
use cached::{SizedCache, Cached};
use std::sync::{Arc,RwLock};
use std::marker::PhantomData;

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn cache<'ph, M, W, S>(size: usize, sink: S) -> cache::MetricCache<'ph, M, W, S>
    where W: Writer<M>, S: Sink<M, W>
{
    cache::MetricCache::new(sink, size)
}

/// The cache key copies the target key.
pub type CachedKey<M> = Arc<M>;

/// The cache writer is transparent.
pub struct CachedMetricWriter<W> {
    target: W,
}

impl<M, W> Writer<CachedKey<M>> for CachedMetricWriter<W>
    where W: Writer<M>
{
    fn write(&self, metric: &CachedKey<M>, value: Value) {
        self.target.write(metric.as_ref(), value)
    }
}

/// A cache to help with ad-hoc defined metrics
/// Does not alter the values of the metrics
pub struct MetricCache<'ph, M, W: 'ph, S> {
    target: S,
    cache: RwLock<SizedCache<String, CachedKey<M>>>,
    phantom: PhantomData<&'ph W>,
}

impl<'ph, M, W, S> MetricCache<'ph, M, W, S> where W: Writer<M>, S: Sink<M, W>  {
    /// Build a new metric cache
    pub fn new(target: S, cache_size: usize) -> MetricCache<'ph, M, W, S> {
        let cache = RwLock::new(SizedCache::with_capacity(cache_size));
        MetricCache { target, cache, phantom: PhantomData {} }
    }
}

impl<'ph, M, W, S> Sink<Arc<M>, CachedMetricWriter<W>> for MetricCache<'ph, M, W, S>
    where W: Writer<M>, S: Sink<M, W>
{
    #[allow(unused_variables)]
    fn new_metric<STR>(&self, kind: MetricKind, name: STR, sampling: Rate) -> Arc<M>
            where STR: AsRef<str>
    {
        // TODO use ref for key, not owned
        let key = name.as_ref().to_string();
        {
            let mut cache = self.cache.write().unwrap();
            let cached_metric = cache.cache_get(&key);
            if let Some(cached_metric) = cached_metric {
                return cached_metric.clone();
            }
        }

        let target_metric = self.target.new_metric(kind, name, sampling);
        let new_metric = Arc::new(target_metric);
        let mut cache = self.cache.write().unwrap();
        cache.cache_set(key, new_metric.clone());
        new_metric
    }

    fn new_writer(&self) -> CachedMetricWriter<W> {
        CachedMetricWriter { target: self.target.new_writer() }
    }
}
