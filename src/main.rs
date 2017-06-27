#![cfg_attr(feature = "bench", feature(test))]

#[cfg(feature="bench")]
extern crate test;
extern crate time;
extern crate lazy_static;

use test::test::Bencher;

mod pcg32;

use pcg32::pcg32_random;

//////////////////
// DEFINITIONS

type ValueType = f32;

type TimeType = u64;

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

lazy_static! {
    static ref SKIP_SCOPE: CloseScope = SkipScope {};
}

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
    fn start() -> TimeType {}
    fn time(&self, start_time: TimeType);
}

trait Scope {
    fn open_scope(&self) -> OpenedScope;
}

trait TagEvent {
    fn tag_event(&self, tags: Option<&[S]>);}

trait TagValue {
    fn tag_value(&self, value: ValueType, tags: Option<&[S]>);
}

trait OpenedScope {
    fn close_scope(self);
}


// CHANNEL

/// Base instruments
trait Meter {
    fn new_event<S: AsRef<str>>(&self, name: S) -> Event;
    fn new_count<S: AsRef<str>>(&self, name: S) -> Value;
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Value;
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Value;
    fn new_scope<S: AsRef<str>>(&self, name: S) -> Scope;
}

/// Per-instrument sampling
trait SampleMeter {
    fn new_sample_event<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Event;
    fn new_sample_count<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value;
    fn new_sample_timer<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value;
    fn new_sample_gauge<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value;
    fn new_sample_scope<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Scope;
}

/// Tag instruments
trait TagMeter {
    fn new_tag_event<S: AsRef<str>>(&self, name: S) -> TagEvent;
    fn new_tag_count<S: AsRef<str>>(&self, name: S) -> TagValue;
    fn new_tag_timer<S: AsRef<str>>(&self, name: S) -> TagValue;
    fn new_tag_gauge<S: AsRef<str>>(&self, name: S) -> TagValue;
}

// (SPI CONTRACT)

// OUTPUT

type ValueOut = Fn(ValueType) -> ();

type TagValueOut = Fn(ValueType, Option<&[AsRef<str>]>) -> ();

trait ChannelOutput {
    fn new_value<S: AsRef<str>>(&self, name: S, m_type: MetricType, sampling: RateType) -> ValueOut;
    fn new_tag_value<S: AsRef<str>>(&self, m_type: MetricType, name: S) -> TagValueOut;
    fn new_scope<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Scope;

    fn write<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType, value: ValueType, tags: Option<&[S]>) {
        self.new_value(m_type, name, sampling).call_mut(value)
    }

    fn tag_write<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: RateType, value: ValueType, tags: Option<&[S]>) {
        self.new_tag_value(m_type, name, sampling).call_mut(value, tags)
    }

    fn open_scope<S: AsRef<str>>(&self, scope_name: S, sampling: RateType) {
        self.new_scope(name, sampling).open_scope()
    }

    fn in_scope<S: AsRef<str>>(&self, scope_name: S, sampling: RateType, mut block: FnMut(ChannelOutput) -> ()) {
        let scope = self.new_scope(name, sampling).open_scope();
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

// SKIP SCOPE

struct SkipScope {}

impl OpenedScope for SkipScope {
    fn close_scope(self) {}
}

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
       self.out.call(value);
    }
}

impl Scope for Metric {
    fn open_scope(&self) {
       self.out.open_scope()
    }
}

impl Event for SampleMetric {
    fn mark(&self) {
        if pcg32_random() < sampling {
            self.out.call(1.0);
        }
    }
}

impl Value for SampleMetric {
    fn value(&self, value: ValueType) {
        if pcg32_random() < sampling {
            self.out.call(value);
        }
    }
}

impl Scope for SampleMetric {
    fn open_scope(&self) {
        if pcg32_random() < sampling {
            out.open_scope()
        } else {
            SKIP_SCOPE
        }
    }
}

impl TagEvent for TagMetric {
    fn tag_event(&self, tags: Option<&[S]>) {
        self.out.call(1.0, tags);
    }
}

impl TagValue for TagMetric {
    fn tag_value(&self, value: ValueType, tags: Option<&[S]>) {
        self.out.call(value, tags);
    }
}


/// A point in time from which elapsed time can be determined
pub struct StartTime (u64);

impl StartTime {
    /// The number of milliseconds elapsed between now and this StartTime
    fn elapsed_ms(self) -> u64 {
        (time::precise_time_ns() - self.0) / 1_000_000
    }
}


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
fn bench_trait(b: &mut Bencher) {
    b.iter(|| {});
}

