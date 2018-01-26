//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use self::ScopeCmd::*;

use time;
use std::sync::Arc;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
// TODO should this be f64? f32?
pub type Value = u64;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(u64);

impl TimeHandle {
    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(time::precise_time_ns())
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_us(self) -> Value {
        (TimeHandle::now().0 - self.0) / 1_000
    }
}

/// Base type for sampling rate.
/// - 1.0 records everything
/// - 0.5 records one of two values
/// - 0.0 records nothing
/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
pub type Rate = f64;

/// Do not sample, use all data.
pub const FULL_SAMPLING_RATE: Rate = 1.0;

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
pub type DefineMetricFn<M> = Arc<Fn(Kind, &str, Rate) -> M + Send + Sync>;

/// A function trait that opens a new metric capture scope.
pub type OpenScopeFn<M> = Arc<Fn(bool) -> ControlScopeFn<M> + Send + Sync>;

/// A function trait that writes to or flushes a certain scope.
pub type ControlScopeFn<M> = Arc<InnerControlScopeFn<M>>;

/// Returns a callback function to send commands to the metric scope.
/// Writes can be performed by passing Some((&Metric, Value))
/// Flushes can be performed by passing None
/// Used to write values to the scope or flush the scope buffer (if applicable).
/// Simple applications may use only one scope.
/// Complex applications may define a new scope fo each operation or request.
/// Scopes can be moved acrossed threads (Send) but are not required to be thread-safe (Sync).
/// Some implementations _may_ be 'Sync', otherwise queue()ing or threadlocal() can be used.
pub struct InnerControlScopeFn<M> {
    flush_on_drop: bool,
    scope_fn: Box<Fn(ScopeCmd<M>)>,
}

// TODO why is this necessary?
unsafe impl<M> Sync for InnerControlScopeFn<M> {}
unsafe impl<M> Send for InnerControlScopeFn<M> {}

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
pub enum ScopeCmd<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

/// Create a new metric scope based on the provided scope function.
pub fn control_scope<M, F>(scope_fn: F) -> ControlScopeFn<M>
    where F: Fn(ScopeCmd<M>) + 'static
{
    Arc::new(InnerControlScopeFn {
        flush_on_drop: true,
        scope_fn: Box::new(scope_fn)
    })
}

impl<M> InnerControlScopeFn<M> {

    /// Write a value to this scope.
    ///
    /// ```rust
    /// let ref mut scope = dipstick::to_log().open_scope(false);
    /// scope.write(&"counter".to_string(), 6);
    /// ```
    ///
    #[inline]
    pub fn write(&self, metric: &M, value: Value) {
        (self.scope_fn)(Write(metric, value))
    }

    /// Flush this scope, if buffered.
    ///
    /// ```rust
    /// let ref mut scope = dipstick::to_log().open_scope(true);
    /// scope.flush();
    /// ```
    ///
    #[inline]
    pub fn flush(&self) {
        (self.scope_fn)(Flush)
    }

}

impl<M> Drop for InnerControlScopeFn<M> {
    fn drop(&mut self) {
        if self.flush_on_drop {
            self.flush()
        }
    }
}

