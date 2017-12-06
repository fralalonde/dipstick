//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;

fn main() {
    let metrics = metrics(
        to_graphite("localhost:2003", "myapp.").expect("Could not connect to graphite")
    );

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep_ms(40);
    }
}
