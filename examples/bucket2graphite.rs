//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    // adding a name to the stats
    let bucket = AtomicBucket::new().named("test");

    // adding two names to Graphite output
    // metrics will be prefixed with "machine1.application.test"
    bucket.drain(
        Graphite::send_to("localhost:2003")
            .expect("Socket")
            .named("machine1")
            .add_name("application"),
    );

    bucket.flush_every(Duration::from_secs(3));

    let counter = bucket.counter("counter_a");
    let timer = bucket.timer("timer_a");
    let gauge = bucket.gauge("gauge_a");
    let marker = bucket.marker("marker_a");

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
