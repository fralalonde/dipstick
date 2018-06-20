//! Maintain aggregated metrics for deferred reporting,
//!
use core::{Kind, Value, Name, WithName, NO_METRIC_OUTPUT, Input, OutputDyn, Metric, WithAttributes, Attributes};
use clock::TimeHandle;
use core::Kind::*;
use error;

use scores::{ScoreType, Scoreboard};
use scores::ScoreType::*;

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

/// A function type to transform aggregated scores into publishable statistics.
pub type StatsFn = Fn(Kind, Name, ScoreType) -> Option<(Kind, Name, Value)> + Send + Sync + 'static;

fn initial_stats() -> &'static StatsFn {
    &summary
}

fn initial_output() -> Arc<OutputDyn + Send + Sync> {
    NO_METRIC_OUTPUT.clone()
}

lazy_static! {
    static ref DEFAULT_AGGREGATE_STATS: RwLock<Arc<StatsFn>> = RwLock::new(Arc::new(initial_stats()));

    static ref DEFAULT_AGGREGATE_OUTPUT: RwLock<Arc<OutputDyn + Send + Sync>> = RwLock::new(initial_output());
}

/// Create a new metric aggregating bucket.
pub fn to_bucket() -> Bucket {
    Bucket::new()
}

/// Central aggregation structure.
/// Maintains a list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct Bucket {
    attributes: Attributes,
    inner: Arc<RwLock<InnerBucket>>,
}

#[derive(Derivative)]
#[derivative(Debug)]
struct InnerBucket {
    metrics: BTreeMap<Name, Arc<Scoreboard>>,
    period_start: TimeHandle,
    #[derivative(Debug = "ignore")]
    stats: Option<Arc<Fn(Kind, Name, ScoreType)
        -> Option<(Kind, Name, Value)> + Send + Sync + 'static>>,
    #[derivative(Debug = "ignore")]
    output: Option<Arc<OutputDyn + Send + Sync + 'static>>,
    publish_metadata: bool,
}

lazy_static! {
    static ref PERIOD_LENGTH: Name = "_period_length".into();
}

impl InnerBucket {
    /// Take a snapshot of aggregated values and reset them.
    /// Compute stats on captured values using assigned or default stats function.
    /// Write stats to assigned or default output.
    pub fn flush_to(&mut self, publish_scope: &Input, stats_fn: &StatsFn) {

        let now = TimeHandle::now();
        let duration_seconds = self.period_start.elapsed_us() as f64 / 1_000_000.0;
        self.period_start = now;

        let mut snapshot: Vec<(&Name, Kind, Vec<ScoreType>)> = self.metrics.iter()
            .flat_map(|(name, scores)| if let Some(values) = scores.reset(duration_seconds) {
                Some((name, scores.metric_kind(), values))
            } else {
                None
            })
            .collect();

        if snapshot.is_empty() {
            // no data was collected for this period
            // TODO repeat previous frame min/max ?
            // TODO update some canary metric ?
        } else {
            // TODO add switch for metadata such as PERIOD_LENGTH
            if self.publish_metadata {
                snapshot.push((&PERIOD_LENGTH, Timer, vec![Sum((duration_seconds * 1000.0) as u64)]));
            }
            for metric in snapshot {
                for score in metric.2 {
                    let filtered = (stats_fn)(metric.1, metric.0.clone(), score);
                    if let Some((kind, name, value)) = filtered {
                        let metric: Metric = publish_scope.new_metric(name, kind);
                        (metric)(value)
                    }
                }
            }
        }
    }

}

impl<S: AsRef<str>> From<S> for Bucket {
    fn from(name: S) -> Bucket {
        Bucket::new().add_name(name.as_ref())
    }
}

impl Bucket {
    /// Build a new metric aggregation
    pub fn new() -> Bucket {
        Bucket {
            attributes: Attributes::default(),
            inner: Arc::new(RwLock::new(InnerBucket {
                metrics: BTreeMap::new(),
                period_start: TimeHandle::now(),
                stats: None,
                output: None,
                publish_metadata: false,
            }))
        }
    }

    /// Set the default aggregated metrics statistics generator.
    pub fn set_default_stats<F>(func: F)
        where
            F: Fn(Kind, Name, ScoreType) -> Option<(Kind, Name, Value)> + Send + Sync + 'static
    {
        *DEFAULT_AGGREGATE_STATS.write().unwrap() = Arc::new(func)
    }

