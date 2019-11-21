//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiOutput::new()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting"))
        .add_target(Stream::to_stdout())
        .locking()
        .metrics();

    // will output metrics twice, once with "both.yeah" prefix and once with "both.ouch" prefix.
    let same_type_metrics = MultiOutput::new()
        .add_target(Stream::to_stderr().named("yeah"))
        .add_target(Stream::to_stderr().named("ouch"))
        .named("both")
        .locking()
        .metrics();

    loop {
        different_type_metrics
            .new_metric("counter_a".into(), InputKind::Counter)
            .write(123, labels![]);
        same_type_metrics
            .new_metric("timer_a".into(), InputKind::Timer)
            .write(6677, labels![]);
        std::thread::sleep(Duration::from_millis(400));
    }
}
