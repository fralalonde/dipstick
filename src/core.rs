//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use clock::TimeHandle;
use scheduler::{set_schedule, CancelHandle};
use async;

use std::time::Duration;
use std::sync::Arc;
use std::ops;
use text;
use error;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
// TODO should this be f64? f32?
pub type Value = u64;

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

/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
#[derive(Debug, Clone, Copy)]
pub enum Sampling {
    /// Do not sample, use all data.
    Full,

    /// Floating point sampling rate
    /// - 1.0+ records everything
    /// - 0.5 records one of two values
    /// - 0.0 records nothing
    SampleRate(f64)
}

impl Default for Sampling {
    fn default() -> Self {
        Sampling::Full
    }
}

/// A metrics buffering strategy.
#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    /// No buffering is performed (default).
    Unbuffered,
    /// A buffer of maximum specified size is used.
    BufferSize(usize),
}

impl Default for Buffering {
    fn default() -> Self {
        Buffering::Unbuffered
    }
}

/// One struct to rule them all.
/// Possible attributes of metric outputs and inputs.
/// Private trait used by impls of specific With* traits.
/// Not all attributes are used by all structs!
/// This is a design choice to centralize code at the expense of slight waste of memory.
/// Fields have also not been made `pub` to make it easy to change this mechanism.
/// Field access must go through `is_` and `get_` methods declared in sub-traits.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    namespace: Name,
    sampling_rate: Sampling,
    buffering: Buffering,
}

/// The only trait that requires concrete impl by metric components.
/// Default impl of actual attributes use this to clone & mutate the original component.
/// This trait is _not_ exposed by the lib.
pub trait WithAttributes: Clone {
    /// Return attributes for evaluation.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn get_attributes(&self) -> &Attributes;

    /// Return attributes of component to be mutated after cloning.
    // TODO replace with fields-in-traits if ever stabilized (https://github.com/nikomatsakis/fields-in-traits-rfc)
    fn mut_attributes(&mut self) -> &mut Attributes;

    /// Clone this component and its attributes before returning it.
    /// This means one of the attributes will be cloned only to be replaced immediately.
    /// But the benefits of a generic solution means we can live with that for a while.
    fn with_attributes<F: Fn(&mut Attributes)>(&self, edit: F) -> Self {
        let mut cloned = self.clone();
        (edit)(cloned.mut_attributes());
        cloned
    }
}

/// Name operations support.
pub trait WithName {
    /// Return the namespace of the component.
    fn get_namespace(&self) -> &Name;

    /// Join namespace and prepend in newly defined metrics.
    fn add_name(&self, name: &str) -> Self;

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, metric_name: Name) -> Name;
}

impl<T: WithAttributes> WithName for T {
    fn get_namespace(&self) -> &Name {
        &self.get_attributes().namespace
    }

    /// Join namespace and prepend in newly defined metrics.
    fn add_name(&self, name: &str) -> Self {
        self.with_attributes(|new_attr| new_attr.namespace = new_attr.namespace.add_name(name))
    }

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, name: Name) -> Name {
        self.get_attributes().namespace.add_name(name)
    }
}

/// Apply statistical sampling to collected metrics data.
pub trait WithSamplingRate: WithAttributes {
    /// Perform random sampling of values according to the specified rate.
    fn with_sampling_rate(&self, sampling_rate: Sampling) -> Self {
        self.with_attributes(|new_attr| new_attr.sampling_rate = sampling_rate)
    }

    /// Get the sampling strategy for this component, if any.
    fn get_sampling(&self) -> Sampling {
        self.get_attributes().sampling_rate
    }
}

/// Determine input buffering strategy, if supported by output.
/// Changing this only affects inputs opened afterwards.
/// Buffering is done on best effort, meaning flush will occur if buffer capacity is exceeded.
pub trait WithBuffering: WithAttributes {
    /// Return a clone with the specified buffering set.
    fn with_buffering(&self, buffering: Buffering) -> Self {
        self.with_attributes(|new_attr| new_attr.buffering = buffering)
    }

    /// Is this component using buffering?
    fn is_buffering(&self) -> bool {
        match self.get_buffering() {
            Buffering::Unbuffered => true,
            _ => false
        }
    }

    /// Return the buffering.
    fn get_buffering(&self) -> Buffering {
        self.get_attributes().buffering
    }
}

/// A namespace for metrics.
/// Does _not_ include the metric's "short" name itself.
/// Can be empty.
#[derive(Debug, Clone, Hash, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Name {
    inner: Vec<String>,
}

impl Name {

    /// Concatenate with another namespace into a new one.
    pub fn add_name(&self, name: impl Into<Name>) -> Self {
        let mut cloned = self.clone();
        cloned.inner.extend_from_slice(&name.into().inner);
        cloned
    }

    /// Returns true if the specified namespace is a subset or is equal to this namespace.
    pub fn starts_with(&self, name: &Name) -> bool {
        (self.inner.len() >= name.inner.len()) && (name.inner[..] == self.inner[..name.inner.len()])
    }

