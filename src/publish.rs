//! Publishes metrics values from a source to a sink.
//!
//! Publishing can be done on request:
//! ```
//! use dipstick::*;
//!
//! let (sink, source) = aggregate();
//! publish(&source, &log("aggregated"), publish::all_stats);
//! ```
//!
//! Publishing can be scheduled to run recurrently.
//! ```
//! use dipstick::*;
//! use std::time::Duration;
//!
//! let (sink, source) = aggregate();
//! let job = publish_every(Duration::from_millis(100), &source, &log("aggregated"), publish::all_stats);
//! // publish will go on until cancelled
//! job.cancel();
//! ```

use core::*;
use core::Kind::*;
use aggregate::AggregateSource;
use scores::ScoreType;
use scores::ScoreType::*;
use std::time::Duration;
use schedule::{schedule, CancelHandle};

/// Schedules the publisher to run at recurrent intervals
pub fn publish_every<E, M, S>(
    duration: Duration,
    source: AggregateSource,
    target: S,
    export: E,
) -> CancelHandle
where
    S: Sink<M> + 'static + Send + Sync,
    M: Clone + Send + Sync,
    E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
{
    schedule(duration, move || publish(&source, &target, &export))
}

/// Define and write metrics from aggregated scores to the target channel
/// If this is called repeatedly it can be a good idea to use the metric cache
/// to prevent new metrics from being created every time.
// TODO require ScopeMetrics instead of Sink
pub fn publish<E, M, S>(source: &AggregateSource, target: &S, export: &E)
where
    S: Sink<M>,
    M: Clone + Send + Sync,
    E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
{
    let publish_scope_fn = target.new_scope(false);
    source.for_each(|metric| {
        let snapshot = metric.reset();
        if snapshot.is_empty() {
            // no data was collected for this period
            // TODO repeat previous frame min/max ?
            // TODO update some canary metric ?
        } else {
            for score in snapshot {
                if let Some(ex) = export(metric.kind, &metric.name, score) {
                    let temp_metric = target.new_metric(ex.0, &ex.1.concat(), 1.0);
                    publish_scope_fn(Scope::Write(&temp_metric, ex.2));
                }
            }
        }
    });
    // TODO parameterize whether to keep ad-hoc metrics after publish
    source.cleanup();
    publish_scope_fn(Scope::Flush)
}

/// A predefined export strategy reporting all aggregated stats for all metric types.
/// Resulting stats are named by appending a short suffix to each metric's name.
pub fn all_stats(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match score {
        Count(hit) => Some((Counter, vec![name, ".count"], hit)),
        Sum(sum) => Some((kind, vec![name, ".sum"], sum)),
        Mean(mean) => Some((kind, vec![name, ".mean"], mean.round() as Value)),
        Max(max) => Some((Gauge, vec![name, ".max"], max)),
        Min(min) => Some((Gauge, vec![name, ".min"], min)),
        Rate(rate) => Some((Gauge, vec![name, ".rate"], rate.round() as Value))
    }
}

/// A predefined export strategy reporting the average value for every non-marker metric.
/// Marker metrics export their hit count instead.
///
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
pub fn average(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match kind {
        Marker => {
            match score {
                Count(count) => Some((Counter, vec![name], count)),
                _ => None,
            }
        }
        _ => {
            match score {
                Mean(avg) => Some((Gauge, vec![name], avg.round() as Value)),
                _ => None,
            }
        }
    }
}

/// A predefined single-stat-per-metric export strategy:
/// - Timers and Counters each export their sums
/// - Markers each export their hit count
/// - Gauges each export their average
///
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
pub fn summary(kind: Kind, name: &str, score: ScoreType) -> Option<(Kind, Vec<&str>, Value)> {
    match kind {
        Marker => {
            match score {
                Count(count) => Some((Counter, vec![name], count)),
                _ => None,
            }
        }
        Counter | Timer => {
            match score {
                Sum(sum) => Some((kind, vec![name], sum)),
                _ => None,
            }
        }
        Gauge => {
            match score {
                Mean(mean) => Some((Gauge, vec![name], mean.round() as Value)),
                _ => None,
            }
        }
    }
}
