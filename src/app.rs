//! This module regroups the application metrics front-end.
//!
//! It provides the differentiated, high-level instruments (timers, counters, gauges...) objects,
//! along with consistent metric naming / grouping facilities.
//!
//! It should also allows additional per-metric configuration parameters.

use std::sync::Arc;

use core::*;
use std::marker::PhantomData;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;


/// Wrap the metrics backend to provide an application-friendly interface.
pub fn metrics<'ph, M, W, S>(sink: S) -> AppMetrics<'ph, M, W, S>
    where W: Writer<M>, S: Sink<M, W>
{
    AppMetrics::new(sink)
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing potential problems.
pub struct Event<M, W> {
    metric: M,
    target_writer: Arc<W>,
}

impl<M, W> Event<M, W> where W: Writer<M> {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.target_writer.write(&self.metric, 1);
    }
}

/// A counter that sends values to the metrics backend
pub struct Gauge<M, W> {
    metric: M,
    target_writer: Arc<W>,
}

impl<M, W> Gauge<M, W> where W: Writer<M> {
    /// Record a value point for this gauge.
    pub fn value<V>(&self, value: V) where V: ToPrimitive {
        self.target_writer.write(&self.metric, value.to_u64().unwrap());
    }
}

/// A gauge that sends values to the metrics backend
pub struct Counter<M, W> where W: Writer<M> {
    metric: M,
    target_writer: Arc<W>,
}

impl<M, W> Counter<M, W> where W: Writer<M> {
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
pub struct Timer<M, W> {
    metric: M,
    target_writer: Arc<W>,
}

impl<M, W> Timer<M, W> where W: Writer<M> {
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

/// Variations of this should also provide control of the metric recording scope.
pub struct AppMetrics<'ph, M, W, S> where M: 'ph, W: Writer<M>, S: Sink<M, W>  {
    prefix: String,
    writer: Arc<W>,
    target: Arc<S>,
    phantom: PhantomData<&'ph M>,
}

impl <'ph, M, W, S> AppMetrics<'ph, M, W, S> where W: Writer<M>, S: Sink<M, W> {
    /// Create a new direct metric dispatch
    pub fn new(target: S) -> AppMetrics<'ph, M, W, S> {
        let target_writer: W = target.new_writer();
        AppMetrics {
            prefix: "".to_string(),
            writer: Arc::new(target_writer),
            target: Arc::new(target),
            phantom: PhantomData {},
        }
    }

    fn qualified_name<AS>(&self, name: AS) -> String
        where AS: Into<String> + AsRef<str>
    {
        // FIXME is there a way to return <S> in both cases?
        if self.prefix.is_empty() {
            return name.into()
        }
        let mut buf:String = self.prefix.clone();
        buf.push_str(name.as_ref());
        buf.to_string()
    }

    /// Get an event counter of the provided name.
    pub fn event<AS>(&self, name: AS) -> Event<M, W>
        where AS: Into<String> + AsRef<str>
    {
        let metric = self.target.new_metric(MetricKind::Event, self.qualified_name(name), 1.0);
        Event { metric, target_writer: self.writer.clone(), }
    }

    /// Get a counter of the provided name.
    pub fn counter<AS>(&self, name: AS) -> Counter<M, W>
        where AS: Into<String> + AsRef<str>
    {
        let metric = self.target.new_metric(MetricKind::Count, self.qualified_name(name), 1.0);
        Counter { metric, target_writer: self.writer.clone(), }
    }

    /// Get a timer of the provided name.
    pub fn timer<AS>(&self, name: AS) -> Timer<M, W>
        where AS: Into<String> + AsRef<str>
    {
        let metric = self.target.new_metric(MetricKind::Time, self.qualified_name(name), 1.0);
        Timer { metric, target_writer: self.writer.clone(), }
    }

    /// Get a gauge of the provided name.
    pub fn gauge<AS>(&self, name: AS) -> Gauge<M, W>
        where AS: Into<String> + AsRef<str>
    {
        let metric = self.target.new_metric(MetricKind::Gauge, self.qualified_name(name), 1.0);
        Gauge { metric, target_writer: self.writer.clone(), }
    }

    /// Prepend the metrics name with a prefix.
    /// Does not affect metrics that were already obtained.
    pub fn with_prefix<IS>(&self, prefix: IS) -> Self where IS: Into<String> {
        AppMetrics {
            prefix: prefix.into(),
            target: self.target.clone(),
            writer: self.writer.clone(),
            phantom: PhantomData {},
        }
    }
}


#[cfg(feature = "bench")]
mod microbench {

    use aggregate::Aggregator;
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
