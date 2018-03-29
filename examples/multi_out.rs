//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    // note that this can also be done using the app_metrics! macro
    let different_type_metrics = metric_scope((
        // combine metrics of different types in a tuple
        to_statsd("localhost:8125").expect("Connecting"),
        to_stdout(),
    ));

    // note that this can also be done using the app_metrics! macro
    let same_type_metrics = metric_scope(
        &[
            // use slices to combine multiple metrics of the same type
            to_stdout().with_name("yeah"),
            to_stdout().with_name("ouch"),
            to_stdout().with_sampling_rate(0.5),
        ][..],
    );

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
