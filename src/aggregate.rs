//! Maintain aggregated metrics for deferred reporting,

use ::*;
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
#[derive(Debug, Clone, Copy)]
pub enum AggregateScore {
    /// No data was reported (yet) for this metric.
    NoData,

    /// Simple score for event counters
    Event {
        /// Number of times the metric was used.
        hit: u64
    },

    /// Score structure for counters, timers and gauges.
    Value {
        /// Number of times the metric was used.
        hit: u64,
        /// Sum of metric values reported.
        sum: u64,
        /// Biggest value reported.
        max: u64,
        /// Smallest value reported.
        min: u64,
    },
}

/// A metric that holds aggregated values
#[derive(Debug)]
pub struct AggregateKey {
    /// The kind of metric.
    pub kind: MetricKind,
    /// The metric's name.
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

/// Since aggregation negates any scope, there only needs to be a single writer ever.
#[derive(Debug, Clone, Copy)]
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

/// Enumerate the metrics being aggregated and their scores.
#[derive(Debug, Clone)]
pub struct AggregateSource(Arc<RwLock<Vec<Arc<AggregateKey>>>>);

impl AggregateSource {

    /// Iterate over every aggregated metric.
    pub fn for_each<F>(&self, ops: F) where F: Fn(&AggregateKey),
    {
        for metric in self.0.read().unwrap().iter() {
            ops(&metric)
        }
    }
}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
#[derive(Debug)]
pub struct MetricAggregator {
    metrics: Arc<RwLock<Vec<Arc<AggregateKey>>>>,
}

impl MetricAggregator {
    /// Build a new metric aggregation point.
    pub fn new() -> MetricAggregator {
        MetricAggregator { metrics: Arc::new(RwLock::new(Vec::new())) }
    }
}

impl AsSource for MetricAggregator {
    fn as_source(&self) -> AggregateSource {
        AggregateSource(self.metrics.clone())
    }
}

impl AsSink<AggregateSink> for MetricAggregator {
    fn as_sink(&self) -> AggregateSink {
        AggregateSink(self.metrics.clone())
    }
}

/// A sink where to send metrics for aggregation.
/// The parameters of aggregation may be set upon creation.
/// Just `clone()` to use as a shared aggregator.
#[derive(Debug, Clone)]
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
    use ::*;
    use test::Bencher;

    #[bench]
    fn time_bench_write_event(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(MetricKind::Event, "event_a", 1.0);
        let writer = sink.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }


    #[bench]
    fn time_bench_write_count(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(MetricKind::Count, "count_a", 1.0);
        let writer = sink.new_writer();
        b.iter(|| writer.write(&metric, 1));
    }

    #[bench]
    fn time_bench_read_event(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(MetricKind::Event, "event_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

    #[bench]
    fn time_bench_read_count(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(MetricKind::Count, "count_a", 1.0);
        b.iter(|| metric.read_and_reset());
    }

}
