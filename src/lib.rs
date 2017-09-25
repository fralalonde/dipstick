//! A fast and modular metrics library decoupling app instrumentation from reporting backend.
//! Similar to popular logging frameworks, but with counters and timers.
//! Can be configured for combined outputs (log + statsd), random sampling, local aggregation
//! of metrics, recurrent background publication, etc.

#![cfg_attr(feature = "bench", feature(test))]

#![warn(
missing_copy_implementations,
//missing_debug_implementations,
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
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

extern crate time;
extern crate cached;
extern crate num;
extern crate scheduled_executor;

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
pub mod macros;
pub mod output;
pub mod app;

pub mod sampling;
pub mod aggregate;
pub mod publish;
pub mod statsd;
pub mod cache;
pub mod multi;
pub mod queue;
pub mod fnsink;

// input
pub use app::metrics;

// generic
pub use fnsink::make_sink;

// buffering
pub use queue::queue;

// transform
pub use cache::cache;
pub use sampling::sample;

// pack + forward
pub use aggregate::aggregate;
pub use publish::{publish, publish_every};

// output
pub use output::{log, print};
pub use statsd::statsd;
