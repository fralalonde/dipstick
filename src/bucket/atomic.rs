//! Maintain aggregated metrics for deferred reporting,

use core::attributes::{Attributes, WithAttributes, Prefixed};
use core::name::{MetricName};
use core::input::{InputKind, InputScope, InputMetric};
use core::output::{OutputDyn, OutputScope, OutputMetric, Output, output_none};
use core::clock::TimeHandle;
use core::{MetricValue, Flush};
use bucket::{ScoreType, stats_summary};
use bucket::ScoreType::*;
use core::error;

use std::mem;
use std::isize;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicIsize;
use std::sync::atomic::Ordering::*;
use std::sync::{Arc};

#[cfg(not(feature="parking_lot"))]
use std::sync::{RwLock};

#[cfg(feature="parking_lot")]
use parking_lot::{RwLock};

use std::fmt;
use std::borrow::Borrow;

/// A function type to transform aggregated scores into publishable statistics.
pub type StatsFn = Fn(InputKind, MetricName, ScoreType) -> Option<(InputKind, MetricName, MetricValue)> + Send + Sync + 'static;

fn initial_stats() -> &'static StatsFn {
    &stats_summary
}

fn initial_drain() -> Arc<OutputDyn + Send + Sync> {
    Arc::new(output_none())
}

lazy_static! {
    static ref DEFAULT_AGGREGATE_STATS: RwLock<Arc<StatsFn>> = RwLock::new(Arc::new(initial_stats()));

    static ref DEFAULT_AGGREGATE_OUTPUT: RwLock<Arc<OutputDyn + Send + Sync>> = RwLock::new(initial_drain());
}

/// Central aggregation structure.
/// Maintains a list of metrics for enumeration when used as source.
#[derive(Debug, Clone)]
pub struct AtomicBucket {
    attributes: Attributes,
    inner: Arc<RwLock<InnerAtomicBucket>>,
}

struct InnerAtomicBucket {
    metrics: BTreeMap<MetricName, Arc<AtomicScores>>,
    period_start: TimeHandle,
    stats: Option<Arc<Fn(InputKind, MetricName, ScoreType)
        -> Option<(InputKind, MetricName, MetricValue)> + Send + Sync + 'static>>,
    drain: Option<Arc<OutputDyn + Send + Sync + 'static>>,
    publish_metadata: bool,
}

impl fmt::Debug for InnerAtomicBucket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "metrics: {:?}", self.metrics)?;
        write!(f, "period_start: {:?}", self.period_start)
    }
}

lazy_static! {
    static ref PERIOD_LENGTH: MetricName = "_period_length".into();
}

impl InnerAtomicBucket {

    pub fn flush(&mut self) -> error::Result<()> {
        let pub_scope = match self.drain {
            Some(ref out) => out.output_dyn(),
            None => read_lock!(DEFAULT_AGGREGATE_OUTPUT).output_dyn(),
        };

        self.flush_to(pub_scope.borrow())?;

        // all metrics published!
        // purge: if bucket is the last owner of the metric, remove it
        // TODO parameterize whether to keep ad-hoc metrics after publish
        let mut purged = self.metrics.clone();
        self.metrics.iter()
            .filter(|&(_k, v)| Arc::strong_count(v) == 1)
            .map(|(k, _v)| k)
            .for_each(|k| { purged.remove(k); });
        self.metrics = purged;

        Ok(())
    }

