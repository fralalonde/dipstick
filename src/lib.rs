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

// FIXME required only for random seed for sampling
extern crate time;

pub mod error;
pub use error::{Error, Result};

pub mod core;
pub use core::{Value, Kind, Marker, Timer, Counter, Gauge,
               Input, Output, OutputDyn,
               Name, WithName, WithSamplingRate, Sampling, Buffering, WithBuffering,
               Cache, Async, RawAsync, RawInput, RawOutput, RawMetric, UnsafeInput, RawOutputDyn,
               output_none, VoidOutput};

#[macro_use]
pub mod macros;

pub mod proxy;
pub use proxy::{Proxy, input_proxy};

mod bucket;
pub use bucket::{Bucket, input_bucket, stats_summary, stats_all, stats_average};

mod text;
pub use text::{output_stdout, output_stderr, TextOutput, TextInput};

mod logging;
pub use logging::{LogOutput, LogInput, output_log};

mod pcg32;

mod scores;
pub use scores::ScoreType;

mod statsd;
pub use statsd::{StatsdOutput, Statsd, output_statsd};

mod graphite;
pub use graphite::{GraphiteOutput, Graphite, output_graphite};

//mod prometheus;
//pub use prometheus::{Prometheus, to_prometheus};

mod map;
pub use map::{StatsMap, output_map};

mod socket;
pub use socket::RetrySocket;

mod cache;
pub use cache::{CacheInput, CacheOutput};

mod multi;
pub use multi::{MultiOutput, MultiInput, output_multi, input_multi};

mod queue;
pub use queue::{Queue, QueueOutput};

mod raw_queue;
pub use raw_queue::{RawQueue, RawQueueOutput};

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
