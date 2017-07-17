#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature="bench")]
extern crate test;
extern crate time;
#[macro_use]
extern crate lazy_static;

mod pcg32;
//mod statsd;

use pcg32::pcg32_random;
use std::collections::HashMap;

//////////////////
// DEFINITIONS

type ValueType = f32;

struct TimeType (u64);

impl TimeType {
    fn now() { time::precise_time_ns() }
    fn elapsed_ms(self) { (TimeType::now() - self.0) / 1_000_000 }
}

type RateType = f32;

//type ChannelTimebase = [u32; 2]; // u32 base (alternating) + u32 offset = u64 time

enum MetricType {
    Event,
    Count,
    Gauge,
    Time,
}

///////////////////
// GLOBALS

//lazy_static! {
//    static ref SKIP_SCOPE: OpenScope = OpenScope {
//        properties: HashMap::new(),
//        close: |_| {}
//    };
//}

//////////////////
// CONTRACT

// INSTRUMENTATION (API CONTRACT)

trait Event {
    fn mark(&self);
}

trait Value {
    fn value(&self, value: ValueType);
}

trait Time {
    fn start() -> TimeType { TimeType::now() }
    fn stop(&self, start_time: TimeType);
    fn time(&self, block: Fn() -> ()) {
        let start = Time::start();
        block.call();
        self.stop(start)
    }
}

trait Scope {
    fn open_scope(&self) -> OpenScope;
}

trait TagEvent {
    fn tag_event<S: AsRef<str>>(&self, tags: Option<&[S]>);}

trait TagValue {
    fn tag_value<S: AsRef<str>>(&self, value: ValueType, tags: Option<&[S]>);
}

trait TagTime {
    fn stop(&self, start_time: TimeType, tags: Option<&[S]>);
    fn time<S: AsRef<str>>(&self, block: Fn() -> (), tags: Option<&[S]>) {
        let start = Time::start();
        block.call();
        self.stop(start, tags)
    }
}

struct ValueMetric {}


struct OpenScope {
    properties: HashMap<String, String>,
    close: CloseScope,
}

impl OpenScope {
    fn push<S: AsRef<str>>(&mut self, key: S, value: S) {
        self.properties.insert(key, value)
    }

    fn close_scope(&mut self) {
        self.close.call(self.properties)
    }
}


// CHANNEL

trait Metrics {
    fn new_event<S: AsRef<str>>(&self, name: S) -> Event { || self.write(MetricType.EVENT, name, 1.0, 1, None) }
    fn new_count<S: AsRef<str>>(&self, name: S) -> Value { |count| self.write(MetricType.COUNT, name, 1.0, count, None) }
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Value { |start_time| self.write(MetricType.TIME, name, 1.0, start_time.elapsed_ms(), None) }
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Value { |value| self.write(MetricType.GAUGE, name, 1.0, value, None) }

    /// Per-instrument sampling
    fn new_sample_event<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Event { || self.write(MetricType.EVENT, name, sampling, 1, None) }
    fn new_sample_count<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |count| self.write(MetricType.COUNT, name, sampling, count, None) }
    fn new_sample_timer<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |start_time| self.write(MetricType.TIME, name, sampling, start_time.elapsed_ms(), None) }
    fn new_sample_gauge<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |value| self.write(MetricType.GAUGE, name, sampling, value, None) }

    /// Tag instruments
    fn new_tag_event<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Event { |tags| self.write(MetricType.EVENT, name, 1.0, 1, Some(tags)) }
    fn new_tag_count<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |count, tags| self.write(MetricType.COUNT, name, 1.0, count, Some(tags)) }
    fn new_tag_timer<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |start_time, tags| self.write(MetricType.TIME, name, 1.0, start_time.elapsed_ms(), Some(tags)) }
    fn new_tag_gauge<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value { |value, tags| self.write(MetricType.GAUGE, name, 1.0, value, Some(tags)) }

    fn write(&self, m_type: MetricType, name: S, sampling: RateType, value: ValueType, tags: Option<&[S]>);
}

struct SyncMetrics {
[]
}

// (SPI CONTRACT)

// OUTPUT