    /// Take a snapshot of aggregated values and reset them.
    /// Compute stats on captured values using assigned or default stats function.
    /// Write stats to assigned or default output.
    pub fn flush_to(&mut self, target: &OutputScope) -> error::Result<()> {

        let now = TimeHandle::now();
        let duration_seconds = self.period_start.elapsed_us() as f64 / 1_000_000.0;
        self.period_start = now;

        let mut snapshot: Vec<(&MetricName, InputKind, Vec<ScoreType>)> = self.metrics.iter()
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
            Ok(())
        } else {
            // TODO add switch for metadata such as PERIOD_LENGTH
            if self.publish_metadata {
                snapshot.push((&PERIOD_LENGTH, InputKind::Timer, vec![Sum((duration_seconds * 1000.0) as isize)]));
            }

            let stats_fn = match self.stats {
                Some(ref stats_fn) => stats_fn.clone(),
                None => read_lock!(DEFAULT_AGGREGATE_STATS).clone(),
            };

            for metric in snapshot {
                for score in metric.2 {
                    let filtered = stats_fn(metric.1, metric.0.clone(), score);
                    if let Some((kind, name, value)) = filtered {
                        let metric: OutputMetric = target.new_metric(name, kind);
                        // TODO provide some bucket context through labels?
                        metric.write(value, labels![])
                    }
                }
            }
            target.flush()
        }
    }

}

impl<S: AsRef<str>> From<S> for AtomicBucket {
    fn from(name: S) -> AtomicBucket {
        AtomicBucket::new().named(name.as_ref())
    }
}

impl AtomicBucket {
    /// Build a new atomic bucket.
    pub fn new() -> AtomicBucket {
        AtomicBucket {
            attributes: Attributes::default(),
            inner: Arc::new(RwLock::new(InnerAtomicBucket {
                metrics: BTreeMap::new(),
                period_start: TimeHandle::now(),
                stats: None,
                drain: None,
                // TODO add API toggle for metadata publish
                publish_metadata: false,
            }))
        }
    }

    /// Set the default aggregated metrics statistics generator.
    pub fn default_stats<F>(func: F)
        where
            F: Fn(InputKind, MetricName, ScoreType) -> Option<(InputKind, MetricName, MetricValue)> + Send + Sync + 'static
    {
        *write_lock!(DEFAULT_AGGREGATE_STATS) = Arc::new(func)
    }

    /// Revert the default aggregated metrics statistics generator to the default `stats_summary`.
    pub fn unset_default_stats() {
        *write_lock!(DEFAULT_AGGREGATE_STATS) = Arc::new(initial_stats())
    }

    /// Set the default bucket aggregated metrics flush output.
    pub fn default_drain(default_config: impl Output + Send + Sync + 'static) {
        *write_lock!(DEFAULT_AGGREGATE_OUTPUT) = Arc::new(default_config);
    }

    /// Revert the default bucket aggregated metrics flush output.
    pub fn unset_default_drain() {
        *write_lock!(DEFAULT_AGGREGATE_OUTPUT) = initial_drain()
    }

    /// Set this bucket's statistics generator.
    #[deprecated(since="0.7.2", note="Use stats()")]
    pub fn set_stats<F>(&self, func: F)
        where
            F: Fn(InputKind, MetricName, ScoreType) -> Option<(InputKind, MetricName, MetricValue)> + Send + Sync + 'static
    {
        self.stats(func)
    }

    /// Set this bucket's statistics generator.
    pub fn stats<F>(&self, func: F)
        where
            F: Fn(InputKind, MetricName, ScoreType) -> Option<(InputKind, MetricName, MetricValue)> + Send + Sync + 'static
    {
        write_lock!(self.inner).stats = Some(Arc::new(func))
    }

    /// Revert this bucket's statistics generator to the default stats.
    pub fn unset_stat(&self) {
        write_lock!(self.inner).stats = None
    }

    /// Set this bucket's aggregated metrics flush output.
    #[deprecated(since="0.7.2", note="Use drain()")]
    pub fn set_drain(&self, new_drain: impl Output + Send + Sync + 'static) {
        self.drain(new_drain)
    }

    /// Set this bucket's aggregated metrics flush output.
    pub fn drain(&self, new_drain: impl Output + Send + Sync + 'static) {
        write_lock!(self.inner).drain = Some(Arc::new(new_drain))
    }

    /// Revert this bucket's flush target to the default output.
    pub fn unset_drain(&self) {
        write_lock!(self.inner).drain = None
    }

