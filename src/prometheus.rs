//! Send metrics to a prometheus server.
// TODO impl this

use core::*;
use error;

use std::net::ToSocketAddrs;

use socket::RetrySocket;

use prometheus_proto as proto;

metrics!{
}

pub struct PrometheusOutput {

}

impl RawOutput for PrometheusOutput {
    type INPUT = Prometheus;
    fn new_raw_input(&self) -> Self::INPUT {
        Prometheus {}
    }
}

pub struct Prometheus {
}

impl RawInput for Prometheus {

    /// Define a metric of the specified type.
    fn new_metric_raw(&self, name: Name, kind: Kind) -> RawMetric {
        RawMetric::new(|_value| {})
    }
}

impl Flush for Prometheus {

    /// Flush does nothing by default.
    fn flush(&self) -> error::Result<()> {
        Ok(())
    }
}

/// Send metrics to a prometheus server at the address and port provided.
pub fn output_prometheus<ADDR>(address: ADDR) -> error::Result<PrometheusOutput>
{
    Ok(PrometheusOutput{})
}

//mod shit {
//    include! {concat!{env!{"RUST_GEN_SRC"}, "/prometheus.rs"}}
//}