//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    let different_type_metrics = app_metrics((
        // combine metrics of different types in a tuple
        to_statsd("localhost:8125").expect("connect"),
        to_stdout(),
    ));

    let same_type_metrics = app_metrics(
        &[
            // use slices to combine multiple metrics of the same type
            to_stdout().with_prefix("yeah"),
            to_stdout().with_prefix("ouch"),
            to_stdout().with_sampling_rate(0.5),
        ][..],
    );

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
