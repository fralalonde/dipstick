use time;
use std::result::Iter;

//////////////////
// DEFINITIONS

pub type Value = u64;

#[derive(Debug)]
pub struct TimeType (u64);

impl TimeType {
    pub fn now() -> TimeType {
        TimeType(time::precise_time_ns())
    }

    pub fn elapsed_ms(self) -> Value {
        (TimeType::now().0 - self.0) / 1_000_000
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

pub trait EventMetric {
    fn mark(&self);
}

pub trait ValueMetric {
    fn value(&self, value: Value);
}

pub trait TimerMetric: ValueMetric {
    fn start(&self) -> TimeType { TimeType::now() }

    fn stop(&self, start_time: TimeType) -> u64 {
        let elapsed_ms = start_time.elapsed_ms();
        self.value(elapsed_ms);
        elapsed_ms
    }
}

/// Identifies a metric dispatch scope.
pub trait MetricScope {
    // TODO enable free-form scope properties
    // fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self;
}

/// Main trait of the metrics API
pub trait MetricDispatch {
    type Event: EventMetric;
    type Value: ValueMetric;
    type Timer: TimerMetric;
    type Scope: MetricScope;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event;
    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value;
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer;
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value;

    fn scope<F>(&mut self, operations: F) where F: Fn(/*&Self::Scope*/);
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
    ($timer: expr, $body: block) => {{
        let start_time = $timer.start();
        $body
        $timer.stop(start_time);
    }};
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
    type Metric: SinkMetric;
    type Writer: SinkWriter<Self::Metric>;

    /// Define a new sink-specific metric that can be used for writing values.
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> Self::Metric;

    /// Open a metric writer to write metrics to.
    /// Some sinks actually reuse the same writer while others allocate resources for every new writer.
    fn new_writer(&self) -> Self::Writer;
}

/// A metric identifier defined by a specific metric sink implementation.
/// Passed back to when writing a metric value
/// May carry state specific to the sink's implementation
pub trait SinkMetric {}

/// A sink-specific target for writing metrics to.
pub trait SinkWriter<M: SinkMetric>: Send {
    /// Write a single metric value
    fn write(&self, metric: &M, value: Value);

    /// Some sinks may have buffering capability.
    /// Flushing makes sure all previously written metrics are propagated
    /// down the sink chain and to any applicable external outputs.
    fn flush(&self) {}
}


