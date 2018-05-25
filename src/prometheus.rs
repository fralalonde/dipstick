
//! Send metrics to a prometheus server.

use core::*;
use output::*;
use error;
use self_metrics::*;

use std::net::ToSocketAddrs;

use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::Write;
use std::fmt::Debug;

use socket::RetrySocket;

metrics!{
    <Aggregate> DIPSTICK_METRICS.with_suffix("prometheus") => {
        Marker SEND_ERR: "send_failed";
        Marker TRESHOLD_EXCEEDED: "bufsize_exceeded";
        Counter SENT_BYTES: "sent_bytes";
    }
}

/// Send metrics to a prometheus server at the address and port provided.
pub fn to_prometheus<ADDR>(address: ADDR) -> error::Result<MetricOutput<Prometheus>>
    where
        ADDR: ToSocketAddrs + Debug + Clone,
{
    debug!("Connecting to prometheus {:?}", address);
    let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));

    Ok(metric_output(
        move |ns, kind, rate| prometheus_metric(ns, kind, rate),
        move || prometheus_scope(&socket, false),
    ))
}

/// Send metrics to a prometheus server at the address and port provided.
pub fn to_buffered_prometheus<ADDR>(address: ADDR) -> error::Result<MetricOutput<Prometheus>>
    where
        ADDR: ToSocketAddrs + Debug + Clone,
{
    debug!("Connecting to prometheus {:?}", address);
    let socket = Arc::new(RwLock::new(RetrySocket::new(address.clone())?));

    Ok(metric_output(
        move |ns, kind, rate| prometheus_metric(ns, kind, rate),
        move || prometheus_scope(&socket, true),
    ))
}

fn prometheus_metric(namespace: &Namespace, kind: Kind, rate: Sampling) -> Prometheus {
    let mut prefix = namespace.join(".");
    prefix.push(' ');

    let mut scale = match kind {
        // timers are in µs, lets give prometheus milliseconds for consistency with statsd
        Kind::Timer => 1000,
        _ => 1,
    };

    if rate < FULL_SAMPLING_RATE {
        // prometheus does not do sampling, so we'll upsample before sending
        let upsample = (1.0 / rate).round() as u64;
        warn!(
            "Metric {:?} '{:?}' being sampled at rate {} will be upsampled \
             by a factor of {} when sent to prometheus.",
            kind, namespace, rate, upsample
        );
        scale *= upsample;
    }

    Prometheus { prefix, scale }
}

fn prometheus_scope(socket: &Arc<RwLock<RetrySocket>>, buffered: bool) -> CommandFn<Prometheus> {
    let buf = ScopeBuffer {
        buffer: Arc::new(RwLock::new(String::new())),
        socket: socket.clone(),
        buffered,
    };
    command_fn(move |cmd| match cmd {
        Command::Write(metric, value) => buf.write(metric, value),
        Command::Flush => buf.flush(),
    })
}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

/// Key of a prometheus metric.
#[derive(Debug, Clone)]
pub struct Prometheus {
    prefix: String,
    scale: u64,
}

/// Wrap string buffer & socket as one.
#[derive(Debug)]
struct ScopeBuffer {
    buffer: Arc<RwLock<String>>,
    socket: Arc<RwLock<RetrySocket>>,
    buffered: bool,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for ScopeBuffer {
    fn drop(&mut self) {
        self.flush()
    }
}

impl ScopeBuffer {
    fn write(&self, metric: &Prometheus, value: Value) {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let start = SystemTime::now();
        match start.duration_since(UNIX_EPOCH) {
            Ok(timestamp) => {
                let mut buf = self.buffer.write().expect("Locking prometheus buffer");

                buf.push_str(&metric.prefix);
                buf.push_str(&value_str);
                buf.push(' ');
                buf.push_str(timestamp.as_secs().to_string().as_ref());
                buf.push('\n');

                if buf.len() > BUFFER_FLUSH_THRESHOLD {
                    TRESHOLD_EXCEEDED.mark();
                    warn!(
                        "Flushing metrics scope buffer to prometheus because its size exceeds \
                         the threshold of {} bytes. ",
                        BUFFER_FLUSH_THRESHOLD
                    );
                    self.flush_inner(&mut buf);
                } else if !self.buffered {
                    self.flush_inner(&mut buf);
                }
            }
            Err(e) => {
                warn!("Could not compute epoch timestamp. {}", e);
            }
        };
    }

    fn flush_inner(&self, buf: &mut String) {
        if !buf.is_empty() {
            let mut sock = self.socket.write().expect("Locking prometheus socket");
            match sock.write(buf.as_bytes()) {
                Ok(size) => {
                    buf.clear();
                    SENT_BYTES.count(size);
                    trace!("Sent {} bytes to prometheus", buf.len());
                }
                Err(e) => {
                    SEND_ERR.mark();
                    // still just a best effort, do not warn! for every failure
                    debug!("Failed to send buffer to prometheus: {}", e);
                }
            };
            buf.clear();
        }
    }

    fn flush(&self) {
        let mut buf = self.buffer.write().expect("Locking prometheus buffer");
        self.flush_inner(&mut buf);
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    pub fn unbufferd_prometheus(b: &mut test::Bencher) {
        let sd = to_prometheus("localhost:8125").unwrap().open_scope();
        let timer = sd.define_metric(&"timer".into(), Kind::Timer, 1000000.0);

        b.iter(|| test::black_box(sd.write(&timer, 2000)));
    }

    #[bench]
    pub fn buffered_prometheus(b: &mut test::Bencher) {
        let sd = to_buffered_prometheus("localhost:8125").unwrap().open_scope();
        let timer = sd.define_metric(&"timer".into(), Kind::Timer, 1000000.0);

        b.iter(|| test::black_box(sd.write(&timer, 2000)));
    }

}
