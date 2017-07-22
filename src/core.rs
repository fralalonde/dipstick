use time;

//////////////////
// DEFINITIONS

pub type Value = u64;

#[derive(Debug)]
pub struct TimeType (u64);

impl TimeType {
    pub fn now() -> TimeType { TimeType(time::precise_time_ns()) }
    pub fn elapsed_ms(self) -> Value { (TimeType::now().0 - self.0) / 1_000_000 }
}

pub type RateType = f64;

#[derive(Debug, Copy, Clone)]
pub enum MetricType {
    Event,
    Count,
    Gauge,
    Time,
}

//////////////////
// CONTRACT

// INSTRUMENTATION (API CONTRACT)

pub trait EventMetric {
    fn event(&self);
}

pub trait ValueMetric {
    fn value(&self, value: Value);
}

pub trait TimerMetric: ValueMetric {
    fn start(&self) -> TimeType { TimeType::now() }

    fn stop(&self, start_time: TimeType) -> u64 {
        let elapsed_ms = start_time.elapsed_ms();
        self.value(elapsed_ms);
        elapsed_ms
    }
}

pub trait MetricScope {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self;
}

pub trait MetricDispatch {
    type Event: EventMetric;
    type Value: ValueMetric;
    type Timer: TimerMetric;
    type Scope: MetricScope;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event;
    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value;
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer;
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value;

    fn scope<F>(&mut self, operations: F) where F: Fn(/*&Self::Scope*/);
}

pub trait MetricSource {
    fn publish(&self);
}

/// A convenience macro to wrap a block or an expression with a start / stop timer.
/// Elapsed time is sent to the supplied statsd client after the computation has been performed.
/// Expression result (if any) is transparently returned.
#[macro_export]
macro_rules! time {
    ($timer: expr, $body: block) => {{
        let start_time = $timer.start();
        $body
        $timer.stop(start_time);
    }};
}

// SINK

pub trait SinkMetric {}

pub trait SinkWriter<M: SinkMetric>: Send {
    fn write(&self, metric: &M, value: Value);
    fn flush(&self) {}
}

pub trait MetricSink {
    type Metric: SinkMetric;
    type Writer: SinkWriter<Self::Metric>;
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Self::Metric;
    fn new_writer(&self) -> Self::Writer;
}
