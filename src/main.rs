#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature="bench")]
extern crate test;

extern crate time;

#[macro_use]
extern crate log;

extern crate scheduled_executor;
extern crate thread_local;

//#[macro_use]
//extern crate cached;

#[macro_use]
pub mod core;

pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate;
pub mod statsd;
pub mod mlog;
//pub mod cache;
pub mod pcg32;

use dual::DualSink;
use dispatch::DirectDispatch;
use sampling::RandomSamplingSink;
use statsd::StatsdSink;
use mlog::LogSink;
use aggregate::sink::{AggregateChannel};
use aggregate::source::{AggregateSource};
use core::{MetricType, MetricSink, MetricWriter, MetricDispatch, ValueMetric, TimerMetric, MetricSource};
use std::thread::sleep;
use scheduled_executor::{CoreExecutor};
use std::time::Duration;

fn main() {
    sample_scheduled_statsd_aggregation()
}

pub fn sample_scheduled_statsd_aggregation() {

    // app metrics aggregate here
    let aggregate = AggregateChannel::new();

    // aggregated metrics are collected here
    let scores = aggregate.scores();

    // define some application metrics
    let app_metrics = DirectDispatch::new(aggregate);
    let counter = app_metrics.new_count("counter_a");
    let timer = app_metrics.new_timer("timer_a");

    // send aggregated metrics to statsd
    let statsd = StatsdSink::new("localhost:8125", "hello.").unwrap();
    let aggregate_metrics = AggregateSource::new(statsd, scores);

    // collect every three seconds
    let executor = CoreExecutor::new().unwrap();
    executor.schedule_fixed_rate(
        Duration::from_secs(3),
        Duration::from_secs(3),
        move |_| aggregate_metrics.publish()
    );

    // generate some metric values
    loop {
        counter.value(11);
        counter.value(22);
        time!(timer, { sleep(Duration::from_millis(10)); });
    }

}

pub fn logging_and_statsd() {

    let statsd = StatsdSink::new("localhost:8125", "goodbye.").unwrap();
    let logging = LogSink::new("metrics");
    let logging_and_statsd = DualSink::new(logging, statsd );
    DirectDispatch::new(logging_and_statsd);

}

pub fn sampling_statsd() {

    let statsd = StatsdSink::new("localhost:8125", "goodbye.").unwrap();
    let sampling_statsd = RandomSamplingSink::new(statsd, 0.1);
    DirectDispatch::new(sampling_statsd);

}


pub fn raw_write() {
    // setup dual metric channels
    let metrics_log = LogSink::new("metrics");

    // define and send metrics using raw channel API
    let counter = metrics_log.define(MetricType::Count, "count_a", 1.0);
    metrics_log.write(|scope| scope.write(&counter, 1));
}

pub fn counter_to_log() {
    let metrics_log = LogSink::new("metrics");
    let metrics = DirectDispatch::new(metrics_log);
    let counter = metrics.new_count("count_a");
    counter.value(1);
}