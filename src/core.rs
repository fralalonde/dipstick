//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use self::Kind::*;
use self::ScopeCmd::*;

use std::sync::Arc;
use std::time::Instant;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
// TODO should this be f64? f32?
pub type Value = u64;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(Instant);

impl TimeHandle {

    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(Instant::now())
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_us(self) -> Value {
        let duration = Instant::now() - self.0;
        duration.as_secs() * 1000000 + (duration.subsec_nanos() / 1000) as Value
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_ms(self) -> Value {
        let duration = Instant::now() - self.0;
        duration.as_secs() * 1000 + (duration.subsec_nanos() / 1000000) as Value
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
pub type OpenScopeFn<M> = Arc<Fn(bool) -> ControlScopeFn<M> + Send + Sync>;

/// Returns a callback function to send commands to the metric scope.
/// Writes can be performed by passing Some((&Metric, Value))
/// Flushes can be performed by passing None
/// Used to write values to the scope or flush the scope buffer (if applicable).
/// Simple applications may use only one scope.
/// Complex applications may define a new scope fo each operation or request.
/// Scopes can be moved acrossed threads (Send) but are not required to be thread-safe (Sync).
/// Some implementations _may_ be 'Sync', otherwise queue()ing or threadlocal() can be used.
#[derive(Clone)]
pub struct ControlScopeFn<M> {
    flush_on_drop: bool,
    scope_fn: Arc<Fn(ScopeCmd<M>)>,
}

unsafe impl<M> Sync for ControlScopeFn<M> {}
unsafe impl<M> Send for ControlScopeFn<M> {}

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
pub enum ScopeCmd<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

impl<M> ControlScopeFn<M> {
    /// Create a new metric scope based on the provided scope function.
    ///
    /// ```rust
    /// use dipstick::ControlScopeFn;
    /// let ref mut scope: ControlScopeFn<String> = ControlScopeFn::new(|_cmd| { /* match cmd {} */  });
    /// ```
    ///
    pub fn new<F>(scope_fn: F) -> Self
        where F: Fn(ScopeCmd<M>) + 'static
    {
        ControlScopeFn {
            flush_on_drop: true,
            scope_fn: Arc::new(scope_fn)
        }
    }

    /// Write a value to this scope.
    ///
    /// ```rust
    /// let ref mut scope = dipstick::to_log().open_scope(false);
    /// scope.write(&"counter".to_string(), 6);
    /// ```
    ///
    #[inline]
    pub fn write(&self, metric: &M, value: Value) {
        (self.scope_fn)(Write(metric, value))
    }

    /// Flush this scope, if buffered.
    ///
    /// ```rust
    /// let ref mut scope = dipstick::to_log().open_scope(true);
    /// scope.flush();
    /// ```
    ///
    #[inline]
    pub fn flush(&self) {
        (self.scope_fn)(Flush)
    }

    /// If scope is buffered, controls whether to flush the scope one last time when it is dropped.
    /// The default is true.
    ///
    /// ```rust
    /// let ref mut scope = dipstick::to_log().open_scope(true).flush_on_drop(false);
    /// ```
    ///
    pub fn flush_on_drop(mut self, enable: bool) -> Self {
        self.flush_on_drop = enable;
        self
    }
}

impl<M> Drop for ControlScopeFn<M> {
    fn drop(&mut self) {
        if self.flush_on_drop {
            self.flush()
        }
    }
}

/// A pair of functions composing a twin "chain of command".
/// This is the building block for the metrics backend.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct Chain<M> {
    #[derivative(Debug = "ignore")] define_metric_fn: DefineMetricFn<M>,

    #[derivative(Debug = "ignore")] scope_metric_fn: OpenScopeFn<M>,
}

impl<M> Chain<M> {
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

impl<M: Send + Sync + Clone + 'static> Chain<M> {
    /// Create a new metric chain with the provided metric definition and scope creation functions.
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

    /// Get an event counter of the provided name.
    pub fn marker<AS: AsRef<str>>(&self, name: AS) -> ScopeMarker<M> {
        let metric = self.define_metric(Marker, name.as_ref(), 1.0);
        ScopeMarker { metric }
    }

    /// Get a counter of the provided name.
    pub fn counter<AS: AsRef<str>>(&self, name: AS) -> ScopeCounter<M> {
        let metric = self.define_metric(Counter, name.as_ref(), 1.0);
        ScopeCounter { metric }
    }

    /// Get a timer of the provided name.
    pub fn timer<AS: AsRef<str>>(&self, name: AS) -> ScopeTimer<M> {
        let metric = self.define_metric(Timer, name.as_ref(), 1.0);
        ScopeTimer { metric }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<AS: AsRef<str>>(&self, name: AS) -> ScopeGauge<M> {
        let metric = self.define_metric(Gauge, name.as_ref(), 1.0);
        ScopeGauge { metric }
    }

    /// Intercept metric definition without changing the metric type.
    pub fn mod_metric<MF>(&self, mod_fn: MF) -> Chain<M>
        where
            MF: Fn(DefineMetricFn<M>) -> DefineMetricFn<M>,
    {
        Chain {
            define_metric_fn: mod_fn(self.define_metric_fn.clone()),
            scope_metric_fn: self.scope_metric_fn.clone(),
        }
    }

    /// Intercept both metric definition and scope creation, possibly changing the metric type.
    pub fn mod_both<MF, N>(&self, mod_fn: MF) -> Chain<N>
        where
            MF: Fn(DefineMetricFn<M>, OpenScopeFn<M>) -> (DefineMetricFn<N>, OpenScopeFn<N>),
            N: Clone + Send + Sync,
    {
        let (metric_fn, scope_fn) =
            mod_fn(self.define_metric_fn.clone(), self.scope_metric_fn.clone());
        Chain {
            define_metric_fn: metric_fn,
            scope_metric_fn: scope_fn,
        }
    }

    /// Intercept scope creation.
    pub fn mod_scope<MF>(&self, mod_fn: MF) -> Self
        where
            MF: Fn(OpenScopeFn<M>) -> OpenScopeFn<M>,
    {
        Chain {
            define_metric_fn: self.define_metric_fn.clone(),
            scope_metric_fn: mod_fn(self.scope_metric_fn.clone()),
        }
    }
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScopeMarker<M> {
    metric: M,
}

impl<M> ScopeMarker<M> {
    /// Record a single event occurence.
    #[inline]
    pub fn mark(&self, scope: &mut ControlScopeFn<M>) {
        scope.write(&self.metric, 1);
    }
}

/// A counter that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScopeCounter<M> {
    metric: M,
}

impl<M> ScopeCounter<M> {
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
pub struct ScopeGauge<M> {
    metric: M,
}

impl<M: Clone> ScopeGauge<M> {
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
pub struct ScopeTimer<M> {
    metric: M,
}

impl<M: Clone> ScopeTimer<M> {
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

//#[cfg(test)]
//mod test {
//    use core::*;
//    use test;
//    use std::f64;
//
//    const ITER: i64 = 5_000;
//    const LOOP: i64 = 50000;
//
//    // a retarded, dirty and generally incorrect tentative at jitter measurement
//    fn jitter(clock: fn() -> i64) {
//        let mut first = 0;
//        let mut last = 0;
//        let mut min = 999_000_000;
//        let mut max = -8888888;
//        let mut delta_sum = 0;
//        let mut dev2_sum = 0;
//
//        for i in 1..ITER {
//            let ts = clock();
//            test::black_box(for _j in 0..LOOP {});
//            last = clock();
//            let delta = last - ts;
//
//            delta_sum += delta;
//            let mean = delta_sum / i;
//
//            let dev2 = (delta - mean) ^ 2;
//            dev2_sum += dev2;
//
//            if delta > max {
//                max = delta
//            }
//            if delta < min {
//                min = delta
//            }
//        }
//
//        println!("runt {}", last - first);
//        println!("mean {}", delta_sum / ITER);
//        println!("dev2 {}", (dev2_sum as f64).sqrt() / ITER as f64);
//        println!("min {}", min);
//        println!("max {}", max);
//    }
//
//
//    #[test]
//    fn jitter_instant() {
//        jitter(|| super::slow_clock_micros())
//    }
//
//    #[test]
//    fn jitter_local_now() {
//        jitter(|| super::slow_clock_micros())
//    }
//
//    #[test]
//    fn jitter_precise_time_ns() {
//        jitter(|| super::imprecise_clock_micros())
//    }
//
//}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

}