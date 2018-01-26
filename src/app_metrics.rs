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
use schedule::*;
use dispatch::*;

use std::time::Duration;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Wrap the metrics backend to provide an application-friendly interface.
/// Open a metric scope to share across the application.
pub fn app_metrics<M, AM>(app_metrics: AM) -> AppMetrics<M>
where
    M: Clone + Send + Sync + 'static,
    AM: Into<AppMetrics<M>>,
{
    app_metrics.into()
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct AppMarker<M> {
    metric: M,
    #[derivative(Debug = "ignore")] scope: ControlScopeFn<M>,
}

impl<M> AppMarker<M> {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.scope.write(&self.metric, 1);
    }
}

/// A counter that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct AppCounter<M> {
    metric: M,
    #[derivative(Debug = "ignore")] scope: ControlScopeFn<M>,
}

impl<M> AppCounter<M> {
    /// Record a value count.
    pub fn count<V>(&self, count: V)
    where
        V: ToPrimitive,
    {
        self.scope.write(&self.metric, count.to_u64().unwrap());
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct AppGauge<M> {
    metric: M,
    #[derivative(Debug = "ignore")] scope: ControlScopeFn<M>,
}

impl<M> AppGauge<M> {
    /// Record a value point for this gauge.
    pub fn value<V>(&self, value: V)
    where
        V: ToPrimitive,
    {
        self.scope.write(&self.metric, value.to_u64().unwrap());
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
pub struct AppTimer<M> {
    metric: M,
    #[derivative(Debug = "ignore")] scope: ControlScopeFn<M>,
}

impl<M> AppTimer<M> {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us<V>(&self, interval_us: V) -> V
    where
        V: ToPrimitive,
    {
        self.scope.write(&self.metric, interval_us.to_u64().unwrap());
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

//// AppMetrics proper

/// Variations of this should also provide control of the metric recording scope.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct AppMetrics<M> {
    #[derivative(Debug = "ignore")] define_metric_fn: DefineMetricFn<M>,
    #[derivative(Debug = "ignore")] scope: ControlScopeFn<M>,
}

impl<M> AppMetrics<M> {
    /// Create new application metrics instance.
    pub fn new(define_metric_fn: DefineMetricFn<M>, scope: ControlScopeFn<M>, ) -> Self {
        AppMetrics { define_metric_fn, scope }
    }
}

impl<M> AppMetrics<M>
    where
        M: Clone + Send + Sync + 'static,
{
    #[inline]
    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> M {
        (self.define_metric_fn)(kind, name, rate)
    }
    
    /// Get an event counter of the provided name.
    pub fn marker<AS: AsRef<str>>(&self, name: AS) -> AppMarker<M> {
        let metric = self.define_metric(Marker, name.as_ref(), 1.0);
        AppMarker {
            metric,
            scope: self.scope.clone(),
        }
    }

    /// Get a counter of the provided name.
    pub fn counter<AS: AsRef<str>>(&self, name: AS) -> AppCounter<M> {
        let metric = self.define_metric(Counter, name.as_ref(), 1.0);
        AppCounter {
            metric,
            scope: self.scope.clone(),
        }
    }

    /// Get a timer of the provided name.
    pub fn timer<AS: AsRef<str>>(&self, name: AS) -> AppTimer<M> {
        let metric = self.define_metric(Timer, name.as_ref(), 1.0);
        AppTimer {
            metric,
            scope: self.scope.clone(),
        }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<AS: AsRef<str>>(&self, name: AS) -> AppGauge<M> {
        let metric = self.define_metric(Gauge, name.as_ref(), 1.0);
        AppGauge {
            metric,
            scope: self.scope.clone(),
        }
    }

    /// Forcefully flush the backing metrics scope.
    /// This is usually not required since static metrics use auto flushing scopes.
    /// The effect, if any, of this method depends on the selected metrics backend.
    pub fn flush(&self) {
        self.scope.flush();
    }

    /// Schedule for the metrics aggregated of buffered by downstream metrics sinks to be
    /// sent out at regular intervals.
    pub fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.scope.clone();
        schedule(period, move || scope.flush())
    }
}

//// Dispatch / Receiver impl

struct AppReceiverMetric<M> {
    metric: M,
    scope: ControlScopeFn<M>,
}

impl<M: Send + Sync + Clone + 'static> Receiver for AppMetrics<M> {
    fn box_metric(&self, kind: Kind, name: &str, rate: Rate) -> Box<ReceiverMetric + Send + Sync> {
        let scope: ControlScopeFn<M> = self.scope.clone();
        let metric: M = self.define_metric(kind, name, rate);

        Box::new(AppReceiverMetric {
            metric,
            scope,
        })
    }

    fn flush(&self) {
        self.flush()
    }
}

impl<M> ReceiverMetric for AppReceiverMetric<M> {
    fn write(&self, value: Value) {
        self.scope.write(&self.metric, value);
    }
}

//// Mutators impl

impl<M: Send + Sync + Clone + 'static> WithNamespace for AppMetrics<M> {
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self {
        let ref ns = names.into();
        AppMetrics {
            define_metric_fn: add_namespace(ns, self.define_metric_fn.clone()),
            scope: self.scope.clone(),
        }
    }
}

impl<M: Send + Sync + Clone + 'static> WithCache for AppMetrics<M> {
    fn with_cache(&self, cache_size: usize) -> Self {
        AppMetrics {
            define_metric_fn: add_cache(cache_size, self.define_metric_fn.clone()),
            scope: self.scope.clone(),
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use ::*;
    use test;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let sink = aggregate(summary, to_void());
        let metrics = app_metrics(sink);
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }

}
