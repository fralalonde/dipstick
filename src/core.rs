use  std::sync::Arc;
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
pub enum MetricKind {
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
pub trait Sink<M, W> where W: Writer<M> {
    /// Define a new sink-specific metric that can be used for writing values.
    fn new_metric<STR: AsRef<str>>(&self, kind: MetricKind, name: STR, sampling: Rate) -> M;

    /// Open a metric writer to write metrics to.
    /// Some sinks reuse the same writer while others allocate resources for every new writer.
    fn new_writer(&self) -> W;
}

/// A sink-specific target for writing metrics to.
pub trait Writer<M> {
    /// Write a single metric value
    fn write(&self, metric: &M, value: Value);

    /// Some sinks may have buffering capability.
    /// Flushing makes sure all previously written metrics are propagated
    /// down the sink chain and to any applicable external outputs.
    fn flush(&self) {}
}

pub trait AsSink<M, W, S> where W: Writer<M>, S: Sink<M, W> {
    /// Get the metric sink.
    fn as_sink(&self) -> S;
}
