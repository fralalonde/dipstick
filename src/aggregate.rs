//! Maintain aggregated metrics for deferred reporting,
//!
use core::{command_fn, Kind, Sampling, Command, Value};
use core::Kind::*;
use output::{OpenScope, NO_METRIC_OUTPUT, MetricOutput};
use scope::{self, MetricScope, MetricInput, Flush, ScheduleFlush, DefineMetric,};
use namespace::WithNamespace;

use scores::{ScoreSnapshot, ScoreType, Scoreboard};
use scores::ScoreType::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A function type to transform aggregated scores into publishable statistics.
pub type StatsFn = Fn(Kind, &str, ScoreType)
    -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static;

lazy_static! {
    static ref DEFAULT_AGGREGATE_STATS: RwLock<Arc<StatsFn>> = RwLock::new(Arc::new(summary));

    static ref DEFAULT_AGGREGATE_OUTPUT: RwLock<Arc<OpenScope + Sync + Send>> =
        RwLock::new(NO_METRIC_OUTPUT.clone());
}

/// Set the default aggregated metrics statistics generator.
pub fn set_aggregate_default_stats<F>(func: F)
where
    F: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
{
    *DEFAULT_AGGREGATE_STATS.write().unwrap() = Arc::new(func)
}

/// Install a new receiver for all aggregateed metrics, replacing any previous receiver.
pub fn set_aggregate_default_output<IS: Into<MetricOutput<T>>, T: Send + Sync + Clone + 'static>
    (new_config: IS)
{
    *DEFAULT_AGGREGATE_OUTPUT.write().unwrap() = Arc::new(new_config.into());
}

fn get_aggregate_default() -> Arc<OpenScope> {
    DEFAULT_AGGREGATE_OUTPUT.read().unwrap().clone()
}

/// 1024 Metrics per scoreboard should be enough?
const DEFAULT_CAPACITY: usize = 1024;

/// Get the default aggregate point.
pub fn new_aggregate() -> MetricAggregate {
    MetricAggregate::with_capacity(DEFAULT_CAPACITY)
}

impl From<MetricAggregate> for MetricScope<Aggregate> {
    fn from(agg: MetricAggregate) -> MetricScope<Aggregate> {
        agg.into_scope()
    }
}

impl From<&'static str> for MetricScope<Aggregate> {
    fn from(prefix: &'static str) -> MetricScope<Aggregate> {
        let scope: MetricScope<Aggregate> = new_aggregate().into();
        if !prefix.is_empty() {
            scope.with_prefix(prefix)
        } else {
            scope
        }
    }
}

impl From<()> for MetricScope<Aggregate> {
    fn from(_: ()) -> MetricScope<Aggregate> {
        new_aggregate().into_scope()
    }
}

/// Central aggregation structure.
/// Maintains a list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct MetricAggregate {
    metrics: Arc<RwLock<HashMap<String, Arc<Scoreboard>>>>,
}

impl MetricAggregate {
    /// Build a new metric aggregation point with initial capacity of metrics to aggregate.
    pub fn with_capacity(size: usize) -> MetricAggregate {
        MetricAggregate {
            metrics: Arc::new(RwLock::new(HashMap::with_capacity(size))),
        }
    }

    fn into_scope(&self) -> MetricScope<Aggregate> {
        let agg_0 = self.clone();
        let agg_1 = self.clone();
        MetricScope::new(
            Arc::new(move |kind, name, rate| agg_0.define_metric(kind, name, rate)),
            command_fn(move |cmd| match cmd {
                Command::Write(metric, value) => {
                    let metric: &Aggregate = metric;
                    metric.update(value)
                }
                Command::Flush => agg_1.flush(),
            })
        )
    }

    /// Discard scores for ad-hoc metrics.
    pub fn cleanup(&self) {
        let orphans: Vec<String> = self.metrics.read().unwrap().iter()
            // is aggregator now the sole owner?
            // TODO use weak ref + impl Drop to mark abandoned metrics (see dispatch)
            .filter(|&(_k, v)| Arc::strong_count(v) == 1)
            .map(|(k, _v)| k.to_string())
            .collect();
        if !orphans.is_empty() {
            let mut remover = self.metrics.write().unwrap();
            orphans.iter().for_each(|k| {
                remover.remove(k);
            });
        }
    }

    ///
    pub fn flush_to(&self, publish_scope: &DefineMetric, stats_fn: Arc<StatsFn>) {
        let snapshot: Vec<ScoreSnapshot> = {
            let metrics = self.metrics.read().expect("Aggregate Lock");
            metrics.values().flat_map(|score| score.reset()).collect()
        };

        if snapshot.is_empty() {
            // no data was collected for this period
            // TODO repeat previous frame min/max ?
            // TODO update some canary metric ?
        } else {
            for metric in snapshot {
                for score in metric.2 {
                    if let Some(ex) = (stats_fn)(metric.0, metric.1.as_ref(), score) {
                        publish_scope
                            .define_metric_object(ex.0, &ex.1.concat(), 1.0)
                            .write(ex.2);
                    }
                }
            }
        }
    }

}

