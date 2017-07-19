use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};
use std::net::UdpSocket;
use std::io::Result;
use std::cell::RefCell;

////////////

pub struct StatsdMetric {
    m_type: MetricType,
    name: String,
    sample: RateType
}

impl DefinedMetric for StatsdMetric {}

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

thread_local! {
    static SEND_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD));
}

pub struct StatsdWrite {
    socket: UdpSocket,
    prefix: String,
}

fn flush(payload: &mut String, socket: &UdpSocket) {
    // TODO check for and report any send() error
    debug!("statsd sending {} bytes", payload.len());
    socket.send(payload.as_bytes());
    payload.clear();
}

impl MetricWrite<StatsdMetric> for StatsdWrite {

    fn write(&self, metric: &StatsdMetric, value: Value) {
        // TODO add metric sample rate
        // TODO preformat per metric
        let entry = format!("{}{}:{}|{}", self.prefix, metric.name, value, match metric.m_type {
            MetricType::Event | MetricType::Count => "c",
            MetricType::Gauge => "g",
            MetricType::Time => "ms"
        });

        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if entry.len() > buf.capacity() {
                // TODO report entry too big to fit in buffer (!?)
                return;
            }

            let remaining = buf.capacity() - buf.len();
            if entry.len() + 1 > remaining {
                // buffer is full, flush before appending
                flush(buf, &self.socket);
            } else {
                if !buf.is_empty() {
                    buf.push('\n')
                }
                buf.push_str(&entry);
            }
        });
    }
}

pub struct StatsdChannel {
    write: StatsdWrite
}

impl StatsdChannel {
    /// Create a new statsd channel
    pub fn new<S: AsRef<str>>(address: &str, prefix_str: S) -> Result<StatsdChannel> {
        let socket = UdpSocket::bind("0.0.0.0:0")?; // NB: CLOEXEC by default
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(StatsdChannel { write: StatsdWrite { socket, prefix: prefix_str.as_ref().to_string() }})
    }
}

impl MetricChannel for StatsdChannel {
    type Metric = StatsdMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> StatsdMetric {
        StatsdMetric {m_type, name: name.as_ref().to_string(), sample}
    }

    type Write = StatsdWrite;

    fn write<F>(&self, metrics: F ) where F: Fn(&Self::Write) {
        metrics(&self.write);
        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if !buf.is_empty() {
                // operation complete, flush any metrics in buffer
                flush(buf, &self.write.socket)
            }
        });
    }
}