    /// Immediately flush the bucket's metrics to the specified scope and stats.
    pub fn flush_to(&self, publish_scope: &OutputScope) -> error::Result<()> {
        let mut inner = write_lock!(self.inner);
        inner.flush_to(publish_scope)
    }

}

impl InputScope for AtomicBucket {
    /// Lookup or create scores for the requested metric.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let scores = write_lock!(self.inner)
            .metrics
            .entry(self.prefix_append(name))
            .or_insert_with(|| Arc::new(AtomicScores::new(kind)))
            .clone();
        InputMetric::new(move |value, _labels| scores.update(value))
    }
}

impl Flush for AtomicBucket {
    /// Collect and reset aggregated data.
    /// Publish statistics
    fn flush(&self) -> error::Result<()> {
        let mut inner = write_lock!(self.inner);
        inner.flush()
    }
}

impl WithAttributes for AtomicBucket {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

const HIT: usize = 0;
const SUM: usize = 1;
const MAX: usize = 2;
const MIN: usize = 3;
const SCORES_LEN: usize = 4;

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
struct AtomicScores {
    /// The kind of metric
    kind: InputKind,
    /// The actual recorded metric scores
    scores: [AtomicIsize; SCORES_LEN],
}

impl AtomicScores {
    /// Create new scores to track summary values of a metric
    pub fn new(kind: InputKind) -> Self {
        AtomicScores {
            kind,
            scores: unsafe { mem::transmute(AtomicScores::blank()) },
        }
    }

    /// Returns the metric's kind.
    pub fn metric_kind(&self) -> InputKind {
        self.kind
    }

    #[inline]
    fn blank() -> [isize; SCORES_LEN] {
        [0, 0, isize::MIN, isize::MAX]
    }

    /// Update scores with new value
    pub fn update(&self, value: MetricValue) {
        // TODO detect & report any concurrent updates / resets for measurement of contention
        // Count is tracked for all metrics
        self.scores[HIT].fetch_add(1, Relaxed);
        match self.kind {
            InputKind::Marker => {}
            InputKind::Level => {
                // Level min & max apply to the _sum_ of values
                // fetch_add only returns the previous sum, so min & max trail behind by one operation
                // instead, pickup the slack by comparing again with the final sum upon `snapshot`
                // this is to avoid making an extra load() on every value
                let prev_sum = self.scores[SUM].fetch_add(value, Relaxed);
                swap_if(&self.scores[MAX], prev_sum, |new, current| new > current);
                swap_if(&self.scores[MIN], prev_sum, |new, current| new < current);
            }
            InputKind::Counter | InputKind::Timer | InputKind::Gauge => {
                // gauges are non cumulative, but we keep the sum to compute the mean
                // TODO use #![feature(atomic_min_max)] when stabilized
                self.scores[SUM].fetch_add(value, Relaxed);
                swap_if(&self.scores[MAX], value, |new, current| new > current);
                swap_if(&self.scores[MIN], value, |new, current| new < current);
            }
        }
    }

    /// Reset scores to zero, return previous values
    fn snapshot(&self, scores: &mut [isize; 4]) -> bool {
        // NOTE copy timestamp, count AND sum _before_ testing for data to reduce concurrent discrepancies
        scores[HIT] = self.scores[HIT].swap(0, AcqRel);
        scores[SUM] = self.scores[SUM].swap(0, AcqRel);

        // if hit count is zero, no values were recorded.
        if scores[HIT] == 0 {
            return false;
        }

        scores[MAX] = self.scores[MAX].swap(isize::MIN, AcqRel);
        scores[MIN] = self.scores[MIN].swap(isize::MAX, AcqRel);

        if self.kind == InputKind::Level {
            // fetch_add only returns the previous sum, so min & max trail behind by one operation
            // pickup the slack by comparing one last time against the final sum
            if scores[SUM] > scores[MAX] {
                scores[MAX] = scores[SUM];
            }
            if scores[SUM] < scores[MIN] {
                scores[MIN] = scores[SUM];
            }
        }

        true
    }

