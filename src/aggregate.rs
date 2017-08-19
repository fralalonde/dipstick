use super::{MetricKind, Rate, Value, MetricWriter, MetricKey, MetricSink};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::usize;

#[derive(Debug)]
enum AtomicScore {
    Event { hit: AtomicUsize },
    Value {
        hit: AtomicUsize,
        sum: AtomicUsize,
        max: AtomicUsize,
        min: AtomicUsize,
    },
}

/// to-be-consumed aggregated values
#[derive(Debug)]
pub enum AggregateScore {
    NoData,
    Event { hit: u64 },
    Value {
        hit: u64,
        sum: u64,
        max: u64,
        min: u64,
    },
}

/// A metric that holds aggregated values
#[derive(Debug)]
pub struct AggregateKey {
    pub kind: MetricKind,
    pub name: String,
    score: AtomicScore,
}

impl AggregateKey {
    /// Update scores with value
    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &AtomicScore::Event { ref hit, .. } => {
                hit.fetch_add(1, Ordering::SeqCst);
            }
            &AtomicScore::Value {
                ref hit,
                ref sum,
                ref max,
                ref min,
                ..
            } => {
                let mut try_max = max.load(Ordering::Acquire);
                while value > try_max {
                    if max.compare_and_swap(try_max, value, Ordering::Release) == try_max {
                        break;
                    } else {
                        try_max = max.load(Ordering::Acquire);
                    }
                }

                let mut try_min = min.load(Ordering::Acquire);
                while value < try_min {
                    if min.compare_and_swap(try_min, value, Ordering::Release) == try_min {
                        break;
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

    /// reset aggregate values, return previous values
    pub fn read_and_reset(&self) -> AggregateScore {
        match self.score {
            AtomicScore::Event { ref hit } => {
                let hit = hit.swap(0, Ordering::Release) as u64;
                if hit == 0 {
                    AggregateScore::NoData
                } else {
                    AggregateScore::Event { hit }
                }
            }
            AtomicScore::Value {
                ref hit,
                ref sum,
                ref max,
                ref min,
            } => {
                let hit = hit.swap(0, Ordering::Release) as u64;
                if hit == 0 {
                    AggregateScore::NoData
                } else {
                    AggregateScore::Value {
                        hit,
                        sum: sum.swap(0, Ordering::Release) as u64,
                        max: max.swap(usize::MIN, Ordering::Release) as u64,
                        min: min.swap(usize::MAX, Ordering::Release) as u64,
                    }
                }
            }
        }
    }
}

impl MetricKey for Arc<AggregateKey> {}

#[derive(Debug)]
pub struct AggregateWrite();

impl MetricWriter<Arc<AggregateKey>> for AggregateWrite {
    fn write(&self, metric: &Arc<AggregateKey>, value: Value) {
        metric.write(value as usize);
    }
}

// there can only be one
lazy_static! {
    static ref AGGREGATE_WRITE: AggregateWrite = AggregateWrite();
}

#[derive(Debug)]
pub struct AggregateSource(Arc<RwLock<Vec<Arc<AggregateKey>>>>);

impl AggregateSource {
    pub fn for_each<F>(&self, ops: F)
    where
        F: Fn(&AggregateKey),
    {
        for metric in self.0.read().unwrap().iter() {
            ops(&metric)
        }
    }
}

#[derive(Debug)]
pub struct MetricAggregator {
    metrics: Arc<RwLock<Vec<Arc<AggregateKey>>>>,
}

impl MetricAggregator {
    pub fn new() -> MetricAggregator {
        MetricAggregator { metrics: Arc::new(RwLock::new(Vec::new())) }
    }

    pub fn source(&self) -> AggregateSource {
        AggregateSource(self.metrics.clone())
    }

    pub fn sink(&self) -> AggregateSink {
        AggregateSink(self.metrics.clone())
    }
}

pub struct AggregateSink(Arc<RwLock<Vec<Arc<AggregateKey>>>>);

impl MetricSink for AggregateSink {
    type Metric = Arc<AggregateKey>;
    type Writer = AggregateWrite;

    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate)
                                 -> Self::Metric {
        let name = name.as_ref().to_string();
        let metric = Arc::new(AggregateKey {
            kind,
            name,
            score: match kind {
                MetricKind::Event => AtomicScore::Event { hit: AtomicUsize::new(0) },
                _ => AtomicScore::Value {
                    hit: AtomicUsize::new(0),
                    sum: AtomicUsize::new(0),
                    max: AtomicUsize::new(usize::MIN),
                    min: AtomicUsize::new(usize::MAX),
                },
            },
        });

        self.0.write().unwrap().push(metric.clone());
        metric
    }

    fn new_writer(&self) -> Self::Writer {
        // TODO return AGGREGATE_WRITE or a immutable field at least
        AggregateWrite()
    }
}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature = "bench")]
mod bench {

    use super::MetricAggregator;
    use core::{MetricType, MetricSink, MetricWriter};
    use test::Bencher;

    #[bench]
    fn time_bench_write_event(b: &mut Bencher) {
        let aggregate = &MetricAggregator::new().sink();
        let metric = aggregate.new_metric(MetricType::Event, "event_a", 1.0);
        let writer = aggregate.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }


    #[bench]
    fn time_bench_write_count(b: &mut Bencher) {
        let aggregate = &MetricAggregator::new().sink();
        let metric = aggregate.new_metric(MetricType::Count, "count_a", 1.0);
        let writer = aggregate.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }

    #[bench]
    fn time_bench_read_event(b: &mut Bencher) {
        let aggregate = &MetricAggregator::new().sink();
        let metric = aggregate.new_metric(MetricType::Event, "event_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

    #[bench]
    fn time_bench_read_count(b: &mut Bencher) {
        let aggregate = &MetricAggregator::new().sink();
        let metric = aggregate.new_metric(MetricType::Count, "count_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

}
