//! Send metrics to a statsd server.

use ::core::*;
use ::error;

use std::net::UdpSocket;
use std::sync::{Arc,RwLock};

pub use std::net::ToSocketAddrs;

/// Send metrics to a statsd server at the address and port provided.
pub fn statsd<STR, ADDR>(address: ADDR, prefix: STR) -> error::Result<StatsdSink>
    where STR: Into<String>, ADDR: ToSocketAddrs
{
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?); // NB: CLOEXEC by default
    socket.set_nonblocking(true)?;
    socket.connect(address)?;
    info!("statsd connected");

    Ok(StatsdSink {
        socket,
        prefix: prefix.into(),
    })
}

/// Key of a statsd metric.
#[derive(Debug)]
pub struct StatsdMetric {
    prefix: String,
    suffix: String,
    scale: u64,
}

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

/// Wrapped buffer & socket as one so that any remainding data can be flushed on Drop.
struct ScopeBuffer {
    str: String,
    socket: Arc<UdpSocket>,
}

impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl  ScopeBuffer {

    fn flush(&mut self) {
        debug!("statsd sending {} bytes", self.str.len());
        // TODO check for and report any send() error
        match self.socket.send(self.str.as_bytes()) {
            Ok(size) => { /* TODO update selfstats */ },
            Err(e) => { /* TODO metric faults */ }
        };
        self.str.clear();
    }
}

/// Allows sending metrics to a statsd server
pub struct StatsdSink {
    socket: Arc<UdpSocket>,
    prefix: String,
}

impl Sink<StatsdMetric> for StatsdSink {

    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> StatsdMetric {
        let mut prefix = String::with_capacity(32);
        prefix.push_str(&self.prefix);
        prefix.push_str(name.as_ref());
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match kind {
            Kind::Event | Kind::Count => "c",
            Kind::Gauge => "g",
            Kind::Time => "ms",
        });

        if sampling < FULL_SAMPLING_RATE {
            suffix.push('@');
            suffix.push_str(&sampling.to_string());
        }

        let scale = match kind {
            Kind::Time => 1000,
            _ => 1
        };

        StatsdMetric { prefix, suffix, scale }
    }

    fn new_scope(&self) -> ScopeFn<StatsdMetric> {
        let buf = RwLock::new(ScopeBuffer { str: String::with_capacity(MAX_UDP_PAYLOAD), socket: self.socket.clone() });
        Arc::new(move |cmd| match cmd {
            Scope::Write(metric, value) => {
                if let Ok(mut buf) = buf.try_write() {
                    let scaled_value = if metric.scale != 1 {
                        value / metric.scale
                    } else {
                        value
                    };
                    let value_str = scaled_value.to_string();
                    let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

                    if entry_len > buf.str.capacity() {
                        // TODO report entry too big to fit in buffer (!?)
                        return;
                    }

                    let remaining = buf.str.capacity() - buf.str.len();
                    if entry_len + 1 > remaining {
                        // buffer is full, flush before appending
                        buf.flush();
                    } else {
                        if !buf.str.is_empty() {
                            // separate from previous entry
                            buf.str.push('\n')
                        }
                        buf.str.push_str(&metric.prefix);
                        buf.str.push_str(&value_str);
                        buf.str.push_str(&metric.suffix);
                    }
                }
            },
            Scope::Flush => {
                if let Ok(mut buf) = buf.try_write() {
                    if !buf.str.is_empty() {
                        // operation complete, flush any metrics in buffer
                        buf.flush();
                    }
                }
            }
        })
    }
}
