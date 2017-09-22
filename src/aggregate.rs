//! Maintain aggregated metrics for deferred reporting,

use core::*;
use std::sync::atomic::{AtomicUsize, Ordering};
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
/// metrics.event("my_event").mark();
/// metrics.event("my_event").mark();
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
    Event {
        hit: AtomicUsize
    },

    /// Value metrics keep track of key highlights.
    Value {
        hit: AtomicUsize,
        sum: AtomicUsize,
        max: AtomicUsize,
        min: AtomicUsize,
    },
}

/// To-be-published snapshot of aggregated score values.
#[derive(Debug, Clone, Copy)]
pub enum ScoresSnapshot {
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
fn compare_and_swap<F>(counter: &AtomicUsize, new_value: usize, retry: F) where F: Fn(usize) -> bool {
    let mut loaded = counter.load(Ordering::Acquire);
    while retry(loaded) {
        if counter.compare_and_swap(loaded, new_value, Ordering::Release) == new_value {
            // success
            break;
        }
        loaded = counter.load(Ordering::Acquire);
    }
}

impl MetricScores {
    /// Update scores with new value
    pub fn write(&self, value: usize) -> () {
        match &self.score {
            &InnerScores::Event { ref hit, .. } => {
                hit.fetch_add(1, Ordering::SeqCst);
            }
            &InnerScores::Value { ref hit, ref sum, ref max, ref min, .. } => {
                compare_and_swap(max, value, |loaded| value > loaded);
                compare_and_swap(min, value, |loaded| value < loaded);
                sum.fetch_add(value, Ordering::Acquire);
                // TODO report any concurrent updates / resets for measurement of contention
                hit.fetch_add(1, Ordering::Acquire);
            }
        }
    }

    /// reset aggregate values, return previous values
    pub fn read_and_reset(&self) -> ScoresSnapshot {
        match self.score {
            InnerScores::Event { ref hit } => {
                match hit.swap(0, Ordering::Release) as u64 {
                    0 => ScoresSnapshot::NoData,
                    hit => ScoresSnapshot::Event { hit }
                }
            }
            InnerScores::Value { ref hit, ref sum, ref max, ref min, .. } => {
                match hit.swap(0, Ordering::Release) as u64 {
                    0 => ScoresSnapshot::NoData,
                    hit => ScoresSnapshot::Value {
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

/// Enumerate the metrics being aggregated and their scores.
#[derive(Clone)]
pub struct AggregateSource(Arc<RwLock<Vec<Arc<MetricScores>>>>);

impl AggregateSource {

    /// Iterate over every aggregated metric.
    pub fn for_each<F>(&self, ops: F) where F: Fn(&MetricScores) {
        for metric in self.0.read().unwrap().iter() {
            ops(&metric)
        }
    }
}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
pub struct Aggregator {
    metrics: Arc<RwLock<Vec<Arc<MetricScores>>>>,
}

impl Aggregator {
    /// Build a new metric aggregation point.
    pub fn new() -> Aggregator {
        Aggregator::with_capacity(0)
    }

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

impl AsSink<Arc<MetricScores>, AggregateSink> for Aggregator {
    fn as_sink(&self) -> AggregateSink {
        AggregateSink(self.metrics.clone())
    }
}

/// A sink where to send metrics for aggregation.
/// The parameters of aggregation may be set upon creation.
/// Just `clone()` to use as a shared aggregator.
#[derive(Clone)]
pub struct AggregateSink(Arc<RwLock<Vec<Arc<MetricScores>>>>);

impl Sink<Arc<MetricScores>> for AggregateSink {
    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: Kind, name: S, sampling: Rate) -> Arc<MetricScores> {
        let name = name.as_ref().to_string();
        let metric = Arc::new(MetricScores {
            kind,
            name,
            score: match kind {
                Kind::Event => InnerScores::Event { hit: AtomicUsize::new(0) },
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

    fn new_scope(&self) -> &Fn(Option<(&Arc<MetricScores>, Value)>) {
        &|cmd| match cmd {
            Some((metric, value)) => metric.write(value as usize),
            None => {}
        }
    }

}

#[cfg(feature = "bench")]
mod microbench {

    use super::Aggregator;
    use ::*;
    use test::Bencher;

    #[bench]
    fn time_bench_write_event(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(MetricKind::Event, "event_a", 1.0);
        let scope = sink.new_scope();
        b.iter(|| scope(Some((&metric, 1))));
    }


    #[bench]
    fn time_bench_write_count(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metric = sink.new_metric(Kind::Count, "count_a", 1.0);
        let scope = sink.new_scope();
        b.iter(|| scope(Some((&metric, 1))));
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
