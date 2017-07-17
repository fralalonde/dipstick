#![cfg_attr(feature = "bench", feature(test))]

#![warn(
missing_copy_implementations,
missing_debug_implementations,
missing_docs,
trivial_casts,
trivial_numeric_casts,
unused_extern_crates,
unused_import_braces,
unused_qualifications,
variant_size_differences,
)]

#![feature(fn_traits)]

#[cfg(feature="bench")]
extern crate test;
extern crate time;
//#[macro_use]
//extern crate error_chain;
//#[macro_use]
//extern crate lazy_static;

//mod pcg32;
//mod statsd;

//use pcg32::pcg32_random;
use std::collections::HashMap;
use std::net::UdpSocket;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};

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

trait EventMetric {
    fn event(&self);
}

trait ValueMetric {
    fn value(&self, value: Value);
}

trait TimeMetric: ValueMetric {
    fn start() -> TimeType { TimeType::now() }

    fn stop(&self, start_time: TimeType) -> u64 {
        let elapsed_ms = start_time.elapsed_ms();
        self.value(elapsed_ms);
        elapsed_ms
    }
}

trait MetricScope {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self;
}

trait Sugar {
    type Event: EventMetric;
    type Value: ValueMetric;
    type Time:  TimeMetric;
    type Scope: MetricScope;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event;
    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value;
    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Time;
    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value;

    fn scope<F>(&self, operations: F) where F: Fn(&Self::Scope);
}


// CHANNEL

trait DefinedMetric {}

const NO_TAGS: Vec<String> = vec!();

trait MetricWrite<M: DefinedMetric> {
    fn write<S: AsRef<str>>(&self, metric: &M, value: Value, tags: Vec<S>);
}

trait Channel {
    type Metric: DefinedMetric;
    type Write: MetricWrite<Self::Metric>;
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Self::Metric;
    fn write<F>(&self, operations: F) where F: Fn(&Self::Write);
}

//////////// Log Channel

struct LogMetric {
    prefix: String
}

impl DefinedMetric for LogMetric {}

struct LogWrite {}

impl MetricWrite<LogMetric> for LogWrite {
    fn write<S: AsRef<str>>(&self, metric: &LogMetric, value: Value, tags: Vec<S>) {
        // TODO format faster
        println!("LOG TAGS {} | Value {}", metric.prefix, value)
    }
}

struct LogChannel {
    write: LogWrite
}

impl LogChannel {
    fn new() -> LogChannel {
        LogChannel { write: LogWrite {}}
    }
}

impl Channel for LogChannel {
    type Metric = LogMetric;
    
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> LogMetric {
        LogMetric { prefix: format!("Type {:?} | Name {} | Sample {}", m_type, name.as_ref(), sample)}
    }

    type Write = LogWrite;

    fn write<F>(&self, metrics: F ) where F: Fn(&Self::Write) {
        metrics(&self.write)
    }
    
}

////////////

struct StatsdMetric {
    m_type: MetricType,
    name: String,
    sample: RateType
}

impl DefinedMetric for StatsdMetric {}

struct StatsdWrite {}

impl MetricWrite<StatsdMetric> for StatsdWrite {

    fn write<S: AsRef<str>>(&self, metric: &StatsdMetric, value: Value, tags: Vec<S>) {
        // TODO send to UDP
        // TODO use tags
        println!("STATSD TAGS {}:{}|{:?}", metric.name, value, metric.m_type)
    }
}

struct StatsdChannel {
    socket: UdpSocket,
    prefix: String,
    write: StatsdWrite
}

impl StatsdChannel {
    fn new<S: AsRef<str>>(address: &str, prefix_str: S) -> std::io::Result<StatsdChannel> {
        let socket = UdpSocket::bind("0.0.0.0:0")?; // NB: CLOEXEC by default
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(StatsdChannel {socket, prefix: prefix_str.as_ref().to_string(), write: StatsdWrite {}})
    }
}

impl Channel for StatsdChannel {
    type Metric = StatsdMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> StatsdMetric {
        StatsdMetric {m_type, name: name.as_ref().to_string(), sample}
    }

    type Write = StatsdWrite;

