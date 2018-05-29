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

#[macro_use]
pub mod macros;

pub mod core;
pub use core::{Value, Sampling, FULL_SAMPLING_RATE, Kind, ROOT_NS, Namespace, WithNamespace};

pub mod output;
pub use output::{MetricOutput, NO_METRIC_OUTPUT, OpenScope};

pub mod dispatch;
pub use dispatch::{MetricDispatch, Dispatch, metric_dispatch};

mod aggregate;
pub use aggregate::{MetricAggregator, Aggregate, summary, all_stats, average};

mod local;
pub use local::{StatsMap, to_buffered_log, to_buffered_stdout, to_log, to_stdout, to_void};

mod input;
pub use input::{Marker, Timer, Counter, Gauge, MetricInput, MetricScope, Flush, ScheduleFlush, DefineMetric, metric_scope};

mod sample;
pub use sample::WithSamplingRate;

mod scores;
pub use scores::ScoreType;

mod statsd;
pub use statsd::{Statsd, to_statsd};

mod graphite;
pub use graphite::{Graphite, to_graphite, to_buffered_graphite};

//mod prometheus;
//pub use prometheus::{Prometheus, to_prometheus, to_buffered_prometheus};

mod socket;
pub use socket::RetrySocket;

mod cache;
pub use cache::{add_cache, WithCache};

mod multi;
pub use multi::*;

mod async_queue;
pub use async_queue::WithAsyncQueue;

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
