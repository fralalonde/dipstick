//! A quick, modular metrics toolkit for Rust applications.

#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs, trivial_casts, trivial_numeric_casts, unused_extern_crates,
        unused_import_braces, unused_qualifications)]

#[cfg(feature = "bench")]
extern crate test;

#[macro_use]
extern crate log;

#[macro_use]
extern crate derivative;
#[macro_use]
extern crate lazy_static;
extern crate atomic_refcell;
extern crate num;

// FIXME required only for random seed for sampling
extern crate time;

pub mod error;
pub use error::{Error, Result};

pub mod core;
pub use core::{Value, Kind, Name, WithName, Marker, Timer, Counter, Gauge, Input, Flush,
               Output, NO_METRIC_OUTPUT, OutputDyn, ScheduleFlush, WithSamplingRate, Sampling, WithBuffering, Buffering};

#[macro_use]
pub mod macros;

pub mod proxy;
pub use proxy::{InputProxy, ROOT_PROXY, to_proxy};

mod bucket;
pub use bucket::{Bucket, to_bucket, summary, all_stats, average};

mod text;
pub use text::{to_stdout, TextOutput, TextInput};
pub use text::{to_void, Void};

mod logging;
pub use logging::{to_log, LogOutput, LogInput};

mod pcg32;

mod scores;
pub use scores::ScoreType;

mod statsd;
pub use statsd::{StatsdOutput, StatsdInput, to_statsd};

mod graphite;
pub use graphite::{GraphiteInput, to_graphite, to_buffered_graphite};

//mod prometheus;
//pub use prometheus::{Prometheus, to_prometheus, to_buffered_prometheus};

mod map;
pub use map::StatsMap;

mod socket;
pub use socket::RetrySocket;

mod cache;
pub use cache::{CacheInput,CacheOutput};

mod multi;
pub use multi::{MultiOutput, MultiInput, to_multi};

mod async;
pub use async::{AsyncInput, AsyncOutput};

mod scheduler;
pub use scheduler::{set_schedule, CancelHandle};

mod self_metrics;
pub use self_metrics::DIPSTICK_METRICS;

mod clock;
pub use clock::{TimeHandle, mock_clock_advance, mock_clock_reset};

// FIXME using * to prevent "use of deprecated" warnings. #[allow(dead_code)] doesnt work?
#[macro_use]
mod deprecated;
pub use deprecated::*;
