//! A quick, modular metrics toolkit for Rust applications.

#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs, trivial_casts, trivial_numeric_casts, unused_extern_crates,
        unused_qualifications)]
#![recursion_limit="32"]

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

//extern crate tiny_http;

#[macro_use]
mod macros;
pub use macros::*;

mod core;
pub use core::{Flush, MetricValue};
pub use core::attributes::{Prefixed, Sampling, Sampled, Buffered, Buffering};
pub use core::name::{MetricName, NameParts};
pub use core::input::{Input, InputDyn, InputScope, InputMetric, Counter, Timer, Marker, Gauge, Level, InputKind};
pub use core::output::{Output, OutputDyn, OutputScope, OutputMetric};
pub use core::scheduler::{ScheduleFlush, CancelHandle};
pub use core::locking::LockingOutput;
pub use core::error::{Result};
pub use core::clock::{TimeHandle};
pub use core::label::{Labels, AppLabel, ThreadLabel};

#[cfg(test)]
pub use core::clock::{mock_clock_advance, mock_clock_reset};

pub use core::proxy::Proxy;

mod output;
pub use output::format::{LineFormat, SimpleFormat, LineOp, LabelOp, LineTemplate, Formatting};
pub use output::stream::{Stream, TextScope};
pub use output::graphite::{Graphite, GraphiteScope, GraphiteMetric};
pub use output::statsd::{Statsd, StatsdScope, StatsdMetric};
pub use output::map::{StatsMap};
pub use output::log::{Log, LogScope};

//#[cfg(feature="prometheus")]
pub use output::prometheus::{Prometheus, PrometheusScope};

mod bucket;
pub use bucket::{ScoreType, stats_all, stats_average, stats_summary};
pub use bucket::atomic::{AtomicBucket};

mod cache;
pub use cache::cache_in::CachedInput;
pub use cache::cache_out::CachedOutput;

mod multi;
pub use multi::multi_in::{MultiInput, MultiInputScope};
pub use multi::multi_out::{MultiOutput, MultiOutputScope};

mod queue;
pub use queue::queue_in::{QueuedInput, InputQueue, InputQueueScope};
pub use queue::queue_out::{QueuedOutput, OutputQueue, OutputQueueScope};
