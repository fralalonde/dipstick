use core::clock::TimeHandle;
use core::{MetricValue, Flush};
use core::name::MetricName;
use core::label::Labels;

use std::sync::Arc;
use std::fmt;

// TODO maybe define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::{ToPrimitive};
pub use num::integer;

/// A function trait that opens a new metric capture scope.
pub trait Input: Send + Sync + 'static + InputDyn {
    /// The type of Scope returned byt this input.
    type SCOPE: InputScope + Send + Sync + 'static;

    /// Open a new scope from this output.
    fn input(&self) -> Self::SCOPE;
}

/// A function trait that opens a new metric capture scope.
pub trait InputDyn: Send + Sync + 'static {
    /// Open a new scope from this output.
    fn input_dyn(&self) -> Arc<InputScope + Send + Sync + 'static>;
}

/// Blanket impl of dyn input trait
impl<T: Input + Send + Sync + 'static> InputDyn for T {
    fn input_dyn(&self) -> Arc<InputScope + Send + Sync + 'static> {
        Arc::new(self.input())
    }
}

/// InputScope
/// Define metrics, write values and flush them.
pub trait InputScope: Flush {
    /// Define a generic metric of the specified type.
    /// It is preferable to use counter() / marker() / timer() / gauge() methods.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric;

    /// Define a counter.
    fn counter(&self, name: &str) -> Counter {
        self.new_metric(name.into(), InputKind::Counter).into()
    }

    /// Define a marker.
    fn marker(&self, name: &str) -> Marker {
        self.new_metric(name.into(), InputKind::Marker).into()
    }

    /// Define a timer.
    fn timer(&self, name: &str) -> Timer {
        self.new_metric(name.into(), InputKind::Timer).into()
    }

    /// Define a gauge.
    fn gauge(&self, name: &str) -> Gauge {
        self.new_metric(name.into(), InputKind::Gauge).into()
    }

    /// Define a level.
    fn level(&self, name: &str) -> Level {
        self.new_metric(name.into(), InputKind::Level).into()
    }
}

/// A metric is actually a function that knows to write a metric value to a metric output.
#[derive(Clone)]
pub struct InputMetric {
    inner: Arc<Fn(MetricValue, Labels) + Send + Sync>
}

impl fmt::Debug for InputMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InputMetric")
    }
}

impl InputMetric {
    /// Utility constructor
    pub fn new<F: Fn(MetricValue, Labels) + Send + Sync + 'static>(metric: F) -> InputMetric {
        InputMetric { inner: Arc::new(metric) }
    }

    /// Collect a new value for this metric.
    #[inline]
    pub fn write(&self, value: MetricValue, labels: Labels) {
        (self.inner)(value, labels)
    }
}

/// Used to differentiate between metric kinds in the backend.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum InputKind {
    /// Handling one item at a time.
    Marker,
    /// Handling cumulative observed quantities.
    Counter,
    /// Handling quantity fluctuations.
    Level,
    /// Reporting instant measurement of a resource at a point in time (non-cumulative).
    Gauge,
    /// Measuring a time interval, internal to the app or provided by an external source.
    Timer,
}

/// Used by the metrics! macro to obtain the InputKind from the stringified type.
impl<'a> From<&'a str> for InputKind {
    fn from(s: &str) -> InputKind {
        match s {
            "Marker" => InputKind::Marker,
            "Counter" => InputKind::Counter,
            "Gauge" => InputKind::Gauge,
            "Timer" => InputKind::Timer,
            "Level" => InputKind::Level,
            _ => panic!("No InputKind '{}' defined", s)
        }
    }
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Debug, Clone)]
pub struct Marker {
    inner: InputMetric,
}

impl Marker {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.inner.write(1, labels![])
    }
}

/// A counter of absolute observed values (non-negative amounts).
/// Used to count to count things that can not be undone:
/// - Bytes sent
/// - Records written
/// - Apples eaten
/// For relative (possibly negative) values, the `Level` counter type can be used.
/// If ag0gregated, minimum and maximum scores will track the collected values, not their sum.
#[derive(Debug, Clone)]
pub struct Counter {
    inner: InputMetric,
}

impl Counter {
    /// Record a value count.
    pub fn count(&self, count: usize) {
        self.inner.write(count as isize, labels![])
    }
}

/// A counter of fluctuating resources accepting positive and negative values.
/// Can be used as a stateful `Gauge` or a as `Counter` of possibly decreasing amounts.
/// - Size of messages in a queue
/// - Strawberries on a conveyor belt
/// If aggregated, minimum and maximum scores will track the sum of values, not the collected values themselves.
#[derive(Debug, Clone)]
pub struct Level {
    inner: InputMetric,
}

impl Level {
    /// Record a positive or negative value count
    pub fn adjust<V: ToPrimitive>(&self, count: V) {
        self.inner.write(count.to_isize().unwrap(), labels![])
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Debug, Clone)]
pub struct Gauge {
    inner: InputMetric,
}

impl Gauge {
    /// Record a value point for this gauge.
    pub fn value<V: ToPrimitive>(&self, value: V) {
        self.inner.write(value.to_isize().unwrap(), labels![])
    }
}

/// A timer that sends values to the metrics backend
/// Timers can record time intervals in multiple ways :
/// - with the time! macrohich wraps an expression or block with start() and stop() calls.
/// - with the time(Fn) methodhich wraps a closure with start() and stop() calls.
/// - with start() and stop() methodsrapping around the operation to time
/// - with the interval_us() method, providing an externally determined microsecond interval
#[derive(Debug, Clone)]
pub struct Timer {
    inner: InputMetric,
}

impl Timer {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us(&self, interval_us: u64) -> u64 {
        self.inner.write(interval_us as isize, labels![]);
        interval_us
    }

    /// Obtain a opaque handle to the current time.
    /// The handle is passed back to the stop() method to record a time interval.
    /// This is actually a convenience method to the TimeHandle::now()
    /// Beware, handles obtained here are not bound to this specific timer instance
    /// _for now_ but might be in the future for safety.
    /// If you require safe multi-timer handles, get them through TimeType::now()
    pub fn start(&self) -> TimeHandle {
        TimeHandle::now()
    }

    /// Record the time elapsed since the start_time handle was obtained.
    /// This call can be performed multiple times using the same handle,
    /// reporting distinct time intervals each time.
    /// Returns the microsecond interval value that was recorded.
    pub fn stop(&self, start_time: TimeHandle) -> MetricValue {
        let elapsed_us = start_time.elapsed_us();
        self.interval_us(elapsed_us) as isize
    }

    /// Record the time taken to execute the provided closure
    pub fn time<F: FnOnce() -> R, R>(&self, operations: F) -> R {
        let start_time = self.start();
        let value: R = operations();
        self.stop(start_time);
        value
    }
}

impl From<InputMetric> for Gauge {
    fn from(metric: InputMetric) -> Gauge {
        Gauge { inner: metric }
    }
}

impl From<InputMetric> for Timer {
    fn from(metric: InputMetric) -> Timer {
        Timer { inner: metric }
    }
}

impl From<InputMetric> for Counter {
    fn from(metric: InputMetric) -> Counter {
        Counter { inner: metric }
    }
}

impl From<InputMetric> for Marker {
    fn from(metric: InputMetric) -> Marker {
        Marker { inner: metric }
    }
}

impl From<InputMetric> for Level {
    fn from(metric: InputMetric) -> Level {
        Level { inner: metric }
    }
}
