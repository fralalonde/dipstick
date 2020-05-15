//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::{Graphite, Input, InputScope, MultiInput, Prefixed, Stream};
use std::time::Duration;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiInput::new()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting"))
        .add_target(Stream::write_to_stdout())
        .metrics();

    // will output metrics twice, once with "both.yeah" prefix and once with "both.ouch" prefix.
    let same_type_metrics = MultiInput::new()
        .add_target(Stream::write_to_stderr().named("yeah"))
        .add_target(Stream::write_to_stderr().named("ouch"))
        .named("both")
        .metrics();

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(400));
    }
}
