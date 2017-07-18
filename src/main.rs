extern crate time;

#[macro_use]
pub mod core;

pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate_sink;
pub mod statsd;
pub mod log;
pub mod pcg32;

use dual::DualChannel;
use dispatch::DirectDispatch;
use sampling::SamplingChannel;
use statsd::StatsdChannel;
use log::LogChannel;
use aggregate_sink::{AggregateChannel, Score};
use core::{MetricType, MetricChannel, MetricWrite, MetricDispatch, ValueMetric, TimerMetric};
use std::sync::atomic::{Ordering};
use std::thread::sleep;

fn main() {
//    let channel_a = SamplingChannel::new( LogChannel::new() );
    let aggregate = AggregateChannel::new();
    let scores = aggregate.scores();

//    let channel_a_ref = &channel_a;
//    let statsd_only_metric = channel_a.define(MetricType::Event, "statsd_event_a", 1.0);

    let sampling_statsd = SamplingChannel::new( StatsdChannel::new("localhost:8125", "hello.").unwrap() );
    let mut channel_x = DualChannel::new( aggregate, sampling_statsd );

    let metric = channel_x.define(MetricType::Count, "count_a", 1.0);
    channel_x.write(|scope| scope.write(&metric, 1));

    channel_x.write(|scope| {
        scope.write(&metric, 1);
//        scope.write(&statsd_only_metric, 1) <- fails at compile time, by design.
    });

    let mut sugar_x = DirectDispatch::new(channel_x);

    let counter = sugar_x.new_count("sugar_count_a");
    counter.value(1);
    counter.value(2);

    let timer = sugar_x.new_timer("sugar_time_a");
    timer.value(1);
    timer.value(2);

    let start_time = timer.start();
    let ten_millis = std::time::Duration::from_millis(10);
    sleep(ten_millis);
    timer.stop(start_time);

    time!(timer, {  /*nothing*/ });

    scores.for_each(|metric| {
        println!("m_type {:?}, name {}", metric.m_type, metric.name);
        match &metric.score {
            &Score::Event {ref start_time, ref hit_count} => {
                println!("start_time {:?}, hit_count {:?}", start_time, hit_count.load(Ordering::Acquire));
            },
            &Score::Value {ref start_time, ref hit_count, ref value_sum, ref max, ref min} => {
                println!("start_time {:?}, hit_count {:?}, value_sum {:?}, max {:?}, min {:?}", start_time,
                         hit_count.load(Ordering::Acquire), value_sum.load(Ordering::Acquire),
                         max.load(Ordering::Acquire), min.load(Ordering::Acquire));
            }
        }
    });

}

//thread_local!(static PROXY_SCOPE: RefCell<Metric> = metric());
