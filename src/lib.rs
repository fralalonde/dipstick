//! A fast and modular metrics library decoupling app instrumentation from reporting backend.
//! Similar to popular logging frameworks, but with counters and timers.
//! Can be configured for combined outputs (log + statsd), random sampling, local aggregation
//! of metrics, recurrent background publication, etc.

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
// variant_size_differences,
)]

#[cfg(feature = "bench")]
extern crate test;

#[macro_use]
extern crate log;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;

extern crate time;
extern crate cached;
extern crate num;
extern crate scheduled_executor;

mod pcg32;
mod multi;
mod queue;

pub mod error {
    //! Dipstick uses error_chain to handle the critical errors that might crop up when assembling the backend.
    error_chain! {
        foreign_links {
            Io(::std::io::Error);
        }
    }
}

pub mod sampling;
pub mod aggregate;
pub mod publish;
pub mod statsd;
pub mod logging;
pub mod cache;
//pub mod queue;

pub use num::ToPrimitive;
pub use std::net::ToSocketAddrs;

pub use aggregate::*;
pub use multi::*;
pub use publish::*;
pub use statsd::*;
pub use logging::*;
pub use cache::*;

use std::sync::Arc;
use std::fmt::Debug;

//////////////////
// TYPES

/// Base type for recorded metric values.
// TODO should this be f64? or impl AsValue and drop num crate
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
/// 1.0 takes everything.
/// 0.5 takes one sample every two measures.
/// 0.0 takes nothing.
pub type Rate = f64;

/// Do not sample, use all data.
pub const FULL_SAMPLING_RATE: Rate = 1.0;

//////////////////
// FRONTEND

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing potential problems.
#[derive(Debug)]
pub struct Event<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    target_writer: Arc<<C as MetricSink>::Writer>,
}

impl<C: MetricSink> Event<C> {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.target_writer.write(&self.metric, 1);
    }
}

/// A counter that sends values to the metrics backend
#[derive(Debug)]
pub struct Gauge<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    target_writer: Arc<<C as MetricSink>::Writer>,
}

impl<C: MetricSink> Gauge<C> {
    /// Record a value point for this gauge.
    pub fn value<V>(&self, value: V) where V: ToPrimitive {
        self.target_writer.write(&self.metric, value.to_u64().unwrap());
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Debug)]
pub struct Counter<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    target_writer: Arc<<C as MetricSink>::Writer>,
}

impl<C: MetricSink> Counter<C> {
    /// Record a value count.
    pub fn count<V>(&self, count: V) where V: ToPrimitive {
        self.target_writer.write(&self.metric, count.to_u64().unwrap());
    }
}

/// A timer that sends values to the metrics backend
/// Timers can record time intervals in multiple ways :
/// - with the time! macro, which wraps an expression or block with start() and stop() calls.
/// - with the time(Fn) method, which wraps a closure with start() and stop() calls.
/// - with start() and stop() methods, wrapping around the operation to time
/// - with the interval_us() method, providing an externally determined microsecond interval
#[derive(Debug)]
pub struct Timer<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    target_writer: Arc<<C as MetricSink>::Writer>,
}

impl<C: MetricSink> Timer<C> {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us<V>(&self, interval_us: V) -> V where V: ToPrimitive {
        self.target_writer.write(&self.metric, interval_us.to_u64().unwrap());
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
    pub fn time<F, R>(&self, operations: F) -> R where F: FnOnce() -> R {
        let start_time = self.start();
        let value: R = operations();
        self.stop(start_time);
        value
    }
}

/// Application metrics are defined here.
#[derive(Debug)]
pub struct Metrics<C: MetricSink + 'static> {
    prefix: String,
    target: Arc<C>,
    writer: Arc<<C as MetricSink>::Writer>,
}

impl<C: MetricSink> Metrics<C> {
    /// Create a new direct metric dispatch
    pub fn new(target: C) -> Metrics<C> {
        let target_writer = target.new_writer();
        Metrics {
            prefix: "".to_string(),
            target: Arc::new(target),
            writer: Arc::new(target_writer),
        }
    }

    fn qualified_name<S: Into<String> + AsRef<str>>(&self, name: S) -> String {
        // FIXME is there a way to return <S> in both cases?
        if self.prefix.is_empty() {
            return name.into()
        }
        let mut buf:String = self.prefix.clone();
        buf.push_str(name.as_ref());
        buf.to_string()
    }

    /// Get an event counter of the provided name.
    pub fn event<S: Into<String> + AsRef<str>>(&self, name: S) -> Event<C> {
        let metric = self.target.new_metric(MetricKind::Event, self.qualified_name(name), 1.0);
        Event {
            metric,
            target_writer: self.writer.clone(),
        }
    }

    /// Get a counter of the provided name.
    pub fn counter<S: Into<String> + AsRef<str>>(&self, name: S) -> Counter<C> {
        let metric = self.target.new_metric(MetricKind::Count, self.qualified_name(name), 1.0);
        Counter {
            metric,
            target_writer: self.writer.clone(),
        }
    }

    /// Get a timer of the provided name.
    pub fn timer<S: Into<String> + AsRef<str>>(&self, name: S) -> Timer<C> {
        let metric = self.target.new_metric(MetricKind::Time, self.qualified_name(name), 1.0);
        Timer{
            metric,
            target_writer: self.writer.clone(),
        }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<S: Into<String> + AsRef<str>>(&self, name: S) -> Gauge<C> {
        let metric = self.target.new_metric(MetricKind::Gauge, self.qualified_name(name), 1.0);
        Gauge{
            metric,
            target_writer: self.writer.clone(),
        }
    }

    /// Prepend the metrics name with a prefix.
    /// Does not affect metrics that were already obtained.
    pub fn with_prefix<S: Into<String>>(&self, prefix: S) -> Self {
        Metrics {
            prefix: prefix.into(),
            target: self.target.clone(),
            writer: self.writer.clone(),
        }
    }
}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature = "bench")]
mod bench {

