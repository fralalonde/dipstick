#![cfg_attr(feature = "bench", feature(test))]

#![warn(
missing_copy_implementations,
missing_debug_implementations,
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_extern_crates,
unused_import_braces,
unused_qualifications,
variant_size_differences,
)]

#![feature(fn_traits)]

#[cfg(feature="bench")]
extern crate test;

extern crate time;

extern crate cached;
extern crate thread_local_object;

#[macro_use]
extern crate log;

pub mod core;
pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate;
pub mod statsd;
pub mod mlog;
pub mod pcg32;
pub mod cache;
