//! Chain of command for unscoped metrics.

use core::*;
use core::Kind::*;
use app_metrics::AppMetrics;

use std::sync::Arc;

use cache::*;
use namespace::*;

/// A pair of functions composing a twin "chain of command".
/// This is the building block for the metrics backend.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct LocalMetrics<M> {
    #[derivative(Debug = "ignore")]
    define_metric_fn: DefineMetricFn<M>,

    #[derivative(Debug = "ignore")]
    scope_metric_fn: OpenScopeFn<M>,
}

impl<M> LocalMetrics<M> {
    /// Define a new metric.
    #[allow(unused_variables)]
    pub fn define_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        (self.define_metric_fn)(kind, name, sampling)
    }

    /// Open a new metric scope.
    /// Scope metrics allow an application to emit per-operation statistics,
    /// For example, producing a per-request performance log.
    ///
    /// Although the scope metrics can be predefined like in ['AppMetrics'], the application needs to
    /// create a scope that will be passed back when reporting scoped metric values.
    ///
    /// ```rust
    /// use dipstick::*;
    /// let scope_metrics = to_log();
    /// let request_counter = scope_metrics.counter("scope_counter");
    /// {
    ///     let ref mut request_scope = scope_metrics.open_scope(true);
    ///     request_counter.count(request_scope, 42);
    /// }
    /// ```
    ///
    pub fn open_scope(&self, buffered: bool) -> ControlScopeFn<M> {
        (self.scope_metric_fn)(buffered)
    }

    /// Open a buffered scope.
    #[inline]
    pub fn buffered_scope(&self) -> ControlScopeFn<M> {
        self.open_scope(true)
    }

    /// Open an unbuffered scope.
    #[inline]
    pub fn unbuffered_scope(&self) -> ControlScopeFn<M> {
        self.open_scope(false)
    }
}

impl<M: Send + Sync + Clone + 'static> LocalMetrics<M> {
    /// Create a new metric chain with the provided metric definition and scope creation functions.
    pub fn new<MF, WF>(make_metric: MF, make_scope: WF) -> Self
    where
        MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
        WF: Fn(bool) -> ControlScopeFn<M> + Send + Sync + 'static,
    {
        LocalMetrics {
            // capture the provided closures in Arc to provide cheap clones
            define_metric_fn: Arc::new(make_metric),
            scope_metric_fn: Arc::new(make_scope),
        }
    }

    /// Get an event counter of the provided name.
    pub fn marker<AS: AsRef<str>>(&self, name: AS) -> LocalMarker<M> {
        let metric = self.define_metric(Marker, name.as_ref(), 1.0);
        LocalMarker { metric }
    }

    /// Get a counter of the provided name.
    pub fn counter<AS: AsRef<str>>(&self, name: AS) -> LocalCounter<M> {
        let metric = self.define_metric(Counter, name.as_ref(), 1.0);
        LocalCounter { metric }
    }

    /// Get a timer of the provided name.
    pub fn timer<AS: AsRef<str>>(&self, name: AS) -> LocalTimer<M> {
        let metric = self.define_metric(Timer, name.as_ref(), 1.0);
        LocalTimer { metric }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<AS: AsRef<str>>(&self, name: AS) -> LocalGauge<M> {
        let metric = self.define_metric(Gauge, name.as_ref(), 1.0);
        LocalGauge { metric }
    }

    /// Intercept both metric definition and scope creation, possibly changing the metric type.
    pub fn mod_both<MF, N>(&self, mod_fn: MF) -> LocalMetrics<N>
    where
        MF: Fn(DefineMetricFn<M>, OpenScopeFn<M>) -> (DefineMetricFn<N>, OpenScopeFn<N>),
        N: Clone + Send + Sync,
    {
        let (metric_fn, scope_fn) =
            mod_fn(self.define_metric_fn.clone(), self.scope_metric_fn.clone());
        LocalMetrics {
            define_metric_fn: metric_fn,
            scope_metric_fn: scope_fn,
        }
    }

    /// Intercept scope creation.
    pub fn mod_scope<MF>(&self, mod_fn: MF) -> Self
    where
        MF: Fn(OpenScopeFn<M>) -> OpenScopeFn<M>,
    {
        LocalMetrics {
            define_metric_fn: self.define_metric_fn.clone(),
            scope_metric_fn: mod_fn(self.scope_metric_fn.clone()),
        }
    }
}

