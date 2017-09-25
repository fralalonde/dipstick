//! A sample application continuously aggregating metrics and
//! sending the aggregated results to statsd every second.

#[macro_use] extern crate dipstick;
extern crate scheduled_executor;

use std::thread::sleep;
use scheduled_executor::CoreExecutor;
use std::time::Duration;
use dipstick::*;

fn main() {
    sample_scheduled_statsd_aggregation()
}

pub fn sample_scheduled_statsd_aggregation() {

    // SAMPLE METRICS SETUP

    // send application metrics to both aggregator and to sampling log
    let (sink, source) = aggregate();

    // schedule aggregated metrics to be sent to statsd every 3 seconds
    let statsd = cache(512, statsd("localhost:8125", "hello.").expect("no statsd"));

    // TODO use publisher publish_every() once it doesnt require 'static publisher
    let exec = CoreExecutor::new().unwrap();
    exec.schedule_fixed_rate(Duration::from_secs(3), Duration::from_secs(3), move |_| {
        publish(&source, &statsd)
    });

    // SAMPLE METRICS USAGE

    // define application metrics
    let metrics = metrics((
        sink,
        sample(0.1, log("metrics:"))));

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

        // TODO use scope to update metrics as one (single log line, single network packet, etc.)
//        app_metrics.with_scope(|scope| {
//            scope.set_property("http_method", "POST").set_property(
//                "user_id",
//                "superdude",
//            );
            event.mark();
            time!(timer, sleep(Duration::from_millis(5)));
            timer.time(|| sleep(Duration::from_millis(5)));
//        });
    }

}
