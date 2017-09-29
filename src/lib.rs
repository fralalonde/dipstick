//! A fast and modular metrics library decoupling app instrumentation from reporting backend.
//! Similar to popular logging frameworks, but with counters and timers.
//! Can be configured for combined outputs (log + statsd), random sampling, local aggregation
//! of metrics, recurrent background publication, etc.

#![cfg_attr(feature = "bench", feature(test))]

#![warn(
missing_copy_implementations,
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_extern_crates,
unused_import_braces,
unused_qualifications,
// variant_size_differences,
)]

#[cfg(feature = "bench")]
extern crate test;

#[macro_use]
extern crate log as log_crate; // avoid namespace conflict with local 'log' module

#[macro_use]
extern crate error_chain;

extern crate time;
extern crate cached;
extern crate num;
#[macro_use] extern crate lazy_static;

mod pcg32;

mod error {
    //! Dipstick uses error_chain to handle the critical errors that might crop up when assembling the backend.
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}

pub mod core;
pub use core::*;

pub mod macros;

mod output;
pub use output::*;

mod app;
pub use app::*;

mod sampling;
pub use sampling::*;

mod aggregate;
pub use aggregate::*;

mod publish;
pub use publish::*;

mod statsd;
pub use statsd::*;

mod cache;
pub use cache::*;

mod multi;
pub use multi::*;

mod async;
pub use async::*;

mod fnsink;
pub use fnsink::*;

mod schedule;
pub use schedule::*;

mod selfmetrics;
pub use selfmetrics::METRICS_SOURCE;
