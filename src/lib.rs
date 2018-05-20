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
pub use core::{Value, Sampling, FULL_SAMPLING_RATE, TimeHandle, Kind, ROOT_NS, Namespace};

pub mod output;
pub use output::{MetricOutput, NO_METRIC_OUTPUT, OpenScope};

#[macro_use]
pub mod dispatch;
pub use dispatch::{MetricDispatch, Dispatch, metric_dispatch};

#[macro_use]
mod aggregate;
pub use aggregate::{MetricAggregator, Aggregate};

mod local;
pub use local::*;

mod scope;
pub use scope::*;

mod sample;
pub use sample::*;

mod scores;
pub use scores::*;

mod statsd;
pub use statsd::*;

mod graphite;
pub use graphite::*;

mod socket;
pub use socket::*;

mod cache;
pub use cache::*;

mod multi;
pub use multi::*;

mod async_queue;
pub use async_queue::*;

mod schedule;
pub use schedule::*;

mod self_metrics;