type ValueOut = Fn(ValueType) -> ();

type TagValueOut = Fn(ValueType, Option<&[AsRef<str>]>) -> ();

type CloseScope = Fn(HashMap<String, String>) -> ();

trait ChannelOutput {
fn new_value<S: AsRef<str>>(&self, name: S, m_type: MetricType, sampling: RateType) -> ValueOut;
fn new_tag_value<S: AsRef<str>>(&self, m_type: MetricType, name: S) -> TagValueOut;
fn new_scope<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Scope;

fn adhoc_write(&self, m_type: MetricType, name: S, sampling: RateType, value: ValueType, tags: Option<&[S]>) {
self.new_value(m_type, name, sampling).call_mut(value)
}

fn adhoc_tag_write(&self, m_type: MetricType, name: S, sampling: RateType, value: ValueType, tags: Option<&[S]>) {
self.new_tag_value(m_type, name, sampling).call_mut(value, tags)
}

fn adhoc_open_scope<S: AsRef<str>>(&self, scope_name: S, sampling: RateType) {
self.new_scope(scope_name, sampling).open_scope()
}

fn in_scope<S: AsRef<str>>(&self, scope_name: S, sampling: RateType, mut block: FnMut(OpenScope) -> ()) {
let scope = self.adhoc_open_scope(scope_name, sampling);
block.call_mut(self);
scope.close_scope()
}
}


/// A convenience macro to wrap a block or an expression with a start / stop timer.
/// Elapsed time is sent to the supplied statsd client after the computation has been performed.
/// Expression result (if any) is transparently returned.
#[macro_export]
macro_rules! time {
($client: expr, $key: expr, $body: block) => (
let start_time = $client.start_time();
$body
$client.stop_time($key, start_time);
);
}

//////////////////
// IMPLEMENTATION

struct Metric {
out: ValueOut,
}

struct SampleMetric {
sampling: RateType,
out: ValueOut,
}

struct TagMetric {
out: TagValueOut,
}

impl Event for Metric {
fn mark(&self) {
self.out.call(1.0);
}
}

impl Value for Metric {
fn value(&self, value: ValueType) {
self.out.call(value)
}
}

impl Time for Metric {
fn stop(&self, start_time: TimeType) { start_time.elapsed_ms() }
}


impl Event for SampleMetric {
fn mark(&self) {
if pcg32_random() < self.sampling {
self.out.call(1.0);
}
}
}

impl Value for SampleMetric {
fn value(&self, value: ValueType) {
if pcg32_random() < self.sampling {
self.out.call(value);
}
}
}

impl Scope for SampleMetric {
fn open_scope(&self) {
if pcg32_random() < self.sampling {
self.out.open_scope()
} else {
OpenScope {
properties: HashMap::new(),
close: |_| {}
}
}
}
}

impl TagEvent for TagMetric {
fn tag_event(&self, tags: Option<&[AsRef<str>]>) {
self.out.call(1.0, tags);
}
}

impl TagValue for TagMetric {
fn tag_value(&self, value: ValueType, tags: Option<&[AsRef<str>]>) {
self.out.call(value, tags);
}
}


///// A point in time from which elapsed time can be determined
//pub struct StartTime (u64);
//
//impl StartTime {
//    /// The number of milliseconds elapsed between now and this StartTime
//    fn elapsed_ms(self) -> u64 {
//        (time::precise_time_ns() - self.0) / 1_000_000
//    }
//}
//

/// eager aggregation
/// expand every new_* to many new_*
struct AggregatingBuffer {

}


/// lazy aggregation
/// expand every new_* to many new_*
struct BufferAggregator {

}

struct Joined {

}

// flush when scope closed
// unscoped passthru
struct ScopeBuffer {

}

// flush every n metrics
struct CountBuffer {

}

// flush every n millis
struct TimeBuffer {

}

// flush every n metrics
struct Buffer {

}

// separate thread
struct Async {

}

struct RandomSampler {

}

struct TimeSampler {

}


#[test]
mod test {

}

#[bench]
mod bench {
use test::test::Bencher;
fn bench_trait(b: &mut Bencher) {
b.iter(|| {});
}


}


