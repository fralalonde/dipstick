//! Prometheus-related functionality.
//! Both push and pull are supported.
//! Both protobuf and text format are supported.
//! - Send metrics to a prometheus aggregator agent.
//! - Serve metrics with basic HTTP server
//! - Print metrics to a buffer provided by an HTTP framework.

use core::{Flush, MetricValue};
use core::input::{InputKind, Input, InputScope, InputMetric};
use core::attributes::{Attributes, WithAttributes, Buffered, Buffering, Prefixed};
use core::name::MetricName;
use core::output::{Output, OutputMetric, OutputScope};
use core::error;

use std::net::ToSocketAddrs;
use std::sync::Arc;

#[cfg(feature="proto")]
use prometheus_proto as proto;

metrics!{
}

/// Prometheus push shared client
pub struct Prometheus {
}

impl Output for Prometheus {
    type SCOPE = PrometheusScope;

    fn output(&self) -> Arc<Input + Send + Sync + 'static> {
        PrometheusScope {}
    }
}

/// Prometheus push client scope
pub struct PrometheusScope {
}

impl PrometheusScope {
    /// Send metrics to a prometheus server at the address and port provided.
    pub fn output<ADDR: ToSocketAddrs>(_address: ADDR) -> error::Result<Prometheus> {
        Ok(Prometheus {})
    }
}

impl OutputScope for PrometheusScope {

    /// Define a metric of the specified type.
    fn new_metric(&self, name: MetricName, _kind: InputKind) -> OutputMetric {
        let mut _prefix = self.prefix_prepend(name).join(".");
        OutputMetric::new(|_value, _labels| {})
    }
}

impl Flush for PrometheusScope {

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

impl WithAttributes for Prometheus {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
}
