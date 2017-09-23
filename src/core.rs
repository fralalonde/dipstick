use time;

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
    Event,
    /// How many items were handled?
    Count,
    /// How much are we using or do we have left?
    Gauge,
    /// How long did this take?
    Time,
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
///
pub trait Sink<M> {
    fn new_metric<STR: AsRef<str>>(&self, kind: Kind, name: STR, sampling: Rate) -> M;

    /// Returns a callback function to send scope commands.
    /// Writes can be performed by passing Some((&Metric, Value))
    /// Flushes can be performed by passing None
    fn new_scope(&self) -> Box<Fn(Option<(&M, Value)>)>;
}


pub trait AsSink<M, S: Sink<M>> {
    /// Get the metric sink.
    fn as_sink(&self) -> S;
}
