#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature = "bench")]
extern crate test;

extern crate time;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

extern crate scheduled_executor;
extern crate thread_local_object;

extern crate cached;
extern crate num;

#[macro_use]
pub mod core;

pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate;
pub mod statsd;
pub mod logging;
pub mod cache;
pub mod pcg32;

use dual::DualSink;
use dispatch::{DirectDispatch, DirectCount, DirectTimer};
use sampling::SamplingSink;
use statsd::StatsdSink;
use logging::LoggingSink;
use aggregate::sink::MetricAggregator;
use aggregate::publish::AggregatePublisher;
use core::{MetricType, MetricSink, MetricWriter, MetricDispatch, CountMetric, GaugeMetric,
           TimerMetric, EventMetric};
use std::thread::sleep;
use scheduled_executor::CoreExecutor;
use std::time::Duration;
use cache::MetricCache;

fn main() {
    sample_scheduled_statsd_aggregation()
}

pub fn sample_scheduled_statsd_aggregation() {

    // SAMPLE METRICS SETUP

    // send application metrics to both aggregator and to sampling log
    let aggregator = MetricAggregator::new();
    let sampling_log = SamplingSink::new(LoggingSink::new("metrics:"), 0.1);
    let dual_sink = DualSink::new(aggregator.sink(), sampling_log);

    // schedule aggregated metrics to be sent to statsd every 3 seconds
    let statsd = MetricCache::new(StatsdSink::new("localhost:8125", "hello.").unwrap(), 512);
    let aggregate_metrics = AggregatePublisher::new(statsd, aggregator.source());
    // TODO publisher should provide its own scheduler
    let exec = CoreExecutor::new().unwrap();
    exec.schedule_fixed_rate(Duration::from_secs(3), Duration::from_secs(3), move |_| {
        aggregate_metrics.publish()
    });

    // SAMPLE METRICS USAGE

    // define application metrics
    let mut app_metrics = DirectDispatch::new(dual_sink);
    let counter = app_metrics.new_count("counter_a");
    let timer = app_metrics.new_timer("timer_b");

    let subsystem_metrics = app_metrics.with_prefix("subsystem.");
    let event = subsystem_metrics.new_event("event_c");
    let gauge = subsystem_metrics.new_gauge("gauge_d");

    loop {
        // report some metric values from our "application" loop
        counter.count(11);
        gauge.value(22);

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

    let statsd = StatsdSink::new("localhost:8125", "goodbye.").unwrap();
    let logging = LoggingSink::new("metrics");
    let logging_and_statsd = DualSink::new(logging, statsd);
    DirectDispatch::new(logging_and_statsd);

}

pub fn sampling_statsd() {

    let statsd = StatsdSink::new("localhost:8125", "goodbye.").unwrap();
    let sampling_statsd = SamplingSink::new(statsd, 0.1);
    DirectDispatch::new(sampling_statsd);

}


pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = LoggingSink::new("metrics");

    // define and send metrics using raw channel API
    let counter = metrics_log.new_metric(MetricType::Count, "count_a", core::FULL_SAMPLING_RATE);
    metrics_log.new_writer().write(&counter, 1);
}

pub fn counter_to_log() {
    let metrics_log = LoggingSink::new("metrics");
    let metrics = DirectDispatch::new(metrics_log);
    let counter = metrics.new_count("count_a");
    counter.count(10.2);
}

const STATSD_SAMPLING_RATE: f64 = 0.0001;

lazy_static! {
    pub static ref METRICS: DirectDispatch<SamplingSink<StatsdSink>> = DirectDispatch::new(
        SamplingSink::new(StatsdSink::new("localhost:8125", env!("CARGO_PKG_NAME")).unwrap(), STATSD_SAMPLING_RATE));

    pub static ref SERVICE_RESPONSE_TIME:     DirectTimer<SamplingSink<StatsdSink>>   = METRICS.new_timer("service.response.time");
    pub static ref SERVICE_RESPONSE_BYTES:    DirectCount<SamplingSink<StatsdSink>>   = METRICS.new_count("service.response.bytes");
}