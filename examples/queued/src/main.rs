#[macro_use] extern crate dipstick;
extern crate scheduled_executor;

use std::thread::sleep;
use scheduled_executor::CoreExecutor;
use std::time::Duration;
use dipstick::*;

use dipstick::core::{Sink, self};

fn main() {
    sample_scheduled_statsd_aggregation()
}

pub fn sample_scheduled_statsd_aggregation() {

    let metrics = metrics(queue(0, stdout("metrics:")));

    let counter = metrics.counter("counter_a");
    let timer = metrics.timer("timer_b");

    let subsystem_metrics = metrics.with_prefix("subsystem.");
    let event = subsystem_metrics.event("event_c");
    let gauge = subsystem_metrics.gauge("gauge_d");

    loop {
        // report some metric values from our "application" loop
        counter.count(11);
        gauge.value(22);

        metrics.counter("ad_hoc").count(4);

        event.mark();
        time!(timer, sleep(Duration::from_millis(5)));
        timer.time(|| sleep(Duration::from_millis(5)));
    }

}
