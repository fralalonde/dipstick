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
extern crate atomic_refcell;
extern crate num;

// FIXME required only for pcg32 seed (for sampling)
extern crate time;

#[cfg(feature = "crossbeam-channel")]
extern crate crossbeam_channel;

#[cfg(feature = "parking_lot")]
extern crate parking_lot;

#[macro_use]
mod macros;
pub use macros::*;

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
pub use core::attributes::{
    Buffered, Buffering, Observe, ObserveWhen, OnFlush, Prefixed, Sampled, Sampling,
};
pub use core::clock::TimeHandle;
pub use core::error::Result;
pub use core::input::{
    Counter, Gauge, Input, InputDyn, InputKind, InputMetric, InputScope, Level, Marker, Timer,
};
pub use core::label::{AppLabel, Labels, ThreadLabel};
pub use core::locking::LockingOutput;
pub use core::name::{MetricName, NameParts};
pub use core::output::{Output, OutputDyn, OutputMetric, OutputScope};
pub use core::scheduler::{Cancel, CancelHandle, ScheduleFlush};
pub use core::void::Void;
pub use core::{Flush, MetricValue};

#[cfg(test)]
pub use core::clock::{mock_clock_advance, mock_clock_reset};

pub use core::proxy::Proxy;

mod output;
pub use output::format::{Formatting, LabelOp, LineFormat, LineOp, LineTemplate, SimpleFormat};
pub use output::graphite::{Graphite, GraphiteMetric, GraphiteScope};
pub use output::log::{Log, LogScope};
pub use output::map::StatsMapScope;
pub use output::statsd::{Statsd, StatsdMetric, StatsdScope};
pub use output::stream::{Stream, TextScope};

//#[cfg(feature="prometheus")]
pub use output::prometheus::{Prometheus, PrometheusScope};

mod bucket;
pub use bucket::atomic::AtomicBucket;
pub use bucket::{stats_all, stats_average, stats_summary, ScoreType};

mod cache;
pub use cache::cache_in::CachedInput;
pub use cache::cache_out::CachedOutput;

mod multi;
pub use multi::multi_in::{MultiInput, MultiInputScope};
pub use multi::multi_out::{MultiOutput, MultiOutputScope};

mod queue;
pub use queue::queue_in::{InputQueue, InputQueueScope, QueuedInput};
pub use queue::queue_out::{OutputQueue, OutputQueueScope, QueuedOutput};
