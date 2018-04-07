//! Static metrics are used to define metrics that share a single persistent metrics scope.
//! Because the scope never changes (it is "global"), all that needs to be provided by the
//! application is the metrics values.
//!
//! Compared to [ScopeMetrics], static metrics are easier to use and provide satisfactory metrics
//! in many applications.
//!
//! If multiple [AppMetrics] are defined, they'll each have their scope.
//!
use core::*;
use core::Kind::*;
use namespace::*;
use cache::*;
use schedule::{schedule, CancelHandle};
use output;

use std::sync::Arc;
use std::time::Duration;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

lazy_static! {
    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<DefineMetric + Send + Sync> =
        output::NO_METRIC_OUTPUT.open_scope_object();
}

/// A non-generic trait to hide MetricScope<M>
pub trait DefineMetric {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn define_metric_object(
        &self,
        kind: Kind,
        name: &str,
        rate: Sampling,
    ) -> Box<WriteMetric + Send + Sync>;

    /// Flush the scope, if it is buffered.
    fn flush_object(&self);
}

/// Dynamic counterpart of the `DispatcherMetric`.
/// Adapter to a metric of unknown type.
pub trait WriteMetric {
    /// Write metric value to a scope.
    /// Observers only receive previously registered handles.
    fn write(&self, value: Value);
}

/// Wrap the metrics backend to provide an application-friendly interface.
/// Open a metric scope to share across the application.
#[deprecated(since = "0.7.0", note = "Use into() instead")]
pub fn app_metrics<M, AM>(scope: AM) -> MetricScope<M>
where
    M: Clone + Send + Sync + 'static,
    AM: Into<MetricScope<M>>,
{
    scope.into()
}

/// Wrap the metrics backend to provide an application-friendly interface.
/// Open a metric scope to share across the application.
pub fn metric_scope<M, AM>(scope: AM) -> MetricScope<M>
where
    M: Clone + Send + Sync + 'static,
    AM: Into<MetricScope<M>>,
{
    scope.into()
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Marker {
    #[derivative(Debug = "ignore")]
    write: WriteFn,
}

impl Marker {
    /// Record a single event occurence.
    pub fn mark(&self) {
        (self.write)(1)
    }
}

/// A counter that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Counter {
    #[derivative(Debug = "ignore")]
    write: WriteFn,
}

