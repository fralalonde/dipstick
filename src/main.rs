#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature="bench")]
extern crate test;

extern crate time;

#[macro_use]
extern crate log;

extern crate scheduled_executor;
extern crate tokio_core;

//#[macro_use]
//extern crate cached;

#[macro_use]
pub mod core;

pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate_sink;
pub mod statsd;
pub mod mlog;
//pub mod cache;
pub mod pcg32;

use dual::DualChannel;
use dispatch::DirectDispatch;
use sampling::SamplingChannel;
use statsd::StatsdChannel;
use mlog::LogChannel;
use aggregate_sink::{AggregateChannel, Score};
use core::{MetricType, MetricChannel, MetricWrite, MetricDispatch, ValueMetric, TimerMetric};
use std::sync::atomic::{Ordering};
use std::thread::sleep;
use scheduled_executor::{CoreExecutor};
use std::time::Duration;

fn main() {
    let aggregate = AggregateChannel::new();
    let scores = aggregate.scores();
    let aggregated_statsd = StatsdChannel::new("localhost:8125", "hello.").unwrap();

    let executor = CoreExecutor::with_name("metric scheduler").unwrap();
    executor.schedule_fixed_rate(
        Duration::from_secs(2),  // Wait 2 seconds before scheduling the first task
        Duration::from_secs(5),  // and schedule every following task at 5 seconds intervals
        move |_| {
            aggregated_statsd.write(|scope| {
                scores.for_each(|metric| {
                    println!("m_type {:?}, name {}", metric.m_type, metric.name);
                    match &metric.score {
                        &Score::Event {ref hit_count, ..} => {
                            let name = format!("{}.count", &metric.name);
                            let temp_metric = aggregated_statsd.define(MetricType::Count, name, 1.0);
                            scope.write(&temp_metric, hit_count.load(Ordering::Acquire) as u64);
                        },
                        &Score::Value {ref hit_count, ref value_sum, ref max, ref min, ..} => {
                            let name = format!("{}.count", &metric.name);
                            let temp_metric = aggregated_statsd.define(MetricType::Count, name, 1.0);
                            scope.write(&temp_metric, hit_count.load(Ordering::Acquire) as u64);

                            let name = format!("{}.sum", &metric.name);
                            let temp_metric = aggregated_statsd.define(MetricType::Count, name, 1.0);
                            scope.write(&temp_metric, value_sum.load(Ordering::Acquire) as u64);

                            let name = format!("{}.max", &metric.name);
                            let temp_metric = aggregated_statsd.define(MetricType::Gauge, name, 1.0);
                            scope.write(&temp_metric, max.load(Ordering::Acquire) as u64);

                            let name = format!("{}.min", &metric.name);
                            let temp_metric = aggregated_statsd.define(MetricType::Gauge, name, 1.0);
                            scope.write(&temp_metric, min.load(Ordering::Acquire) as u64);
                        }
                    }
                });
            });
        }
    );

    // setup dual metric channels
    let direct_statsd = StatsdChannel::new("localhost:8125", "goodbye.").unwrap();
    let direct_sampling_statsd = SamplingChannel::new(direct_statsd);
    let logging = LogChannel::new();
    let direct_logging_and_statsd = DualChannel::new( logging, direct_sampling_statsd );

    // define and send metrics using raw channel API
    let metric = direct_logging_and_statsd.define(MetricType::Count, "count_a", 1.0);
    direct_logging_and_statsd.write(|scope| scope.write(&metric, 1));

    // define metrics using sweet dispatch API over aggregator channel
    let direct_aggregate = DirectDispatch::new(aggregate);

    let counter = direct_aggregate.new_count("sugar_count_a");
    let timer = direct_aggregate.new_timer("sugar_time_a");

    // "application" body
    loop {
        counter.value(1);
        counter.value(2);

        timer.value(1);
        timer.value(2);

        let start_time = timer.start();
        let ten_millis = std::time::Duration::from_millis(10);
        sleep(ten_millis);
        timer.stop(start_time);

        time!(timer, { sleep(ten_millis); });
    }

}
