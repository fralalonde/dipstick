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

// FIXME required only for pcg32 seed (for sampling)
extern crate time;

#[macro_use]
mod macros;

mod core;
pub use core::{Flush, Value};
pub use core::component::*;
pub use core::input::*;
pub use core::output::*;
pub use core::scheduler::*;
pub use core::out_lock::*;
pub use core::error::{Error, Result};
pub use core::clock::{TimeHandle, mock_clock_advance, mock_clock_reset};
pub use core::proxy::Proxy;

mod output;
pub use output::text::*;
pub use output::graphite::*;
pub use output::statsd::*;
pub use output::map::*;
pub use output::logging::*;

mod aggregate;
pub use aggregate::bucket::*;
pub use aggregate::scores::*;

mod cache;
pub use cache::cache_in::CachedInput;
pub use cache::cache_out::CachedOutput;

mod multi;
pub use multi::multi_in::*;
pub use multi::multi_out::*;

mod queue;
pub use queue::queue_in::*;
pub use queue::queue_out::*;

// FIXME using * to prevent "use of deprecated" warnings. #[allow(dead_code)] doesnt work?
//#[macro_use]
//mod deprecated;
//pub use deprecated::*;
