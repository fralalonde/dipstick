//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds and
//! printing complete stats every 10 seconds.

extern crate dipstick;

use std::time::Duration;
use dipstick::*;

fn main() {
    let (to_quick_aggregate, from_quick_aggregate) = aggregate();
    let (to_slow_aggregate, from_slow_aggregate) = aggregate();

    let app_metrics = metrics((to_quick_aggregate, to_slow_aggregate));

    publish_every(
        Duration::from_secs(3),
        from_quick_aggregate,
        to_stdout(),
        summary,
    );

    publish_every(
        Duration::from_secs(10),
        from_slow_aggregate,
        to_stdout(),
        all_stats,
    );

    let counter = app_metrics.counter("counter_a");
    let timer = app_metrics.timer("timer_a");
    let gauge = app_metrics.gauge("gauge_a");
    let marker = app_metrics.marker("marker_a");

    loop {
        // add counts forever, non-stop
        counter.count(11);
        counter.count(12);
        counter.count(13);

        timer.interval_us(11_000_000);
        timer.interval_us(12_000_000);
        timer.interval_us(13_000_000);

        gauge.value(11);
        gauge.value(12);
        gauge.value(13);

        marker.mark();
    }

}
