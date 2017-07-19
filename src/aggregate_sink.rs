use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel, TimeType};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

pub enum Score {
    Event { start_time: TimeType, hit_count: AtomicUsize },
    Value { start_time: TimeType, hit_count: AtomicUsize, value_sum: AtomicUsize, max: AtomicUsize, min: AtomicUsize },
}

pub struct AggregateMetric {
    pub m_type: MetricType,
    pub name: String,
    pub score: Score,
}

impl AggregateMetric {

    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &Score::Event {ref hit_count, ..} => {
                hit_count.fetch_add(1, Ordering::Acquire);
            },
            &Score::Value {ref hit_count, ref value_sum, ref max, ref min, ..} => {
                let mut try_max = max.load(Ordering::Acquire);
                while value > try_max {
                    if max.compare_and_swap(try_max, value, Ordering::Release) == try_max {
                        break
                    } else {
                        try_max = max.load(Ordering::Acquire);
                    }
                }

                let mut try_min = min.load(Ordering::Acquire);
                while value < try_min {
                    if min.compare_and_swap(try_min, value, Ordering::Release) == try_min {
                        break
                    } else {
                        try_min = min.load(Ordering::Acquire);
                    }
                }
                value_sum.fetch_add(value, Ordering::Acquire);
                // TODO report any concurrent updates / resets for measurement of contention
                hit_count.fetch_add(1, Ordering::Acquire);
            }
        }
    }

    pub fn reset(&self) {
        match &self.score {
            &Score::Event {ref start_time, ref hit_count} => {
//                *start_time = TimeType::now();
                hit_count.store(0, Ordering::Release)
            },
            &Score::Value {ref start_time, ref hit_count, ref value_sum,ref max, ref min} => {
//                *start_time = TimeType::now();
                hit_count.store(0, Ordering::Release);
                value_sum.store(0, Ordering::Release);
                max.store(0, Ordering::Release);
                min.store(0, Ordering::Release);
            }
        }
    }
}

impl DefinedMetric for Arc<AggregateMetric> {
}

pub struct AggregateWrite {
}

impl MetricWrite<Arc<AggregateMetric>> for AggregateWrite {
    fn write(&self, metric: &Arc<AggregateMetric>, value: Value) {
        metric.write(value as usize);
    }
}

pub struct ScoreIterator {
    metrics: Arc<RwLock<Vec<Arc<AggregateMetric>>>>,
}

impl ScoreIterator {
    pub fn for_each<F>(&self, operations: F) where F: Fn(&AggregateMetric) {
        for metric in self.metrics.read().unwrap().iter() {
            operations(metric);
            metric.reset()
        };
    }
}

pub struct AggregateChannel {
    write: AggregateWrite,
    metrics: Arc<RwLock<Vec<Arc<AggregateMetric>>>>,
}

impl AggregateChannel {

    pub fn new() -> AggregateChannel {
        AggregateChannel { write: AggregateWrite {}, metrics: Arc::new(RwLock::new(Vec::new())) }
    }

    pub fn scores(&self) -> ScoreIterator {
        ScoreIterator { metrics : self.metrics.clone() }
    }

}

impl MetricChannel for AggregateChannel {
    type Metric = Arc<AggregateMetric>;
    type Write = AggregateWrite;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType) -> Arc<AggregateMetric> {
        let name = name.as_ref().to_string();
        let metric = Arc::new(AggregateMetric {
            m_type, name, score: match m_type {
                MetricType::Event => Score::Event {
                        start_time: TimeType::now(),
                        hit_count: AtomicUsize::new(0) },
                _ => Score::Value {
                        start_time: TimeType::now(),
                        hit_count: AtomicUsize::new(0),
                        value_sum: AtomicUsize::new(0),
                        max: AtomicUsize::new(0),
                        min: AtomicUsize::new(0) },
            }
        });

        self.metrics.write().unwrap().push(metric.clone());
        metric
    }

    fn write<F>(&self, operations: F ) where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature="bench")]
mod bench {

    use super::AggregateChannel;
    use core::{MetricType, MetricChannel, MetricWrite};
    use test::Bencher;

    #[bench]
    fn time_bench_ten_percent(b: &mut Bencher) {
        let aggregate = &AggregateChannel::new();
        let metric = aggregate.define(MetricType::Event, "count_a", 1.0);
        b.iter(|| aggregate.write(|scope| scope.write(&metric, 1)));
    }


}