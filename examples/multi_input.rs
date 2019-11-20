//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::{Graphite, Input, InputScope, MultiInput, Prefixed, Stream, Locking, Log, QueuedOutput};
use std::time::Duration;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiInput::new()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting").locking())
        .add_target(Log::to_log())
        .metrics();

    // will output metrics twice, once with "both.yeah" prefix and once with "both.ouch" prefix.
    let same_type_metrics = MultiInput::new()
        .add_target(Stream::to_stderr().named("yeah").queued(5))
        .add_target(Stream::to_stderr().named("ouch").queued(5))
        .named("both")
        .metrics();

    loop {
        different_type_metrics.counter("counter_a").count(123);
        same_type_metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(400));
    }
}