impl Counter {
    /// Record a value count.
    pub fn count<V: ToPrimitive>(&self, count: V) {
        (self.write)(count.to_u64().unwrap())
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Gauge {
    #[derivative(Debug = "ignore")]
    write: WriteFn,
}

impl Gauge {
    /// Record a value point for this gauge.
    pub fn value<V: ToPrimitive>(&self, value: V) {
        (self.write)(value.to_u64().unwrap())
    }
}

/// A timer that sends values to the metrics backend
/// Timers can record time intervals in multiple ways :
/// - with the time! macrohich wraps an expression or block with start() and stop() calls.
/// - with the time(Fn) methodhich wraps a closure with start() and stop() calls.
/// - with start() and stop() methodsrapping around the operation to time
/// - with the interval_us() method, providing an externally determined microsecond interval
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Timer {
    #[derivative(Debug = "ignore")]
    write: WriteFn,
}

impl Timer {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us<V: ToPrimitive>(&self, interval_us: V) -> V {
        (self.write)(interval_us.to_u64().unwrap());
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
    pub fn time<F, R>(&self, operations: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start_time = self.start();
        let value: R = operations();
        self.stop(start_time);
        value
    }
}

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Metrics instead")]
pub type AppMetrics<M> = MetricScope<M>;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Marker instead")]
pub type AppMarker = Marker;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Counter instead")]
pub type AppCounter = Counter;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Gauge instead")]
pub type AppGauge = Gauge;

/// Help transition to new syntax
#[deprecated(since = "0.7.0", note = "Use Timer instead")]
pub type AppTimer = Timer;

/// Variations of this should also provide control of the metric recording scope.
#[derive(Derivative, Clone)]
pub struct MetricScope<M> {
    flush_on_drop: bool,
    #[derivative(Debug = "ignore")]
    define_fn: DefineMetricFn<M>,
    #[derivative(Debug = "ignore")]
    command_fn: CommandFn<M>,
}

impl<M> MetricScope<M> {
    /// Create new application metrics instance.
    pub fn new(define_metric_fn: DefineMetricFn<M>, scope: CommandFn<M>) -> Self {
        MetricScope {
            flush_on_drop: true,
            define_fn: define_metric_fn,
            command_fn: scope,
        }
    }
}

fn write_fn<M, D: MetricInput<M> + Clone + Send + Sync + 'static>(scope: &D, kind: Kind, name: &str) -> WriteFn
    where
        M: Clone + Send + Sync + 'static,
{
    let scope = scope.clone();
    let metric = scope.define_metric(kind, name, 1.0);
    Arc::new(move |value| scope.write(&metric, value))
}

/// Define metrics, write values and flush them.
pub trait MetricInput<M>: Clone + Flush + Send + Sync + 'static
    where
        M: Clone + Send + Sync + 'static,
{
    /// Define an event counter of the provided name.
    fn marker(&self, name: &str) -> Marker {
        Marker { write: write_fn(self, Marker, name) }
    }

    /// Define a counter of the provided name.
    fn counter(&self, name: &str) -> Counter {
        Counter { write: write_fn(self, Counter, name) }
    }

    /// Define a timer of the provided name.
    fn timer(&self, name: &str) -> Timer {
        Timer { write: write_fn(self, Timer, name) }
    }

    /// Define a gauge of the provided name.
    fn gauge(&self, name: &str) -> Gauge {
        Gauge { write: write_fn(self, Gauge, name) }
    }

    /// Define a metric of the specified type.
    fn define_metric(&self, kind: Kind, name: &str, rate: Sampling) -> M;

    /// Record or send a value for a previously defined metric.
    fn write(&self, metric: &M, value: Value);
}

/// Scopes can implement buffering, requiring flush operations to commit metric values.
pub trait Flush {
    /// Flushes any recorded metric value.
    /// Has no effect on unbuffered metrics.
    fn flush(&self);
}

impl<M> MetricInput<M> for MetricScope<M>
where
    M: Clone + Send + Sync + 'static,
{
    fn define_metric(&self, kind: Kind, name: &str, rate: Sampling) -> M {
        (self.define_fn)(kind, name, rate)
    }

    fn write(&self, metric: &M, value: Value) {
        self.command_fn.write(metric, value);
    }
}

/// Scopes can implement buffering, in which case they can be flushed.
impl<M> Flush for MetricScope<M> {

    /// Commit all recorded metrics values since the previous call or since the scope was opened.
    /// Has no effect if scope is unbuffered.
    fn flush(&self) {
        self.command_fn.flush();
    }
}

/// Schedule for the metrics aggregated of buffered by downstream metrics sinks to be
/// sent out at regular intervals.
pub trait ScheduleFlush: Flush + Clone + Send + 'static {

    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.clone();
        schedule(period, move || scope.flush())
    }
}

impl<M: Clone + Send + 'static> ScheduleFlush for MetricScope<M> {}

impl<M> Drop for MetricScope<M> {
    fn drop(&mut self) {
        if self.flush_on_drop {
            self.flush()
        }
    }
}

//// Dispatch / Receiver impl

struct MetricWriter<M> {
    define_fn: M,
    command_fn: CommandFn<M>,
}

impl<M: Send + Sync + Clone + 'static> DefineMetric for MetricScope<M> {
    fn define_metric_object(&self, kind: Kind, name: &str, rate: Sampling)
            -> Box<WriteMetric + Send + Sync>
    {
        Box::new(MetricWriter {
            define_fn: self.define_metric(kind, name, rate),
            command_fn: self.command_fn.clone(),
        })
    }

    fn flush_object(&self) {
        self.flush();
    }
}

impl<M> WriteMetric for MetricWriter<M> {
    fn write(&self, value: Value) {
        self.command_fn.write(&self.define_fn, value);
    }
}

//// Mutators impl

impl<M: Send + Sync + Clone + 'static> WithNamespace for MetricScope<M> {
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self {
        let ns = &names.into();
        MetricScope {
            flush_on_drop: self.flush_on_drop,
            define_fn: add_namespace(ns, self.define_fn.clone()),
            command_fn: self.command_fn.clone(),
        }
    }
}

impl<M: Send + Sync + Clone + 'static> WithCache for MetricScope<M> {
    fn with_cache(&self, cache_size: usize) -> Self {
        MetricScope {
            flush_on_drop: self.flush_on_drop,
            define_fn: add_cache(cache_size, self.define_fn.clone()),
            command_fn: self.command_fn.clone(),
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use ::*;
    use test;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let metrics = default_aggregate();
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }

}
