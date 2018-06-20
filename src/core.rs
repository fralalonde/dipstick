//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use clock::TimeHandle;
use queue;
use raw_queue;
use cache;

use std::sync::{Arc, Mutex};
use std::ops;
use std::rc::Rc;
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
/// All strategies other than `Unbuffered` are applied as a best-effort, meaning that the buffer
/// may be flushed at any moment before reaching the limit, for any or no reason in particular.
#[derive(Debug, Clone, Copy)]
pub enum Buffering {
    /// No buffering is performed (default).
    Unbuffered,

    /// A buffer of maximum specified size is used.
    BufferSize(usize),

    /// Buffer as much as possible.
    Unlimited,
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
        // FIXME (perf) store name in reverse to prepend with an actual push() to the vec
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
            Buffering::Unbuffered => false,
            _ => true
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

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref NO_METRIC_OUTPUT: Arc<OutputDyn + Send + Sync> = Arc::new(text::to_void());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<Input + Send + Sync> = NO_METRIC_OUTPUT.new_input_dyn();
}

/// A function trait that opens a new metric capture scope.
pub trait Output: OutputDyn + Send + Sync + 'static + Sized {
    /// Type of input scope provided by this output.
    type INPUT: Input + Send + Sync + 'static + Clone;

    /// Open a new input scope from this output.
    fn new_input(&self) -> Self::INPUT;

}

/// Wrap this output behind an asynchronous metrics dispatch queue.
/// This is not strictly required for multi threading since the provided inputs
/// are already Send + Sync but might be desired to lower the latency
pub trait Async: OutputDyn + Send + Sync + 'static + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn async(self, queue_length: usize) -> queue::QueueOutput {
        queue::QueueOutput::new(self, queue_length)
    }
}

/// Wrap an output with a metric definition cache.
/// This is useless if all metrics are statically declared but can provide performance
/// benefits if some metrics are dynamically defined at runtime.
pub trait Cache: OutputDyn + Send + Sync + 'static + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn cache(self, max_size: usize) -> cache::CacheOutput {
        cache::CacheOutput::new(self, max_size)
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

/// Define metrics, write values and flush them.
pub trait Input: Send + Sync {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: Name, kind: Kind) -> Metric;

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }

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

/// A metric is actually a function that knows to write a metric value to a metric output.
#[derive(Clone)]
pub struct Metric {
    inner: Arc<Fn(Value) + Send + Sync>
}

impl Metric {
    /// Utility constructor
    pub fn new<F: Fn(Value) + Send + Sync + 'static>(wfn: F) -> Metric {
        Metric { inner: Arc::new(wfn) }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: Value) {
        (self.inner)(value)
    }
}

/// A function trait that opens a new metric capture scope.
pub trait RawOutput: RawOutputDyn + Send + Sync + 'static + Sized {
    /// Type of input scope provided by this output.
    type INPUT: RawInput + 'static  ;

    /// Open a new input scope from this output.
    fn new_raw_input(&self) -> Self::INPUT;
}

/// Wrap this raw output behind an asynchronous metrics dispatch queue.
pub trait RawAsync: RawOutput + Sized {
    /// Wrap this output with an asynchronous dispatch queue of specified length.
    fn async(self, queue_length: usize) -> raw_queue::QueueRawOutput {
        raw_queue::QueueRawOutput::new(self, queue_length)
    }
}

/// Dynamic variant of the RawOutput trait
pub trait RawOutputDyn {
    /// Open a new metric input with dynamic typing.
    fn new_raw_input_dyn(&self) -> RawInputBox;
}

/// Blanket impl that provides RawOutputs their dynamic flavor.
impl<T: RawOutput + Send + Sync + 'static> RawOutputDyn for T {
    fn new_raw_input_dyn(&self) -> RawInputBox {
        RawInputBox (Rc::new(self.new_raw_input()))
    }
}

/// Provide thread-safe locking to RawInput implementers.
#[derive(Clone)]
pub struct LockingInputBox {
    attributes: Attributes,
    inner: Arc<Mutex<RawInputBox>>
}

impl WithAttributes for LockingInputBox {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Input for LockingInputBox {

    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let name = self.qualified_name(name);
        let raw_metric = self.inner.lock().expect("RawInput Lock").new_metric(name, kind);
        let mutex = self.inner.clone();
        Metric::new(move |value| {
            let _guard = mutex.lock().expect("RawMetric Lock");
            raw_metric.write(value)
        } )
    }

    fn flush(&self) -> error::Result<()> {
        self.inner.lock().expect("RawInput Lock").flush()
    }
}

/// Blanket impl that provides RawOutputs their dynamic flavor.
impl<T: RawOutput + Send + Sync + 'static> Output for T {
    type INPUT = LockingInputBox;

    fn new_input(&self) -> Self::INPUT {
        LockingInputBox {
            attributes: Attributes::default(),
            inner: Arc::new(Mutex::new(RawInputBox (Rc::new(self.new_raw_input()))))
        }
    }
}


/// Define metrics, write values and flush them.
pub trait RawInput {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: Name, kind: Kind) -> RawMetric;

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

/// Wrap a RawInput to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread, which RawAsync does.
#[derive(Clone)]
pub struct RawInputBox ( Rc<RawInput + 'static> );

unsafe impl Send for RawInputBox {}
unsafe impl Sync for RawInputBox {}

impl ops::Deref for RawInputBox {
    type Target = RawInput + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}

/// A raw metric is just like a Metric, but bound to a single thread (not Send nor Sync)

pub struct RawMetric {
    inner: Box<Fn(Value)>
}

impl RawMetric {
    /// Utility constructor
    pub fn new<F: Fn(Value) + 'static>(wfn: F) -> RawMetric {
        RawMetric { inner: Box::new(wfn) }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: Value) {
        (self.inner)(value)
    }
}

unsafe impl Send for RawMetric {}
unsafe impl Sync for RawMetric {}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Marker {
    #[derivative(Debug = "ignore")]
    inner: Metric,
}

impl Marker {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.inner.write(1)
    }
}

/// A counter that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Counter {
    #[derivative(Debug = "ignore")]
    inner: Metric,
}

impl Counter {
    /// Record a value count.
    pub fn count<V: ToPrimitive>(&self, count: V) {
        self.inner.write(count.to_u64().unwrap())
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Gauge {
    #[derivative(Debug = "ignore")]
    inner: Metric,
}

impl Gauge {
    /// Record a value point for this gauge.
    pub fn value<V: ToPrimitive>(&self, value: V) {
        self.inner.write(value.to_u64().unwrap())
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
    inner: Metric,
}

impl Timer {
    /// Record a microsecond interval for this timer
    /// Can be used in place of start()/stop() if an external time interval source is used
    pub fn interval_us<V: ToPrimitive>(&self, interval_us: V) -> V {
        self.inner.write(interval_us.to_u64().unwrap());
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

impl From<Metric> for Gauge {
    fn from(wfn: Metric) -> Gauge {
        Gauge { inner: wfn }
    }
}

impl From<Metric> for Timer {
    fn from(wfn: Metric) -> Timer {
        Timer { inner: wfn }
    }
}

impl From<Metric> for Counter {
    fn from(wfn: Metric) -> Counter {
        Counter { inner: wfn }
    }
}

impl From<Metric> for Marker {
    fn from(wfn: Metric) -> Marker {
        Marker { inner: wfn }
    }
}


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

