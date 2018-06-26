//! A quick, modular metrics toolkit for Rust applications.

#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs, trivial_casts, trivial_numeric_casts, unused_extern_crates,
        unused_import_braces, unused_qualifications)]
#![recursion_limit="32"]

#[cfg(feature = "bench")]
extern crate test;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;
extern crate atomic_refcell;
extern crate num;

#[cfg(feature="protobuf")]
extern crate protobuf;

// FIXME required only for random seed for sampling
extern crate time;

pub mod error;
pub use error::{Error, Result};

pub mod core;
pub use core::{Value, Kind, Marker, Timer, Counter, Gauge,
               Flush, Scope, Output, OutputDyn,
               Name, AddPrefix, WithSampling, Sampling, Buffering, WithBuffering,
               WithMetricCache, WithQueue, WithRawQueue, RawScope, RawOutput, RawMetric, UnsafeScope, RawOutputDyn,
               output_none, VoidOutput};

#[macro_use]
pub mod macros;

pub mod proxy;
pub use proxy::Proxy;

mod bucket;
pub use bucket::{Bucket, stats_summary, stats_all, stats_average};

mod text;
pub use text::{TextOutput, Text};

mod logging;
pub use logging::{LogOutput, Log};

mod pcg32;

mod scores;
pub use scores::ScoreType;

mod statds;
pub use statds::{StatsdOutput, Statsd};

mod graphite;
pub use graphite::{GraphiteOutput, Graphite};

#[cfg(feature="prometheus")]
mod prometheus;
#[cfg(feature="prometheus, proto")]
mod prometheus_proto;
#[cfg(feature="prometheus")]
pub use prometheus::{Prometheus, PrometheusOutput};

mod map;
pub use map::StatsMap;

mod socket;
pub use socket::RetrySocket;

mod cache;
pub use cache::{Cache, CacheOutput};

mod multi;
pub use multi::{MultiOutput, Multi};

mod multi_raw;
pub use multi_raw::{MultiRawOutput, MultiRaw};

mod queue;
pub use queue::{Queue, QueueOutput};

mod queue_raw;
pub use queue_raw::{RawQueue, RawQueueOutput};

mod scheduler;
pub use scheduler::{set_schedule, CancelHandle, ScheduleFlush};

mod metrics;
pub use metrics::DIPSTICK_METRICS;

mod clock;
pub use clock::{TimeHandle, mock_clock_advance, mock_clock_reset};

// FIXME using * to prevent "use of deprecated" warnings. #[allow(dead_code)] doesnt work?
#[macro_use]
mod deprecated;
pub use deprecated::*;
