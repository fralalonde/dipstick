//! A sample application sending ad-hoc metrics to graphite.

use dipstick::*;
use std::time::Duration;

fn main() {
    let metrics = Graphite::send_to("localhost:2003")
        .expect("Connected")
        .named("my_app")
        .metrics();

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
