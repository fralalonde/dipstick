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

/// Graphite output holds a socket to a graphite server.
/// The socket is shared between scopes opened from the output.
#[derive(Clone, Debug)]
pub struct GraphiteOutput {
    attributes: Attributes,
    socket: Arc<RwLock<RetrySocket>>,
}

impl RawOutput for GraphiteOutput {
    type SCOPE = Graphite;

    fn open_scope_raw(&self) -> Graphite {
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

impl WithMetricCache for GraphiteOutput {}
impl WithQueue for GraphiteOutput {}

/// Graphite Input
#[derive(Debug, Clone)]
pub struct Graphite {
    attributes: Attributes,
    buffer: Rc<RefCell<String>>,
    socket: Arc<RwLock<RetrySocket>>,
}

impl Graphite {
    /// Send metrics to a graphite server at the address and port provided.
    pub fn output<A: ToSocketAddrs + Debug + Clone>(address: A) -> error::Result<GraphiteOutput> {
        debug!("Connecting to graphite {:?}", address);
        let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));

        Ok(GraphiteOutput {
            attributes: Attributes::default(),
            socket,
        })
    }
}

impl RawScope for Graphite {
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

        RawMetric::new(move |value| {
            cloned.print(&metric, value);
        })
    }
}

impl Flush for Graphite {

    fn flush(&self) -> error::Result<()> {
        let buf = self.buffer.borrow_mut();
        self.flush_inner(buf)
    }
}

impl Graphite {
    fn print(&self, metric: &GraphiteMetric, value: Value)  {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();

        let mut buffer = self.buffer.borrow_mut();
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
                    buffer = self.buffer.borrow_mut();
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };

        if !self.is_buffering() {
            if let Err(e) = self.flush_inner(buffer) {
                debug!("Could not send to graphite {}", e)
            }
        }
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
        if let Err(err) = self.flush() {
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
    pub fn immediate_graphite(b: &mut test::Bencher) {
        let sd = Graphite::output("localhost:2003").unwrap().open_scope_raw();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

    #[bench]
    pub fn buffering_graphite(b: &mut test::Bencher) {
        let sd = Graphite::output("localhost:2003").unwrap()
            .with_buffering(Buffering::BufferSize(65465)).open_scope_raw();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

}
