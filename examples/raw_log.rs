//! Use the metrics backend directly to log a metric value.
//! Applications should use the metrics()-provided instruments instead.

use dipstick::{Input, InputScope, labels};

fn main() {
    raw_write()
}

pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = dipstick::Log::to_log().metrics();

    // define and send metrics using raw channel API
    let counter = metrics_log.new_metric("count_a".into(), dipstick::InputKind::Counter);
    counter.write(1, labels![]);
}
