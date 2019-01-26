//! Send metrics to a Prometheus server.

use core::attributes::{Buffered, Attributes, WithAttributes, Prefixed};
use core::name::MetricName;
use core::{Flush, MetricValue};
use core::input::InputKind;
use core::metrics;
use core::output::{Output, OutputScope, OutputMetric};
use core::error;
use queue::queue_out;
use cache::cache_out;
use core::label::Labels;

use std::rc::Rc;
use std::cell::{RefCell, RefMut};

/// Prometheus output holds a socket to a Prometheus server.
/// The socket is shared between scopes opened from the output.
#[derive(Clone, Debug)]
pub struct Prometheus {
    attributes: Attributes,
    push_url: String,
}

impl Output for Prometheus {
    type SCOPE = PrometheusScope;

    fn new_scope(&self) -> Self::SCOPE {
        PrometheusScope {
            attributes: self.attributes.clone(),
            buffer: Rc::new(RefCell::new(String::new())),
            push_url: self.push_url.clone(),
        }
    }
}

impl Prometheus {
    /// Send metrics to a Prometheus "push gateway" at the URL provided.
    /// URL path must include group identifier labels `job`
    /// as shown in https://github.com/prometheus/pushgateway#command-line
    /// For example `http://pushgateway.example.org:9091/metrics/job/some_job`
    pub fn push_to(url: &str) -> error::Result<Prometheus> {
        debug!("Pushing to Prometheus {:?}", url);

        Ok(Prometheus {
            attributes: Attributes::default(),
            push_url: url.to_string(),
        })
    }
}

impl WithAttributes for Prometheus {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for Prometheus {}

/// Prometheus Input
#[derive(Debug, Clone)]
pub struct PrometheusScope {
    attributes: Attributes,
    buffer: Rc<RefCell<String>>,
    push_url: String,
}

impl OutputScope for PrometheusScope {
    /// Define a metric of the specified type.
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let prefix = self.prefix_prepend(name).join("_");

        let scale = match kind {
            // timers are in µs, but we give Prometheus milliseconds
            InputKind::Timer => 1000,
            _ => 1,
        };

        let cloned = self.clone();
        let metric = PrometheusMetric { prefix, scale };

        OutputMetric::new(move |value, labels| {
            cloned.print(&metric, value, labels);
        })
    }
}

impl Flush for PrometheusScope {

    fn flush(&self) -> error::Result<()> {
        let buf = self.buffer.borrow_mut();
        self.flush_inner(buf)
    }
}

impl PrometheusScope {
    fn print(&self, metric: &PrometheusMetric, value: MetricValue, labels: Labels)  {
        let scaled_value = value / metric.scale;
        let value_str = scaled_value.to_string();

        let mut strbuf = String::new();
        // prometheus format be like `http_requests_total{method="post",code="200"} 1027 1395066363000`
        strbuf.push_str(&metric.prefix);

        let labels_map = labels.into_map();
        if !labels_map.is_empty() {
            strbuf.push('{');
            let mut i = labels_map.into_iter();
            let mut next = i.next();
            while let Some((k, v)) = next {
                strbuf.push_str(&k);
                strbuf.push_str("=\"");
                strbuf.push_str(&v);
                next = i.next();
                if next.is_some() {
                    strbuf.push_str("\",");
                } else {
                    strbuf.push('"');
                }
            }
            strbuf.push_str("} ");
        } else {
            strbuf.push(' ');
        }
        strbuf.push_str(&value_str);

        let buffer = self.buffer.borrow_mut();
        if strbuf.len() + buffer.len() > BUFFER_FLUSH_THRESHOLD {
            metrics::PROMETHEUS_OVERFLOW.mark();
            warn!("Prometheus Buffer Size Exceeded: {}", BUFFER_FLUSH_THRESHOLD);
            let _ = self.flush_inner(buffer);
        } else {
            if !self.is_buffered() {
                if let Err(e) = self.flush_inner(buffer) {
                    debug!("Could not send to Prometheus {}", e)
                }
            }
        }
    }

    fn flush_inner(&self, mut buf: RefMut<String>) -> error::Result<()> {
        if buf.is_empty() { return Ok(()) }

        match minreq::get(self.push_url.as_ref()).with_body(buf.as_ref()).send() {
            Ok(_res) => {
                metrics::PROMETHEUS_SENT_BYTES.count(buf.len());
                trace!("Sent {} bytes to Prometheus", buf.len());
                buf.clear();
                Ok(())
            }
            Err(e) => {
                metrics::PROMETHEUS_SEND_ERR.mark();
                debug!("Failed to send buffer to Prometheus: {}", e);
                Err(e.into())
            }
        }
    }
}

impl WithAttributes for PrometheusScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

impl Buffered for PrometheusScope {}

impl queue_out::QueuedOutput for Prometheus {}
impl cache_out::CachedOutput for Prometheus {}

/// Its hard to see how a single scope could get more metrics than this.
// TODO make configurable?
const BUFFER_FLUSH_THRESHOLD: usize = 65_536;

/// Key of a Prometheus metric.
#[derive(Debug, Clone)]
pub struct PrometheusMetric {
    prefix: String,
    scale: isize,
}

/// Any remaining buffered data is flushed on Drop.
impl Drop for PrometheusScope {
    fn drop(&mut self) {
        if let Err(err) = self.flush() {
            warn!("Could not flush Prometheus metrics upon Drop: {}", err)
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use core::attributes::*;
    use core::input::*;
    use super::*;
    use test;

    #[bench]
    pub fn immediate_prometheus(b: &mut test::Bencher) {
        let sd = Prometheus::push_to("localhost:2003").unwrap().metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }

    #[bench]
    pub fn buffering_prometheus(b: &mut test::Bencher) {
        let sd = Prometheus::push_to("localhost:2003").unwrap()
            .buffered(Buffering::BufferSize(65465)).metrics();
        let timer = sd.new_metric("timer".into(), InputKind::Timer);

        b.iter(|| test::black_box(timer.write(2000, labels![])));
    }

}
