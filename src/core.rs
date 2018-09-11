//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use clock::TimeHandle;
use error;

use std::sync::{Arc, Mutex};
use std::ops;
use std::rc::Rc;
use std::fmt;
use std::collections::HashMap;

// TODO maybe define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
pub type Value = u64;

/////// ATTRIBUTES

/// The actual distribution (random, fixed-cycled, etc) depends on selected sampling method.
#[derive(Debug, Clone, Copy)]
pub enum Sampling {
    /// Do not sample, use all data.
    Full,

    /// Floating point sampling rate
    /// - 1.0+ records everything
    /// - 0.5 records one of two values
    /// - 0.0 records nothing
    Random(f64)
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
/// Possible attributes of metric outputs and scopes.
/// Private trait used by impls of specific With* traits.
/// Not all attributes are used by all structs!
/// This is a design choice to centralize code at the expense of slight waste of memory.
/// Fields have also not been made `pub` to make it easy to change this mechanism.
/// Field access must go through `is_` and `get_` methods declared in sub-traits.
#[derive(Debug, Clone, Default)]
pub struct Attributes {
    namespace: Name,
    sampling: Sampling,
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
pub trait AddPrefix {
    /// Return the namespace of the component.
    fn get_namespace(&self) -> &Name;

    /// Join namespace and prepend in newly defined metrics.
    fn add_prefix(&self, name: &str) -> Self;

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, metric_name: Name) -> Name;
}

/// Name operations support.
pub trait AddTag {
    /// Return the namespace of the component.
    fn get_tags(&self) -> &Arc<HashMap<String, String>>;

    /// Join namespace and prepend in newly defined metrics.
    fn add_tag(&self, name: &str) -> Self;

}

impl<T: WithAttributes> AddPrefix for T {
    fn get_namespace(&self) -> &Name {
        &self.get_attributes().namespace
    }

    /// Join namespace and prepend in newly defined metrics.
    fn add_prefix(&self, name: &str) -> Self {
        self.with_attributes(|new_attr| new_attr.namespace = new_attr.namespace.concat(name))
    }

    /// Append the specified name to the local namespace and return the concatenated result.
    fn qualified_name(&self, name: Name) -> Name {
        // FIXME (perf) store name in reverse to prepend with an actual push() to the vec
        self.get_attributes().namespace.concat(name)
    }
}

/// Apply statistical sampling to collected metrics data.
pub trait Sampled: WithAttributes {
    /// Perform random sampling of values according to the specified rate.
    fn sampled(&self, sampling: Sampling) -> Self {
        self.with_attributes(|new_attr| new_attr.sampling = sampling)
    }

    /// Get the sampling strategy for this component, if any.
    fn get_sampling(&self) -> Sampling {
        self.get_attributes().sampling
    }
}

/// Determine scope buffering strategy, if supported by output.
/// Changing this only affects scopes opened afterwards.
/// Buffering is done on best effort, meaning flush will occur if buffer capacity is exceeded.
pub trait Buffered: WithAttributes {
    /// Return a clone with the specified buffering set.
    fn buffered(&self, buffering: Buffering) -> Self {
        self.with_attributes(|new_attr| new_attr.buffering = buffering)
    }

    /// Is this component using buffering?
    fn is_buffered(&self) -> bool {
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
    pub fn concat(&self, name: impl Into<Name>) -> Self {
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

////// INPUT

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref NO_METRIC_OUTPUT: Arc<InputDyn + Send + Sync> = Arc::new(Void::metrics());

    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<InputScope + Send + Sync> = NO_METRIC_OUTPUT.input_dyn();
}

/// A function trait that opens a new metric capture scope.
pub trait Input: Send + Sync + 'static + InputDyn {
    /// The type of Scope returned byt this input.
    type SCOPE: InputScope + Send + Sync + 'static;

    /// Open a new scope from this output.
    fn input(&self) -> Self::SCOPE;
}

/// A function trait that opens a new metric capture scope.
pub trait InputDyn: Send + Sync + 'static {
    /// Open a new scope from this output.
    fn input_dyn(&self) -> Arc<InputScope + Send + Sync + 'static>;
}

/// Blanket impl of dyn input trait
impl<T: Input + Send + Sync + 'static> InputDyn for T {
    fn input_dyn(&self) -> Arc<InputScope + Send + Sync + 'static> {
        Arc::new(self.input())
    }
}

