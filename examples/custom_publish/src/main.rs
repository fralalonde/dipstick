//! A demonstration of customization of exported aggregated metrics.
//! Using match on origin metric kind or score type to alter publication output.

extern crate dipstick;

use std::time::Duration;
use dipstick::*;

fn main() {
    // send application metrics to both aggregator and to sampling log
    let (to_aggregate, from_aggregate) = aggregate();

    let app_metrics = metrics(to_aggregate);

    // schedule aggregated metrics to be printed every 3 seconds
    let to_console = to_stdout();

    publish_every(Duration::from_secs(3), from_aggregate, to_console, |kind, name, score|
        match kind {
            // do not export gauge scores
            Kind::Gauge => None,

            _ => match score {
                // prepend and append to metric name
                ScoreType::HitCount(hit) => Some((Kind::Counter, vec!["name customized_with_prefix:", &name, " and a suffix: "], hit)),

                // scaling the score value and appending unit to name
                ScoreType::SumOfValues(sum) => Some((kind, vec![&name, "_millisecond"], sum * 1000)),

                // using the unmodified metric name
                ScoreType::AverageValue(avg) => Some((kind, vec![&name], avg)),
                _ => None /* do not export min and max */
            }

        }
    );

    let counter = app_metrics.counter("counter_a");
    let timer = app_metrics.timer("timer_b");
    let gauge = app_metrics.gauge("gauge_c");
    loop {
        counter.count(11);
        timer.interval_us(654654);
        gauge.value(3534);
    }

}
