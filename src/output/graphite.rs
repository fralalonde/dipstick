//! Send metrics to a graphite server.

use crate::attributes::{Attributes, Buffered, MetricId, OnFlush, Prefixed, WithAttributes};
use crate::input::InputKind;
use crate::input::{Input, InputMetric, InputScope};
use crate::metrics;
use crate::name::MetricName;
use crate::output::socket::RetrySocket;
use crate::{error, CachedInput, QueuedInput};
use crate::{Flush, MetricValue};

use std::net::ToSocketAddrs;

use std::fmt::Debug;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::{RwLock, RwLockWriteGuard};

#[cfg(feature = "parking_lot")]
use parking_lot::{RwLock, RwLockWriteGuard};

/// Graphite Input holds a socket to a graphite server.
/// The socket is shared between scopes opened from the Input.
#[derive(Clone, Debug)]
pub struct Graphite {
    attributes: Attributes,
    socket: Arc<RwLock<RetrySocket>>,
}

impl Input for Graphite {
    type SCOPE = GraphiteScope;

    fn metrics(&self) -> Self::SCOPE {
        GraphiteScope {
            attributes: self.attributes.clone(),
            buffer: Arc::new(RwLock::new(String::new())),
            socket: self.socket.clone(),
        }
    }
}

impl Graphite {
    /// Send metrics to a graphite server at the address and port provided.
    pub fn send_to<A: ToSocketAddrs + Debug + Clone>(address: A) -> error::Result<Graphite> {
        debug!("Connecting to graphite {:?}", address);
        let socket = Arc::new(RwLock::new(RetrySocket::new(address)?));

        Ok(Graphite {
            attributes: Attributes::default(),
            socket,
        })
    }
}

impl WithAttributes for Graphite {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Buffered for Graphite {}

/// Graphite Input
#[derive(Debug, Clone)]
pub struct GraphiteScope {
    attributes: Attributes,
    buffer: Arc<RwLock<String>>,
    socket: Arc<RwLock<RetrySocket>>,
}

impl InputScope for GraphiteScope {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let mut prefix = self.prefix_prepend(name.clone()).join(".");
        prefix.push(' ');

        let scale = match kind {
            // timers are in µs, but we give graphite milliseconds
            InputKind::Timer => 1000,
            _ => 1,
        };

        let cloned = self.clone();
        let metric = GraphiteMetric { prefix, scale };
        let metric_id = MetricId::forge("graphite", name);

        InputMetric::new(metric_id, move |value, _labels| {
            cloned.print(&metric, value);
        })
    }
}

impl Flush for GraphiteScope {
    fn flush(&self) -> error::Result<()> {
        self.notify_flush_listeners();
        let buf = write_lock!(self.buffer);
        self.flush_inner(buf)
    }
}

impl GraphiteScope {
    fn print(&self, metric: &GraphiteMetric, value: MetricValue) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();

        let mut buffer = write_lock!(self.buffer);
        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                buffer.push_str(&metric.prefix);
                buffer.push_str(&value_str);
                buffer.push(' ');
                buffer.push_str(&timestamp.as_secs().to_string());
                buffer.push('\n');

                if buffer.len() > BUFFER_FLUSH_THRESHOLD {
                    metrics::GRAPHITE_OVERFLOW.mark();
                    warn!("Graphite Buffer Size Exceeded: {}", BUFFER_FLUSH_THRESHOLD);
                    let _ = self.flush_inner(buffer);
                    buffer = write_lock!(self.buffer);
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };

        if self.is_buffered() {
            if let Err(e) = self.flush_inner(buffer) {
                debug!("Could not send to graphite {}", e)
            }
        }
    }

    fn flush_inner(&self, mut buf: RwLockWriteGuard<String>) -> error::Result<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let mut sock = write_lock!(self.socket);
        match sock.write_all(buf.as_bytes()) {
            Ok(()) => {
                metrics::GRAPHITE_SENT_BYTES.count(buf.len());
                trace!("Sent {} bytes to graphite", buf.len());
                buf.clear();
                Ok(())
            }
            Err(e) => {
                metrics::GRAPHITE_SEND_ERR.mark();
                debug!("Failed to send buffer to graphite: {}", e);
                Err(e.into())
            }
        }
    }
}

impl WithAttributes for GraphiteScope {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Buffered for GraphiteScope {}

impl QueuedInput for Graphite {}
impl CachedInput for Graphite {}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct GraphiteMetric {
    prefix: String,
    scale: isize,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for GraphiteScope {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Could not flush graphite metrics upon Drop: {}", err)
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use crate::attributes::*;
    use crate::input::*;

    #[bench]
    pub fn immediate_graphite(b: &mut test::Bencher) {
        let sd = Graphite::send_to("localhost:2003").unwrap().metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }

    #[bench]
    pub fn buffering_graphite(b: &mut test::Bencher) {
        let sd = Graphite::send_to("localhost:2003")
            .unwrap()
            .buffered(Buffering::BufferSize(65465))
            .metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }
}
