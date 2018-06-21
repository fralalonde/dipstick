//! Use the metrics backend directly to log a metric value.
//! Applications should use the metrics()-provided instruments instead.

extern crate dipstick;

use dipstick::{Output, Input};

fn main() {
    raw_write()
}

pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = dipstick::output_log().new_input();

    // define and send metrics using raw channel API
    let counter = metrics_log.new_metric(
        "count_a".into(),
        dipstick::Kind::Counter,
    );
    counter.write(1);
}
