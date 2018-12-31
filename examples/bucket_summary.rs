//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds

extern crate dipstick;

use std::time::Duration;
use dipstick::*;
use std::io;

fn main() {

    let app_metrics = AtomicBucket::new();
    app_metrics.set_drain(Stream::write_to(io::stdout()));

    app_metrics.flush_every(Duration::from_secs(3));

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
