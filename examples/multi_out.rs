//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiOutput::new()
        .with_output(to_graphite("localhost:2003").expect("Connecting"))
        .with_output(to_stdout()).new_input();

    // will output metrics twice, once with "cool.yeah" prefix and once with "cool.ouch" prefix.
    let same_type_metrics = MultiOutput::new()
        .with_output(to_stdout().add_name("yeah"))
        .with_output(to_stdout().add_name("ouch"))
        .add_name("cool").new_input();

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
