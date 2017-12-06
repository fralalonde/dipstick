//! Send metrics to a statsd server.

use core::*;
use error;
use selfmetrics::*;

use std::net::UdpSocket;
use std::sync::{Arc, RwLock};
pub use std::net::ToSocketAddrs;

/// Send metrics to a statsd server at the address and port provided.
pub fn to_statsd<ADDR>(address: ADDR, prefix: &str) -> error::Result<StatsdSink>
where
    ADDR: ToSocketAddrs,
{
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?); // NB: CLOEXEC by default
    socket.set_nonblocking(true)?;
    socket.connect(address)?;

    Ok(StatsdSink {
        socket,
        prefix: String::from(prefix),
    })
}

lazy_static! {
    static ref STATSD_METRICS: AppMetrics<Aggregate, AggregateSink> =
                                            SELF_METRICS.with_prefix("statsd.");

    static ref SEND_ERR: Marker<Aggregate> = STATSD_METRICS.marker("send_failed");
    static ref SENT_BYTES: Counter<Aggregate> = STATSD_METRICS.counter("sent_bytes");
}

/// Key of a statsd metric.
#[derive(Debug, Clone)]
pub struct StatsdMetric {
    prefix: String,
    suffix: String,
    scale: u64,
}

/// Use a safe maximum size for UDP to prevent fragmentation.
// TODO make configurable?
const MAX_UDP_PAYLOAD: usize = 576;

/// Wrapped string buffer & socket as one.
#[derive(Debug)]
struct ScopeBuffer {
    buffer: String,
    socket: Arc<UdpSocket>,
    auto_flush: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl ScopeBuffer {
    fn write (&mut self, metric: &StatsdMetric, value: Value) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();
        let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

        if entry_len > self.buffer.capacity() {
            // TODO report entry too big to fit in buffer (!?)
            return;
        }

        let remaining = self.buffer.capacity() - self.buffer.len();
        if entry_len + 1 > remaining {
            // buffer is full, flush before appending
            self.flush();
        } else {
            if !self.buffer.is_empty() {
                // separate from previous entry
                self.buffer.push('\n')
            }
            self.buffer.push_str(&metric.prefix);
            self.buffer.push_str(&value_str);
            self.buffer.push_str(&metric.suffix);
        }
        if self.auto_flush {
            self.flush();
        }
    }

    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            match self.socket.send(self.buffer.as_bytes()) {
                Ok(size) => {
                    SENT_BYTES.count(size);
                    trace!("Sent {} bytes to statsd", self.buffer.len());
                }
                Err(e) => {
                    SEND_ERR.mark();
                    debug!("Failed to send packet to statsd: {}", e);
                }
            };
            self.buffer.clear();
        }
    }
}

/// Allows sending metrics to a statsd server
#[derive(Debug)]
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
            Kind::Marker | Kind::Counter => "c",
            Kind::Gauge => "g",
            Kind::Timer => "ms",
        });

        if sampling < FULL_SAMPLING_RATE {
            suffix.push_str("|@");
            suffix.push_str(&sampling.to_string());
        }

        let scale = match kind {
            // timers are in µs, statsd wants ms
            Kind::Timer => 1000,
            _ => 1,
        };

        StatsdMetric {
            prefix,
            suffix,
            scale,
        }
    }

    #[allow(unused_variables)]
    fn new_scope(&self, auto_flush: bool) -> ScopeFn<StatsdMetric> {
        let buf = RwLock::new(ScopeBuffer {
            buffer: String::with_capacity(MAX_UDP_PAYLOAD),
            socket: self.socket.clone(),
            auto_flush,
        });
        Arc::new(move |cmd| {
            if let Ok(mut buf) = buf.write() {
                match cmd {
                    Scope::Write(metric, value) => buf.write(metric, value),
                    Scope::Flush => buf.flush(),
                }
            }
        })
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn timer_statsd(b: &mut test::Bencher) {
        let sd = to_statsd("localhost:8125", "a.").unwrap();
        let timer = sd.new_metric(Kind::Timer, "timer", 1000000.0);
        let scope = sd.new_scope(false);

        b.iter(|| test::black_box(scope.as_ref()(Scope::Write(&timer, 2000))));
    }

}
