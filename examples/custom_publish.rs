//! A demonstration of customization of exported aggregated metrics.
//! Using match on origin metric kind or score type to alter publication output.

extern crate dipstick;

use std::time::Duration;
use dipstick::*;

fn main() {
    fn custom_statistics(
        kind: Kind,
        name: &str,
        score: ScoreType,
    ) -> Option<(Kind, Vec<&str>, Value)> {
        match (kind, score) {
            // do not export gauge scores
            (Kind::Gauge, _) => None,

            // prepend and append to metric name
            (_, ScoreType::Count(count)) => Some((
                Kind::Counter,
                vec!["name customized_with_prefix:", &name, " and a suffix: "],
                count,
            )),

            // scaling the score value and appending unit to name
            (kind, ScoreType::Sum(sum)) => Some((kind, vec![&name, "_millisecond"], sum * 1000)),

            // using the unmodified metric name
            (kind, ScoreType::Mean(avg)) => Some((kind, vec![&name], avg.round() as u64)),

            // do not export min and max
            _ => None,
        }
    }

    // send application metrics to aggregator
    let to_aggregate = aggregate();

    default_aggregate_config(to_stdout());
    set_default_aggregate_statistics(custom_statistics);

    let app_metrics = metric_scope(to_aggregate);

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
