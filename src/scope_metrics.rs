//! Scope metrics allow an application to emit per-operation statistics,
//! like generating a per-request performance log.
//!
//! Although the scope metrics can be predefined like in [GlobalMetrics], the application needs to
//! create a scope that will be passed back when reporting scoped metric values.
/*!
Per-operation metrics can be recorded and published using `scope_metrics`:
```rust
let scope_metrics = scope_metrics(to_log());
let request_counter = scope_metrics.counter("scope_counter");
{
let request_scope = scope_metrics.open_scope();
request_counter.count(request_scope, 42);
request_counter.count(request_scope, 42);
}
```
*/
use core::*;
use core::ScopeCmd::*;
use std::sync::{Arc, RwLock};

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Wrap the metrics backend to provide an application-friendly interface.
/// When reporting a value, scoped metrics also need to be passed a [Scope].
pub fn scoped_metrics<M>(chain: Chain<M>) -> ScopedMetrics<M>
where
    M: 'static + Clone + Send + Sync,
{
    ScopedMetrics {
        prefix: "".to_string(),
        chain: Arc::new(chain),
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
    pub fn mark(&self, scope: &mut ControlScopeFn<M>) {
        (scope)(Write(&self.metric, 1));
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
    pub fn count<V>(&self, scope: &mut ControlScopeFn<M>, count: V)
    where
        V: ToPrimitive,
    {
        (scope)(Write(&self.metric, count.to_u64().unwrap()));
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
    pub fn value<V>(&self, scope: &mut ControlScopeFn<M>, value: V)
    where
        V: ToPrimitive,
    {
        (scope)(Write(&self.metric, value.to_u64().unwrap()));
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
pub struct ScopeTimer<M> {
    metric: M,
}

impl<M: Clone> ScopeTimer<M> {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us<V>(&self, scope: &mut ControlScopeFn<M>, interval_us: V) -> V
    where
        V: ToPrimitive,
    {
        (scope)(Write(&self.metric, interval_us.to_u64().unwrap()));
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
    pub fn stop(&self, scope: &mut ControlScopeFn<M>, start_time: TimeHandle) -> u64 {
        let elapsed_us = start_time.elapsed_us();
        self.interval_us(scope, elapsed_us)
    }

    /// Record the time taken to execute the provided closure
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

/// Variations of this should also provide control of the metric recording scope.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct ScopedMetrics<M> {
    prefix: String,
    chain: Arc<Chain<M>>,
}

impl<M> ScopedMetrics<M>
where
    M: 'static + Clone + Send + Sync,
{
    fn qualified_name<AS>(&self, name: AS) -> String
    where
        AS: Into<String> + AsRef<str>,
    {
        // FIXME is there a way to return <S> in both cases?
        if self.prefix.is_empty() {
            return name.into();
        }
        let mut buf: String = self.prefix.clone();
        buf.push_str(name.as_ref());
        buf.to_string()
    }

    /// Get an event counter of the provided name.
    pub fn marker<AS>(&self, name: AS) -> ScopeMarker<M>
    where
        AS: Into<String> + AsRef<str>,
    {
        let metric = self.chain.define_metric(
            Kind::Marker,
            &self.qualified_name(name),
            1.0,
        );
        ScopeMarker { metric }
    }

    /// Get a counter of the provided name.
    pub fn counter<AS>(&self, name: AS) -> ScopeCounter<M>
    where
        AS: Into<String> + AsRef<str>,
    {
        let metric = self.chain.define_metric(
            Kind::Counter,
            &self.qualified_name(name),
            1.0,
        );
        ScopeCounter { metric }
    }

    /// Get a timer of the provided name.
    pub fn timer<AS>(&self, name: AS) -> ScopeTimer<M>
    where
        AS: Into<String> + AsRef<str>,
    {
        let metric = self.chain.define_metric(
            Kind::Timer,
            &self.qualified_name(name),
            1.0,
        );
        ScopeTimer { metric }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<AS>(&self, name: AS) -> ScopeGauge<M>
    where
        AS: Into<String> + AsRef<str>,
    {
        let metric = self.chain.define_metric(
            Kind::Gauge,
            &self.qualified_name(name),
            1.0,
        );
        ScopeGauge { metric }
    }

    /// Prepend the metrics name with a prefix.
    /// Does not affect metrics that were already obtained.
    pub fn with_prefix<IS>(&self, prefix: IS) -> Self
    where
        IS: Into<String>,
    {
        ScopedMetrics {
            prefix: prefix.into(),
            chain: self.chain.clone(),
        }
    }

    /// Create a new scope to report metric values.
    pub fn open_scope(&self) -> ControlScopeFn<M> {
        let scope_buffer = RwLock::new(ScopeBuffer {
            buffer: Vec::new(),
            scope: self.chain.open_scope(false),
        });
        Arc::new(move |cmd: ScopeCmd<M>| {
            let mut buf = scope_buffer.write().expect("Lock metric scope.");
            match cmd {
                Write(metric, value) => {
                    buf.buffer.push(ScopeCommand {
                        metric: (*metric).clone(),
                        value,
                    })
                }
                Flush => buf.flush(),
            }
        })
    }
}

/// Save the metrics for delivery upon scope close.
struct ScopeCommand<M> {
    metric: M,
    value: Value,
}

struct ScopeBuffer<M: Clone> {
    buffer: Vec<ScopeCommand<M>>,
    scope: ControlScopeFn<M>,
}

impl<M: Clone> ScopeBuffer<M> {
    fn flush(&mut self) {
        for cmd in self.buffer.drain(..) {
            (self.scope)(Write(&cmd.metric, cmd.value))
        }
        (self.scope)(Flush)
    }
}

impl<M: Clone> Drop for ScopeBuffer<M> {
    fn drop(&mut self) {
        self.flush()
    }
}

#[cfg(feature = "bench")]
mod bench {

    use ::*;
    use test;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let sink = aggregate(5, summary, to_stdout());
        let metrics = global_metrics(sink);
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }

}
