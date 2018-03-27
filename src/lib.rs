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

mod pcg32;
mod lru_cache;

pub mod error;
pub use error::*;

#[macro_use]
pub mod macros;

pub mod core;
pub use core::*;

pub mod context;
pub use context::*;

#[macro_use]
pub mod delegate;
pub use delegate::*;

#[macro_use]
mod aggregate;
pub use aggregate::*;

mod output;
pub use output::*;

mod metrics;
pub use metrics::*;

mod sample;
pub use sample::*;

mod scores;
pub use scores::*;


mod statsd;
pub use statsd::*;

mod namespace;
pub use namespace::*;

mod graphite;
pub use graphite::*;

mod http;
pub use http::*;

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