impl MetricInput<Aggregate> for MetricAggregate {
    /// Define an event counter of the provided name.
    fn marker(&self, name: &str) -> scope::Marker {
        self.into_scope().marker(name)
    }

    /// Define a counter of the provided name.
    fn counter(&self, name: &str) -> scope::Counter {
        self.into_scope().counter(name)
    }

    /// Define a timer of the provided name.
    fn timer(&self, name: &str) -> scope::Timer {
        self.into_scope().timer(name)
    }

    /// Define a gauge of the provided name.
    fn gauge(&self, name: &str) -> scope::Gauge {
        self.into_scope().gauge(name)
    }

    /// Lookup or create a scoreboard for the requested metric.
    fn define_metric(&self, kind: Kind, name: &str, _rate: Sampling) -> Aggregate {
        self.metrics
            .write()
            .expect("Locking aggregator")
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Scoreboard::new(kind, name.to_string())))
            .clone()
    }

    #[inline]
    fn write(&self, metric: &Aggregate, value: Value) {
        metric.update(value)
    }
}

impl Flush for MetricAggregate {
    /// Collect and reset aggregated data.
    /// Publish statistics
    fn flush(&self) {
        let default_publish_fn = DEFAULT_AGGREGATE_STATS.read().unwrap().clone();
        let publish_scope = get_aggregate_default().open_scope_object();

        self.flush_to(publish_scope.as_ref(), default_publish_fn);

        // TODO parameterize whether to keep ad-hoc metrics after publish
        // source.cleanup();
        publish_scope.flush()
    }
}

impl ScheduleFlush for MetricAggregate {}

impl From<MetricAggregate> for Arc<DefineMetric + Send + Sync + 'static> {
    fn from(metrics: MetricAggregate) -> Arc<DefineMetric + Send + Sync + 'static> {
        metrics.into()
    }
}

/// The type of metric created by the Aggregator.
pub type Aggregate = Arc<Scoreboard>;

/// A predefined export strategy reporting all aggregated stats for all metric types.
/// Resulting stats are named by appending a short suffix to each metric's name.
pub fn all_stats(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match score {
        Count(hit) => Some((Counter, vec![name, ".count"], hit)),
        Sum(sum) => Some((kind, vec![name, ".sum"], sum)),
        Mean(mean) => Some((kind, vec![name, ".mean"], mean.round() as Value)),
        Max(max) => Some((Gauge, vec![name, ".max"], max)),
        Min(min) => Some((Gauge, vec![name, ".min"], min)),
        Rate(rate) => Some((Gauge, vec![name, ".rate"], rate.round() as Value)),
    }
}

/// A predefined export strategy reporting the average value for every non-marker metric.
/// Marker metrics export their hit count instead.
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
pub fn average(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match kind {
        Marker => match score {
            Count(count) => Some((Counter, vec![name], count)),
            _ => None,
        },
        _ => match score {
            Mean(avg) => Some((Gauge, vec![name], avg.round() as Value)),
            _ => None,
        },
    }
}

/// A predefined single-stat-per-metric export strategy:
/// - Timers and Counters each export their sums
/// - Markers each export their hit count
/// - Gauges each export their average
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
pub fn summary(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match kind {
        Marker => match score {
            Count(count) => Some((Counter, vec![name], count)),
            _ => None,
        },
        Counter | Timer => match score {
            Sum(sum) => Some((kind, vec![name], sum)),
            _ => None,
        },
        Gauge => match score {
            Mean(mean) => Some((Gauge, vec![name], mean.round() as Value)),
            _ => None,
        },
    }
}

#[cfg(feature = "bench")]
mod bench {

    use test;
    use core::Kind::{Counter, Marker};
    use aggregate::new_aggregate;
    use scope::MetricInput;

    #[bench]
    fn aggregate_marker(b: &mut test::Bencher) {
        let sink = new_aggregate();
        let metric = sink.define_metric(Marker, "event_a", 1.0);
        b.iter(|| test::black_box(sink.write(&metric, 1)));
    }

    #[bench]
    fn aggregate_counter(b: &mut test::Bencher) {
        let sink = new_aggregate();
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(sink.write(&metric, 1)));
    }

    #[bench]
    fn reset_marker(b: &mut test::Bencher) {
        let metric = new_aggregate().define_metric(Marker, "marker", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

    #[bench]
    fn reset_counter(b: &mut test::Bencher) {
        let metric = new_aggregate().define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

}
