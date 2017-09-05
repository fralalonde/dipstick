//! Cache defined metrics.

// TODO one cache per metric kind

use ::*;
use cached::{SizedCache, Cached};
use std::sync::{Arc,RwLock};
use std::fmt;

// TODO get rid of this struct+impl, replace it with
// impl <C: MetricSink> SinkMetric for Arc<C::Metric> {}
// and let sink just return a clone the cached Arc
// which I could not make it compile because:
// "the type parameter `C` is not constrained by the impl trait, self type,
// or predicates E0207 unconstrained type parameter"
// which is strange because Arc<C::Metric> looks like a <C> constraint to me...
// one solution might require SinkMetric<PHANTOM> (everywhere!),
// not tried because it would be HORRIBLE
// for now we use this "wrapping reification" of Arc<> which needs to be allocated everytime
// if you know how to fix it that'd be great
#[derive(Debug)]
/// The cache key copies the target key.
pub struct CachedKey<C: MetricSink>(Arc<C::Metric>);

impl<C: MetricSink> MetricKey for CachedKey<C> {}

/// The cache writer is transparent.
#[derive(Debug)]
pub struct CachedMetricWriter<C: MetricSink> {
    target: C::Writer,
}

impl<C: MetricSink> MetricWriter<CachedKey<C>> for CachedMetricWriter<C> {
    fn write(&self, metric: &CachedKey<C>, value: Value) {
        self.target.write(metric.0.as_ref(), value)
    }
}

/// A cache to help with ad-hoc defined metrics
/// Does not alter the values of the metrics
pub struct MetricCache<C: MetricSink> {
    target: C,
    cache: RwLock<SizedCache<String, Arc<C::Metric>>>,
}

impl<C: MetricSink> fmt::Debug for MetricCache<C> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(self.target.fmt(f)?)
    }
}

impl<C: MetricSink> MetricCache<C> {
    /// Build a new metric cache
    pub fn new(target: C, cache_size: usize) -> MetricCache<C> {
        let cache = RwLock::new(SizedCache::with_capacity(cache_size));
        MetricCache { target, cache }
    }
}

impl<C: MetricSink> MetricSink for MetricCache<C> {
    type Metric = CachedKey<C>;
    type Writer = CachedMetricWriter<C>;

    #[allow(unused_variables)]
    fn new_metric<S>(&self, kind: MetricKind, name: S, sampling: Rate) -> Self::Metric
            where S: AsRef<str>    {

        // TODO use ref for key, not owned
        let key = name.as_ref().to_string();
        {
            let mut cache = self.cache.write().unwrap();
            let cached_metric = cache.cache_get(&key);
            if let Some(cached_metric) = cached_metric {
                return CachedKey(cached_metric.clone());
            }
        }

        let target_metric = self.target.new_metric(kind, name, sampling);
        let new_metric = Arc::new(target_metric);
        let mut cache = self.cache.write().unwrap();
        cache.cache_set(key, new_metric.clone());
        CachedKey(new_metric)
    }

    fn new_writer(&self) -> Self::Writer {
        CachedMetricWriter { target: self.target.new_writer() }
    }
}
