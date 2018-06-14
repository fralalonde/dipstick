//! Use the metrics backend directly to log a metric value.
//! Applications should use the metrics()-provided instruments instead.

extern crate dipstick;

use dipstick::{MetricOutput, MetricInput};

fn main() {
    raw_write()
}

pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = dipstick::to_log().open();

    // define and send metrics using raw channel API
    let counter = metrics_log.define_metric(
        &"count_a".into(),
        dipstick::Kind::Counter,
    );
    counter.write(1);
}
