//! Maintain aggregated metrics for deferred reporting,
//!
use core::{control_scope, Kind, Sampling, ScopeCmd, Value};
use core::Kind::*;
use config::{OpenScope, DEFAULT_CONFIG, NO_METRIC_CONFIG};
use scope::MetricScope;
use namespace::WithNamespace;

use scores::{ScoreSnapshot, ScoreType, Scoreboard};
use scores::ScoreType::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Define aggregate metrics.
#[macro_export]
macro_rules! aggregate_metrics {
    (pub $METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {<Aggregate> pub $METRIC_ID = $e }
    };
    (pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Aggregate> pub $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {<Aggregate> $METRIC_ID = $e }
    };
    ($METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Aggregate> $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        metrics! {<Aggregate> $METRIC_ID => { $($REMAINING)* } }
    };
    ($e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Aggregate> $e => { $($REMAINING)* } }
    };
}

lazy_static! {
    static ref DEFAULT_AGGREGATE_STATS:
        RwLock<Arc<Fn(Kind, &str, ScoreType) ->
                Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static>> =
            RwLock::new(Arc::new(summary));

    static ref AGGREGATE_REGISTRY:
        RwLock<HashMap<String, Arc<RwLock<HashMap<String, Arc<Scoreboard>>>>>> =
            RwLock::new(HashMap::new());

    static ref DEFAULT_AGGREGATE_SCOPE: RwLock<Arc<OpenScope + Sync + Send>> =
        RwLock::new(NO_METRIC_CONFIG.clone());
}

/// Set the default aggregated metrics statistics generator.
pub fn set_default_aggregate_statistics<F>(func: F)
where
    F: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
{
    *DEFAULT_AGGREGATE_STATS.write().unwrap() = Arc::new(func)
}

/// Install a new receiver for all aggregateed metrics, replacing any previous receiver.
pub fn set_aggregate_default<
    IS: Into<Arc<OpenScope + Sync + Send>>,
    T: Send + Sync + Clone + 'static,
>(
    new_config: IS,
) {
    *DEFAULT_AGGREGATE_SCOPE.write().unwrap() = new_config.into();
}

/// Get the named aggregate point.
/// Uses the stored instance if it already exists, otherwise creates it.
/// All aggregate points are automatically entered in the aggregate registry and kept FOREVER.
fn aggregate_name(name: &str) -> MetricScope<Aggregate> {
    let metrics = AGGREGATE_REGISTRY
        .write()
        .expect("Aggregate Registry")
        .entry(name.into())
        .or_insert_with(|| Arc::new(RwLock::new(HashMap::new())))
        .clone();
    MetricAggregate { metrics }.into()
}

/// Get the default aggregate point.
pub fn aggregate() -> MetricScope<Aggregate> {
    aggregate_name("_DEFAULT")
}

impl From<MetricAggregate> for MetricScope<Aggregate> {
    fn from(agg: MetricAggregate) -> MetricScope<Aggregate> {
        let agg_1 = agg.clone();
        MetricScope::new(
            Arc::new(move |kind, name, rate| agg.define_metric(kind, name, rate)),
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let metric: &Aggregate = metric;
                    metric.update(value)
                }
                ScopeCmd::Flush => agg_1.flush(),
            }),
        )
    }
}

impl From<&'static str> for MetricScope<Aggregate> {
    fn from(prefix: &'static str) -> MetricScope<Aggregate> {
        let app_metrics: MetricScope<Aggregate> = aggregate();
        if !prefix.is_empty() {
            app_metrics.with_prefix(prefix)
        } else {
            app_metrics
        }
    }
}

impl From<()> for MetricScope<Aggregate> {
    fn from(_: ()) -> MetricScope<Aggregate> {
        let scope: MetricScope<Aggregate> = aggregate();
        scope
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

    /// Lookup or create a scoreboard for the requested metric.
    pub fn define_metric(&self, kind: Kind, name: &str, _rate: Sampling) -> Aggregate {
        self.metrics
            .write()
            .expect("Locking aggregator")
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Scoreboard::new(kind, name.to_string())))
            .clone()
    }

    /// Collect and reset aggregated data.
    /// Publish statistics
    pub fn flush(&self) {
        let snapshot: Vec<ScoreSnapshot> = {
            let metrics = self.metrics.read().expect("Aggregate Lock");
            metrics.values().flat_map(|score| score.reset()).collect()
        };

        let default_publish_fn = DEFAULT_AGGREGATE_STATS.read().unwrap().clone();

        let publish_scope = DEFAULT_CONFIG.read().unwrap().open_scope();
        if snapshot.is_empty() {
            // no data was collected for this period
            // TODO repeat previous frame min/max ?
            // TODO update some canary metric ?
        } else {
            for metric in snapshot {
                for score in metric.2 {
                    if let Some(ex) = (default_publish_fn)(metric.0, metric.1.as_ref(), score) {
                        publish_scope
                            .define_metric(ex.0, &ex.1.concat(), 1.0)
                            .write(ex.2);
                    }
                }
            }
        }

        // TODO parameterize whether to keep ad-hoc metrics after publish
        // source.cleanup();
        publish_scope.flush()
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
    use aggregate::aggregate;

    #[bench]
    fn aggregate_marker(b: &mut test::Bencher) {
        let sink = aggregate();
        let metric = sink.define_metric(Marker, "event_a", 1.0);
        b.iter(|| test::black_box(sink.write(&metric, 1)));
    }

    #[bench]
    fn aggregate_counter(b: &mut test::Bencher) {
        let sink = aggregate();
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(sink.write(&metric, 1)));
    }

    #[bench]
    fn reset_marker(b: &mut test::Bencher) {
        let agg = aggregate();
        let metric = agg.define_metric(Marker, "marker", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

    #[bench]
    fn reset_counter(b: &mut test::Bencher) {
        let agg = aggregate();
        let metric = agg.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

}