    /// Remove any global customization of the default aggregation statistics.
    pub fn unset_default_stats() {
        *DEFAULT_AGGREGATE_STATS.write().unwrap() = Arc::new(initial_stats())
    }

    /// Install a new receiver for all aggregateed metrics, replacing any previous receiver.
    pub fn set_default_output(default_config: impl OutputDyn + Send + Sync + 'static) {
        *DEFAULT_AGGREGATE_OUTPUT.write().unwrap() = Arc::new(default_config);
    }

    /// Install a new receiver for all aggregateed metrics, replacing any previous receiver.
    pub fn unset_default_output() {
        *DEFAULT_AGGREGATE_OUTPUT.write().unwrap() = initial_output()
    }

    /// Set the default aggregated metrics statistics generator.
    pub fn set_stats<F>(&self, func: F)
        where
            F: Fn(Kind, Name, ScoreType) -> Option<(Kind, Name, Value)> + Send + Sync + 'static
    {
        self.inner.write().expect("Aggregator").stats = Some(Arc::new(func))
    }

    /// Set the default aggregated metrics statistics generator.
    pub fn unset_stats<F>(&self) {
        self.inner.write().expect("Aggregator").stats = None
    }

    /// Install a new receiver for all aggregated metrics, replacing any previous receiver.
    pub fn set_output(&self, new_config: impl OutputDyn + Send + Sync + 'static) {
        self.inner.write().expect("Aggregator").output = Some(Arc::new(new_config))
    }

    /// Install a new receiver for all aggregated metrics, replacing any previous receiver.
    pub fn unset_output(&self) {
        self.inner.write().expect("Aggregator").output = None
    }

    /// Flush the aggregator scores using the specified scope and stats.
    pub fn flush_to(&self, publish_scope: &Input, stats_fn: &StatsFn) {
        let mut inner = self.inner.write().expect("Aggregator");
        inner.flush_to(publish_scope, stats_fn);
    }

//    /// Discard scores for ad-hoc metrics.
//    pub fn cleanup(&self) {
//        let orphans: Vec<Name> = self.inner.read().expect("Aggregator").metrics.iter()
//            // is aggregator now the sole owner?
//            // TODO use weak ref + impl Drop to mark abandoned metrics (see dispatch)
//            .filter(|&(_k, v)| Arc::strong_count(v) == 1)
//            .map(|(k, _v)| k.to_string())
//            .collect();
//        if !orphans.is_empty() {
//            let remover = &mut self.inner.write().unwrap().metrics;
//            orphans.iter().for_each(|k| {
//                remover.remove(k);
//            });
//        }
//    }

}

impl Input for Bucket {
    /// Lookup or create a scoreboard for the requested metric.
    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let scoreb = self.inner
            .write()
            .expect("Aggregator")
            .metrics
            .entry(self.qualified_name(name))
            .or_insert_with(|| Arc::new(Scoreboard::new(kind)))
            .clone();
        Metric::new(move |value| scoreb.update(value))
    }

    /// Collect and reset aggregated data.
    /// Publish statistics
    fn flush(&self) -> error::Result<()> {
        let mut inner = self.inner.write().expect("Aggregator");

        let stats_fn = match &inner.stats {
            &Some(ref stats_fn) => stats_fn.clone(),
            &None => DEFAULT_AGGREGATE_STATS.read().unwrap().clone(),
        };

        let pub_scope = match &inner.output {
            &Some(ref out) => out.new_input_dyn(),
            &None => DEFAULT_AGGREGATE_OUTPUT.read().unwrap().new_input_dyn(),
        };

        inner.flush_to(pub_scope.as_ref(), stats_fn.as_ref());

        // TODO parameterize whether to keep ad-hoc metrics after publish
        // source.cleanup();
        pub_scope.flush()
    }
}

