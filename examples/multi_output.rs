//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;
use std::io;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiOutput::output()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting"))
        .add_target(Stream::write_to(io::stdout()))
        .input();

    // will output metrics twice, once with "cool.yeah" prefix and once with "cool.ouch" prefix.
    let same_type_metrics = MultiOutput::output()
        .add_target(Stream::write_to(io::stderr()).add_prefix("out_1"))
        .add_target(Stream::write_to(io::stderr()).add_prefix("out_2"))
        .add_prefix("out_both").input();

    loop {
        different_type_metrics.new_metric("counter_a".into(), InputKind::Counter).write(123, labels![]);
        same_type_metrics.new_metric("timer_a".into(), InputKind::Timer).write(6677, labels![]);
        std::thread::sleep(Duration::from_millis(400));
    }
}
