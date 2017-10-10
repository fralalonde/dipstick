//! Maintain aggregated metrics for deferred reporting,

use core::*;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::sync::{Arc, RwLock};
use std::usize;

/// Aggregate metrics in memory.
/// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
/// Needs to be connected to a publish to be useful.
///
/// ```
/// use dipstick::*;
///
/// let (sink, source) = aggregate();
/// let metrics = metrics(sink);
///
/// metrics.marker("my_event").mark();
/// metrics.marker("my_event").mark();
/// ```
pub fn aggregate() -> (AggregateSink, AggregateSource) {
    let agg = Aggregator::new();
    (agg.as_sink(), agg.as_source())
}


/// Core aggregation structure for a single metric.
/// Hit count is maintained for all types.
/// If hit count is zero, then no values were recorded.
#[derive(Debug)]
enum InnerScores {
    /// Event metrics need not record more than a hit count.
    Event { hit: AtomicUsize },

    /// Value metrics keep track of key highlights.
    Value {
        hit: AtomicUsize,
        sum: AtomicUsize,
        max: AtomicUsize,
        min: AtomicUsize,
    },
}

#[derive(Debug, Clone, Copy)]
/// Possibly aggregated scores.
pub enum ScoreType {
    /// Number of times the metric was used.
    HitCount(u64),
    /// Sum of metric values reported.
    SumOfValues(u64),
    /// Biggest value reported.
    MaximumValue(u64),
    /// Smallest value reported.
    MinimumValue(u64),
    /// Approximative average value (hit count / sum, non-atomic)
    AverageValue(u64),
}

/// To-be-published snapshot of aggregated score values for a metric.
pub type ScoresSnapshot = Vec<ScoreType>;

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
pub struct MetricScores {
    /// The kind of metric.
    pub kind: Kind,

    /// The metric's name.
    pub name: String,

    score: InnerScores,
}

/// Spinlock update of max and min values.
/// Retries until success or clear loss to concurrent update.
#[inline]
fn compare_and_swap<F>(counter: &AtomicUsize, new_value: usize, retry: F)
where
    F: Fn(usize) -> bool,
{
    let mut loaded = counter.load(Acquire);
    while retry(loaded) {
        if counter.compare_and_swap(loaded, new_value, Release) == new_value {
            // success
            break;
        }
        loaded = counter.load(Acquire);
    }
}

impl MetricScores {
    /// Update scores with new value
    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &InnerScores::Event { ref hit, .. } => {
                hit.fetch_add(1, SeqCst);
            }
            &InnerScores::Value {
                ref hit,
                ref sum,
                ref max,
                ref min,
                ..
            } => {
                compare_and_swap(max, value, |loaded| value > loaded);
                compare_and_swap(min, value, |loaded| value < loaded);
                sum.fetch_add(value, Acquire);
                // TODO report any concurrent updates / resets for measurement of contention
                hit.fetch_add(1, Acquire);
            }
        }
    }

    /// reset aggregate values, return previous values
    pub fn read_and_reset(&self) -> ScoresSnapshot {
        let mut snapshot = Vec::new();
        match self.score {
            InnerScores::Event { ref hit } => {
                match hit.swap(0, Release) as u64 {
                    // hit count is the only meaningful metric for markers
                    // rate could be nice too but we don't time-derived (yet)
                    hit if hit > 0 => snapshot.push(ScoreType::HitCount(hit)),
                    _ => {}
                }
            }
            InnerScores::Value {
                ref hit,
                ref sum,
                ref max,
                ref min,
                ..
            } => {
                match hit.swap(0, Release) as u64 {
                    hit if hit > 0 => {
                        let sum = sum.swap(0, Release) as u64;

                        match self.kind {
                            Kind::Gauge => {
                                // sum and hit are meaningless for Gauge metrics
                            }
                            _ => {
                                snapshot.push(ScoreType::HitCount(hit));
                                snapshot.push(ScoreType::SumOfValues(sum));
                            }
                        }

                        // NOTE best-effort averaging
                        // - hit and sum are not incremented nor read as one
                        // - integer division is not rounding
                        // assuming values will still be good enough to be useful
                        snapshot.push(ScoreType::AverageValue(sum / hit));
                        snapshot.push(ScoreType::MaximumValue(
                            max.swap(usize::MIN, Release) as u64,
                        ));
                        snapshot.push(ScoreType::MinimumValue(
                            min.swap(usize::MAX, Release) as u64,
                        ));
                    }
                    _ => {}
                }
            }
        }
        snapshot
    }
}

