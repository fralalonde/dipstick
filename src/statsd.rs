use core::{MetricType, RateType, Value, MetricWriter, SinkMetric, MetricSink};
use std::net::UdpSocket;
use std::io::Result;
use std::cell::RefCell;

pub struct StatsdMetric {
    prefix: String,
    suffix: String,
}

impl SinkMetric for StatsdMetric {}

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

thread_local! {
    static SEND_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD));
}

pub struct StatsdWriter {
    socket: UdpSocket,
}

fn flush(payload: &mut String, socket: &UdpSocket) {
    // TODO check for and report any send() error
    debug!("statsd sending {} bytes", payload.len());
    socket.send(payload.as_bytes());
    payload.clear();
}

impl MetricWriter<StatsdMetric> for StatsdWriter {

    fn write(&self, metric: &StatsdMetric, value: Value) {
        let value_str = value.to_string();
        let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if entry_len > buf.capacity() {
                // TODO report entry too big to fit in buffer (!?)
                return;
            }

            let remaining = buf.capacity() - buf.len();
            if entry_len + 1 > remaining {
                // buffer is full, flush before appending
                flush(buf, &self.socket);
            } else {
                if !buf.is_empty() {
                    // separate from previous entry
                    buf.push('\n')
                }
                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push_str(&metric.suffix);
            }
        });
    }
}

/// Allows sending metrics to a statsd server
pub struct StatsdSink {
    write: StatsdWriter,
    prefix: String,
}

impl StatsdSink {
    /// Create a new statsd sink to the specified address with the specified prefix
    pub fn new<S: AsRef<str>>(address: &str, prefix_str: S) -> Result<StatsdSink> {
        let socket = UdpSocket::bind("0.0.0.0:0")?; // NB: CLOEXEC by default
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(StatsdSink { write: StatsdWriter { socket }, prefix: prefix_str.as_ref().to_string()})
    }
}

impl MetricSink for StatsdSink {
    type Metric = StatsdMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> StatsdMetric {
        let mut prefix = String::with_capacity(32);
        prefix.push_str(&self.prefix);
        prefix.push_str(name.as_ref());
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match m_type {
            MetricType::Event | MetricType::Count => "c",
            MetricType::Gauge => "g",
            MetricType::Time => "ms"
        });

        if (sample < 1.0) {
            suffix.push('@');
            suffix.push_str(&sample.to_string());
        }

        StatsdMetric {prefix, suffix}
    }

    type Write = StatsdWriter;

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

