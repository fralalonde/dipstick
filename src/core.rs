use time;

//////////////////
// DEFINITIONS

pub type Value = u64;

pub struct TimeType (u64);

impl TimeType {
    fn now() -> TimeType { TimeType(time::precise_time_ns()) }
    fn elapsed_ms(self) -> Value { (TimeType::now().0 - self.0) / 1_000_000 }
}

pub type RateType = f32;

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
    fn start() -> TimeType { TimeType::now() }

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

    fn scope<F>(&self, operations: F) where F: Fn(&Self::Scope);
}


// CHANNEL

pub trait DefinedMetric {}

pub trait MetricWrite<M: DefinedMetric> {
    fn write(&self, metric: &M, value: Value);
}

pub trait Channel {
    type Metric: DefinedMetric;
    type Write: MetricWrite<Self::Metric>;
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Self::Metric;
    fn write<F>(&self, operations: F) where F: Fn(&Self::Write);
}
