//! Publishes metrics values from a source to a sink.
//!
//! Publishing can be done on request:
//! ```
//! use dipstick::*;
//!
//! let app_metrics = aggregate(summary, to_stdout());
//! publish(&source, &log("aggregated"), publish::all_stats);
//! ```
//!
//! Publishing can be scheduled to run recurrently.
//! ```
//! use dipstick::*;
//! use std::time::Duration;
//!
//! let (sink, source) = aggregate(summary, to_stdout());
//! let job = publish_every(Duration::from_millis(100), &source, &log("aggregated"), all_stats);
//! // publish will go on until cancelled
//! job.cancel();
//! ```

use core::*;
use context::*;
use core::Kind::*;
use scores::{ScoreSnapshot, ScoreType};
use scores::ScoreType::*;
use std::fmt::Debug;

