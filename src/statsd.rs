//! Send metrics to a statsd server.

use core::*;
use scope_metrics::*;
use error;
use self_metrics::*;

use std::net::UdpSocket;
use std::sync::{Arc, RwLock};

pub use std::net::ToSocketAddrs;

mod_metrics!(Aggregate, STATSD_METRICS = DIPSTICK_METRICS.with_prefix("statsd"));
mod_marker!(Aggregate, STATSD_METRICS, { SEND_ERR: "send_failed" });
mod_counter!(Aggregate, STATSD_METRICS, { SENT_BYTES: "sent_bytes" });

/// Send metrics to a statsd server at the address and port provided.
pub fn to_statsd<ADDR>(address: ADDR) -> error::Result<ScopeMetrics<Statsd>>
where
    ADDR: ToSocketAddrs,
{
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
    socket.set_nonblocking(true)?;
    socket.connect(address)?;

    Ok(ScopeMetrics::new(
        move |kind, name, rate| {
            let mut prefix = String::with_capacity(32);
            prefix.push_str(name);
            prefix.push(':');

            let mut suffix = String::with_capacity(16);
            suffix.push('|');
            suffix.push_str(match kind {
                Kind::Marker | Kind::Counter => "c",
                Kind::Gauge => "g",
                Kind::Timer => "ms",
            });

            if rate < FULL_SAMPLING_RATE {
                suffix.push_str("|@");
                suffix.push_str(&rate.to_string());
            }

            let scale = match kind {
                // timers are in µs, statsd wants ms
                Kind::Timer => 1000,
                _ => 1,
            };

            Statsd {
                prefix,
                suffix,
                scale,
            }
        },
        move |buffered| {
            let buf = RwLock::new(ScopeBuffer {
                buffer: String::with_capacity(MAX_UDP_PAYLOAD),
                socket: socket.clone(),
                buffered,
            });
            control_scope(move |cmd| {
                if let Ok(mut buf) = buf.write() {
                    match cmd {
                        ScopeCmd::Write(metric, value) => buf.write(metric, value),
                        ScopeCmd::Flush => buf.flush(),
                    }
                }
            })
        },
    ))
}

/// Key of a statsd metric.
#[derive(Debug, Clone)]
pub struct Statsd {
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
    buffered: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl ScopeBuffer {
    fn write(&mut self, metric: &Statsd, value: Value) {
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
        if self.buffered {
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

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn timer_statsd(b: &mut test::Bencher) {
        let sd = to_statsd("localhost:8125").unwrap();
        let timer = sd.define_metric(Kind::Timer, "timer", 1000000.0);
        let scope = sd.open_scope(false);

        b.iter(|| test::black_box(scope.write(&timer, 2000)));
    }

}
