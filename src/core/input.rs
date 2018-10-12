use core::clock::TimeHandle;
use core::{Value, Flush};
use core::name::Name;
use ::{Labels};

use std::sync::Arc;
use std::fmt;

// TODO maybe define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

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
    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric;

    /// Define a counter.
    fn counter(&self, name: &str) -> Counter {
        self.new_metric(name.into(), Kind::Counter).into()
    }

    /// Define a marker.
    fn marker(&self, name: &str) -> Marker {
        self.new_metric(name.into(), Kind::Marker).into()
    }

    /// Define a timer.
    fn timer(&self, name: &str) -> Timer {
        self.new_metric(name.into(), Kind::Timer).into()
    }

    /// Define a gauge.
    fn gauge(&self, name: &str) -> Gauge {
        self.new_metric(name.into(), Kind::Gauge).into()
    }

}

/// A metric is actually a function that knows to write a metric value to a metric output.
#[derive(Clone)]
pub struct InputMetric {
    inner: Arc<Fn(Value, Labels) + Send + Sync>
}

impl fmt::Debug for InputMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InputMetric")
    }
}

impl InputMetric {
    /// Utility constructor
    pub fn new<F: Fn(Value, Labels) + Send + Sync + 'static>(metric: F) -> InputMetric {
        InputMetric { inner: Arc::new(metric) }
    }

    /// Collect a new value for this metric.
    #[inline]
    pub fn write(&self, value: Value, labels: Labels) {
        (self.inner)(value, labels)
    }
}

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

/// Used by the metrics! macro to obtain the Kind from the stringified type.
impl<'a> From<&'a str> for Kind {
    fn from(s: &str) -> Kind {
        match s {
            "Marker" => Kind::Marker,
            "Counter" => Kind::Counter,
            "Gauge" => Kind::Gauge,
            "Timer" => Kind::Timer,
            _ => panic!("No Kind '{}' defined", s)
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

/// A counter that sends values to the metrics backend
#[derive(Debug, Clone)]
pub struct Counter {
    inner: InputMetric,
}

impl Counter {
    /// Record a value count.
    pub fn count<V: ToPrimitive>(&self, count: V) {
        self.inner.write(count.to_u64().unwrap(), labels![])
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
        self.inner.write(value.to_u64().unwrap(), labels![])
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
    pub fn interval_us<V: ToPrimitive>(&self, interval_us: V) -> V {
        self.inner.write(interval_us.to_u64().unwrap(), labels![]);
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
    pub fn stop(&self, start_time: TimeHandle) -> u64 {
        let elapsed_us = start_time.elapsed_us();
        self.interval_us(elapsed_us)
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
