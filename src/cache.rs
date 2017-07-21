use core::{MetricType, RateType, Value, MetricSink, SinkMetric, SinkWriter};
use cached::{SizedCache, Cached};
use std::rc::Rc;
use std::sync::RwLock;

// METRIC

impl <C: MetricSink> SinkMetric for Rc<C::Metric> {}

// WRITER

struct CachedMetricWriter<C: MetricSink> ( C::Write );

impl <C: MetricSink> SinkWriter<Rc<C::Metric>> for CachedMetricWriter<C> {
    fn write(&self, metric: &Rc<C::Metric>, value: Value) {
        self.0.write(&metric, value)
    }
}

// SINK

pub struct MetricCache<C: MetricSink> {
    target: C,
    cache: RwLock<SizedCache<String, Rc<C::Metric>>>,
}

impl <C: MetricSink> MetricCache<C> {
    pub fn new(target: C, cache_size: usize) -> MetricCache<C> {
        let cache = RwLock::new(SizedCache::with_capacity(cache_size));
        MetricCache { target, cache }
    }
}

impl <C: MetricSink> MetricSink for MetricCache<C> {
    type Metric = Rc<C::Metric>;
    type Write = CachedMetricWriter<C>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType) -> Rc<C::Metric> {
        let key = name.as_ref().to_string();
        {
            let mut cache = self.cache.write().unwrap();
            let cached_metric = cache.cache_get(&key);
            if let Some(cached_metric) = cached_metric {
                return cached_metric.clone();
            }
        }
        let target_metric = self.target.define(m_type, name, sampling);
        let metric = Rc::new( target_metric );
        let mut cache = self.cache.write().unwrap();
        cache.cache_set(key, metric.clone());
        metric
    }

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}
