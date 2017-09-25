//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.
//! Metric caching allows re-use of the counter, skipping cost of redefining it on each use.

#[macro_use] extern crate dipstick;
extern crate scheduled_executor;

use std::thread::sleep;
use scheduled_executor::CoreExecutor;
use std::time::Duration;
use dipstick::*;

fn main() {
    let metrics = metrics(
        cache(1, (
            statsd("localhost:8125", "myapp.").expect("Could not connect to statsd"),
            print())));

    loop {
        metrics.counter("counter_a").count(123);
    }
}