impl<M> From<LocalMetrics<M>> for AppMetrics<M> {
    fn from(metrics: LocalMetrics<M>) -> AppMetrics<M> {
        AppMetrics::new(metrics.define_metric_fn.clone(), metrics.open_scope(false))
    }
}

impl<M: Send + Sync + Clone + 'static> WithCache for LocalMetrics<M> {
    fn with_cache(&self, cache_size: usize) -> Self {
        LocalMetrics {
            define_metric_fn: add_cache(cache_size, self.define_metric_fn.clone()),
            scope_metric_fn: self.scope_metric_fn.clone(),
        }
    }
}

impl<M: Send + Sync + Clone + 'static> WithNamespace for LocalMetrics<M> {
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self {
        let ref ninto = names.into();
        LocalMetrics {
            define_metric_fn: add_namespace(ninto, self.define_metric_fn.clone()),
            scope_metric_fn: self.scope_metric_fn.clone(),
        }
    }
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LocalMarker<M> {
    metric: M,
}

impl<M> LocalMarker<M> {
    /// Record a single event occurence.
    #[inline]
    pub fn mark(&self, scope: &mut ControlScopeFn<M>) {
        scope.write(&self.metric, 1);
    }
}

/// A counter that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LocalCounter<M> {
    metric: M,
}

impl<M> LocalCounter<M> {
    /// Record a value count.
    #[inline]
    pub fn count<V>(&self, scope: &mut ControlScopeFn<M>, count: V)
    where
        V: ToPrimitive,
    {
        scope.write(&self.metric, count.to_u64().unwrap());
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LocalGauge<M> {
    metric: M,
}

impl<M: Clone> LocalGauge<M> {
    /// Record a value point for this gauge.
    #[inline]
    pub fn value<V>(&self, scope: &mut ControlScopeFn<M>, value: V)
    where
        V: ToPrimitive,
    {
        scope.write(&self.metric, value.to_u64().unwrap());
    }
}

/// A timer that sends values to the metrics backend
/// Timers can record time intervals in multiple ways :
/// - with the time! macro which wraps an expression or block with start() and stop() calls.
/// - with the time(Fn) method which wraps a closure with start() and stop() calls.
/// - with start() and stop() methods wrapping around the operation to time
/// - with the interval_us() method, providing an externally determined microsecond interval
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LocalTimer<M> {
    metric: M,
}

impl<M: Clone> LocalTimer<M> {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    #[inline]
    pub fn interval_us<V>(&self, scope: &mut ControlScopeFn<M>, interval_us: V) -> V
    where
        V: ToPrimitive,
    {
        scope.write(&self.metric, interval_us.to_u64().unwrap());
        interval_us
    }

    /// Obtain a opaque handle to the current time.
    /// The handle is passed back to the stop() method to record a time interval.
    /// This is actually a convenience method to the TimeHandle::now()
    /// Beware, handles obtained here are not bound to this specific timer instance
    /// _for now_ but might be in the future for safety.
    /// If you require safe multi-timer handles, get them through TimeType::now()
    #[inline]
    pub fn start(&self) -> TimeHandle {
        TimeHandle::now()
    }

    /// Record the time elapsed since the start_time handle was obtained.
    /// This call can be performed multiple times using the same handle,
    /// reporting distinct time intervals each time.
    /// Returns the microsecond interval value that was recorded.
    #[inline]
    pub fn stop(&self, scope: &mut ControlScopeFn<M>, start_time: TimeHandle) -> u64 {
        let elapsed_us = start_time.elapsed_us();
        self.interval_us(scope, elapsed_us)
    }

    /// Record the time taken to execute the provided closure
    #[inline]
    pub fn time<F, R>(&self, scope: &mut ControlScopeFn<M>, operations: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start_time = self.start();
        let value: R = operations();
        self.stop(scope, start_time);
        value
    }
}