    /// Map raw scores (if any) to applicable statistics
    pub fn reset(&self, duration_seconds: f64) -> Option<Vec<ScoreType>> {
        let mut scores = AtomicScores::blank();
        if self.snapshot(&mut scores) {

            let mut snapshot = Vec::new();
            match self.kind {
                InputKind::Marker => {
                    snapshot.push(Count(scores[HIT]));
                    snapshot.push(Rate(scores[HIT] as f64 / duration_seconds))
                }
                InputKind::Gauge => {
                    snapshot.push(Max(scores[MAX]));
                    snapshot.push(Min(scores[MIN]));
                    snapshot.push(Mean(scores[SUM] as f64 / scores[HIT] as f64));
                }
                InputKind::Timer => {
                    snapshot.push(Count(scores[HIT]));
                    snapshot.push(Sum(scores[SUM]));

                    snapshot.push(Max(scores[MAX]));
                    snapshot.push(Min(scores[MIN]));
                    snapshot.push(Mean(scores[SUM] as f64 / scores[HIT] as f64));
                    // timer rate uses the COUNT of timer calls per second (not SUM)
                    snapshot.push(Rate(scores[HIT] as f64 / duration_seconds))
                }
                InputKind::Counter => {
                    snapshot.push(Count(scores[HIT]));
                    snapshot.push(Sum(scores[SUM]));

                    snapshot.push(Max(scores[MAX]));
                    snapshot.push(Min(scores[MIN]));
                    snapshot.push(Mean(scores[SUM] as f64 / scores[HIT] as f64));
                    // counter rate uses the SUM of values per second (e.g. to get bytes/s)
                    snapshot.push(Rate(scores[SUM] as f64 / duration_seconds))
                }
                InputKind::Level => {
                    snapshot.push(Count(scores[HIT]));
                    snapshot.push(Sum(scores[SUM]));

                    snapshot.push(Max(scores[MAX]));
                    snapshot.push(Min(scores[MIN]));
                    snapshot.push(Mean(scores[SUM] as f64 / scores[HIT] as f64));
                    // counter rate uses the SUM of values per second (e.g. to get bytes/s)
                    snapshot.push(Rate(scores[SUM] as f64 / duration_seconds))
                }
            }
            Some(snapshot)
        } else {
            None
        }
    }
}

/// Spinlock until success or clear loss to concurrent update.
#[inline]
fn swap_if(counter: &AtomicIsize, new_value: isize, compare: fn(isize, isize) -> bool) {
    let mut current = counter.load(Acquire);
    while compare(new_value, current) {
        if counter.compare_and_swap(current, new_value, Release) == new_value {
            // update successful
            break;
        }
        // race detected, retry
        current = counter.load(Acquire);
    }
}

#[cfg(feature = "bench")]
mod bench {

    use test;
    use super::*;

    #[bench]
    fn update_marker(b: &mut test::Bencher) {
        let metric = AtomicScores::new(InputKind::Marker);
        b.iter(|| test::black_box(metric.update(1)));
    }

    #[bench]
    fn update_count(b: &mut test::Bencher) {
        let metric = AtomicScores::new(InputKind::Counter);
        b.iter(|| test::black_box(metric.update(4)));
    }

    #[bench]
    fn empty_snapshot(b: &mut test::Bencher) {
        let metric = AtomicScores::new(InputKind::Counter);
        let scores = &mut AtomicScores::blank();
        b.iter(|| test::black_box(metric.snapshot(scores)));
    }

    #[bench]
    fn aggregate_marker(b: &mut test::Bencher) {
        let sink = AtomicBucket::new();
        let metric = sink.new_metric("event_a".into(), InputKind::Marker);
        b.iter(|| test::black_box(metric.write(1, labels![])));
    }

    #[bench]
    fn aggregate_counter(b: &mut test::Bencher) {
        let sink = AtomicBucket::new();
        let metric = sink.new_metric("count_a".into(), InputKind::Counter);
        b.iter(|| test::black_box(metric.write(1, labels![])));
    }

}

#[cfg(test)]
mod test {
    use super::*;
    use bucket::{stats_all, stats_average, stats_summary};

