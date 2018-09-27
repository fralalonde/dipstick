//! Prometheus-related functionality.
//! Both push and pull are supported.
//! Both protobuf and text format are supported.
//! - Send metrics to a prometheus aggregator agent.
//! - Serve metrics with basic HTTP server
//! - Print metrics to a buffer provided by an HTTP framework.

use core::{Flush, Value};
use core::input::{Kind, Input, InputScope, InputMetric};
use core::component::{Attributes, WithAttributes, Buffered, Buffering, Naming};
use core::name::Name;
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
    fn new_metric(&self, _name: Name, _kind: Kind) -> OutputMetric {
        OutputMetric::new(|_value| {})
    }
}

impl Flush for PrometheusScope {

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}
