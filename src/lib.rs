//! A quick, modular metrics toolkit for Rust applications.

#![cfg_attr(feature = "bench", feature(test))]
#![warn(missing_docs, trivial_casts, trivial_numeric_casts, unused_extern_crates,
        unused_qualifications)]
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
pub use core::attributes::{Naming, Sampling, Sampled, Buffered, Buffering};
pub use core::name::{Name, NameParts};
pub use core::input::{Input, InputDyn, InputScope, InputMetric, Counter, Timer, Marker, Gauge, Kind};
pub use core::output::{Output, OutputDyn, OutputScope, OutputMetric};
pub use core::scheduler::{ScheduleFlush, CancelHandle};
pub use core::out_lock::{LockingScopeBox};
pub use core::error::{Error, Result};
pub use core::clock::{TimeHandle};

#[cfg(test)]
pub use core::clock::{mock_clock_advance, mock_clock_reset};

pub use core::proxy::Proxy;

mod output;
pub use output::format::{Format, LineFormat};
pub use output::text::{Text, TextScope};
pub use output::graphite::{Graphite, GraphiteScope, GraphiteMetric};
pub use output::statsd::{Statsd, StatsdScope, StatsdMetric};
pub use output::map::{StatsMap};
pub use output::log::{Log, LogScope};

mod aggregate;
pub use aggregate::bucket::{Bucket, stats_all, stats_average, stats_summary};
pub use aggregate::scores::{ScoreType, Scoreboard};

mod cache;
pub use cache::cache_in::CachedInput;
pub use cache::cache_out::CachedOutput;

mod multi;
pub use multi::multi_in::{MultiInput, MultiInputScope};
pub use multi::multi_out::{MultiOutput, MultiOutputScope};

mod queue;
pub use queue::queue_in::{QueuedInput, InputQueue, InputQueueScope};
pub use queue::queue_out::{QueuedOutput, OutputQueue, OutputQueueScope};
