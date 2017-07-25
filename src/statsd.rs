use core::{MetricType, Rate, Value, MetricWriter, MetricKey, MetricSink, FULL_SAMPLING_RATE};
use std::net::UdpSocket;
use std::io::Result;
use std::cell::RefCell;
use std::sync::Arc;

#[derive(Debug)]
pub struct StatsdKey {
    prefix: String,
    suffix: String,
}

impl MetricKey for StatsdKey {}

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

thread_local! {
    static SEND_BUFFER: RefCell<String> = RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD));
}

#[derive(Debug)]
pub struct StatsdWriter {
    socket: Arc<UdpSocket>,
}

fn flush(payload: &mut String, socket: &UdpSocket) {
    debug!("statsd sending {} bytes", payload.len());
    // TODO check for and report any send() error
    socket.send(payload.as_bytes())/*.unwrap()*/;
    payload.clear();
}

impl MetricWriter<StatsdKey> for StatsdWriter {

    fn write(&self, metric: &StatsdKey, value: Value) {
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

    fn flush(&self) {
        SEND_BUFFER.with(|cell| {
            let ref mut buf = cell.borrow_mut();
            if !buf.is_empty() {
                // operation complete, flush any metrics in buffer
                flush(buf, &self.socket)
            }
        })
    }

}

impl Drop for StatsdWriter {
    fn drop(&mut self) {
        self.flush();
    }
}

/// Allows sending metrics to a statsd server
#[derive(Debug)]
pub struct StatsdSink {
    socket: Arc<UdpSocket>,
    prefix: String,
}

impl StatsdSink {
    /// Create a new statsd sink to the specified address with the specified prefix
    pub fn new<S: AsRef<str>>(address: &str, prefix_str: S) -> Result<StatsdSink> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?); // NB: CLOEXEC by default
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(StatsdSink { socket, prefix: prefix_str.as_ref().to_string()})
    }
}

impl MetricSink for StatsdSink {
    type Metric = StatsdKey;
    type Writer = StatsdWriter;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> StatsdKey {
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

        if sampling < FULL_SAMPLING_RATE {
            suffix.push('@');
            suffix.push_str(&sampling.to_string());
        }

        StatsdKey {prefix, suffix}
    }

    fn new_writer(&self) -> StatsdWriter {
        StatsdWriter { socket: self.socket.clone() }
    }
}

