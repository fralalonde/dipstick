extern crate time;

pub mod core;
pub mod dual;
pub mod dispatch;
pub mod sampling;
//pub mod aggregate;
pub mod statsd;
pub mod log;
pub mod pcg32;

use dual::DualChannel;
use dispatch::DirectDispatch;
use sampling::SamplingChannel;
use statsd::StatsdChannel;
use log::LogChannel;
use core::{MetricType, Channel, MetricWrite, MetricDispatch, ValueMetric};

fn main() {
    let channel_a = SamplingChannel::new( LogChannel::new() );
//    let statsd_only_metric = channel_a.define(MetricType::Event, "statsd_event_a", 1.0);

    let channel_b = SamplingChannel::new( StatsdChannel::new("localhost:8125", "hello.").unwrap() );
    let channel_x = DualChannel::new( channel_a, channel_b );

    let metric = channel_x.define(MetricType::Count, "count_a", 1.0);
    channel_x.write(|scope| scope.write(&metric, 1));

    channel_x.write(|scope| {
        scope.write(&metric, 1);
//        scope.write(&statsd_only_metric, 1) <- fails at compile time, by design.
    });

    let sugar_x = DirectDispatch::new(channel_x);
    let counter = sugar_x.new_count("sugar_count_a");
    counter.value(1);

}

//thread_local!(static PROXY_SCOPE: RefCell<Metric> = metric());
