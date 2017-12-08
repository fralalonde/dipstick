//! Maintain aggregated metrics for deferred reporting,

use std::collections::HashMap;
use core::*;
use core::Kind::*;
use std::sync::{Arc, RwLock};
use self::ScoreType::*;
use scores::*;

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
    AverageValue(f64),
    /// Approximative mean rate (hit count / period length in seconds, non-atomic)
    MeanRate(f64),
}

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
pub struct MetricScores {
    /// The kind of metric.
    pub kind: Kind,

    /// The metric's name.
    pub name: String,

    scores: Scoreboard,
}

impl MetricScores {

    /// Update aggregated values
    pub fn write(&self, value: Value) {
        self.scores.update(value)
    }

    /// Reset aggregate values, return previous values
    /// To-be-published snapshot of aggregated score values for a metric.
    pub fn read_and_reset(&self) -> Vec<ScoreType> {
        let (values, now) = self.scores.reset();

        // if hit count is zero, then no values were recorded.
        if values.hit_count() == 0 { return vec![] }

        let mut snapshot = Vec::new();
        let mean_rate = values.hit_count() as f64 /
            ((values.start_time_ns() - now) as f64 / 1_000_000_000.0);
        match self.kind {
            Marker => {
                snapshot.push(HitCount(values.hit_count()));
                snapshot.push(MeanRate(mean_rate))
            },
            Gauge => {
                snapshot.push(MaximumValue(values.max()));
                snapshot.push(MinimumValue(values.min()));
            },
            Timer | Counter => {
                snapshot.push(HitCount(values.hit_count()));
                snapshot.push(SumOfValues(values.sum()));

                snapshot.push(MaximumValue(values.max()));
                snapshot.push(MinimumValue(values.min()));
                // NOTE following derived metrics are a computed as a best-effort between atomics
                // NO GUARANTEES
                snapshot.push(AverageValue(values.sum() as f64 / values.hit_count() as f64));
                snapshot.push(MeanRate(mean_rate))
            },
        }
        snapshot
    }
}

/// Enumerate the metrics being aggregated and their scores.
#[derive(Debug, Clone)]
pub struct AggregateSource(Arc<RwLock<HashMap<String, Arc<MetricScores>>>>);

impl AggregateSource {
    /// Iterate over every aggregated metric.
    // TODO impl Iterator
    pub fn for_each<F>(&self, ops: F)
    where
        F: Fn(&MetricScores),
    {
        for metric in self.0.read().unwrap().values() {
            ops(&metric)
        }
    }

    /// Discard scores for ad-hoc metrics.
    pub fn cleanup(&self) {
        let orphans: Vec<String> = self.0.read().unwrap().iter()
            // is aggregator now the sole owner?
            .filter(|&(_k, v)| Arc::strong_count(v) == 1)
            .map(|(k, _v)| k.to_string())
            .collect();
        if !orphans.is_empty() {
            let mut remover = self.0.write().unwrap();
            orphans.iter().for_each(|k| {remover.remove(k);});
        }
    }

}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct Aggregator {
    metrics: Arc<RwLock<HashMap<String, Arc<MetricScores>>>>,
}

impl Aggregator {
    /// Build a new metric aggregation point.
    pub fn new() -> Aggregator {
        Aggregator::with_capacity(0)
    }

    /// Build a new metric aggregation point with specified initial capacity of metrics to aggregate.
    pub fn with_capacity(size: usize) -> Aggregator {
        Aggregator { metrics: Arc::new(RwLock::new(HashMap::with_capacity(size))) }
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

impl AsSink for Aggregator {
    type Metric = Aggregate;
    type Sink = AggregateSink;

    /// Get the metric sink.
    fn as_sink(&self) -> Self::Sink {
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
pub struct AggregateSink(Arc<RwLock<HashMap<String, Aggregate>>>);


impl Sink<Aggregate> for AggregateSink {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> Aggregate {
        self.0.write().unwrap().entry(name.to_string()).or_insert_with(||
            Arc::new(MetricScores {
                kind,
                name: name.to_string(),
                scores: Scoreboard::new()
            })).clone()
    }

    #[allow(unused_variables)]
    fn new_scope(&self, auto_flush: bool) -> ScopeFn<Aggregate> {
        Arc::new(|cmd| match cmd {
            Scope::Write(metric, value) => metric.write(value),
            Scope::Flush => {}
        })
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    fn time_bench_write_event(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Marker, &"event_a", 1.0);
        let scope = sink.new_scope(false);
        b.iter(|| test::black_box(scope(Scope::Write(&metric, 1))));
    }


    #[bench]
    fn time_bench_write_count(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Counter, &"count_a", 1.0);
        let scope = sink.new_scope(false);
        b.iter(|| test::black_box(scope(Scope::Write(&metric, 1))));
    }

    #[bench]
    fn time_bench_read_event(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Marker, &"marker_a", 1.0);
        b.iter(|| test::black_box(metric.read_and_reset()));
    }

    #[bench]
    fn time_bench_read_count(b: &mut test::Bencher) {
        let (sink, _source) = aggregate();
        let metric = sink.new_metric(Counter, &"count_a", 1.0);
        b.iter(|| test::black_box(metric.read_and_reset()));
    }

}
