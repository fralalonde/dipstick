//! Maintain aggregated metrics for deferred reporting,
//!
use core::*;
use local_metrics::*;
use app_metrics::*;
use namespace::*;
use output::to_void;

use scores::*;
use publish::*;

use std::fmt::Debug;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Aggregate metrics in memory.
/// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
/// Needs to be connected to a publish to be useful.
/// ```
/// use dipstick::*;
/// let metrics: AppMetrics<_> = aggregate(summary, to_stdout()).into();
/// metrics.marker("my_event").mark();
/// metrics.marker("my_event").mark();
/// ```
pub fn aggregate<E, M>(stat_fn: E, to_chain: LocalMetrics<M>) -> Aggregator
where
    E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
    M: Clone + Send + Sync + Debug + 'static,
{
    Aggregator {
        metrics: Arc::new(RwLock::new(HashMap::new())),
        publish: Arc::new(Publisher::new(stat_fn, to_chain)),
    }
}

impl From<Aggregator> for AppMetrics<Aggregate> {
    fn from(agg: Aggregator) -> AppMetrics<Aggregate> {
        let agg_1 = agg.clone();
        AppMetrics::new(
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

impl From<&'static str> for AppMetrics<Aggregate> {
    fn from(prefix: &'static str) -> AppMetrics<Aggregate> {
        let app_metrics: AppMetrics<Aggregate> = aggregate(summary, to_void()).into();
        app_metrics.with_prefix(prefix)
    }
}

/// Central aggregation structure.
/// Maintains a list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct Aggregator {
    metrics: Arc<RwLock<HashMap<String, Arc<Scoreboard>>>>,
    publish: Arc<Publish>,
}

impl Aggregator {
    /// Build a new metric aggregation point with specified initial capacity of metrics to aggregate.
    pub fn with_capacity(size: usize, publish: Arc<Publish>) -> Aggregator {
        Aggregator {
            metrics: Arc::new(RwLock::new(HashMap::with_capacity(size))),
            publish: publish.clone(),
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
    pub fn define_metric(&self, kind: Kind, name: &str, _rate: Rate) -> Aggregate {
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
        let metrics = self.metrics.read().expect("Locking metrics scoreboards");
        let snapshot = metrics.values().flat_map(|score| score.reset()).collect();
        self.publish.publish(snapshot);
    }
}

/// The type of metric created by the Aggregator.
pub type Aggregate = Arc<Scoreboard>;

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;
    use core::Kind::*;
    use output::*;

    #[bench]
    fn aggregate_marker(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Marker, "event_a", 1.0);
        let scope = sink.open_scope(false);
        b.iter(|| test::black_box(scope.write(&metric, 1)));
    }

    #[bench]
    fn aggregate_counter(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        let scope = sink.open_scope(false);
        b.iter(|| test::black_box(scope.write(&metric, 1)));
    }

    #[bench]
    fn reset_marker(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Marker, "marker_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

    #[bench]
    fn reset_counter(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

}
