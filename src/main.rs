#![cfg_attr(feature = "bench", feature(test))]


#[cfg(feature="bench")]
extern crate test;
extern crate time;
//#[macro_use]
//extern crate lazy_static;

//mod pcg32;
//mod statsd;

//use pcg32::pcg32_random;
use std::collections::HashMap;

//////////////////
// DEFINITIONS

type Value = u64;

struct TimeType (u64);

impl TimeType {
    fn now() -> TimeType { TimeType(time::precise_time_ns()) }
    fn elapsed_ms(self) -> Value { (TimeType::now().0 - self.0) / 1_000_000 }
}

type RateType = f32;

#[derive(Debug, Copy, Clone)]
enum MetricType {
    Event,
    Count,
    Gauge,
    Time,
}

//////////////////
// CONTRACT

// INSTRUMENTATION (API CONTRACT)

trait Event {
    fn event(&self);
}

struct ValueMetric ();

impl ValueMetric {
    fn value(value: Value) -> () {}
}

struct TimeMetric ();

impl TimeMetric {
    fn start() -> TimeType { TimeType::now() }
}

// CHANNEL

trait MetricId {}

trait Channel {
    type Metric: MetricId;
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Self::Metric;
    fn write<S: AsRef<str>>(&self, metric: &Self::Metric, value: Value, tags: Option<&[S]>);
//    fn scope<S: AsRef<str>>(&self, properties: Option<&HashMap<String, String>>, Fn(Channel) -> bool);
}

////////////

struct LogMetric {
    prefix: String
}

impl MetricId for LogMetric {}

struct LogChannel {}

impl Channel for LogChannel {
    type Metric = LogMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> LogMetric {
        LogMetric { prefix: format!("Type {:?} | Name {} | Sample {}", m_type, name.as_ref(), sample)}
    }

    fn write<S: AsRef<str>>(&self, metric: &LogMetric, value: Value, tags: Option<&[S]>) {
        // TODO format faster
        println!("{} | Value {}", metric.prefix, value)
    }
}

////////////

struct StatsdMetric {
    m_type: MetricType,
    name: String,
    sample: RateType
}

impl MetricId for StatsdMetric {}

struct StatsdChannel {}

impl Channel for StatsdChannel {
    type Metric = StatsdMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> StatsdMetric {
        StatsdMetric {m_type, name: name.as_ref().to_string(), sample}
    }

    fn write<S: AsRef<str>>(&self, metric: &StatsdMetric, value: Value, tags: Option<&[S]>) {
        println!("BBM {:?} {} {}", metric.m_type, metric.name, value)
    }
}

////////////

//thread_local!(static PROXY_SCOPE: RefCell<Dust> = dust());

struct ProxyMetric<M: MetricId> {
    target: M
}

impl <M: MetricId> MetricId for ProxyMetric<M> {}

struct ProxyChannel<C: Channel> {
    proxy_channel: C,
}

impl <C: Channel> Channel for ProxyChannel<C> {
    type Metric = ProxyMetric<C::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> ProxyMetric<C::Metric> {
        let pm = self.proxy_channel.define(m_type, name, sample);
        ProxyMetric { target: pm }
    }

    fn write<S: AsRef<str>>(&self, metric: &ProxyMetric<C::Metric>, value: Value, tags: Option<&[S]>) {
        println!("Proxy");
        self.proxy_channel.write(&metric.target, value, tags)
    }
}

////////////

struct DualMetric<M1: MetricId, M2: MetricId> {
    metric_1: M1,
    metric_2: M2,
}

impl <M1: MetricId, M2: MetricId> MetricId for DualMetric<M1, M2> {}

struct DualChannel<C1: Channel, C2: Channel> {
    channel_a: C1,
    channel_b: C2,
}

impl <C1: Channel, C2: Channel> Channel for DualChannel<C1, C2> {
    type Metric = DualMetric<C1::Metric, C2::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> DualMetric<C1::Metric, C2::Metric> {
        let metric_1 = self.channel_a.define(m_type, &name, sample);
        let metric_2 = self.channel_b.define(m_type, &name, sample);
        DualMetric { metric_1, metric_2  }
    }

    fn write<S: AsRef<str>>(&self, metric: &DualMetric<C1::Metric, C2::Metric>, value: Value, tags: Option<&[S]>) {
        println!("Channel A");
        self.channel_a.write(&metric.metric_1, value, tags);
        println!("Channel B");
        self.channel_b.write(&metric.metric_2, value, tags);
    }
}


////////////

fn main() {
    let channel_a = ProxyChannel {proxy_channel: LogChannel {}};
    let channel_b = ProxyChannel {proxy_channel: StatsdChannel {}};
    let channel_x = DualChannel { channel_a, channel_b };
    let z = channel_x.define(MetricType::Count, "count_a", 1.0);
    channel_x.write(&z, 1, Some(&["TAG"]));
}

