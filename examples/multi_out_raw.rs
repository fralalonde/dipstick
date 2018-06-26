//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;
use std::io;

fn main() {
    // will output metrics to graphite and to stdout
    let different_type_metrics = MultiRaw::output()
        .add_raw_target(Graphite::output("localhost:2003").expect("Connecting"))
        .add_raw_target(Text::output(io::stdout()))
        .open_scope_raw();

    // will output metrics twice, once with "cool.yeah" prefix and once with "cool.ouch" prefix.
    let same_type_metrics = MultiRaw::output()
        .add_raw_target(Text::output(io::stdout()).add_prefix("yeah"))
        .add_raw_target(Text::output(io::stdout()).add_prefix("ouch"))
        .add_prefix("cool").open_scope();

    loop {
        different_type_metrics.new_metric_raw("counter_a".into(), Kind::Counter).write(123);
        same_type_metrics.new_metric_raw("timer_a".into(), Kind::Timer).write(6677);
        std::thread::sleep(Duration::from_millis(400));
    }
}
