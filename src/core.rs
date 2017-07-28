use time;
use std::result::Iter;
use num::ToPrimitive;

//////////////////
// DEFINITIONS

pub type Value = u64;

#[derive(Debug)]
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

pub type Rate = f64;

pub const FULL_SAMPLING_RATE: Rate = 1.0;

#[derive(Debug, Copy, Clone)]
pub enum MetricType {
    Event,
    Count,
    Gauge,
    Time,
}

// Application contract

/// A monotonic counter metric trait.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing potential problems.
pub trait EventMetric {
    fn mark(&self);
}

/// A trait for counters and gauges to report values.
pub trait GaugeMetric {
    fn value<V>(&self, value: V) where V: ToPrimitive;
}

/// A trait for counters and gauges to report values.
pub trait CountMetric {
    fn count<V>(&self, count: V) where V: ToPrimitive;
}

/// A trait for timers to report values.
/// Timers can record time intervals in multiple ways :
/// - with the time! macro, which wraps an expression or block with start() and stop() calls.
/// - with the time(Fn) method, which wraps a closure with start() and stop() calls.
/// - with start() and stop() methods, wrapping around the operation to time
/// - with the interval_us() method, providing an externally determined microsecond interval
pub trait TimerMetric {
    /// Obtain a opaque handle to the current time.
    /// The handle is passed back to the stop() method to record a time interval.
    /// This is actually a convenience method to the TimeHandle::now()
    /// Beware, handles obtained here are not bound to this specific timer instance
    /// _for now_ but might be in the future for safety.
    /// If you require safe multi-timer handles, get them through TimeType::now()
    fn start(&self) -> TimeHandle {
        TimeHandle::now()
    }

    /// Record the time elapsed since the start_time handle was obtained.
    /// This call can be performed multiple times using the same handle,
    /// reporting distinct time intervals each time.
    /// Returns the microsecond interval value that was recorded.
    fn stop(&self, start_time: TimeHandle) -> u64 {
        let elapsed_us = start_time.elapsed_us();
        self.interval_us(elapsed_us)
    }

    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    fn interval_us<V>(&self, count: V) -> V where V: ToPrimitive;

    /// Record the time taken to execute the provided closure
    fn time<F, R>(&self, operations: F) -> R where F: FnOnce() -> R {
        let start_time = self.start();
        let value: R = operations();
        self.stop(start_time);
        value
    }
}

///// A dispatch scope provides a way to group metric values
///// for an operations (i.e. serving a request, processing a message)
//pub trait DispatchScope {
//    /// Free-form properties can be set fluently for the scope, providing downstream metric
//    /// components with contextual information (i.e. user name, message id, etc)
//    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self;
//}

/// Main trait of the metrics API
pub trait MetricDispatch {
    /// type of event metric for this dispatch
    type Event: EventMetric;

    /// type of value metric for this dispatch
    type Count: CountMetric;

    /// type of value metric for this dispatch
    type Gauge: GaugeMetric;

    /// type of timer metric for this dispatch
    type Timer: TimerMetric;

//    /// type of scope for this dispatch
//    type Scope: DispatchScope;

    /// define a new event metric
    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event;

    /// define a new count metric
    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Count;

    /// define a new gauge metric
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Gauge;

    /// define a new timer metric
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer;

//    fn with_scope<F>(&mut self, operations: F)
//    where
//        F: Fn(&Self::Scope);
}

/// Metric sources allow a group of metrics to be defined and written as one.
/// Source implementers may get their data from internally aggregated or buffered metrics
/// or they may read existing metrics not defined by the app (OS counters, etc)
pub trait MetricPublisher {
    fn publish(&self);
}

/// A convenience macro to wrap a block or an expression with a start / stop timer.
/// Elapsed time is sent to the supplied statsd client after the computation has been performed.
/// Expression result (if any) is transparently returned.
#[macro_export]
macro_rules! time {
    ($timer: expr, $body: expr) => {{
        let start_time = $timer.start();
        let value = $body;
        $timer.stop(start_time);
        value
    }}
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
pub trait MetricSink {
    type Metric: MetricKey;
    type Writer: MetricWriter<Self::Metric>;

    /// Define a new sink-specific metric that can be used for writing values.
    fn new_metric<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> Self::Metric;

    /// Open a metric writer to write metrics to.
    /// Some sinks reuse the same writer while others allocate resources for every new writer.
    fn new_writer(&self) -> Self::Writer;
}

/// A metric identifier defined by a specific metric sink implementation.
/// Passed back to when writing a metric value
/// May carry state specific to the sink's implementation
pub trait MetricKey {}

/// A sink-specific target for writing metrics to.
pub trait MetricWriter<M: MetricKey>: Send {
    /// Write a single metric value
    fn write(&self, metric: &M, value: Value);

    /// Some sinks may have buffering capability.
    /// Flushing makes sure all previously written metrics are propagated
    /// down the sink chain and to any applicable external outputs.
    fn flush(&self) {}
}
