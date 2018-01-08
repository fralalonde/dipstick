//! Maintain aggregated metrics for deferred reporting,
//!
use core::*;
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
/// let sink = aggregate(4, summary, to_stdout());
/// let metrics = global_metrics(sink);
/// metrics.marker("my_event").mark();
/// metrics.marker("my_event").mark();
/// ```
pub fn aggregate<E, M>(stat_fn: E, to_chain: Chain<M>) -> Chain<Aggregate>
where
    E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
    M: Clone + Send + Sync + Debug + 'static,
{
    let metrics = Arc::new(RwLock::new(HashMap::new()));
    let metrics0 = metrics.clone();

    let publish = Arc::new(Publisher::new(stat_fn, to_chain));

    Chain::new(
        move |kind, name, _rate| {
            metrics
                .write()
                .unwrap()
                .entry(name.to_string())
                .or_insert_with(|| Arc::new(Scoreboard::new(kind, name.to_string())))
                .clone()
        },
        move |_buffered| {
            let metrics = metrics0.clone();
            let publish = publish.clone();
            ControlScopeFn::new(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let metric: &Aggregate = metric;
                    metric.update(value)
                },
                ScopeCmd::Flush => {
                    let metrics = metrics.read().expect("Lock scoreboards for a snapshot.");
                    let snapshot = metrics.values().flat_map(|score| score.reset()).collect();
                    publish.publish(snapshot);
                }
            })
        },
    )
}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
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
    fn time_bench_write_event(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Marker, "event_a", 1.0);
        let scope = sink.open_scope(false);
        b.iter(|| test::black_box(scope.write(&metric, 1)));
    }

    #[bench]
    fn time_bench_write_count(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        let scope = sink.open_scope(false);
        b.iter(|| test::black_box(scope.write(&metric, 1)));
    }

    #[bench]
    fn time_bench_read_event(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Marker, "marker_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

    #[bench]
    fn time_bench_read_count(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(metric.reset()));
    }

}
