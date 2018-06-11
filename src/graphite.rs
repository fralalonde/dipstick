//! Send metrics to a graphite server.

use core::*;
use aggregate::*;
use error;
use self_metrics::DIPSTICK_METRICS;

use std::net::ToSocketAddrs;

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::fmt::Debug;

use socket::RetrySocket;

metrics!{
    <MetricAggregator> DIPSTICK_METRICS.with_prefix("graphite") => {
        Marker SEND_ERR: "send_failed";
        Marker TRESHOLD_EXCEEDED: "bufsize_exceeded";
        Counter SENT_BYTES: "sent_bytes";
    }
}

#[derive(Clone, Debug)]
pub struct GraphiteOutput {
    namespace: Namespace,
    socket: Arc<RwLock<RetrySocket>>,
    buffered: bool,
}

impl MetricOutput for GraphiteOutput {

    type Input = Graphite;

    fn open(&self) -> Graphite {
        Graphite {
            namespace: self.namespace.clone(),
            buffer: ScopeBuffer {
                buffer: Arc::new(RwLock::new(String::new())),
                socket: self.socket.clone(),
                buffered: self.buffered,
            }
        }
    }
}

/// Graphite MetricInput
#[derive(Debug)]
pub struct Graphite {
    namespace: Namespace,
    buffer: ScopeBuffer,
}

impl MetricInput for Graphite {
    /// Define a metric of the specified type.
    fn define_metric(&self, namespace: &Namespace, kind: Kind) -> WriteFn {
        let mut prefix = namespace.join(".");
        prefix.push(' ');

        let scale = match kind {
            // timers are in µs, but we give graphite milliseconds
            Kind::Timer => 1000,
            _ => 1,
        };

        let buffer = self.buffer.clone();
        let metric = GraphiteMetric { prefix, scale };
        WriteFn::new(move |value| {
            if let Err(err) = buffer.write(&metric, value) {
                debug!("Graphite buffer write failed: {}", err);
                SEND_ERR.mark();
            }
        })
    }

}

impl Flush for Graphite {
    fn flush(&self) -> error::Result<()> {
        self.buffer.flush()
    }
}

/// Send metrics to a graphite server at the address and port provided.
pub fn to_graphite<ADDR: ToSocketAddrs + Debug + Clone>(address: ADDR)
    -> error::Result<GraphiteOutput>
{
    debug!("Connecting to graphite {:?}", address);
    let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));

    Ok(GraphiteOutput {
        namespace: ROOT_NS.clone(),
        socket,
        buffered: true
    })
}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct GraphiteMetric {
    prefix: String,
    scale: u64,
}

/// Wrap string buffer & socket as one.
#[derive(Debug, Clone)]
struct ScopeBuffer {
    buffer: Arc<RwLock<String>>,
    socket: Arc<RwLock<RetrySocket>>,
    buffered: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Could not flush graphite metrics upon Drop: {}", err)
        }
    }
}

impl ScopeBuffer {
    fn write(&self, metric: &GraphiteMetric, value: Value) -> error::Result<()> {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();
        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                let mut buf = self.buffer.write().expect("Locking graphite buffer");

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
                    self.flush_inner(&mut buf)?;
                } else if !self.buffered {
                    self.flush_inner(&mut buf)?;
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };
        Ok(())
    }

    fn flush_inner(&self, buf: &mut String) -> error::Result<()> {
        if buf.is_empty() { return Ok(()) }

        let mut sock = self.socket.write().expect("Locking graphite socket");
        match sock.write_all(buf.as_bytes()) {
            Ok(()) => {
                buf.clear();
                SENT_BYTES.count(buf.len());
                trace!("Sent {} bytes to graphite", buf.len());
                Ok(())
            }
            Err(e) => {
                SEND_ERR.mark();
                debug!("Failed to send buffer to graphite: {}", e);
                Err(e.into())
            }
        }
    }

    fn flush(&self) -> error::Result<()> {
        let mut buf = self.buffer.write().expect("Locking graphite buffer");
        self.flush_inner(&mut buf)
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn unbufferd_graphite(b: &mut test::Bencher) {
        let sd = to_graphite("localhost:8125").unwrap().open_scope();
        let timer = sd.define_metric(&"timer".into(), Kind::Timer, 1000000.0);

        b.iter(|| test::black_box(sd.write(&timer, 2000)));
    }

    #[bench]
    pub fn buffered_graphite(b: &mut test::Bencher) {
        let sd = to_buffered_graphite("localhost:8125").unwrap().open_scope();
        let timer = sd.define_metric(&"timer".into(), Kind::Timer, 1000000.0);

        b.iter(|| test::black_box(sd.write(&timer, 2000)));
    }

}
