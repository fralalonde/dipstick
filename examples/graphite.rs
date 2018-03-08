//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

//extern crate badlog;
extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
//    badlog::init(Some("info"));

    let metrics = app_metrics(
        to_graphite("localhost:2003").expect("Connecting")
            .with_namespace(&["my", "app"][..]));

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