/// InputScope
/// Define metrics, write values and flush them.
pub trait InputScope: Flush {
    /// Define a generic metric of the specified type.
    /// It is preferable to use counter() / marker() / timer() / gauge() methods.
    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric;

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
pub struct InputMetric {
    inner: Arc<Fn(Value) + Send + Sync>
}

impl fmt::Debug for InputMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "InputMetric")
    }
}

impl InputMetric {
    /// Utility constructor
    pub fn new<F: Fn(Value) + Send + Sync + 'static>(metric: F) -> InputMetric {
        InputMetric { inner: Arc::new(metric) }
    }

    /// Collect a new value for this metric.
    #[inline]
    pub fn write(&self, value: Value) {
        (self.inner)(value)
    }
}


////// OUTPUT

/// Define metrics, write values and flush them.
pub trait OutputScope: Flush {

    /// Define a raw metric of the specified type.
    fn new_metric(&self, name: Name, kind: Kind) -> OutputMetric;

}

impl OutputMetric {
    /// Utility constructor
    pub fn new<F: Fn(Value) + 'static>(metric: F) -> OutputMetric {
        OutputMetric { inner: Rc::new(metric) }
    }

    /// Some may prefer the `metric.write(value)` form to the `(metric)(value)` form.
    /// This shouldn't matter as metrics should be of type Counter, Marker, etc.
    #[inline]
    pub fn write(&self, value: Value) {
        (self.inner)(value)
    }
}


/// A function trait that opens a new metric capture scope.
pub trait Output: Send + Sync + 'static + OutputDyn {
    /// The type of Scope returned byt this output.
    type SCOPE: OutputScope;

    /// Open a new scope from this output.
    fn output(&self) -> Self::SCOPE;
}

/// A function trait that opens a new metric capture scope.
pub trait OutputDyn {
    /// Open a new scope from this output.
    fn output_dyn(&self) -> Rc<OutputScope + 'static>;
}

/// Blanket impl of dyn output trait
impl<T: Output + Send + Sync + 'static> OutputDyn for T {
    fn output_dyn(&self) -> Rc<OutputScope + 'static> {
        Rc::new(self.output())
    }
}

//////// FLUSH

/// Both InputScope and OutputScope share the ability to flush the recorded data.
pub trait Flush {

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }

}

///////// LOCKING INPUT -> OUTPUT ADAPTER


/// Provide thread-safe locking to RawScope implementers.
#[derive(Clone)]
pub struct LockingScopeBox {
    attributes: Attributes,
    inner: Arc<Mutex<UnsafeScope>>
}

impl WithAttributes for LockingScopeBox {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl InputScope for LockingScopeBox {

    fn new_metric(&self, name: Name, kind: Kind) -> InputMetric {
        let name = self.qualified_name(name);
        let raw_metric = self.inner.lock().expect("RawScope Lock").new_metric(name, kind);
        let mutex = self.inner.clone();
        InputMetric::new(move |value| {
            let _guard = mutex.lock().expect("OutputMetric Lock");
            raw_metric.write(value)
        } )
    }

}

impl Flush for LockingScopeBox {
    fn flush(&self) -> error::Result<()> {
        self.inner.lock().expect("OutputScope Lock").flush()
    }
}

/// Blanket impl that provides RawOutputs their dynamic flavor.
impl<T: Output + Send + Sync + 'static> Input for T {
    type SCOPE = LockingScopeBox;

    fn input(&self) -> Self::SCOPE {
        LockingScopeBox {
            attributes: Attributes::default(),
            inner: Arc::new(Mutex::new(UnsafeScope(self.output_dyn())))
        }
    }
}

///////// UNSAFE INPUT -> OUTPUT ADAPTER

