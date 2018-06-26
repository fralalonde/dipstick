//! Prometheus-related functionality.
//! Both push and pull are supported.
//! Both protobuf and text format are supported.
//! - Send metrics to a prometheus aggregator agent.
//! - Serve metrics with basic HTTP server
//! - Print metrics to a buffer provided by an HTTP framework.

use core::*;
use error;

use std::net::ToSocketAddrs;

#[cfg(feature="proto")]
use prometheus_proto as proto;

metrics!{
}

/// Prometheus push shared client
pub struct PrometheusOutput {
}

impl RawOutput for PrometheusOutput {
    type SCOPE = Prometheus;
    fn open_scope_raw(&self) -> Self::SCOPE {
        Prometheus {}
    }
}

/// Prometheus push client scope
pub struct Prometheus {
}

impl Prometheus {
    /// Send metrics to a prometheus server at the address and port provided.
    pub fn output<ADDR: ToSocketAddrs>(_address: ADDR) -> error::Result<PrometheusOutput> {
        Ok(PrometheusOutput{})
    }
}

impl RawScope for Prometheus {

    /// Define a metric of the specified type.
    fn new_metric_raw(&self, _name: Name, _kind: Kind) -> RawMetric {
        RawMetric::new(|_value| {})
    }
}

impl Flush for Prometheus {

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}