    /// Combine name parts into a string.
    pub fn join(&self, separator: &str) -> String {

        let mut buf = String::with_capacity(64);
        let mut i = self.inner.iter();
        if let Some(n) = i.next() {
            buf.push_str(n.as_ref());
        } else {
            return "".into()
        }
        for n in i {
            buf.push_str(separator);
            buf.push_str(n.as_ref());
        }
        buf
    }
}

impl ops::Deref for Name {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for Name {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<S: Into<String>> From<S> for Name {
    fn from(name: S) -> Name {
        let name: String = name.into();
        if name.is_empty() {
            Name::default()
        } else {
            Name { inner: vec![name] }
        }
    }
}

/// A function trait that opens a new metric capture scope.
pub trait Output: OutputDyn + Send + Sync + 'static + Sized {
    /// Type of input scope provided by this output.
    type INPUT: Input + 'static + Clone;

    /// Open a new input scope from this output.
    fn new_input(&self) -> Self::INPUT;

    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn async(self, queue_length: usize) -> async::AsyncOutput {
        async::AsyncOutput::new(self, queue_length)
    }

}

/// Dynamic variant of the Output trait
pub trait OutputDyn {
    /// Open a new metric input with dynamic typing.
    fn new_input_dyn(&self) -> Arc<Input + Send + Sync + 'static>;
}

/// Blanket impl that provides Outputs their dynamic flavor.
impl<T: Output + Send + Sync + 'static> OutputDyn for T {
    fn new_input_dyn(&self) -> Arc<Input + Send + Sync + 'static> {
        Arc::new(self.new_input())
    }
}

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref NO_METRIC_OUTPUT: Arc<OutputDyn + Send + Sync> = Arc::new(text::to_void());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<Input + Send + Sync> = NO_METRIC_OUTPUT.new_input_dyn();
}

/// Define metrics, write values and flush them.
pub trait Input: Send + Sync + Flush {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: Name, kind: Kind) -> WriteFn;

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

/// Enable programmatic buffering of metrics output
pub trait Flush {
    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

/// Enable background periodical publication of metrics
pub trait ScheduleFlush {
    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle;
}

impl<T: Flush + Send + Sync + Clone + 'static> ScheduleFlush for T {
    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.clone();
        set_schedule(period, move || {
            if let Err(err) = scope.flush() {
                error!("Could not flush metrics: {}", err);
            }
        })
    }
}

/// A function that writes to a certain metric input.
#[derive(Clone)]
pub struct WriteFn {
    inner: Arc<Fn(Value) + Send + Sync>
}

impl WriteFn {
    /// Utility constructor
    pub fn new<F: Fn(Value) + Send + Sync + 'static>(wfn: F) -> WriteFn    {
        WriteFn { inner: Arc::new(wfn) }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: Value) {
        (self)(value)
    }
}

impl ops::Deref for WriteFn {
    type Target = (Fn(Value) + Send + Sync);

    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
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

impl From<WriteFn> for Gauge {
    fn from(wfn: WriteFn) -> Gauge {
        Gauge { write: wfn }
    }
}

impl From<WriteFn> for Timer {
    fn from(wfn: WriteFn) -> Timer {
        Timer { write: wfn }
    }
}

impl From<WriteFn> for Counter {
    fn from(wfn: WriteFn) -> Counter {
        Counter { write: wfn }
    }
}

impl From<WriteFn> for Marker {
    fn from(wfn: WriteFn) -> Marker {
        Marker { write: wfn }
    }
}

///// A function trait that writes to or flushes a certain scope.
////#[derive(Clone)]
//pub struct CommandFn<M> {
//    inner: Arc<Fn(Command<M>) + Send + Sync + 'static>
//}
//
//impl<M> Clone for CommandFn<M> {
//    fn clone(&self) -> CommandFn<M> {
//        CommandFn {
//            inner: self.inner.clone()
//        }
//    }
//}

///// An method dispatching command enum to manipulate metric scopes.
///// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
///// Using a command pattern allows buffering, async queuing and inline definition of writers.
//pub enum Command<'a, M: 'a> {
//    /// Write the value for the metric.
//    /// Takes a reference to minimize overhead in single-threaded scenarios.
//    Write(&'a M, Value),
//
//    /// Flush the scope buffer, if applicable.
//    Flush,
//}
//
///// Create a new metric scope based on the provided scope function.
//pub fn command_fn<M, F>(scope_fn: F) -> CommandFn<M>
//where
//    F: Fn(Command<M>) + Send + Sync + 'static,
//{
//    CommandFn {
//        inner: Arc::new(scope_fn)
//    }
//}
//
//impl<M> CommandFn<M> {
//    /// Write a value to this scope.
//    #[inline]
//    pub fn write(&self, metric: &M, value: Value) {
//        (self.inner)(Write(metric, value))
//    }
//
//    /// Flush this scope.
//    /// Has no effect if scope is unbuffered.
//    #[inline]
//    pub fn flush(&self) {
//        (self.inner)(Flush)
//    }
//}

#[cfg(feature = "bench")]
mod bench {

    use core::*;
    use clock::TimeHandle;
    use test;
    use bucket::Bucket;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut test::Bencher) {
        let metrics = Bucket::new();
        let marker = metrics.marker("aaa");
        b.iter(|| test::black_box(marker.mark()));
    }
}

