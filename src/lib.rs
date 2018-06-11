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
extern crate time;

pub mod error;
pub use error::{Error, Result};

pub mod core;
pub use core::{Value, Sampling, FULL_SAMPLING_RATE, Kind, ROOT_NS, Namespace, Namespaced,
               Marker, Timer, Counter, Gauge, MetricInput, MetricOutput, NO_METRIC_OUTPUT,
               OpenScope, ScheduleFlush};

#[macro_use]
pub mod macros;

pub mod dispatch;
pub use dispatch::{MetricDispatch, ROOT_DISPATCH};

mod aggregate;
pub use aggregate::{MetricAggregator, summary, all_stats, average};

mod text;
pub use text::{to_buffered_stdout, to_stdout, TextOutput, BufferedTextOutput, BufferedTextInput};
pub use text::{to_buffered_log, to_log, LogOutput, BufferedLogOutput, BufferedLogInput};
pub use text::{to_void, Void};

//mod sample;
//pub use sample::WithSamplingRate;

mod scores;
pub use scores::ScoreType;
//
//mod statsd;
//pub use statsd::{Statsd, to_statsd};

mod graphite;
pub use graphite::{Graphite, to_graphite};

//mod prometheus;
//pub use prometheus::{Prometheus, to_prometheus, to_buffered_prometheus};

mod map;
pub use map::StatsMap;

mod socket;
pub use socket::RetrySocket;

//mod cache;
//pub use cache::{add_cache, WithCache};

//mod multi;
//pub use multi::*;
//
//mod async_queue;
//pub use async_queue::WithAsyncQueue;

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
