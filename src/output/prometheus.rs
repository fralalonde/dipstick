//! Prometheus-related functionality.
//! Both push and pull are supported.
//! Both protobuf and text format are supported.

use core::{Flush};
use core::input::{InputKind};
use core::attributes::{Attributes, WithAttributes, Buffered, Buffering, Prefixed};
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::error;
use output::socket::RetrySocket;

use std::net::ToSocketAddrs;
use std::sync::{Arc, RwLock};
use std::fmt::Debug;
use std::io::Write;

use prometheus::{Opts, Registry, IntGauge, IntCounter, Encoder, ProtobufEncoder, TextEncoder,};

metrics!{
}

#[derive(Clone, Debug)]
enum PrometheusEncoding {
    JSON,
    PROTOBUF,
}

/// Prometheus push shared client
/// Holds a shared socket to a Prometheus host.
#[derive(Clone, Debug)]
pub struct Prometheus {
    attributes: Attributes,
    socket: Arc<RwLock<RetrySocket>>,
    encoding: PrometheusEncoding,
}

impl Prometheus {
    /// Send metrics to a prometheus server at the address and port provided.
    pub fn send_json_to<A: ToSocketAddrs + Debug + Clone>(address: A) -> error::Result<Prometheus> {
        Ok(Prometheus {
            attributes: Attributes::default(),
            socket: Arc::new(RwLock::new(RetrySocket::new(address.clone())?)),
            encoding: PrometheusEncoding::JSON,
        })
    }

    /// Send metrics to a prometheus server at the address and port provided.
    pub fn send_protobuf_to<A: ToSocketAddrs + Debug + Clone>(address: A) -> error::Result<Prometheus> {
        Ok(Prometheus {
            attributes: Attributes::default(),
            socket: Arc::new(RwLock::new(RetrySocket::new(address.clone())?)),
            encoding: PrometheusEncoding::PROTOBUF,
        })
    }
}

impl Output for Prometheus {
    type SCOPE = PrometheusScope;

    fn output(&self) -> Self::SCOPE {
        PrometheusScope {
            attributes: self.attributes.clone(),
            registry: Registry::new(),
            socket: self.socket.clone(),
            encoding: self.encoding.clone(),
        }
    }
}

impl WithAttributes for Prometheus {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}

/// Prometheus push client scope
#[derive(Clone)]
pub struct PrometheusScope {
    attributes: Attributes,
    registry: Registry,
    socket: Arc<RwLock<RetrySocket>>,
    encoding: PrometheusEncoding,
}

impl OutputScope for PrometheusScope {
    /// Define a metric of the specified type.
    // TODO enable labels
    fn new_metric(&self, name: MetricName, kind: InputKind) -> OutputMetric {
        let name = self.prefix_prepend(name).join(".");
        match kind {
            InputKind::Counter | InputKind::Timer => {
                let counter = self.register_counter(name);
                OutputMetric::new(move |value, _labels| counter.inc_by(value as i64))
            },
            InputKind::Marker => {
                let counter = self.register_counter(name);
                OutputMetric::new(move |_value, _labels| counter.inc())
            },
            InputKind::Gauge | InputKind::Level => self.new_gauge(name)
        }
    }
}

impl PrometheusScope {

    fn register_counter(&self, name: String) -> IntCounter {
        let opts = Opts::new(name, "".into());
        let counter = IntCounter::with_opts(opts).expect("Prometheus IntCounter");
        self.registry.register(Box::new(counter.clone())).expect("Prometheus IntCounter");
        counter
    }

    fn new_gauge(&self, name: String) -> OutputMetric {
        let opts = Opts::new(name, "".into());
        let gauge = IntGauge::with_opts(opts).expect("Prometheus IntGauge");
        self.registry.register(Box::new(gauge.clone())).expect("Prometheus IntGauge");
        OutputMetric::new(move |value, _labels|
            gauge.add(value as i64)
        )
    }

}

impl Flush for PrometheusScope {

    fn flush(&self) -> error::Result<()> {
        let metric_families = self.registry.gather();
        let mut buffer = vec![];

        match self.encoding {
            PrometheusEncoding::JSON => {
                let encoder = TextEncoder::new();
                encoder.encode(&metric_families, &mut buffer)?
            },
            PrometheusEncoding::PROTOBUF => {
                let encoder = ProtobufEncoder::new();
                encoder.encode(&metric_families, &mut buffer)?
            },
        }

        let mut socket = self.socket.write().expect("Lock Prometheus Socket");
        Ok(socket.write_all(&mut buffer)?)
    }
}

impl WithAttributes for PrometheusScope {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
