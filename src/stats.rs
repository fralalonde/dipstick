//! Definitions of standard aggregated statistic types and functions

use crate::input::InputKind;
use crate::name::MetricName;
use crate::MetricValue;

/// Possibly aggregated scores.
#[derive(Debug, Clone, Copy)]
pub enum ScoreType {
    /// Number of times the metric was used.
    Count(isize),
    /// Sum of metric values reported.
    Sum(isize),
    /// Biggest value observed.
    Max(isize),
    /// Smallest value observed.
    Min(isize),
    /// Average value (hit count / sum, non-atomic)
    Mean(f64),
    /// Mean rate (hit count / period length in seconds, non-atomic)
    Rate(f64),
}

/// A predefined export strategy reporting all aggregated stats for all metric types.
/// Resulting stats are named by appending a short suffix to each metric's name.
#[allow(dead_code)]
pub fn stats_all(
    kind: InputKind,
    name: MetricName,
    score: ScoreType,
) -> Option<(InputKind, MetricName, MetricValue)> {
    match score {
        ScoreType::Count(hit) => Some((InputKind::Counter, name.make_name("count"), hit)),
        ScoreType::Sum(sum) => Some((kind, name.make_name("sum"), sum)),
        ScoreType::Mean(mean) => Some((kind, name.make_name("mean"), mean.round() as MetricValue)),
        ScoreType::Max(max) => Some((InputKind::Gauge, name.make_name("max"), max)),
        ScoreType::Min(min) => Some((InputKind::Gauge, name.make_name("min"), min)),
        ScoreType::Rate(rate) => Some((
            InputKind::Gauge,
            name.make_name("rate"),
            rate.round() as MetricValue,
        )),
    }
}

/// A predefined export strategy reporting the average value for every non-marker metric.
/// Marker metrics export their hit count instead.
/// Since there is only one stat per metric, there is no risk of collision
/// and so exported stats copy their metric's name.
#[allow(dead_code)]
pub fn stats_average(
    kind: InputKind,
    name: MetricName,
    score: ScoreType,
) -> Option<(InputKind, MetricName, MetricValue)> {
    match kind {
        InputKind::Marker => match score {
            ScoreType::Count(count) => Some((InputKind::Counter, name, count)),
            _ => None,
        },
        _ => match score {
            ScoreType::Mean(avg) => Some((InputKind::Gauge, name, avg.round() as MetricValue)),
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
pub fn stats_summary(
    kind: InputKind,
    name: MetricName,
    score: ScoreType,
) -> Option<(InputKind, MetricName, MetricValue)> {
    match kind {
        InputKind::Marker => match score {
            ScoreType::Count(count) => Some((InputKind::Counter, name, count)),
            _ => None,
        },
        InputKind::Counter | InputKind::Timer => match score {
            ScoreType::Sum(sum) => Some((kind, name, sum)),
            _ => None,
        },
        InputKind::Gauge | InputKind::Level => match score {
            ScoreType::Mean(mean) => Some((InputKind::Gauge, name, mean.round() as MetricValue)),
            _ => None,
        },
    }
}
