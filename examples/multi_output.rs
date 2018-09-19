//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;
use std::io;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiOutput::metrics()
        .add_target(Graphite::send_to("localhost:2003").expect("Connecting"))
        .add_target(Text::write_to(io::stdout()))
        .input();

    // will output metrics twice, once with "cool.yeah" prefix and once with "cool.ouch" prefix.
    let same_type_metrics = MultiOutput::metrics()
        .add_target(Text::write_to(io::stdout()).add_prefix("yeah"))
        .add_target(Text::write_to(io::stdout()).add_prefix("ouch"))
        .add_prefix("cool").input();

    loop {
        different_type_metrics.new_metric("counter_a".into(), Kind::Counter).write(123);
        same_type_metrics.new_metric("timer_a".into(), Kind::Timer).write(6677);
        std::thread::sleep(Duration::from_millis(400));
    }
}
