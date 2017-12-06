//! Send metrics to a graphite server.

use core::*;
use error;
use selfmetrics::*;

use std::net::TcpStream;
use std::sync::{Arc, RwLock};
pub use std::net::ToSocketAddrs;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::fmt::Debug;

/// Send metrics to a graphite server at the address and port provided.
pub fn to_graphite<ADDR>(address: ADDR, prefix: &str) -> error::Result<GraphiteSink>
where
    ADDR: ToSocketAddrs + Debug,
{
    debug!("Connecting to graphite {:?}", address);
    let socket = TcpStream::connect(address)?;
    socket.set_nonblocking(true)?;

    Ok(GraphiteSink {
        socket: Arc::new(RwLock::new(socket)),
        prefix: String::from(prefix),
    })
}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65536;

lazy_static! {
    static ref GRAPHITE_METRICS: AppMetrics<Aggregate, AggregateSink> =
                                            SELF_METRICS.with_prefix("graphite.");

    static ref SEND_ERR: Marker<Aggregate> = GRAPHITE_METRICS.marker("send_failed");
    static ref SENT_BYTES: Counter<Aggregate> = GRAPHITE_METRICS.counter("sent_bytes");
    static ref TRESHOLD_EXCEEDED: Marker<Aggregate> =
                                            GRAPHITE_METRICS.marker("threshold_exceeded");
}

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct GraphiteMetric {
    prefix: String,
    scale: u64,
}

/// Wrap string buffer & socket as one.
#[derive(Debug)]
struct ScopeBuffer {
    buffer: Arc<RwLock<String>>,
    socket: Arc<RwLock<TcpStream>>,
    auto_flush: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl ScopeBuffer {
    fn write (&self, metric: &GraphiteMetric, value: Value) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();
        let flush = match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                let mut buf = self.buffer.write().expect("Could not lock graphite buffer.");
                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push(' ');
                buf.push_str(timestamp.as_secs().to_string().as_ref());
                buf.push('\n');
                if buf.len() > BUFFER_FLUSH_THRESHOLD {
                    TRESHOLD_EXCEEDED.mark();
                    warn!("Flushing metrics scope buffer to graphite because its size exceeds \
                        the threshold of {} bytes. ", BUFFER_FLUSH_THRESHOLD);
                    true
                } else if self.auto_flush {
                    true
                } else {
                    false
                }
            },
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
                false
            },
        };

        if flush {
            self.flush();
        }
    }

    fn flush(&self) {
        // TODO locking is getting out of hand - use some Cell... or make scopes !Sync
        let mut buf = self.buffer.write().expect("Could not lock graphite buffer.");
        if !buf.is_empty() {
            let mut sock = self.socket.write().expect("Could not lock graphite socket.");
            match sock.write(buf.as_bytes()) {
                Ok(size) => {
                    buf.clear();
                    SENT_BYTES.count(size);
                    trace!("Sent {} bytes to graphite", buf.len());
                }
                Err(e) => {
                    SEND_ERR.mark();
                    debug!("Failed to send buffer to graphite: {}", e);
                }
            };
            buf.clear();
        }
    }
}

/// Allows sending metrics to a graphite server
#[derive(Debug)]
pub struct GraphiteSink {
    socket: Arc<RwLock<TcpStream>>,
    prefix: String,
}

impl Sink<GraphiteMetric> for GraphiteSink {
    fn new_metric(&self, kind: Kind, name: &str, rate: Rate) -> GraphiteMetric {
        let mut prefix = String::with_capacity(32);
        prefix.push_str(&self.prefix);
        prefix.push_str(name.as_ref());
        prefix.push(' ');

        let mut scale = match kind {
            // timers are in µs, lets give graphite milliseconds for consistency with statsd
            Kind::Timer => 1000,
            _ => 1,
        };

        if rate < FULL_SAMPLING_RATE {
            // graphite does not do sampling, so we'll upsample before sending
            let upsample = (1.0 / rate).round() as u64;
            warn!("Metric {:?} '{}' being sampled at rate {} will be upsampled \
                by a factor of {} when sent to graphite.", kind, name, rate, upsample);
            scale = scale * upsample;
        }

        GraphiteMetric {
            prefix,
            scale,
        }
    }

    #[allow(unused_variables)]
    fn new_scope(&self, auto_flush: bool) -> ScopeFn<GraphiteMetric> {
        let buf = ScopeBuffer {
            buffer: Arc::new(RwLock::new(String::new())),
            socket: self.socket.clone(),
            auto_flush,
        };
        Arc::new(move |cmd| {
            match cmd {
                Scope::Write(metric, value) => buf.write(metric, value),
                Scope::Flush => buf.flush(),
            }
        })
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn timer_graphite(b: &mut test::Bencher) {
        let sd = to_graphite("localhost:8125", "a.").unwrap();
        let timer = sd.new_metric(Kind::Timer, "timer", 1000000.0);
        let scope = sd.new_scope(false);

        b.iter(|| test::black_box(scope.as_ref()(Scope::Write(&timer, 2000))));
    }

}
