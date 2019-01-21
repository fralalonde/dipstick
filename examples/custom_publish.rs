//! A demonstration of customization of exported aggregated metrics.
//! Using match on origin metric kind or score type to alter publication output.

extern crate dipstick;

use std::time::Duration;
use dipstick::*;

fn main() {
    fn custom_statistics(
        kind: InputKind,
        mut name: MetricName,
        score: ScoreType,
    ) -> Option<(InputKind, MetricName, MetricValue)> {
        match (kind, score) {
            // do not export gauge scores
            (InputKind::Gauge, _) => None,

            // prepend and append to metric name
            (_, ScoreType::Count(count)) => {
                if let Some(last) = name.pop_back() {
                    Some((
                        InputKind::Counter,
                        name.append("customized_add_prefix")
                            .append(format!("{}_and_a_suffix", last)),
                        count,
                    ))
                } else {
                    None
                }
            },

            // scaling the score value and appending unit to name
            (kind, ScoreType::Sum(sum)) => Some((kind, name.append("per_thousand"), sum / 1000)),

            // using the unmodified metric name
            (kind, ScoreType::Mean(avg)) => Some((kind, name, avg.round() as MetricValue)),

            // do not export min and max
            _ => None,
        }
    }

    // send application metrics to aggregator
    AtomicBucket::default_drain(Stream::to_stderr());
    AtomicBucket::default_stats(custom_statistics);

    let app_metrics = AtomicBucket::new();

    // schedule aggregated metrics to be printed every 3 seconds
    app_metrics.flush_every(Duration::from_secs(3));

    let counter = app_metrics.counter("counter_a");
    let timer = app_metrics.timer("timer_b");
    let gauge = app_metrics.gauge("gauge_c");
    loop {
        counter.count(11);
        timer.interval_us(654654);
        gauge.value(3534);
    }
}
