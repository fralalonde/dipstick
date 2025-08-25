//! Send metrics to a statsd server.

use crate::attributes::{
    Attributes, Buffered, MetricId, OnFlush, Prefixed, Sampled, Sampling, WithAttributes,
};
use crate::input::InputKind;
use crate::input::{Input, InputMetric, InputScope};
use crate::metrics;
use crate::name::MetricName;
use crate::pcg32;
use crate::{CachedInput, QueuedInput};
use crate::{Flush, MetricValue};
use std::fmt::Write;

use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::sync::Arc;

#[cfg(not(feature = "parking_lot"))]
use std::sync::{RwLock, RwLockWriteGuard};

#[cfg(feature = "parking_lot")]
use parking_lot::{RwLock, RwLockWriteGuard};
use std::io;

/// Use a safe maximum size for UDP to prevent fragmentation.
// TODO make configurable?
const MAX_UDP_PAYLOAD: usize = 576;

/// Statsd Input holds a datagram (UDP) socket to a statsd server.
/// The socket is shared between scopes opened from the Input.
#[derive(Clone, Debug)]
pub struct Statsd {
    attributes: Attributes,
    socket: Arc<UdpSocket>,
}

impl Statsd {
    /// Send metrics to a statsd server at the address and port provided.
    pub fn send_to<ADDR: ToSocketAddrs>(address: ADDR) -> io::Result<Statsd> {
        let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
        socket.set_nonblocking(true)?;
        socket.connect(address)?;

        Ok(Statsd {
            attributes: Attributes::default(),
            socket,
        })
    }
}

impl Buffered for Statsd {}

impl Sampled for Statsd {}

impl QueuedInput for Statsd {}

impl CachedInput for Statsd {}

impl Input for Statsd {
    type SCOPE = StatsdScope;

    fn metrics(&self) -> Self::SCOPE {
        StatsdScope {
            attributes: self.attributes.clone(),
            buffer: Arc::new(RwLock::new(String::with_capacity(MAX_UDP_PAYLOAD))),
            socket: self.socket.clone(),
        }
    }
}

impl WithAttributes for Statsd {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

/// Statsd Input
#[derive(Debug, Clone)]
pub struct StatsdScope {
    attributes: Attributes,
    buffer: Arc<RwLock<String>>,
    socket: Arc<UdpSocket>,
}

impl Sampled for StatsdScope {}

impl InputScope for StatsdScope {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> InputMetric {
        let mut prefix = self.prefix_prepend(name.clone()).join(".");
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match kind {
            InputKind::Marker | InputKind::Counter => "c",
            InputKind::Gauge | InputKind::Level => "g",
            InputKind::Timer => "ms",
        });

        let scale = match kind {
            // timers are in µs, statsd wants ms
            InputKind::Timer => 1000,
            _ => 1,
        };

        let cloned = self.clone();
        let metric_id = MetricId::forge("statsd", name);

        if let Sampling::Random(float_rate) = self.get_sampling() {
            let _ = writeln!(suffix, "|@{float_rate}");
            let int_sampling_rate = pcg32::to_int_rate(float_rate);
            let metric = StatsdMetric {
                prefix,
                suffix,
                scale,
            };

            InputMetric::new(metric_id, move |value, _labels| {
                if pcg32::accept_sample(int_sampling_rate) {
                    cloned.print(&metric, value)
                }
            })
        } else {
            suffix.push('\n');
            let metric = StatsdMetric {
                prefix,
                suffix,
                scale,
            };
            InputMetric::new(metric_id, move |value, _labels| {
                cloned.print(&metric, value)
            })
        }
    }
}

impl Flush for StatsdScope {
    fn flush(&self) -> io::Result<()> {
        self.notify_flush_listeners();
        let buf = write_lock!(self.buffer);
        self.flush_inner(buf)
    }
}

impl StatsdScope {
    fn print(&self, metric: &StatsdMetric, value: MetricValue) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();
        let entry_len = metric.prefix.len() + value_str.len() + metric.suffix.len();

        let mut buffer = write_lock!(self.buffer);
        if entry_len > buffer.capacity() {
            // TODO report entry too big to fit in buffer (!?)
            return;
        }

        let available = buffer.capacity() - buffer.len();
        if entry_len + 1 > available {
            // buffer is nearly full, make room
            let _ = self.flush_inner(buffer);
            buffer = write_lock!(self.buffer);
        } else {
            if !buffer.is_empty() {
                // separate from previous entry
                buffer.push('\n')
            }
            buffer.push_str(&metric.prefix);
            buffer.push_str(&value_str);
            buffer.push_str(&metric.suffix);
        }

        if !self.is_buffered() {
            if let Err(e) = self.flush_inner(buffer) {
                debug!("Could not send to statsd {e}")
            }
        }
    }

    fn flush_inner(&self, mut buffer: RwLockWriteGuard<String>) -> io::Result<()> {
        if !buffer.is_empty() {
            match self.socket.send(buffer.as_bytes()) {
                Ok(size) => {
                    metrics::STATSD_SENT_BYTES.count(size);
                    trace!("Sent {} bytes to statsd", buffer.len());
                }
                Err(e) => {
                    metrics::STATSD_SEND_ERR.mark();
                    return Err(e);
                }
            };
            buffer.clear();
        }
        Ok(())
    }
}

impl WithAttributes for StatsdScope {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl Buffered for StatsdScope {}

/// Key of a statsd metric.
#[derive(Debug, Clone)]
pub struct StatsdMetric {
    prefix: String,
    suffix: String,
    scale: isize,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for StatsdScope {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Could not flush statsd metrics upon Drop: {err}")
        }
    }
}

// use crate::output::format::LineOp::{ScaledValueAsText, ValueAsText};
//
// impl LineFormat for StatsdScope {
//     fn template(&self, name: &MetricName, kind: InputKind) -> LineTemplate {
//         let mut prefix = name.join(".");
//         prefix.push(':');
//
//         let mut suffix = String::with_capacity(16);
//         suffix.push('|');
//         suffix.push_str(match kind {
//             InputKind::Marker | InputKind::Counter => "c",
//             InputKind::Gauge | InputKind::Level => "g",
//             InputKind::Timer => "ms",
//         });
//
//         // specify sampling rate if any
//         if let Sampling::Random(float_rate) = self.get_sampling() {
//             suffix.push_str(&format! {"|@{}\n", float_rate});
//         }
//
//         // scale timer values
//         let op_value_text = match kind {
//             // timers are in µs, statsd wants ms
//             InputKind::Timer => ScaledValueAsText(1000.0),
//             _ => ValueAsText,
//         };
//
//         LineTemplate::new(vec![
//             LineOp::Literal(prefix.into_bytes()),
//             op_value_text,
//             LineOp::Literal(suffix.into_bytes()),
//             LineOp::NewLine,
//         ])
//     }
// }

#[cfg(feature = "bench")]
mod bench {
    use super::*;
    use crate::attributes::*;
    use crate::input::*;

    #[bench]
    pub fn immediate_statsd(b: &mut test::Bencher) {
        let sd = Statsd::send_to("localhost:2003").unwrap().metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }

    #[bench]
    pub fn buffering_statsd(b: &mut test::Bencher) {
        let sd = Statsd::send_to("localhost:2003")
            .unwrap()
            .buffered(Buffering::BufferSize(65465))
            .metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }
}
