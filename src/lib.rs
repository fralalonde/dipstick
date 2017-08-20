#![cfg_attr(feature = "bench", feature(test))]

#![warn(
missing_copy_implementations,
missing_debug_implementations,
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_extern_crates,
unused_import_braces,
unused_qualifications,
variant_size_differences,
)]

#[cfg(feature = "bench")]
extern crate test;

extern crate time;

extern crate cached;
extern crate thread_local_object;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;
extern crate num;
extern crate scheduled_executor;

#[macro_use]
extern crate error_chain;

mod errors {
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}

use errors::*;

pub mod dual;
pub mod dispatch;
pub mod sampling;
pub mod aggregate;
pub mod publish;
pub mod statsd;
pub mod logging;
pub mod pcg32;
pub mod cache;

pub use num::ToPrimitive;
pub use std::net::ToSocketAddrs;

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
pub enum MetricKind {
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
    /// Record a single event occurence.
    fn mark(&self);
}

/// A trait for counters and gauges to report values.
pub trait GaugeMetric {
    /// Record a value point for this gauge.
    fn value<V>(&self, value: V) where V: ToPrimitive;
}

/// A trait for counters and gauges to report values.
pub trait CountMetric {
    /// Record a value count.
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

    /// define a new event metric
    fn event<S: AsRef<str>>(&self, name: S) -> Self::Event;

    /// define a new count metric
    fn counter<S: AsRef<str>>(&self, name: S) -> Self::Count;

    /// define a new gauge metric
    fn gauge<S: AsRef<str>>(&self, name: S) -> Self::Gauge;

    /// define a new timer metric
    fn timer<S: AsRef<str>>(&self, name: S) -> Self::Timer;

    fn with_prefix<S: AsRef<str>>(&self, prefix: S) -> Self;
}

/// A dispatch scope provides a way to group metric values
/// for an operations (i.e. serving a request, processing a message)
pub trait DispatchScope {
    /// Free-form properties can be set fluently for the scope, providing downstream metric
    /// components with contextual information (i.e. user name, message id, etc)
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self;
}

pub trait ScopingDispatch {
    /// type of scope for this dispatch
    type Scope: DispatchScope;

    fn with_scope<F>(&mut self, operations: F)
        where
            F: Fn(&Self::Scope);
}

/// Metric sources allow a group of metrics to be defined and written as one.
/// Source implementers may get their data from internally aggregated or buffered metrics
/// or they may read existing metrics not defined by the app (OS counters, etc)
pub trait MetricPublish {
    fn publish(&self);
}

///////////
//// MACROS

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

////////////
//// BACKEND

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
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate) -> Self::Metric;

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

pub trait Builder<T> {
    fn build(&self) -> Result<T>;
}

pub fn metrics<S>(sink: S) -> dispatch::DirectDispatch<S> where S: MetricSink {
    dispatch::DirectDispatch::new(sink)
}

pub fn sample<S>(rate: Rate, sink: S) -> sampling::SamplingSink<S> where S: MetricSink {
    sampling::SamplingSink::new(sink, rate)
}

pub fn cache<S>(size: usize, sink: S) -> cache::MetricCache<S> where S: MetricSink {
    cache::MetricCache::new(sink, size)
}

pub fn log<S: AsRef<str>>(log: S) -> logging::LoggingSink {
    logging::LoggingSink::new(log)
}

pub fn statsd<S: AsRef<str>, A: ToSocketAddrs>(connection: A, prefix: S) -> Result<statsd::StatsdSink> {
    Ok(statsd::StatsdSink::new(connection, prefix)?)
}

pub fn combine<S1: MetricSink, S2: MetricSink>(s1: S1, s2: S2) -> dual::DualSink<S1, S2> {
    dual::DualSink::new(s1, s2)
}

pub fn aggregate() -> aggregate::MetricAggregator {
    aggregate::MetricAggregator::new()
}

pub fn publish<S1: MetricSink>(source: aggregate::AggregateSource, s1: S1) -> publish::AggregatePublisher<S1> {
    publish::AggregatePublisher::new(s1, source)
}