    fn write<F>(&self, metrics: F )
        where F: Fn(&Self::Write) {
        metrics(&self.write)
    }
}

////////////

struct ProxyMetric<M: DefinedMetric> {
    target: M
}

impl <M: DefinedMetric> DefinedMetric for ProxyMetric<M> {}

struct ProxyWrite<C: Channel> {
    target: C,
}

impl <C: Channel> MetricWrite<ProxyMetric<<C as Channel>::Metric>> for ProxyWrite<C> {

    fn write<S: AsRef<str>>(&self, metric: &ProxyMetric<<C as Channel>::Metric>, value: Value, tags: Vec<S>) {
        println!("Proxy");
        self.target.write(|scope| scope.write(&metric.target, value, tags))
    }
}

struct ProxyChannel<C: Channel> {
    write: ProxyWrite<C>
}

impl <C: Channel> ProxyChannel<C> {
    fn new(target: C) -> ProxyChannel<C> {
        ProxyChannel { write: ProxyWrite { target }}
    }
}

impl <C: Channel> Channel for ProxyChannel<C> {
    type Metric = ProxyMetric<C::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> ProxyMetric<C::Metric> {
        let pm = self.write.target.define(m_type, name, sample);
        ProxyMetric { target: pm }
    }

    type Write = ProxyWrite<C>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

////////////

enum StatsType {
    HitCount,
    Sum,
    MeanValue,
    Max,
    Min,
    MeanRate
}

struct AggregateMetric {
    hit_count: AtomicUsize,
    value_sum: AtomicUsize,
    value_max: AtomicUsize,
    value_min: AtomicUsize,
}

impl AggregateMetric {
    fn new() -> AggregateMetric {
        AggregateMetric{ hit_count: 0, value_sum: 0, value_max: 0, value_min: 0}
    }
}

impl DefinedMetric for AggregateMetric {

}

struct AggregateWrite<C: Channel> {
    target: C,
}

impl <C: Channel> MetricWrite<AggregateMetric> for AggregateWrite<C> {
    fn write<S: AsRef<str>>(&self, metric: &AggregateMetric, value: Value, tags: Vec<S>) {
        println!("Aggregate");
        metric.hit_count.fetch_add(1, Ordering::Relaxed);
        metric.value_sum.fetch_add(value, Ordering::Relaxed);

//        self.target.write(|scope| scope.write(metric, value, tags))
    }
}

struct AggregateChannel<C: Channel> {
    write: AggregateWrite<C>,
    stats: Vec<AggregateMetric>
}

impl <C: Channel> AggregateChannel<C> {
    fn new(target: C) -> AggregateChannel<C> {
        AggregateChannel { write: AggregateWrite { target }, stats: Vec::new()}
    }
}

impl <C: Channel> Channel for AggregateChannel<C> {
    type Metric = AggregateMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> AggregateMetric {
        let pm = self.write.target.define(m_type, name, sample);
        let mut exp = match m_type {
            MetricType::Gauge => {vec!(
                self.write.target.define(m_type, format!("{}.avg", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample)
                                      )}
            MetricType::Count => {vec!(
                self.write.target.define(m_type, format!("{}.avg", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.sum", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
                                      )}
            MetricType::Time => {vec!(
                self.write.target.define(m_type, format!("{}.avg", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.sum", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
                                     )}
            MetricType::Event => {vec!(
                self.write.target.define(m_type, format!("{}.count", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
                                      )}
        };
        AggregateMetric::new()
    }

    type Write = AggregateWrite<C>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}


////////////

struct SugarEvent<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

struct SugarValue<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

struct SugarTime<C: Channel> {
    metric: <C as Channel>::Metric,
    target: Rc<C>,
}

struct SugarScope {
}

impl <C: Channel> EventMetric for SugarEvent<C>  {
    fn event(&self) {
        self.target.write(|scope| scope.write(&self.metric, 1, NO_TAGS))
    }
}

impl <C: Channel> ValueMetric for SugarValue<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value, NO_TAGS))
    }
}

impl <C: Channel> ValueMetric for SugarTime<C> {
    fn value(&self, value: Value) {
        self.target.write(|scope| scope.write(&self.metric, value, NO_TAGS))
    }
}

impl <C: Channel> TimeMetric for SugarTime<C> {}

impl MetricScope for SugarScope {
    fn set_property<S: AsRef<str>>(&self, key: S, value: S) -> &Self {
        self
    }
}

struct SugarChannel<C: Channel> {
    target: Rc<C>
}

impl <C: Channel> SugarChannel<C> {
    fn new(target: C) -> SugarChannel<C> {
        SugarChannel { target: Rc::new(target) }
    }
}

impl <C: Channel> Sugar for SugarChannel<C> {
    type Event = SugarEvent<C>;
    type Value = SugarValue<C>;
    type Time = SugarTime<C>;
    type Scope = SugarScope;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event {
        let metric = self.target.define(MetricType::Event, name, 1.0);
        SugarEvent { metric, target: self.target.clone() }
    }

    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Count, name, 1.0);
        SugarValue { metric, target: self.target.clone() }
    }

    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Time {
        let metric = self.target.define(MetricType::Time, name, 1.0);
        SugarTime { metric, target: self.target.clone() }
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Value {
        let metric = self.target.define(MetricType::Gauge, name, 1.0);
        SugarValue { metric, target: self.target.clone() }
    }

    fn scope<F>(&self, operations: F) where F: Fn(&Self::Scope) {}
}

////////////

struct DualMetric<M1: DefinedMetric, M2: DefinedMetric> {
    metric_1: M1,
    metric_2: M2,
}

impl <M1: DefinedMetric, M2: DefinedMetric> DefinedMetric for DualMetric<M1, M2> {}

struct DualWrite<C1: Channel, C2: Channel> {
    channel_a: C1,
    channel_b: C2,
}

impl <C1: Channel, C2: Channel> MetricWrite<DualMetric<<C1 as Channel>::Metric, <C2 as Channel>::Metric>> for DualWrite<C1, C2> {
    fn write<S: AsRef<str>>(&self, metric: &DualMetric<<C1 as Channel>::Metric, <C2 as Channel>::Metric>, value: Value, tags: Vec<S>) {
        println!("Channel A");
        self.channel_a.write(|scope| scope.write(&metric.metric_1, value, tags));
        println!("Channel B");
        self.channel_b.write(|scope| scope.write(&metric.metric_2, value, tags));
    }
}

struct DualChannel<C1: Channel, C2: Channel> {
    write: DualWrite<C1, C2>
}

impl <C1: Channel, C2: Channel> DualChannel<C1, C2> {
    fn new(channel_a: C1, channel_b: C2) -> DualChannel<C1, C2> {
        DualChannel { write: DualWrite {channel_a, channel_b}}
    }
}

impl <C1: Channel, C2: Channel> Channel for DualChannel<C1, C2> {
    type Metric = DualMetric<C1::Metric, C2::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> DualMetric<C1::Metric, C2::Metric> {
        let metric_1 = self.write.channel_a.define(m_type, &name, sample);
        let metric_2 = self.write.channel_b.define(m_type, &name, sample);
        DualMetric { metric_1, metric_2  }
    }

    type Write = DualWrite<C1, C2>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}


////////////

fn main() {
    let channel_a = ProxyChannel::new( LogChannel::new() );
    let statsd_only_metric = channel_a.define(MetricType::Event, "statsd_event_a", 1.0);

    let channel_b = ProxyChannel::new( StatsdChannel::new("localhost:8125", "hello.").unwrap() );
    let channel_x = DualChannel::new( channel_a, channel_b );

    let metric = channel_x.define(MetricType::Count, "count_a", 1.0);
    channel_x.write(|scope| scope.write(&metric, 1, NO_TAGS));

    channel_x.write(|scope| {
        scope.write(&metric, 1, NO_TAGS);
//        scope.write(&statsd_only_metric, 1, Some(&["TAG"])) <- this fails AT COMPILE TIME. FUCK YEAH!
    });

    let sugar_x = SugarChannel::new(channel_x);
    let counter = sugar_x.new_count("sugar_count_a");
    counter.value(1);

}

//thread_local!(static PROXY_SCOPE: RefCell<Metric> = metric());
