//! Send metrics to a prometheus server.
// TODO impl this

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
    <Aggregate> DIPSTICK_METRICS.add_prefix("prometheus") => {
        Marker SEND_ERR: "send_failed";
        Marker TRESHOLD_EXCEEDED: "bufsize_exceeded";
        Counter SENT_BYTES: "sent_bytes";
    }
}

/// Send metrics to a prometheus server at the address and port provided.
pub fn output_prometheus<ADDR>(address: ADDR) -> error::Result<MetricOutput<Prometheus>>
    where
        ADDR: ToSocketAddrs + Debug + Clone,
{
}