/// Enumerate the metrics being aggregated and their scores.
#[derive(Debug, Clone)]
pub struct AggregateSource(Arc<RwLock<Vec<Arc<MetricScores>>>>);

impl AggregateSource {
    /// Iterate over every aggregated metric.
    pub fn for_each<F>(&self, ops: F)
    where
        F: Fn(&MetricScores),
    {
        for metric in self.0.read().unwrap().iter() {
            ops(&metric)
        }
    }
}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct Aggregator {
    metrics: Arc<RwLock<Vec<Arc<MetricScores>>>>,
}

impl Aggregator {
    /// Build a new metric aggregation point.
    pub fn new() -> Aggregator {
        Aggregator::with_capacity(0)
    }

    /// Build a new metric aggregation point with specified initial capacity of metrics to aggregate.
    pub fn with_capacity(size: usize) -> Aggregator {
        Aggregator { metrics: Arc::new(RwLock::new(Vec::with_capacity(size))) }
    }
}

/// Something that can be seen as a metric source.
pub trait AsSource {
    /// Get the metric source.
    fn as_source(&self) -> AggregateSource;
}

impl AsSource for Aggregator {
    fn as_source(&self) -> AggregateSource {
        AggregateSource(self.metrics.clone())
    }
}

impl AsSink<Aggregate, AggregateSink> for Aggregator {
    fn as_sink(&self) -> AggregateSink {
        AggregateSink(self.metrics.clone())
    }
}

/// The type of metric created by the AggregateSink.
/// Each Aggregate
pub type Aggregate = Arc<MetricScores>;

/// A sink where to send metrics for aggregation.
/// The parameters of aggregation may be set upon creation.
/// Just `clone()` to use as a shared aggregator.
#[derive(Debug, Clone)]
pub struct AggregateSink(Arc<RwLock<Vec<Aggregate>>>);

impl Sink<Aggregate> for AggregateSink {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Aggregate {
        let name = name.to_string();
        let metric = Arc::new(MetricScores {
            kind,
            name,
            score: match kind {
                Kind::Marker => InnerScores::Event { hit: AtomicUsize::new(0) },
                _ => InnerScores::Value {
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

    #[allow(unused_variables)]
    fn new_scope(&self, auto_flush: bool) -> ScopeFn<Aggregate> {
        Arc::new(|cmd| match cmd {
            Scope::Write(metric, value) => metric.write(value as usize),
            Scope::Flush => {}
        })
    }
}

#[cfg(feature = "bench")]
mod microbench {

    use super::*;
    use ::*;
    use test;

    #[bench]
    fn time_bench_write_event(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Kind::Marker, &"event_a", 1.0);
        let scope = sink.new_scope(false);
        b.iter(|| test::black_box(scope(Scope::Write(&metric, 1))));
    }


    #[bench]
    fn time_bench_write_count(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Kind::Counter, &"count_a", 1.0);
        let scope = sink.new_scope(false);
        b.iter(|| test::black_box(scope(Scope::Write(&metric, 1))));
    }

    #[bench]
    fn time_bench_read_event(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Kind::Marker, &"marker_a", 1.0);
        b.iter(|| test::black_box(metric.read_and_reset()));
    }

    #[bench]
    fn time_bench_read_count(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Kind::Counter, &"count_a", 1.0);
        b.iter(|| test::black_box(metric.read_and_reset()));
    }

}
