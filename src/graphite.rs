//! Send metrics to a graphite server.

use core::*;
use error;
use metrics;

use std::net::ToSocketAddrs;

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::fmt::Debug;

use socket::RetrySocket;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};

/// Send metrics to a graphite server at the address and port provided.
pub fn output_graphite<A: ToSocketAddrs + Debug + Clone>(address: A) -> error::Result<GraphiteOutput> {
    debug!("Connecting to graphite {:?}", address);
    let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));

    Ok(GraphiteOutput {
        attributes: Attributes::default(),
        socket,
        buffered: false
    })
}

/// Graphite output holds a socket to a graphite server.
/// The connection is shared between all graphite inputs originating from it.
#[derive(Clone, Debug)]
pub struct GraphiteOutput {
    attributes: Attributes,
    socket: Arc<RwLock<RetrySocket>>,
    buffered: bool,
}

impl RawOutput for GraphiteOutput {

    type INPUT = Graphite;

    fn new_raw_input(&self) -> Graphite {
        Graphite {
            attributes: self.attributes.clone(),
            buffer: Rc::new(RefCell::new(String::new())),
            socket: self.socket.clone(),
        }
    }
}

impl WithAttributes for GraphiteOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl WithBuffering for GraphiteOutput {}

impl Cache for GraphiteOutput {}
impl Async for GraphiteOutput {}

/// Graphite Input
#[derive(Debug, Clone)]
pub struct Graphite {
    attributes: Attributes,
    buffer: Rc<RefCell<String>>,
    socket: Arc<RwLock<RetrySocket>>,
}

impl RawInput for Graphite {
    /// Define a metric of the specified type.
    fn new_metric_raw(&self, name: Name, kind: Kind) -> RawMetric {
        let mut prefix = self.qualified_name(name).join(".");
        prefix.push(' ');

        let scale = match kind {
            // timers are in µs, but we give graphite milliseconds
            Kind::Timer => 1000,
            _ => 1,
        };

        let cloned = self.clone();
        let metric = GraphiteMetric { prefix, scale };

        if self.is_buffering() {
            RawMetric::new(move |value| {
                if let Err(err) = cloned.buf_write(&metric, value) {
                    debug!("Graphite buffer write failed: {}", err);
                    metrics::GRAPHITE_SEND_ERR.mark();
                }
            })
        } else {
            RawMetric::new(move |value| {
                if let Err(err) = cloned.buf_write(&metric, value).and_then(|_| cloned.flush_raw()) {
                    debug!("Graphite buffer write failed: {}", err);
                    metrics::GRAPHITE_SEND_ERR.mark();
                }

            })
        }
    }

    fn flush_raw(&self) -> error::Result<()> {
        let buf = self.buffer.borrow_mut();
        self.flush_inner(buf)
    }
}

impl Graphite {
    fn buf_write(&self, metric: &GraphiteMetric, value: Value) -> error::Result<()> {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();
        let mut buf = self.buffer.borrow_mut();
        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push(' ');
                buf.push_str(&timestamp.as_secs().to_string());
                buf.push('\n');

                if buf.len() > BUFFER_FLUSH_THRESHOLD {
                    metrics::GRAPHITE_OVERFLOW.mark();
                    warn!("Graphite Buffer Size Exceeded: {}", BUFFER_FLUSH_THRESHOLD);
                    self.flush_inner(buf)?;
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };
        Ok(())
    }

    fn flush_inner(&self, mut buf: RefMut<String>) -> error::Result<()> {
        if buf.is_empty() { return Ok(()) }

        let mut sock = self.socket.write().expect("Lock Graphite Socket");
        match sock.write_all(buf.as_bytes()) {
            Ok(()) => {
                buf.clear();
                metrics::GRAPHITE_SENT_BYTES.count(buf.len());
                trace!("Sent {} bytes to graphite", buf.len());
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

impl WithAttributes for Graphite {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl WithBuffering for Graphite {}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

/// Key of a graphite metric.
#[derive(Debug, Clone)]
pub struct GraphiteMetric {
    prefix: String,
    scale: u64,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for Graphite {
    fn drop(&mut self) {
        if let Err(err) = self.flush_raw() {
            warn!("Could not flush graphite metrics upon Drop: {}", err)
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use core::*;
    use super::*;
    use test;

    #[bench]
    pub fn unbuffered_graphite(b: &mut test::Bencher) {
        let sd = output_graphite("localhost:2003").unwrap().new_raw_input();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

    #[bench]
    pub fn buffered_graphite(b: &mut test::Bencher) {
        let sd = output_graphite("localhost:2003").unwrap().with_buffering(Buffering::BufferSize(65465)).new_raw_input();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

}
