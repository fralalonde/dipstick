//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;

fn main() {
    let metrics = metrics(
        //! Metric caching allows re-use of the counter, skipping cost of redefining it on each use.
        cache(1, (
            statsd("localhost:8125", "myapp.").expect("Could not connect to statsd"),
            print())));

    loop {
        metrics.counter("counter_a").count(123);
    }
}