/// Wrap a RawScope to make it Send + Sync, allowing it to travel the world of threads.
/// Obviously, it should only still be used from a single thread or dragons may occur.
#[derive(Clone)]
pub struct UnsafeScope(Rc<OutputScope + 'static> );

unsafe impl Send for UnsafeScope {}
unsafe impl Sync for UnsafeScope {}

impl UnsafeScope {
    /// Wrap a dynamic RawScope to make it Send + Sync.
    pub fn new(scope: Rc<OutputScope + 'static>) -> Self {
        UnsafeScope(scope)
    }
}

impl ops::Deref for UnsafeScope {
    type Target = OutputScope + 'static;
    fn deref(&self) -> &Self::Target {
        Rc::as_ref(&self.0)
    }
}

/// Output metrics are not thread safe.
#[derive(Clone)]
pub struct OutputMetric {
    inner: Rc<Fn(Value)>
}

impl fmt::Debug for OutputMetric {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Box<Fn(Value)>")
    }
}

unsafe impl Send for OutputMetric {}
unsafe impl Sync for OutputMetric {}


////////////// INSTRUMENTS

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

/// Used by the metrics! macro to obtain the Kind from the stringified type.
impl<'a> From<&'a str> for Kind {
    fn from(s: &str) -> Kind {
        match s {
            "Marker" => Kind::Marker,
            "Counter" => Kind::Counter,
            "Gauge" => Kind::Gauge,
            "Timer" => Kind::Timer,
            _ => panic!("No Kind '{}' defined", s)
        }
    }
}

/// A monotonic counter metric.
/// Since value is only ever increased by one, no value parameter is provided,
/// preventing programming errors.
#[derive(Debug, Clone)]
pub struct Marker {
    inner: InputMetric,
}

impl Marker {
    /// Record a single event occurence.
    pub fn mark(&self) {
        self.inner.write(1)
    }
}

/// A counter that sends values to the metrics backend
#[derive(Debug, Clone)]
pub struct Counter {
    inner: InputMetric,
}

impl Counter {
    /// Record a value count.
    pub fn count<V: ToPrimitive>(&self, count: V) {
        self.inner.write(count.to_u64().unwrap())
    }
}

/// A gauge that sends values to the metrics backend
#[derive(Debug, Clone)]
pub struct Gauge {
    inner: InputMetric,
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
#[derive(Debug, Clone)]
pub struct Timer {
    inner: InputMetric,
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
    pub fn time<F: FnOnce() -> R, R>(&self, operations: F) -> R {
        let start_time = self.start();
        let value: R = operations();
        self.stop(start_time);
        value
    }
}

impl From<InputMetric> for Gauge {
    fn from(metric: InputMetric) -> Gauge {
        Gauge { inner: metric }
    }
}

impl From<InputMetric> for Timer {
    fn from(metric: InputMetric) -> Timer {
        Timer { inner: metric }
    }
}

impl From<InputMetric> for Counter {
    fn from(metric: InputMetric) -> Counter {
        Counter { inner: metric }
    }
}

impl From<InputMetric> for Marker {
    fn from(metric: InputMetric) -> Marker {
        Marker { inner: metric }
    }
}


/// VOID INPUT & OUTPUT

/// Discard metrics output.
#[derive(Clone)]
pub struct Void {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidInput {}

/// Discard metrics output.
#[derive(Clone)]
pub struct VoidOutput {}

impl Void {
    /// Void metrics builder.
    pub fn metrics() -> Self {
        Void {}
    }
}

impl Output for Void {
    type SCOPE = VoidOutput;
    fn output(&self) -> VoidOutput {
        VoidOutput {}
    }
}

impl OutputScope for VoidOutput {
    fn new_metric(&self, _name: Name, _kind: Kind) -> OutputMetric {
        OutputMetric::new(|_value| {})
    }
}

impl Flush for VoidOutput {
}

/// Discard all metric values sent to it.
pub fn output_none() -> Void {
    Void {}
}

#[cfg(test)]
mod test {
    use core::*;

    #[test]
    fn test_to_void() {
        let c = Void::metrics().input();
        let m = c.new_metric("test".into(), Kind::Marker);
        m.write(33);
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
