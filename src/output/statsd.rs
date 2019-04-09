//! Send metrics to a statsd server.

use cache::cache_out;
use core::attributes::{
    Attributes, Buffered, OnFlush, Prefixed, Sampled, Sampling, WithAttributes,
};
use core::error;
use core::input::InputKind;
use core::metrics;
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::pcg32;
use core::{Flush, MetricValue};
use queue::queue_out;

use std::cell::{RefCell, RefMut};
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::rc::Rc;
use std::sync::Arc;

/// Use a safe maximum size for UDP to prevent fragmentation.
// TODO make configurable?
const MAX_UDP_PAYLOAD: usize = 576;

/// Statsd output holds a datagram (UDP) socket to a statsd server.
/// The socket is shared between scopes opened from the output.
#[derive(Clone, Debug)]
pub struct Statsd {
    attributes: Attributes,
    socket: Arc<UdpSocket>,
}

impl Statsd {
    /// Send metrics to a statsd server at the address and port provided.
    pub fn send_to<ADDR: ToSocketAddrs>(address: ADDR) -> error::Result<Statsd> {
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

impl queue_out::QueuedOutput for Statsd {}
impl cache_out::CachedOutput for Statsd {}

impl Output for Statsd {
    type SCOPE = StatsdScope;

    fn new_scope(&self) -> Self::SCOPE {
        StatsdScope {
            attributes: self.attributes.clone(),
            buffer: Rc::new(RefCell::new(String::with_capacity(MAX_UDP_PAYLOAD))),
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
    buffer: Rc<RefCell<String>>,
    socket: Arc<UdpSocket>,
}

impl Sampled for StatsdScope {}

impl OutputScope for StatsdScope {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let mut prefix = self.prefix_prepend(name).join(".");
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

        if let Sampling::Random(float_rate) = self.get_sampling() {
            suffix.push_str(&format! {"|@{}\n", float_rate});
            let int_sampling_rate = pcg32::to_int_rate(float_rate);
            let metric = StatsdMetric {
                prefix,
                suffix,
                scale,
            };

            OutputMetric::new(move |value, _labels| {
                if pcg32::accept_sample(int_sampling_rate) {
                    cloned.print(&metric, value)
                }
            })
        } else {
            suffix.push_str("\n");
            let metric = StatsdMetric {
                prefix,
                suffix,
                scale,
            };
            OutputMetric::new(move |value, _labels| cloned.print(&metric, value))
        }
    }
}

impl Flush for StatsdScope {
    fn flush(&self) -> error::Result<()> {
        self.notify_flush_listeners();
        let buf = self.buffer.borrow_mut();
        self.flush_inner(buf)
    }
}

impl StatsdScope {
    fn print(&self, metric: &StatsdMetric, value: MetricValue) {
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

        if !self.is_buffered() {
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
                    return Err(e.into());
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
            warn!("Could not flush statsd metrics upon Drop: {}", err)
        }
    }
}

// TODO use templates for statsd format
//lazy_static!({
//    static ref STATSD_FORMAT: StatsdFormat = StatsdFormat;
//});
//
//#[derive(Default)]
//pub struct StatsdFormat;
//
//impl Format for StatsdFormat {
//    fn template(&self, name: &Name, kind: InputKind) -> Template {
//        let mut before_value = name.join(".");
//        before_value.push(':');
//
//        let mut after_value = String::with_capacity(16);
//        after_value.push('|');
//        after_value.push_str(match kind {
//            InputKind::Marker | InputKind::Counter => "c",
//            InputKind::Gauge => "g",
//            InputKind::Timer => "ms",
//        });
//
//        // specify sampling rate if any
//        if let Some(Sampling::Random(float_rate)) = self.get_sampling() {
//            suffix.push_str(&format! {"|@{}\n", float_rate});
//        }
//
//        // scale timer values
//        let value_text = match kind {
//            // timers are in µs, statsd wants ms
//            InputKind::Timer => ScaledValueAsText(1000),
//            _ => ValueAsText,
//        };
//
//        Template {
//            commands: vec![
//                StringLit(before_value),
//                value_text,
//                StringLit(after_value),
//                NewLine,
//            ]
//        }
//    }
//}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use core::attributes::*;
    use core::input::*;
    use test;

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