    use core::clock::{mock_clock_advance, mock_clock_reset};
    use output::map::StatsMap;

    use std::time::Duration;
    use std::collections::BTreeMap;

    fn make_stats(stats_fn: &'static StatsFn) -> BTreeMap<String, MetricValue> {
        mock_clock_reset();

        let metrics = AtomicBucket::new().named("test");
        metrics.stats(stats_fn);

        let counter = metrics.counter("counter_a");
        let counter_b = metrics.counter("counter_b");
        let timer = metrics.timer("timer_a");
        let gauge = metrics.gauge("gauge_a");
        let level = metrics.level("level_a");
        let marker = metrics.marker("marker_a");

        marker.mark();
        marker.mark();
        marker.mark();

        counter.count(10);
        counter.count(20);

        counter_b.count(9);
        counter_b.count(18);
        counter_b.count(3);

        timer.interval_us(10_000_000);
        timer.interval_us(20_000_000);

        gauge.value(10);
        gauge.value(20);

        level.adjust(789);
        level.adjust(-7789);
        level.adjust(77788);

        mock_clock_advance(Duration::from_secs(3));

        let map = StatsMap::default();
        metrics.flush_to(&map).unwrap();
        map.into()
    }

    #[test]
    fn external_aggregate_all_stats() {
        let map = make_stats(&stats_all);

        assert_eq!(map["test.counter_a.count"], 2);
        assert_eq!(map["test.counter_a.sum"], 30);
        assert_eq!(map["test.counter_a.mean"], 15);
        assert_eq!(map["test.counter_a.min"], 10);
        assert_eq!(map["test.counter_a.max"], 20);
        assert_eq!(map["test.counter_a.rate"], 10);

        assert_eq!(map["test.counter_b.count"], 3);
        assert_eq!(map["test.counter_b.sum"], 30);
        assert_eq!(map["test.counter_b.mean"], 10);
        assert_eq!(map["test.counter_b.min"], 3);
        assert_eq!(map["test.counter_b.max"], 18);
        assert_eq!(map["test.counter_b.rate"], 10);

        assert_eq!(map["test.timer_a.count"], 2);
        assert_eq!(map["test.timer_a.sum"], 30_000_000);
        assert_eq!(map["test.timer_a.min"], 10_000_000);
        assert_eq!(map["test.timer_a.max"], 20_000_000);
        assert_eq!(map["test.timer_a.mean"], 15_000_000);
        assert_eq!(map["test.timer_a.rate"], 1);

        assert_eq!(map["test.gauge_a.mean"], 15);
        assert_eq!(map["test.gauge_a.min"], 10);
        assert_eq!(map["test.gauge_a.max"], 20);

        assert_eq!(map["test.level_a.mean"], 23596);
        assert_eq!(map["test.level_a.min"], -7000);
        assert_eq!(map["test.level_a.max"], 70788);

        assert_eq!(map["test.marker_a.count"], 3);
        assert_eq!(map["test.marker_a.rate"], 1);
    }

    #[test]
    fn external_aggregate_summary() {
        let map = make_stats(&stats_summary);

        assert_eq!(map["test.counter_a"], 30);
        assert_eq!(map["test.counter_b"], 30);
        assert_eq!(map["test.level_a"], 23596);
        assert_eq!(map["test.timer_a"], 30_000_000);
        assert_eq!(map["test.gauge_a"], 15);
        assert_eq!(map["test.marker_a"], 3);
    }

    #[test]
    fn external_aggregate_average() {
        let map = make_stats(&stats_average);

        assert_eq!(map["test.counter_a"], 15);
        assert_eq!(map["test.counter_b"], 10);
        assert_eq!(map["test.level_a"], 23596);
        assert_eq!(map["test.timer_a"], 15_000_000);
        assert_eq!(map["test.gauge_a"], 15);
        assert_eq!(map["test.marker_a"], 3);
    }
}
