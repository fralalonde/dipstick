//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {

    let metrics1 = global_metrics(
        (
            // use tuples to combine metrics of different types
            to_statsd("localhost:8125").expect("connect"),
            to_stdout()
        )
    );

    let metrics2 = global_metrics(
         &[
            // use slices to combine multiple metrics of the same type
            prefix("yeah.", to_stdout()),
            prefix("ouch.", to_stdout()),
            prefix("nooo.", to_stdout()),
        ][..]
    );

    loop {
        metrics1.counter("counter_a").count(123);
        metrics2.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
