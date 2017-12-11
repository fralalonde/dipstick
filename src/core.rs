//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use time;
use std::sync::Arc;

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


/// Dynamic metric definition function.
/// Metrics can be defined from any thread, concurrently (Fn is Sync).
/// The resulting metrics themselves can be also be safely shared across threads (<M> is Send + Sync).
/// Concurrent usage of a metric is done using threaded scopes.
/// Shared concurrent scopes may be provided by some backends (aggregate).
pub type DefineMetricFn<M> = Arc<Fn(Kind, &str, Rate) -> M + Send + Sync>;

/// A function trait that opens a new metric capture scope.
///
pub type OpenScopeFn<M> = Arc<Fn(bool) -> ControlScopeFn<M> + Send + Sync>;

/// Returns a callback function to send commands to the metric scope.
/// Writes can be performed by passing Some((&Metric, Value))
/// Flushes can be performed by passing None
/// Used to write values to the scope or flush the scope buffer (if applicable).
/// Simple applications may use only one scope.
/// Complex applications may define a new scope fo each operation or request.
/// Scopes can be moved acrossed threads (Send) but are not required to be thread-safe (Sync).
/// Some implementations _may_ be 'Sync', otherwise queue()ing or threadlocal() can be used.
///
pub type ControlScopeFn<M> = Arc<Fn(ScopeCmd<M>) + Send + Sync>;

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
///
pub enum ScopeCmd<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

/// A pair of functions composing a twin "chain of command".
/// This is the building block for the metrics backend.
///
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Chain<M> {
    #[derivative(Debug = "ignore")]
    define_metric_fn: DefineMetricFn<M>,

    #[derivative(Debug = "ignore")]
    scope_metric_fn: OpenScopeFn<M>,
}

impl<M: Send + Sync> Chain<M> {

    /// Define a new metric.
    ///
    #[allow(unused_variables)]
    pub fn define_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        (self.define_metric_fn)(kind, name, sampling)
    }

    /// Open a new metric scope.
    ///
    #[allow(unused_variables)]
    pub fn open_scope(&self, auto_flush: bool) -> ControlScopeFn<M> {
        (self.scope_metric_fn)(auto_flush)
    }

    /// Create a new metric chain with the provided metric definition and scope creation functions.
    ///
    pub fn new<MF, WF>(make_metric: MF, make_scope: WF) -> Self
        where
            MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
            WF: Fn(bool) -> ControlScopeFn<M> + Send + Sync + 'static,
    {
        Chain {
            // capture the provided closures in Arc to provide cheap clones
            define_metric_fn: Arc::new(make_metric),
            scope_metric_fn: Arc::new(make_scope),
        }
    }

    /// Intercept metric definition without changing the metric type.
    ///
    pub fn mod_metric<MF>(&self, mod_fn: MF) -> Chain<M>
    where
        MF: Fn(DefineMetricFn<M>) -> DefineMetricFn<M>,
    {
        Chain {
            define_metric_fn: mod_fn(self.define_metric_fn.clone()),
            scope_metric_fn: self.scope_metric_fn.clone()
        }
    }

    /// Intercept both metric definition and scope creation, possibly changing the metric type.
    ///
    pub fn mod_both<MF, N>(&self, mod_fn: MF) -> Chain<N>
    where
        MF: Fn(DefineMetricFn<M>, OpenScopeFn<M>) -> (DefineMetricFn<N>, OpenScopeFn<N>),
        N: Clone + Send + Sync,
    {
        let (metric_fn, scope_fn) = mod_fn(self.define_metric_fn.clone(), self.scope_metric_fn.clone());
        Chain {
            define_metric_fn: metric_fn,
            scope_metric_fn: scope_fn
        }
    }

    /// Intercept scope creation.
    ///
    pub fn mod_scope<MF>(&self, mod_fn: MF) -> Self
    where
        MF: Fn(OpenScopeFn<M>) -> OpenScopeFn<M>,
    {
        Chain {
            define_metric_fn: self.define_metric_fn.clone(),
            scope_metric_fn: mod_fn(self.scope_metric_fn.clone())
        }
    }
}



