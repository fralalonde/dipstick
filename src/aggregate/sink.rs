use core::{MetricType, RateType, Value, SinkWriter, SinkMetric, MetricSink};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::usize;

#[derive(Debug)]
enum AtomicScore {
    Event { hit: AtomicUsize },
    Value { hit: AtomicUsize, sum: AtomicUsize, max: AtomicUsize, min: AtomicUsize },
}

/// consumed aggregated values
#[derive(Debug)]
pub enum AggregateScore {
    Event { hit: u64 },
    Value { hit: u64, sum: u64, max: u64, min: u64 },
}

#[derive(Debug)]
pub struct AggregateMetric {
    pub m_type: MetricType,
    pub name: String,
    score: AtomicScore,
}

impl AggregateMetric {

    /// add value to score
    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &AtomicScore::Event {ref hit, ..} => {
                hit.fetch_add(1, Ordering::SeqCst);
            },
            &AtomicScore::Value {ref hit, ref sum, ref max, ref min, ..} => {
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
                sum.fetch_add(value, Ordering::Acquire);
                // TODO report any concurrent updates / resets for measurement of contention
                hit.fetch_add(1, Ordering::Acquire);
            }
        }
    }

    /// reset aggregate values, save previous values to
    pub fn read_and_reset(&self) -> AggregateScore {
        match self.score {
            AtomicScore::Event {ref hit} =>
                AggregateScore::Event {
                    hit: hit.swap(0, Ordering::Release) as u64 },
            AtomicScore::Value {ref hit, ref sum,ref max, ref min} => {
                AggregateScore::Value {
                    hit: hit.swap(0, Ordering::Release) as u64,
                    sum: sum.swap(0, Ordering::Release) as u64,
                    max: max.swap(usize::MIN, Ordering::Release) as u64,
                    min: min.swap(usize::MAX, Ordering::Release) as u64,
                }
            }
        }
    }
}

impl SinkMetric for Arc<AggregateMetric> {
}

#[derive(Debug)]
pub struct AggregateWrite {
}

impl SinkWriter<Arc<AggregateMetric>> for AggregateWrite {
    fn write(&self, metric: &Arc<AggregateMetric>, value: Value) {
        metric.write(value as usize);
    }
}

#[derive(Debug)]
pub struct ScoreIterator {
    metrics: Arc<RwLock<Vec<Arc<AggregateMetric>>>>,
}

impl ScoreIterator {
    pub fn for_each<F>(&self, operations: F) where F: Fn(&AggregateMetric) {
        for metric in self.metrics.read().unwrap().iter() {
            operations(metric);
        };
    }
}

#[derive(Debug)]
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

impl MetricSink for AggregateChannel {
    type Metric = Arc<AggregateMetric>;
    type Writer = AggregateWrite;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType) -> Arc<AggregateMetric> {
        let name = name.as_ref().to_string();
        let metric = Arc::new(AggregateMetric {
            m_type, name, score: match m_type {
                MetricType::Event => AtomicScore::Event {
                        hit: AtomicUsize::new(0) },
                _ => AtomicScore::Value {
                        hit: AtomicUsize::new(0),
                        sum: AtomicUsize::new(0),
                        max: AtomicUsize::new(usize::MIN),
                        min: AtomicUsize::new(usize::MAX) },
            }
        });

        self.metrics.write().unwrap().push(metric.clone());
        metric
    }

    fn new_writer(&self) -> AggregateWrite {
        AggregateWrite{ }
    }

}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature="bench")]
mod bench {

    use super::AggregateChannel;
    use core::{MetricType, MetricSink, SinkWriter};
    use test::Bencher;

    #[bench]
    fn time_bench_write_event(b: &mut Bencher) {
        let aggregate = &AggregateChannel::new();
        let metric = aggregate.define(MetricType::Event, "event_a", 1.0);
        let writer = aggregate.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }


    #[bench]
    fn time_bench_write_count(b: &mut Bencher) {
        let aggregate = &AggregateChannel::new();
        let metric = aggregate.define(MetricType::Count, "count_a", 1.0);
        let writer = aggregate.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }

    #[bench]
    fn time_bench_read_event(b: &mut Bencher) {
        let aggregate = &AggregateChannel::new();
        let metric = aggregate.define(MetricType::Event, "event_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

    #[bench]
    fn time_bench_read_count(b: &mut Bencher) {
        let aggregate = &AggregateChannel::new();
        let metric = aggregate.define(MetricType::Count, "count_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

}