    use aggregate::MetricAggregator;
    use ::*;
    use test::Bencher;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut Bencher) {
        let (sink, source) = aggregate();
        let metrics = metrics(sink);
        let event = metrics.event("aaa");
        b.iter(|| event.mark());
    }

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

/// Used to differentiate between metric kinds in the backend.
#[derive(Debug, Copy, Clone)]
pub enum MetricKind {
    /// Is one item handled?
    Event,
    /// How many items are handled?
    Count,
    /// How much are we using or do we have left?
    Gauge,
    /// How long does this take?
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
pub trait MetricSink: Debug {
    /// Metric identifier type of this sink.
    type Metric: MetricKey + Debug + Send + Sync;

    /// Metric writer type of this sink.
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
pub trait MetricKey: Debug {}

/// A sink-specific target for writing metrics to.
pub trait MetricWriter<M: MetricKey>: Send + Sync + Debug {
    /// Write a single metric value
    fn write(&self, metric: &M, value: Value);

    /// Some sinks may have buffering capability.
    /// Flushing makes sure all previously written metrics are propagated
    /// down the sink chain and to any applicable external outputs.
    fn flush(&self) {}
}

/// Metric source trait
pub trait AsSource {
    /// Get the metric source.
    fn as_source(&self) -> AggregateSource;
}

/// Metric sink trait
pub trait AsSink<S: MetricSink> {
    /// Get the metric sink.
    fn as_sink(&self) -> S;
}

/// Metric sink trait
pub trait IntoSink<S: MetricSink> {
    /// Get the metric sink.
    fn into_sink(self) -> S;
}

/// Tautology - Any sink can turn into itself, duh.
impl<S: MetricSink> IntoSink<S> for S {
    fn into_sink(self) -> Self {
        self
    }
}

/// Wrap the metrics backend to provide an application-friendly interface.
pub fn metrics<IS, S>(sink: IS) -> Metrics<S> where IS: IntoSink<S>, S: MetricSink {
    Metrics::new(sink.into_sink())
}

/// Perform random sampling of values according to the specified rate.
pub fn sample<IS, S>(rate: Rate, sink: IS) -> sampling::SamplingSink<S> where IS: IntoSink<S>, S: MetricSink {
    sampling::SamplingSink::new(sink.into_sink(), rate)
}

/// Cache metrics to prevent them from being re-defined on every use.
/// Use of this should be transparent, this has no effect on the values.
/// Stateful sinks (i.e. Aggregate) may naturally cache their definitions.
pub fn cache<IS, S>(size: usize, sink: IS) -> cache::MetricCache<S> where IS: IntoSink<S>, S: MetricSink {
    cache::MetricCache::new(sink.into_sink(), size)
}

/// Send metric to a logger.
/// This uses the basic log crate as it is configured for the application.
pub fn log<S: AsRef<str>>(log: S) -> logging::LoggingSink {
    logging::LoggingSink::new(log)
}

/// Send metrics to a statsd server at the address and port provided.
pub fn statsd<S: AsRef<str>, A: ToSocketAddrs>(address: A, prefix: S) -> error::Result<statsd::StatsdSink> {
    Ok(statsd::StatsdSink::new(address, prefix)?)
}

/// Twin metrics sink
impl<IS1, S1, IS2, S2> IntoSink<multi::DoubleSink<S1, S2>> for (IS1, IS2) where IS1: IntoSink<S1>, S1: MetricSink,  IS2: IntoSink<S2>, S2: MetricSink  {
    fn into_sink(self) -> multi::DoubleSink<S1, S2> {
        multi::DoubleSink::new(self.0.into_sink(), self.1.into_sink())
    }
}

/// Triple metrics sink
impl<IS1, S1, IS2, S2, IS3, S3> IntoSink<multi::DoubleSink<S1, multi::DoubleSink<S2, S3>>> for (IS1, IS2, IS3) where IS1: IntoSink<S1>, S1: MetricSink, IS2: IntoSink<S2>, S2: MetricSink,  IS3: IntoSink<S3>, S3: MetricSink  {
    fn into_sink(self) -> multi::DoubleSink<S1, multi::DoubleSink<S2, S3>> {
        multi::DoubleSink::new(self.0.into_sink(), multi::DoubleSink::new(self.1.into_sink(), self.2.into_sink()))
    }
}

/// Aggregate metrics in memory.
/// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
/// Needs to be connected to a publish to be useful.
///
/// ```
/// use dipstick::*;
///
/// let (sink, source) = aggregate();
/// let metrics = metrics(sink);
///
/// metrics.event("my_event").mark();
/// metrics.event("my_event").mark();
/// ```
pub fn aggregate() -> (AggregateSink, AggregateSource) {
    let agg = MetricAggregator::new();
    (agg.as_sink(), agg.as_source())
}

/// Publishes all metrics from a source to a backend.
///
/// ```
/// use dipstick::*;
///
/// let (sink, source) = aggregate();
/// let publisher = publish(source, log("aggregated"));
///
/// publisher.publish()
/// ```
pub fn publish<S: MetricSink + Sync>(source: AggregateSource, sink: S) -> AggregatePublisher<S> {
    publish::AggregatePublisher::new(source, sink,)
}