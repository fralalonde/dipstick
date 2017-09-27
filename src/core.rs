//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use time;
use std::sync::Arc;

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

//////////////
//// BACKEND

/// Used to differentiate between metric kinds in the backend.
#[derive(Debug, Copy, Clone)]
pub enum Kind {
    /// Was one item handled?
    Marker,
    /// How many items were handled?
    Counter,
    /// How much are we using or do we have left?
    Gauge,
    /// How long did this take?
    Timer,
}

/// Scope creation function.
/// Returns a callback function to send commands to the metric scope.
/// Used to write values to the scope or flush the scope buffer (if applicable).
/// Simple applications may use only one scope.
/// Complex applications may define a new scope fo each operation or request.
/// Scopes can be moved acrossed threads (Send) but are not required to be thread-safe (Sync).
/// Some implementations _may_ be 'Sync', otherwise queue()ing or threadlocal() can be used.
pub type ScopeFn<M> = Arc<Fn(Scope<M>) + Send + Sync>;

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
pub enum Scope<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

/// Main trait of the metrics backend API.
/// Defines a component that can be used when setting up a metrics backend stack.
/// Intermediate sinks transform how metrics are defined and written:
/// - Sampling
/// - Dual
/// - Cache
/// Terminal sinks store or propagate metric values to other systems.
/// - Statsd
/// - Log
/// - Aggregate
/// Print metrics to Generic.
pub trait Sink<M> where M: Send + Sync {
    /// Define a new metric instrument of the requested kind, with the specified name and sample rate.
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M;

    /// Returns a callback function to send scope commands.
    /// Writes can be performed by passing Some((&Metric, Value))
    /// Flushes can be performed by passing None
    fn new_scope(&self) -> ScopeFn<M>;
}

/// Expose the `Sink` nature of a multi-faceted struct.
pub trait AsSink<M, S: Sink<M>> where M: Send + Sync {
    /// Get the metric sink.
    fn as_sink(&self) -> S;
}
