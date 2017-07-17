use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, Channel};
use std::net::UdpSocket;
use std::io::Result;

////////////

pub struct StatsdMetric {
    m_type: MetricType,
    name: String,
    sample: RateType
}

impl DefinedMetric for StatsdMetric {}

pub struct StatsdWrite {}

impl MetricWrite<StatsdMetric> for StatsdWrite {

    fn write(&self, metric: &StatsdMetric, value: Value) {
        // TODO send to UDP
        // TODO use tags
        println!("STATSD TAGS {}:{}|{:?}", metric.name, value, metric.m_type)
    }
}

pub struct StatsdChannel {
    socket: UdpSocket,
    prefix: String,
    write: StatsdWrite
}

impl StatsdChannel {
    pub fn new<S: AsRef<str>>(address: &str, prefix_str: S) -> Result<StatsdChannel> {
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

