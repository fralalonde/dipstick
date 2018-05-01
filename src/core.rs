//! Dipstick metrics core types and traits.
//! This is mostly centered around the backend.
//! Application-facing types are in the `app` module.

use self::Command::*;

use std::sync::Arc;

use chrono::{Local, DateTime};

use time;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

/// Base type for recorded metric values.
// TODO should this be f64? f32?
pub type Value = u64;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(i64);

/// takes 250ns but works every time
pub fn accurate_clock_micros() -> i64 {
    let local: DateTime<Local> = Local::now();
    let mut micros = local.timestamp() * 1_000_000;
    micros += local.timestamp_subsec_micros() as i64;
    micros
}

/// takes 25ns but fails to advance time on occasion
pub fn fast_clock_micros() -> i64 {
    (time::precise_time_ns() / 1000) as i64
}

// another quick way
//fn now_micros() -> i64 {
//    let t = time::get_time();
//    (t.sec * 1_000_000) + (t.nsec as i64 / 1000)
//}

impl TimeHandle {

    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(fast_clock_micros())
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_us(self) -> Value {
        (TimeHandle::now().0 - self.0) as Value
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_ms(self) -> Value {
        self.elapsed_us() / 1_000
    }
}

//impl From<usize> for TimeHandle {
//    fn from(s: usize) -> TimeHandle {
//        TimeHandle(s as i64)
//    }
//}
//
//impl From<TimeHandle> for usize {
//    fn from(s: TimeHandle) -> usize {
//        s.0 as usize
//    }
//}

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
    fn get_slow_time(b: &mut test::Bencher) {
        b.iter(|| test::black_box(accurate_clock_micros()));
    }

    #[bench]
    fn get_imprecise_time(b: &mut test::Bencher) {
        b.iter(|| test::black_box(fast_clock_micros()));
    }

}