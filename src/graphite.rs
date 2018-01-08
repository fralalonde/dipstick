//! Send metrics to a graphite server.

use core::*;
use error;
use self_metrics::*;

use std::net::ToSocketAddrs;

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::fmt::Debug;

use socket::RetrySocket;

/// Send metrics to a graphite server at the address and port provided.
pub fn to_graphite<ADDR>(address: ADDR) -> error::Result<Chain<Graphite>>
where
    ADDR: ToSocketAddrs + Debug + Clone,
{
    debug!("Connecting to graphite {:?}", address);
    let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));
    Ok(Chain::new(
        move |kind, name, rate| {
            let mut prefix = String::with_capacity(32);
            prefix.push_str(name);
            prefix.push(' ');

            let mut scale = match kind {
                // timers are in µs, lets give graphite milliseconds for consistency with statsd
                Kind::Timer => 1000,
                _ => 1,
            };

            if rate < FULL_SAMPLING_RATE {
                // graphite does not do sampling, so we'll upsample before sending
                let upsample = (1.0 / rate).round() as u64;
                warn!(
                    "Metric {:?} '{}' being sampled at rate {} will be upsampled \
                     by a factor of {} when sent to graphite.",
                    kind, name, rate, upsample
                );
                scale = scale * upsample;
            }

            Graphite { prefix, scale }
        },
        move |buffered| {
            let buf = ScopeBuffer {
                buffer: Arc::new(RwLock::new(String::new())),
                socket: socket.clone(),
                buffered,
            };
            ControlScopeFn::new(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => buf.write(metric, value),
                ScopeCmd::Flush => buf.flush(),
            })
        },
    ))
}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

lazy_static! {
    static ref GRAPHITE_METRICS: AppMetrics<Aggregate> = SELF_METRICS.with_prefix("graphite");
    static ref SEND_ERR: AppMarker<Aggregate> = GRAPHITE_METRICS.marker("send_failed");
    static ref SENT_BYTES: AppCounter<Aggregate> = GRAPHITE_METRICS.counter("sent_bytes");
    static ref TRESHOLD_EXCEEDED: AppMarker<Aggregate> = GRAPHITE_METRICS.marker("bufsize_exceeded");
}

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct Graphite {
    prefix: String,
    scale: u64,
}

/// Wrap string buffer & socket as one.
#[derive(Debug)]
struct ScopeBuffer {
    buffer: Arc<RwLock<String>>,
    socket: Arc<RwLock<RetrySocket>>,
    buffered: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl ScopeBuffer {
    fn write(&self, metric: &Graphite, value: Value) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();
        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                let mut buf = self.buffer.write().expect("Lock graphite buffer.");

                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push(' ');
                buf.push_str(timestamp.as_secs().to_string().as_ref());
                buf.push('\n');

                if buf.len() > BUFFER_FLUSH_THRESHOLD {
                    TRESHOLD_EXCEEDED.mark();
                    warn!(
                        "Flushing metrics scope buffer to graphite because its size exceeds \
                         the threshold of {} bytes. ",
                        BUFFER_FLUSH_THRESHOLD
                    );
                    self.flush_inner(&mut buf);
                } else if !self.buffered {
                    self.flush_inner(&mut buf);
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };
    }

    fn flush_inner(&self, buf: &mut String) {
        if !buf.is_empty() {
            let mut sock = self.socket.write().expect("Lock graphite socket.");
            match sock.write(buf.as_bytes()) {
                Ok(size) => {
                    buf.clear();
                    SENT_BYTES.count(size);
                    trace!("Sent {} bytes to graphite", buf.len());
                }
                Err(e) => {
                    SEND_ERR.mark();
                    // still just a best effort, do not warn! for every failure
                    debug!("Failed to send buffer to graphite: {}", e);
                }
            };
            buf.clear();
        }
    }

    fn flush(&self) {
        let mut buf = self.buffer.write().expect("Lock graphite buffer.");
        self.flush_inner(&mut buf);
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn timer_graphite(b: &mut test::Bencher) {
        let sd = to_graphite("localhost:8125").unwrap();
        let timer = sd.define_metric(Kind::Timer, "timer", 1000000.0);
        let scope = sd.open_scope(false);

        b.iter(|| test::black_box(scope.write(&timer, 2000)));
    }

}