impl WithAttributes for Bucket {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// A predefined export strategy reporting all aggregated stats for all metric types.
/// Resulting stats are named by appending a short suffix to each metric's name.
#[allow(dead_code)]
pub fn all_stats(kind: Kind, name: Name, score: ScoreType) -> Option<(Kind, Name, Value)> {
    match score {
        Count(hit) => Some((Counter, name.add_name("count"), hit)),
        Sum(sum) => Some((kind, name.add_name("sum"), sum)),
        Mean(mean) => Some((kind, name.add_name("mean"), mean.round() as Value)),
        Max(max) => Some((Gauge, name.add_name("max"), max)),
        Min(min) => Some((Gauge, name.add_name("min"), min)),
        Rate(rate) => Some((Gauge, name.add_name("rate"), rate.round() as Value)),
    }
}

/// A predefined export strategy reporting the average value for every non-marker metric.
/// Marker metrics export their hit count instead.
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
#[allow(dead_code)]
pub fn average(kind: Kind, name: Name, score: ScoreType) -> Option<(Kind, Name, Value)> {
    match kind {
        Marker => match score {
            Count(count) => Some((Counter, name, count)),
            _ => None,
        },
        _ => match score {
            Mean(avg) => Some((Gauge, name, avg.round() as Value)),
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
#[allow(dead_code)]
pub fn summary(kind: Kind, name: Name, score: ScoreType) -> Option<(Kind, Name, Value)> {
    match kind {
        Marker => match score {
            Count(count) => Some((Counter, name, count)),
            _ => None,
        },
        Counter | Timer => match score {
            Sum(sum) => Some((kind, name, sum)),
            _ => None,
        },
        Gauge => match score {
            Mean(mean) => Some((Gauge, name, mean.round() as Value)),
            _ => None,
        },
    }
}

#[cfg(feature = "bench")]
mod bench {

    use test;
    use core::*;
    use bucket::Bucket;

    #[bench]
    fn aggregate_marker(b: &mut test::Bencher) {
        let sink = Bucket::new();
        let metric = sink.new_metric("event_a".into(), Kind::Marker);
        b.iter(|| test::black_box(metric.write(1)));
    }

    #[bench]
    fn aggregate_counter(b: &mut test::Bencher) {
        let sink = Bucket::new();
        let metric = sink.new_metric("count_a".into(), Kind::Counter);
        b.iter(|| test::black_box(metric.write(1)));
    }

}

#[cfg(test)]
mod test {
    use core::*;
    use bucket::{Bucket, all_stats, summary, average, StatsFn};
    use clock::{mock_clock_advance, mock_clock_reset};
    use map::StatsMap;

    use std::time::Duration;
    use std::collections::BTreeMap;

    fn make_stats(stats_fn: &StatsFn) -> BTreeMap<String, Value> {
        mock_clock_reset();

        let metrics = Bucket::new().add_name("test");

        let counter = metrics.counter("counter_a");
        let timer = metrics.timer("timer_a");
        let gauge = metrics.gauge("gauge_a");
        let marker = metrics.marker("marker_a");

        marker.mark();
        marker.mark();
        marker.mark();

        counter.count(10);
        counter.count(20);

        timer.interval_us(10_000_000);
        timer.interval_us(20_000_000);

        gauge.value(10);
        gauge.value(20);

        mock_clock_advance(Duration::from_secs(3));

        // TODO expose & use flush_to()
        let stats = StatsMap::new();
        metrics.flush_to(&stats, stats_fn);
        stats.into()
    }

    #[test]
    fn external_aggregate_all_stats() {
        let map = make_stats(&all_stats);

        assert_eq!(map["test.counter_a.count"], 2);
        assert_eq!(map["test.counter_a.sum"], 30);
        assert_eq!(map["test.counter_a.mean"], 15);
        assert_eq!(map["test.counter_a.rate"], 10);

        assert_eq!(map["test.timer_a.count"], 2);
        assert_eq!(map["test.timer_a.sum"], 30_000_000);
        assert_eq!(map["test.timer_a.min"], 10_000_000);
        assert_eq!(map["test.timer_a.max"], 20_000_000);
        assert_eq!(map["test.timer_a.mean"], 15_000_000);
        assert_eq!(map["test.timer_a.rate"], 1);

        assert_eq!(map["test.gauge_a.mean"], 15);
        assert_eq!(map["test.gauge_a.min"], 10);
        assert_eq!(map["test.gauge_a.max"], 20);

        assert_eq!(map["test.marker_a.count"], 3);
        assert_eq!(map["test.marker_a.rate"], 1);
    }

    #[test]
    fn external_aggregate_summary() {
        let map = make_stats(&summary);

        assert_eq!(map["test.counter_a"], 30);
        assert_eq!(map["test.timer_a"], 30_000_000);
        assert_eq!(map["test.gauge_a"], 15);
        assert_eq!(map["test.marker_a"], 3);
    }

    #[test]
    fn external_aggregate_average() {
        let map = make_stats(&average);

        assert_eq!(map["test.counter_a"], 15);
        assert_eq!(map["test.timer_a"], 15_000_000);
        assert_eq!(map["test.gauge_a"], 15);
        assert_eq!(map["test.marker_a"], 3);
    }
}
