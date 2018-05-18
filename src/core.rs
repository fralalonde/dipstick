//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use self::Command::*;

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
pub type Sampling = f64;

/// Do not sample, use all data.
pub const FULL_SAMPLING_RATE: Sampling = 1.0;

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

/// A namespace for metrics.
/// Does _not_ include the metric's "short" name itself.
/// Can be empty.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct Namespace {
    inner: Vec<String>
}

lazy_static! {
    /// Root namespace contains no string parts.
    pub static ref ROOT_NS: Namespace = Namespace { inner: vec![] };
}

//impl<'a> Index<&'a str> for Namespace {
//    type Output = Self;
//
//    /// Returns a copy of this namespace with the "index" appended to it.
//    /// Returned reference should be dereferenceable:
//    ///
//    /// ```
//    /// let sub_ns = *ROOT_NS["sub_ns"];
//    /// ```
//    ///
//    fn index(&self, index: &'a str) -> &Self::Output {
//        let mut clone = self.inner.clone();
//        if !index.is_empty() {
//            clone.push(index.into());
//        }
//        &Namespace{ inner: clone }
//    }
//}

impl Namespace {

    /// Append name to the namespace, returning a modified copy.
    pub fn with_suffix(&self, name: &str) -> Self {
        let mut new = self.inner.clone();
        new.push(name.into());
        Namespace { inner: new }
    }

    /// Returns a copy of this namespace with the second namespace appended.
    /// Both original namespaces stay untouched.
    pub fn extend(&self, names: &Namespace) -> Self {
        Namespace {
            inner: {
                let mut new = self.inner.clone();
                new.extend_from_slice(&names.inner);
                new
            }
        }
    }

    /// Combine name parts into a string.
    pub fn join(&self, name: &str, separator: &str) -> String {
        if self.inner.is_empty() {
            return name.into()
        }
        let mut buf = String::with_capacity(64);
        for n in &self.inner {
            buf.push_str(n.as_ref());
            buf.push_str(separator);
        }
        buf.push_str(name);
        buf
    }
}

impl From<()> for Namespace {
    fn from(_name: ()) -> Namespace {
        ROOT_NS.clone()
    }
}

impl<'a> From<&'a str> for Namespace {
    fn from(name: &'a str) -> Namespace {
        ROOT_NS.with_suffix(name.as_ref())
    }
}

impl From<String> for Namespace {
    fn from(name: String) -> Namespace {
        ROOT_NS.with_suffix(name.as_ref())
    }
}

/// Dynamic metric definition function.
/// Metrics can be defined from any thread, concurrently (Fn is Sync).
/// The resulting metrics themselves can be also be safely shared across threads (<M> is Send + Sync).
/// Concurrent usage of a metric is done using threaded scopes.
/// Shared concurrent scopes may be provided by some backends (aggregate).
pub type DefineMetricFn<M> = Arc<Fn(&Namespace, Kind, &str, Sampling) -> M + Send + Sync>;

/// A function trait that opens a new metric capture scope.
pub type OpenScopeFn<M> = Arc<Fn() -> CommandFn<M> + Send + Sync>;

/// A function trait that writes to or flushes a certain scope.
pub type WriteFn = Arc<Fn(Value) + Send + Sync + 'static>;

/// A function trait that writes to or flushes a certain scope.
#[derive(Clone)]
pub struct CommandFn<M> {
    inner: Arc<Fn(Command<M>) + Send + Sync + 'static>
}

/// An method dispatching command enum to manipulate metric scopes.
/// Replaces a potential `Writer` trait that would have methods `write` and `flush`.
/// Using a command pattern allows buffering, async queuing and inline definition of writers.
pub enum Command<'a, M: 'a> {
    /// Write the value for the metric.
    /// Takes a reference to minimize overhead in single-threaded scenarios.
    Write(&'a M, Value),

    /// Flush the scope buffer, if applicable.
    Flush,
}

/// Create a new metric scope based on the provided scope function.
pub fn command_fn<M, F>(scope_fn: F) -> CommandFn<M>
where
    F: Fn(Command<M>) + Send + Sync + 'static,
{
    CommandFn {
        inner: Arc::new(scope_fn)
    }
}

impl<M> CommandFn<M> {
    /// Write a value to this scope.
    #[inline]
    pub fn write(&self, metric: &M, value: Value) {
        (self.inner)(Write(metric, value))
    }

    /// Flush this scope.
    /// Has no effect if scope is unbuffered.
    #[inline]
    pub fn flush(&self) {
        (self.inner)(Flush)
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    fn get_instant(b: &mut test::Bencher) {
        b.iter(|| test::black_box(TimeHandle::now()));
    }

}