//! A quick, modular metrics toolkit for Rust applications.

#![cfg_attr(feature = "bench", feature(test))]
#![warn(
    missing_docs,
    trivial_casts,
    trivial_numeric_casts,
    unused_extern_crates,
    unused_qualifications
)]
#![recursion_limit = "32"]

#[cfg(feature = "bench")]
extern crate test;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
mod macros;
pub use crate::macros::*;

#[cfg(not(feature = "parking_lot"))]
macro_rules! write_lock {
    ($WUT:expr) => {
        $WUT.write().unwrap()
    };
}

#[cfg(feature = "parking_lot")]
macro_rules! write_lock {
    ($WUT:expr) => {
        $WUT.write()
    };
}

#[cfg(not(feature = "parking_lot"))]
macro_rules! read_lock {
    ($WUT:expr) => {
        $WUT.read().unwrap()
    };
}

#[cfg(feature = "parking_lot")]
macro_rules! read_lock {
    ($WUT:expr) => {
        $WUT.read()
    };
}

mod core;
pub use crate::core::attributes::{
    Buffered, Buffering, Observe, ObserveWhen, OnFlush, Prefixed, Sampled, Sampling,
};
pub use crate::core::clock::TimeHandle;
pub use crate::core::error::Result;
pub use crate::core::input::{
    Counter, Gauge, Input, InputDyn, InputKind, InputMetric, InputScope, Level, Marker, Timer,
};
pub use crate::core::label::{AppLabel, Labels, ThreadLabel};
pub use crate::core::locking::LockingOutput;
pub use crate::core::name::{MetricName, NameParts};
pub use crate::core::output::{Output, OutputDyn, OutputMetric, OutputScope};
pub use crate::core::scheduler::{Cancel, CancelHandle, ScheduleFlush};
pub use crate::core::void::Void;
pub use crate::core::{Flush, MetricValue};

#[cfg(test)]
pub use crate::core::clock::{mock_clock_advance, mock_clock_reset};

pub use crate::core::proxy::Proxy;

mod output;
pub use crate::output::format::{
    Formatting, LabelOp, LineFormat, LineOp, LineTemplate, SimpleFormat,
};
pub use crate::output::graphite::{Graphite, GraphiteMetric, GraphiteScope};
pub use crate::output::log::{Log, LogScope};
pub use crate::output::map::StatsMapScope;
pub use crate::output::statsd::{Statsd, StatsdMetric, StatsdScope};
pub use crate::output::stream::{Stream, TextScope};

//#[cfg(feature="prometheus")]
pub use crate::output::prometheus::{Prometheus, PrometheusScope};

mod bucket;
pub use crate::bucket::atomic::AtomicBucket;
pub use crate::bucket::{stats_all, stats_average, stats_summary, ScoreType};

mod cache;
pub use crate::cache::cache_in::CachedInput;
pub use crate::cache::cache_out::CachedOutput;

mod multi;
pub use crate::multi::multi_in::{MultiInput, MultiInputScope};
pub use crate::multi::multi_out::{MultiOutput, MultiOutputScope};

mod queue;
pub use crate::queue::queue_in::{InputQueue, InputQueueScope, QueuedInput};
pub use crate::queue::queue_out::{OutputQueue, OutputQueueScope, QueuedOutput};
