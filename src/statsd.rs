//! Send metrics to a statsd server.

use core::{Input, Output, Value, Metric, Attributes, WithAttributes, Kind,
           Name, WithSamplingRate, WithName, WithBuffering, Sampling, Cache, Async};
use pcg32;
use error;
use metrics;

use std::net::UdpSocket;
use std::sync::{Arc, RwLock};

pub use std::net::ToSocketAddrs;

/// Send metrics to a statsd server at the address and port provided.
pub fn output_statsd<ADDR: ToSocketAddrs>(address: ADDR) -> error::Result<StatsdOutput> {
    let socket = Arc::new(UdpSocket::bind("0.0.0.0:0")?);
    socket.set_nonblocking(true)?;
    socket.connect(address)?;

    Ok(StatsdOutput {
        attributes: Attributes::default(),
        socket,
    })
}

/// Statsd output holds a UDP client socket to a statsd host.
/// The output's connection is shared between all inputs originating from it.
#[derive(Debug, Clone)]
pub struct StatsdOutput {
    attributes: Attributes,
    socket: Arc<UdpSocket>,
}

impl Output for StatsdOutput {
    type INPUT = Statsd;
    fn new_input(&self) -> Self::INPUT {
        Statsd {
            attributes: self.attributes.clone(),
            buffer: Arc::new(RwLock::new(InputBuffer {
                buffer: String::with_capacity(MAX_UDP_PAYLOAD),
                socket: self.socket.clone(),
                buffering: self.is_buffering(),
            })),
        }
    }
}

impl WithAttributes for StatsdOutput {
    fn get_attributes(&self) -> &Attributes {
        &self.attributes
    }
    fn mut_attributes(&mut self) -> &mut Attributes {
        &mut self.attributes
    }
}

impl WithBuffering for StatsdOutput {}
impl WithSamplingRate for StatsdOutput {}

impl Cache for StatsdOutput {}
impl Async for StatsdOutput {}

/// Metrics input for statsd.
#[derive(Clone)]
pub struct Statsd {
    attributes: Attributes,
    buffer: Arc<RwLock<InputBuffer>>,
}

impl Input for Statsd {
    fn new_metric(&self, name: Name, kind: Kind) -> Metric {
        let mut prefix = self.qualified_name(name).join(".");
        prefix.push(':');

        let mut suffix = String::with_capacity(16);
        suffix.push('|');
        suffix.push_str(match kind {
            Kind::Marker | Kind::Counter => "c",
            Kind::Gauge => "g",
            Kind::Timer => "ms",
        });

        let buffer = self.buffer.clone();
        let scale = match kind {
            // timers are in µs, statsd wants ms
            Kind::Timer => 1000,
            _ => 1,
        };

        if let Sampling::SampleRate(float_rate) = self.get_sampling() {
            suffix.push_str(&format!{"|@{}", float_rate});
            let int_sampling_rate = pcg32::to_int_rate(float_rate);

            Metric::new(move |value| {
                if pcg32::accept_sample(int_sampling_rate) {
                    let mut buffer = buffer.write().expect("InputBuffer");
                    buffer.write(&prefix, &suffix, scale, value)
                }
            })
        } else {
            Metric::new(move |value| {
                let mut buffer = buffer.write().expect("InputBuffer");
                buffer.write(&prefix, &suffix, scale, value)
            })
        }
    }

    fn flush(&self) -> error::Result<()> {
        let mut buffer = self.buffer.write().expect("InputBuffer");
        Ok(buffer.flush()?)
    }
}

impl WithSamplingRate for Statsd {}

impl WithAttributes for Statsd {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Use a safe maximum size for UDP to prevent fragmentation.
// TODO make configurable?
const MAX_UDP_PAYLOAD: usize = 576;

/// Wrapped string buffer & socket as one.
#[derive(Debug)]
struct InputBuffer {
    buffer: String,
    socket: Arc<UdpSocket>,
    buffering: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for InputBuffer {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Couldn't flush statsd buffer on Drop: {}", err)
        }
    }
}

impl InputBuffer {
    fn write(&mut self, prefix: &str, suffix: &str, scale: u64, value: Value) {
        let scaled_value = value / scale;
        let value_str = scaled_value.to_string();
        let entry_len = prefix.len() + value_str.len() + suffix.len();

        if entry_len > self.buffer.capacity() {
            // TODO report entry too big to fit in buffer (!?)
            return;
        }

        let remaining = self.buffer.capacity() - self.buffer.len();
        if entry_len + 1 > remaining {
            // buffer is full, flush before appending
            let _ = self.flush();
        } else {
            if !self.buffer.is_empty() {
                // separate from previous entry
                self.buffer.push('\n')
            }
            self.buffer.push_str(prefix);
            self.buffer.push_str(&value_str);
            self.buffer.push_str(suffix);
        }
        if self.buffering {
            let _ = self.flush();
        }
    }

    fn flush(&mut self) -> error::Result<()> {
        if !self.buffer.is_empty() {
            match self.socket.send(self.buffer.as_bytes()) {
                Ok(size) => {
                    metrics::STATSD_SENT_BYTES.count(size);
                    trace!("Sent {} bytes to statsd", self.buffer.len());
                }
                Err(e) => {
                    metrics::STATSD_SEND_ERR.mark();
                    return Err(e.into())
                }
            };
            self.buffer.clear();
        }
        Ok(())
    }
}

#[cfg(feature = "bench")]
mod bench {

    use core::*;
    use super::*;
    use test;

    #[bench]
    pub fn timer_statsd(b: &mut test::Bencher) {
        let sd = output_statsd("localhost:8125").unwrap().new_input_dyn();
        let timer = sd.new_metric("timer".into(), Kind::Timer);

        b.iter(|| test::black_box(timer.write(2000)));
    }

}
