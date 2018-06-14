//! An app demonstrating the basics of the metrics front-end.
//! Defines metrics of each kind and use them to print values to the console in multiple ways.

extern crate dipstick;

use dipstick::*;

fn main() {
    // print only 1 out of every 10000 metrics recorded
    let app_metrics = to_statsd("statsd:8125").expect("Statsd")
        .with_sampling_rate(Sampling::SampleRate(0.0001)).open_scope();

    let marker = app_metrics.marker("marker_a");

    loop {
        marker.mark();
    }
}
