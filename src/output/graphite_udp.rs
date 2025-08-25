//! Send metrics to a graphite server.

use crate::attributes::{Attributes, Buffered, MetricId, OnFlush, Prefixed, WithAttributes};
use crate::input::InputKind;
use crate::input::{Input, InputMetric, InputScope};
use crate::metrics;
use crate::name::MetricName;

use crate::{CachedInput, QueuedInput};
use crate::{Flush, MetricValue};

use std::net::ToSocketAddrs;

use std::fmt::Debug;
use std::net::UdpSocket;
use std::sync::Arc;

use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(not(feature = "parking_lot"))]
use std::sync::{RwLock, RwLockWriteGuard};

#[cfg(feature = "parking_lot")]
use parking_lot::{RwLock, RwLockWriteGuard};
use std::io;

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

/// GraphiteUdp Input holds a socket to a graphite server.
/// The socket is shared between scopes opened from the Input.
#[derive(Clone, Debug)]
pub struct GraphiteUdp {
    attributes: Attributes,
    socket: Arc<UdpSocket>,
}

impl Input for GraphiteUdp {
    type SCOPE = GraphiteUdpScope;

    fn metrics(&self) -> Self::SCOPE {
        GraphiteUdpScope {
            attributes: self.attributes.clone(),
            buffer: Arc::new(RwLock::new(String::with_capacity(MAX_UDP_PAYLOAD))),
            socket: self.socket.clone(),
        }
    }
}

impl GraphiteUdp {
    /// Send metrics to a graphite server at the address and port provided.
    pub fn send_to<ADDR: ToSocketAddrs>(address: ADDR) -> io::Result<GraphiteUdp> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(GraphiteUdp {
            attributes: Attributes::default(),
            socket,
        })
    }
}

impl WithAttributes for GraphiteUdp {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Buffered for GraphiteUdp {}

/// GraphiteUdp Input
#[derive(Debug, Clone)]
pub struct GraphiteUdpScope {
    attributes: Attributes,
    buffer: Arc<RwLock<String>>,
    socket: Arc<UdpSocket>,
}

impl InputScope for GraphiteUdpScope {
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
        let metric = GraphiteUdpMetric { prefix, scale };
        let metric_id = MetricId::forge("graphite", name);

        InputMetric::new(metric_id, move |value, _labels| {
            cloned.print(&metric, value);
        })
    }
}

impl Flush for GraphiteUdpScope {
    fn flush(&self) -> io::Result<()> {
        self.notify_flush_listeners();
        let buf = write_lock!(self.buffer);
        self.flush_inner(buf)
    }
}

impl GraphiteUdpScope {
    fn print(&self, metric: &GraphiteUdpMetric, value: MetricValue) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();
        let start = SystemTime::now();

        let mut buffer = write_lock!(self.buffer);

        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                let metric = format!(
                   "{}{} {}\n",
                    &metric.prefix,
                    &value_str,
                    &timestamp.as_secs().to_string()
                );
                let entry_len = metric.len();
                let available = buffer.capacity() - buffer.len();
                if entry_len > buffer.capacity() {
                    // entry simply too  big to fit in buffer
                    return;
                }
                if entry_len > available {
                    let _ = self.flush_inner(buffer);
                    buffer = write_lock!(self.buffer);
                } 
                buffer.push_str(&metric);                
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };
        if !self.is_buffered() {
            if let Err(e) = self.flush_inner(buffer) {
                debug!("Could not send to graphite {}", e)
            }
        }
    }

    fn flush_inner(&self, mut buffer: RwLockWriteGuard<String>) -> io::Result<()> {
        if !buffer.is_empty() {
            match self.socket.send(buffer.as_bytes()) {
                Ok(size) => {
                    metrics::GRAPHITE_SENT_BYTES.count(size);
                    trace!("Sent {} bytes to graphite", buffer.len());
                }
                Err(e) => {
                    metrics::GRAPHITE_SEND_ERR.mark();
                    debug!("Failed to send buffer to graphite: {}", e);
                    return Err(e);
                }
            };
            buffer.clear();
        }
        Ok(())
    }
}

impl WithAttributes for GraphiteUdpScope {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Buffered for GraphiteUdpScope {}

impl QueuedInput for GraphiteUdp {}
impl CachedInput for GraphiteUdp {}

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct GraphiteUdpMetric {
    prefix: String,
    scale: isize,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for GraphiteUdpScope {
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
        let sd = GraphiteUdp::send_to("localhost:2003").unwrap().metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }

    #[bench]
    pub fn buffering_graphite(b: &mut test::Bencher) {
        let sd = GraphiteUdp::send_to("localhost:2003")
            .unwrap()
            .buffered(Buffering::BufferSize(65465))
            .metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }
}
