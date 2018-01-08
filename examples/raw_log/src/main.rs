//! Use the metrics backend directly to log a metric value.
//! Applications should use the metrics()-provided instruments instead.

extern crate dipstick;

use dipstick::*;

fn main() {
    raw_write()
}

pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = to_log();

    // define and send metrics using raw channel API
    let counter = metrics_log.define_metric(Kind::Counter, "count_a", FULL_SAMPLING_RATE);
    metrics_log.open_scope(true).write(&counter, 1);
}
