//! Send metrics to a statsd server.

use core::*;
use error;
use metrics;

use std::net::ToSocketAddrs;

use std::sync::Arc;

use pcg32;

use std::net::UdpSocket;
use std::rc::Rc;
use std::cell::{RefCell, RefMut};

/// Use a safe maximum size for UDP to prevent fragmentation.
// TODO make configurable?
const MAX_UDP_PAYLOAD: usize = 576;

/// Statsd output holds a datagram (UDP) socket to a statsd server.
/// The socket is shared between scopes opened from the output.
#[derive(Clone, Debug)]
pub struct StatsdOutput {
    attributes: Attributes,
    socket: Arc<UdpSocket>,
}

impl WithBuffering for StatsdOutput {}
impl WithSampling for StatsdOutput {}

impl WithMetricCache for StatsdOutput {}
impl WithQueue for StatsdOutput {}

impl RawOutput for StatsdOutput {
    type SCOPE = Statsd;

    fn open_scope_raw(&self) -> Statsd {
        Statsd {
            attributes: self.attributes.clone(),
            buffer: Rc::new(RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD))),
            socket: self.socket.clone(),
        }
    }
}

impl WithAttributes for StatsdOutput {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Statsd Input
#[derive(Debug, Clone)]
pub struct Statsd {
    attributes: Attributes,
    buffer: Rc<RefCell<String>>,
    socket: Arc<UdpSocket>,
}

impl Statsd {
    /// Send metrics to a statsd server at the address and port provided.
    pub fn output<ADDR: ToSocketAddrs>(address: ADDR) -> error::Result<StatsdOutput> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(StatsdOutput {
            attributes: Attributes::default(),
            socket,
        })
    }
}

impl WithSampling for Statsd {}

impl RawScope for Statsd {
    /// Define a metric of the specified type.
    fn new_metric_raw(&self, name: Name, kind: Kind) -> RawMetric {
        let mut prefix = self.qualified_name(name).join(".");
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match kind {
            Kind::Marker | Kind::Counter => "c",
            Kind::Gauge => "g",
            Kind::Timer => "ms",
        });

        let scale = match kind {
            // timers are in µs, statsd wants ms
            Kind::Timer => 1000,
            _ => 1,
        };

        let cloned = self.clone();

        if let Sampling::Random(float_rate) = self.get_sampling() {
            suffix.push_str(&format!{"|@{}\n", float_rate});
            let int_sampling_rate = pcg32::to_int_rate(float_rate);
            let metric = StatsdMetric { prefix, suffix, scale };

            RawMetric::new(move |value| {
                if pcg32::accept_sample(int_sampling_rate) {
                    cloned.print(&metric, value)
                }
            })
        } else {
            suffix.push_str("\n");
            let metric = StatsdMetric { prefix, suffix, scale };
            RawMetric::new(move |value| {
                cloned.print(&metric, value)
            })
        }
    }
}

impl Flush for Statsd {

    fn flush(&self) -> error::Result<()> {
        let buf = self.buffer.borrow_mut();
        self.flush_inner(buf)
    }
}

impl Statsd {
    fn print(&self, metric: &StatsdMetric, value: Value)  {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();
        let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

        let mut buffer = self.buffer.borrow_mut();
        if entry_len > buffer.capacity() {
            // TODO report entry too big to fit in buffer (!?)
            return;
        }

        let remaining = buffer.capacity() - buffer.len();
        if entry_len + 1 > remaining {
            // buffer is nearly full, make room
            let _ = self.flush_inner(buffer);
            buffer = self.buffer.borrow_mut();

        } else {
            if !buffer.is_empty() {
                // separate from previous entry
                buffer.push('\n')
            }
            buffer.push_str(&metric.prefix);
            buffer.push_str(&value_str);
            buffer.push_str(&metric.suffix);
        }

        if !self.is_buffering() {
            if let Err(e) = self.flush_inner(buffer) {
                debug!("Could not send to statsd {}", e)
            }
        }
    }

    fn flush_inner(&self, mut buffer: RefMut<String>) -> error::Result<()> {
        if !buffer.is_empty() {
            match self.socket.send(buffer.as_bytes()) {
                Ok(size) => {
                    metrics::STATSD_SENT_BYTES.count(size);
                    trace!("Sent {} bytes to statsd", buffer.len());
                }
                Err(e) => {
                    metrics::STATSD_SEND_ERR.mark();
                    return Err(e.into())
                }
            };
            buffer.clear();
        }
        Ok(())
    }
}

impl WithAttributes for Statsd {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl WithBuffering for Statsd {}

/// Key of a statsd metric.
#[derive(Debug, Clone)]
pub struct StatsdMetric {
    prefix: String,
    suffix: String,
    scale: u64,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for Statsd {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Could not flush statsd metrics upon Drop: {}", err)
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use core::*;
    use super::*;
    use test;

    #[bench]
    pub fn immediate_statsd(b: &mut test::Bencher) {
        let sd = Statsd::output("localhost:2003").unwrap().open_scope_raw();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

    #[bench]
    pub fn buffering_statsd(b: &mut test::Bencher) {
        let sd = Statsd::output("localhost:2003").unwrap()
            .with_buffering(Buffering::BufferSize(65465)).open_scope_raw();
        let timer = sd.new_metric_raw("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

}
