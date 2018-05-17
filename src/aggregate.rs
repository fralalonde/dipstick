//! Maintain aggregated metrics for deferred reporting,
//!
use core::*;
use scores::*;
use publish::*;

use std::fmt::Debug;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Aggregate metrics in memory.
/// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
/// Needs to be connected to a publish to be useful.
pub fn aggregate<E, M>(stat_fn: E, to_chain: Chain<M>) -> Chain<Aggregate>
where
    E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
    M: Clone + Send + Sync + Debug + 'static,
{
    let agg = Aggregator::with_capacity(1024, Arc::new(Publisher::new(stat_fn, to_chain)));
    let agg0 = agg.clone();

    Chain::new(
        move |kind, name, rate| agg.define_metric(kind, name, rate),
        move |_buffered| {
            let agg1 = agg0.clone();
            ControlScopeFn::new(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => agg1.write(metric, value),
                ScopeCmd::Flush => agg1.flush()
            })
        },
    )
}

/// Central aggregation structure.
/// Since `AggregateKey`s themselves contain scores, the aggregator simply maintains
/// a shared list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct Aggregator {
    inner: Arc<RwLock<InnerAggregator>>,
    publish: Arc<Publish>,
}

#[derive(Debug)]
struct InnerAggregator {
    metrics: HashMap<String, Arc<Scoreboard>>,
    period_start: Instant,
}

impl Aggregator {
    /// Build a new metric aggregation point with specified initial capacity of metrics to aggregate.
    pub fn with_capacity(size: usize, publish: Arc<Publish>) -> Aggregator {
        Aggregator {
            inner: Arc::new(RwLock::new(InnerAggregator {
                metrics: HashMap::with_capacity(size),
                period_start: Instant::now(),
            })),
            publish: publish.clone(),
        }
    }

    /// Discard scores for ad-hoc metrics.
    pub fn cleanup(&self) {
        let orphans: Vec<String> = self.inner.read().expect("Scores").metrics.iter()
            // is aggregator now the sole owner?
            .filter(|&(_k, v)| Arc::strong_count(v) == 1)
            .map(|(k, _v)| k.to_string())
            .collect();
        if !orphans.is_empty() {
            let remover = &mut self.inner.write().expect("Scores").metrics;
            orphans.iter().for_each(|k| {
                remover.remove(k);
            });
        }
    }

    fn define_metric(&self, kind: Kind, name: &str, _sampling: Rate) -> Aggregate {
        self.inner.write().expect("Scores").metrics
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Scoreboard::new(kind, name.to_string())))
            .clone()
    }

    #[inline]
    fn write(&self, metric: &Aggregate, value: Value) {
        metric.update(value)
    }

    fn flush(&self) {
        let mut inner = self.inner.write().expect("Scores");
        let now = Instant::now();
        let duration = now - inner.period_start;
        let duration_seconds = (duration.subsec_nanos() / 1_000_000_000) as f64 + duration.as_secs() as f64;
        let snapshot: Vec<ScoreSnapshot> = inner.metrics.values().flat_map(|score| score.reset(duration_seconds)).collect();
//        snapshot.push((Kind::Counter, "_duration_ms".to_string(), vec![ScoreType::Sum((duration_seconds * 1000.0) as u64)]));
        inner.period_start = now;
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
        b.iter(|| test::black_box(metric.reset(1.0)));
    }

    #[bench]
    fn time_bench_read_count(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metric = sink.define_metric(Counter, "count_a", 1.0);
        b.iter(|| test::black_box(metric.reset(1.0)));
    }

}
