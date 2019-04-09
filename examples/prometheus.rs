//! A sample application sending ad-hoc metrics to prometheus.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    let metrics = Prometheus::push_to("http:// prometheus:9091/metrics/job/prometheus_example")
        .expect("Prometheus Socket")
        .named("my_app")
        .metrics();

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
