#![cfg_attr(feature = "bench", feature(test))]

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
    fn value(value: Value) {}
}

struct TimeMetric ();

impl TimeMetric {
    fn start() -> TimeType { TimeType::now() }
}

// CHANNEL

trait DefinedMetric {}

trait MetricWrite<M: DefinedMetric> {
    fn write<S: AsRef<str>>(&self, metric: &M, value: Value, tags: Option<&[S]>);
}

trait Channel {
    type Metric: DefinedMetric;
    type Write: MetricWrite<Self::Metric>;
    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> Self::Metric;
    fn write<F>(&self, metrics: F) where F: Fn(&Self::Write);
}

//////////// Log Channel

struct LogMetric {
    prefix: String
}

impl DefinedMetric for LogMetric {}

struct LogWrite {}

impl MetricWrite<LogMetric> for LogWrite {
    fn write<S: AsRef<str>>(&self, metric: &LogMetric, value: Value, tags: Option<&[S]>) {
        // TODO format faster
        println!("{} | Value {}", metric.prefix, value)
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
    fn write<S: AsRef<str>>(&self, metric: &StatsdMetric, value: Value, tags: Option<&[S]>) {
        // TODO send to UDP
        println!("Statsd {:?} {} {}", metric.m_type, metric.name, value)
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
    proxy_channel: C,
}

impl <C: Channel> MetricWrite<ProxyMetric<<C as Channel>::Metric>> for ProxyWrite<C> {
    fn write<S: AsRef<str>>(&self, metric: &ProxyMetric<<C as Channel>::Metric>, value: Value, tags: Option<&[S]>) {
        println!("Proxy");
        self.proxy_channel.write(|scope| scope.write(&metric.target, value, tags))
    }
}

struct ProxyChannel<C: Channel> {
    write: ProxyWrite<C>
}

impl <C: Channel> ProxyChannel<C> {
    fn new(proxy_channel: C) -> ProxyChannel<C> {
        ProxyChannel { write: ProxyWrite { proxy_channel }}
    }
}

impl <C: Channel> Channel for ProxyChannel<C> {
    type Metric = ProxyMetric<C::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> ProxyMetric<C::Metric> {
        let pm = self.write.proxy_channel.define(m_type, name, sample);
        ProxyMetric { target: pm }
    }

    type Write = ProxyWrite<C>;

    fn write<F>(&self, metrics: F )
        where F: Fn(&Self::Write) {
        metrics(&self.write)
    }
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
    fn write<S: AsRef<str>>(&self, metric: &DualMetric<<C1 as Channel>::Metric, <C2 as Channel>::Metric>, value: Value, tags: Option<&[S]>) {
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

    fn write<F>(&self, metrics: F )
        where F: Fn(&Self::Write) {
        metrics(&self.write)
    }
}


////////////

fn main() {
    let channel_a = ProxyChannel::new( LogChannel::new() );
    let channel_b = ProxyChannel::new( StatsdChannel::new("localhost:8125", "hello.").unwrap() );
    let channel_x = DualChannel::new( channel_a, channel_b );
    let metric = channel_x.define(MetricType::Count, "count_a", 1.0);
    channel_x.write(|scope| scope.write(&metric, 1, Some(&["TAG"])));
}

//thread_local!(static PROXY_SCOPE: RefCell<Metric> = metric());
