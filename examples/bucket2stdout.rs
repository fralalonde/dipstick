//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds

extern crate dipstick;

use std::time::Duration;
use std::io;
use dipstick::*;

fn main() {
    let metrics = Bucket::new().add_naming("test");

    // Bucket::set_default_output(to_stdout());
    metrics.set_target(Stream::write_to(io::stdout()));

    metrics.flush_every(Duration::from_secs(3));

    let counter = metrics.counter("counter_a");
    let timer = metrics.timer("timer_a");
    let gauge = metrics.gauge("gauge_a");
    let marker = metrics.marker("marker_a");

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
