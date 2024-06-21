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

#[cfg(doctest)]
#[macro_use]
extern crate doc_comment;
#[cfg(doctest)]
doctest!("../README.md");
#[cfg(doctest)]
doctest!("../HANDBOOK.md");

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

mod attributes;
mod clock;
mod input;
mod label;
mod metrics;
mod name;
mod pcg32;
mod proxy;
mod scheduler;

mod atomic;
mod stats;

mod cache;
mod lru_cache;

mod multi;
mod queue;

pub use crate::attributes::{
    Attributes, Buffered, Buffering, MetricId, Observe, ObserveWhen, OnFlush, OnFlushCancel,
    Prefixed, Sampled, Sampling, WithAttributes,
};
pub use crate::clock::TimeHandle;
pub use crate::input::{
    Counter, Gauge, Input, InputDyn, InputKind, InputMetric, InputScope, Level, Marker, Timer,
};
pub use crate::label::{AppLabel, Labels, ThreadLabel};
pub use crate::name::{MetricName, NameParts};
pub use crate::output::void::Void;
pub use crate::scheduler::{Cancel, CancelGuard, CancelHandle, ScheduleFlush};

#[cfg(test)]
pub use crate::clock::{mock_clock_advance, mock_clock_reset};

pub use crate::proxy::Proxy;

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

pub use crate::atomic::AtomicBucket;
pub use crate::cache::CachedInput;
pub use crate::multi::{MultiInput, MultiInputScope};
pub use crate::queue::{InputQueue, InputQueueScope, QueuedInput};
pub use crate::stats::{stats_all, stats_average, stats_summary, ScoreType};

use std::io;

/// Base type for recorded metric values.
pub type MetricValue = isize;

/// Both InputScope and OutputScope share the ability to flush the recorded data.
pub trait Flush {
    /// Flush does nothing by default.
    fn flush(&self) -> io::Result<()>;
}

#[cfg(feature = "bench")]
pub mod bench {

    use super::clock::*;
    use super::input::*;
    use crate::AtomicBucket;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let metrics = AtomicBucket::new();
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }
}
