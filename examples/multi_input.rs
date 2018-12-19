//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::{MultiInput, Graphite, Stream, Input, InputScope, Prefixed};
use std::time::Duration;
use std::io;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiInput::input()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting"))
        .add_target(Stream::to_stdout())
        .input();

    // will output metrics twice, once with "cool.yeah" prefix and once with "cool.ouch" prefix.
    let same_type_metrics = MultiInput::input()
        .add_target(Stream::to_stderr().add_prefix("yeah"))
        .add_target(Stream::to_stderr().add_prefix("ouch"))
        .add_prefix("cool")
        .input();

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(400));
    }
}
