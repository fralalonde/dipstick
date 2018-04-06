//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use self::Command::*;

use std::sync::Arc;

use chrono::{Local, DateTime};

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
// TODO should this be f64? f32?
pub type Value = u64;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(i64);

fn now_micros() -> i64 {
    let local: DateTime<Local> = Local::now();
    let mut micros = local.timestamp() * 1_000_000;
    micros += local.timestamp_subsec_micros() as i64;
    micros
}

impl TimeHandle {

    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(now_micros())
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_us(self) -> Value {
        (TimeHandle::now().0 - self.0) as Value
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_ms(self) -> Value {
        self.elapsed_us() / 1_000
    }
}

impl From<usize> for TimeHandle {
    fn from(s: usize) -> TimeHandle {
        TimeHandle(s as i64)
    }
}

impl From<TimeHandle> for usize {
    fn from(s: TimeHandle) -> usize {
        s.0 as usize
    }
}

/// Base type for sampling rate.
/// - 1.0 records everything
/// - 0.5 records one of two values
/// - 0.0 records nothing
/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
pub type Sampling = f64;

/// Do not sample, use all data.
pub const FULL_SAMPLING_RATE: Sampling = 1.0;

/// Used to differentiate between metric kinds in the backend.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Kind {
    /// Handling one item at a time.
    Marker,
    /// Handling quantities or multiples.
    Counter,
    /// Reporting instant measurement of a resource at a point in time.
    Gauge,
    /// Measuring a time interval, internal to the app or provided by an external source.
    Timer,
}

/// Dynamic metric definition function.
/// Metrics can be defined from any thread, concurrently (Fn is Sync).
/// The resulting metrics themselves can be also be safely shared across threads (<M> is Send + Sync).
/// Concurrent usage of a metric is done using threaded scopes.
/// Shared concurrent scopes may be provided by some backends (aggregate).
pub type DefineMetricFn<M> = Arc<Fn(Kind, &str, Sampling) -> M + Send + Sync>;

/// A function trait that opens a new metric capture scope.
pub type OpenScopeFn<M> = Arc<Fn() -> CommandFn<M> + Send + Sync>;

/// A function trait that writes to or flushes a certain scope.
pub type WriteFn = Arc<Fn(Value) + Send + Sync + 'static>;

/// A function trait that writes to or flushes a certain scope.
#[derive(Clone)]
pub struct CommandFn<M> {
    inner: Arc<Fn(Command<M>) + Send + Sync + 'static>
}

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
pub enum Command<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

/// Create a new metric scope based on the provided scope function.
pub fn command_fn<M, F>(scope_fn: F) -> CommandFn<M>
where
    F: Fn(Command<M>) + Send + Sync + 'static,
{
    CommandFn {
        inner: Arc::new(scope_fn)
    }
}

impl<M> CommandFn<M> {
    /// Write a value to this scope.
    #[inline]
    pub fn write(&self, metric: &M, value: Value) {
        (self.inner)(Write(metric, value))
    }

    /// Flush this scope.
    /// Has no effect if scope is unbuffered.
    #[inline]
    pub fn flush(&self) {
        (self.inner)(Flush)
    }
}
