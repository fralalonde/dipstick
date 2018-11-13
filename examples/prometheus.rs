//! A sample application sending ad-hoc metrics to prometheus.

extern crate dipstick;

use dipstick::*;
use std::time::Duration;

fn main() {
    let metrics =
        Prometheus::send_json_to("localhost:2003")
            .expect("Prometheus Socket")
            .add_prefix("my_app")
            .input();

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}
