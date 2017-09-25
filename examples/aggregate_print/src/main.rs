//! A sample application continuously aggregating metrics and
//! sending the aggregated results to the console every three seconds.

#[macro_use] extern crate dipstick;
extern crate scheduled_executor;

use std::thread::sleep;
use scheduled_executor::CoreExecutor;
use std::time::Duration;
use dipstick::*;

fn main() {
    // send application metrics to both aggregator and to sampling log
    let (to_aggregate, from_aggregate) = aggregate();

    let app_metrics = metrics(to_aggregate);

    // schedule aggregated metrics to be printed every 3 seconds
    let to_console = print();

    publish_every(Duration::from_secs(3), from_aggregate, to_console);

    let counter = app_metrics.counter("counter_a");
    loop {
        counter.count(11);
    }
}
