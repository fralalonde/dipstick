//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    let metrics = Statsd::send_to("localhost:8125")
        .expect("Connected")
        .sampled(Sampling::Random(0.2))
        .named("my_app")
        .metrics();

    let counter = metrics.counter("counter_a");

    loop {
        for i in 1..11 {
            counter.count(i);
        }
        std::thread::sleep(Duration::from_millis(3000));
    }
}
