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
    let aggregator = aggregate();

    // schedule aggregated metrics to be sent to statsd every 3 seconds
    let statsd = cache(512, statsd("localhost:8125", "hello.").expect("no statsd"));
    let aggregate_metrics = publish(aggregator.source(), statsd);

    // TODO use publisher publish_every() once it doesnt require 'static publisher
    let exec = CoreExecutor::new().unwrap();
    exec.schedule_fixed_rate(Duration::from_secs(3), Duration::from_secs(3), move |_| {
        aggregate_metrics.publish()
    });

    // SAMPLE METRICS USAGE

    // define application metrics
    let mut metrics = metrics(combine(
        aggregator.sink(),
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

pub fn logging_and_statsd() {

    let statsd = statsd("localhost:8125", "goodbye.").unwrap();
    let logging = log("metrics");
    let logging_and_statsd = combine(logging, statsd);
    metrics(logging_and_statsd);

}

pub fn sampling_statsd() -> dipstick::error::Result<()> {
    metrics(sample(0.1, statsd("localhost:8125", "goodbye.")?));
    Ok(())
}


pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = log("metrics");

    // define and send metrics using raw channel API
    let counter = metrics_log.new_metric(MetricKind::Count, "count_a", dipstick::FULL_SAMPLING_RATE);
    metrics_log.new_writer().write(&counter, 1);
}

pub fn counter_to_log() {
    let metrics_log = log("metrics");
    let metrics = metrics(metrics_log);
    let counter = metrics.counter("count_a");
    counter.count(10.2);
}

const STATSD_SAMPLING_RATE: f64 = 0.0001;

//lazy_static! {
//    pub static ref METRICS: DirectDispatch<SamplingSink<StatsdSink>> = DirectDispatch::new(
//        SamplingSink::new(StatsdSink::new("localhost:8125", env!("CARGO_PKG_NAME")).unwrap(), STATSD_SAMPLING_RATE));
//
//    pub static ref SERVICE_RESPONSE_TIME:     DirectTimer<SamplingSink<StatsdSink>>   = METRICS.new_timer("service.response.time");
//    pub static ref SERVICE_RESPONSE_BYTES:    DirectCount<SamplingSink<StatsdSink>>   = METRICS.new_count("service.response.bytes");
//}