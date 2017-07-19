use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};
use cached::SizedCache;

////////////

//pub struct InstrumentCacheMetric<M: DefinedMetric> {
//    target: M
//}
//
//impl <M: DefinedMetric> DefinedMetric for InstrumentCacheMetric<M> {}
//
//pub struct InstrumentCacheWrite<C: MetricChannel> {
//    target: C,
//}
//
//impl <C: MetricChannel> MetricWrite<InstrumentCacheMetric<<C as MetricChannel>::Metric>> for InstrumentCacheWrite<C> {
//
//    fn write(&self, metric: &InstrumentCacheMetric<<C as MetricChannel>::Metric>, value: Value) {
//        debug!("InstrumentCache");
//        self.target.write(|scope| scope.write(&metric.target, value))
//    }
//}

pub struct InstrumentCacheChannel<C: MetricChannel> {
    target: C,
    cache: SizedCache<String, C::Metric>,
}

impl <C: MetricChannel> InstrumentCacheChannel<C> {
    pub fn new(target: C) -> InstrumentCacheChannel<C> {
        let cache = SizedCache::with_capacity(1024);
        InstrumentCacheChannel { target, cache }
    }
}

impl <C: MetricChannel> MetricChannel for InstrumentCacheChannel<C> {
    type Metric = C::Metric;
    type Write = C::Write;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> C::Metric {
        let key = name.as_ref().to_string();
        {
            let mut cache = self.cache.lock().unwrap();
            let res = cached::Cached::cache_get(&mut *cache, &key);
            if let Some(res) = res { return res.clone(); }
        }
        let val = (||$body)();
        let mut cache = $cachename.lock().unwrap();
        $crate::Cached::cache_set(&mut *cache, key, val.clone());
        val


        self.cache.cache_get(name).;
        let pm = self.target.define(m_type, name, sample);
        InstrumentCacheMetric { target: pm }
    }

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}
