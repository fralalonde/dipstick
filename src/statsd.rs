use std::net::UdpSocket;
use std::io::Result;

/// Use a safe maximum size for UDP to prevent fragmentation.
const MAX_UDP_PAYLOAD: usize = 576;

pub const FULL_SAMPLING_RATE: f64 = 1.0;

pub trait SendStats: Sized {
    fn send_stats(&self, str: String);
}

/// Real implementation, send a UDP packet for every stat
impl SendStats for UdpSocket {
    fn send_stats(&self, str: String) {
        match self.send(str.as_bytes()) {
            Ok(_) => {}, // TODO count packets sent for batch reporting
            _ => {}// TODO count send errors for batch reporting
        }
    }
}

/// A client to send application metrics to a statsd server over UDP.
/// Multiple instances may be required if different sampling rates or prefix a required within the same application.
pub struct StatsdOutlet<S: SendStats> {
    sender: S,
    prefix: String,
}

pub type StatsdClient = StatsdOutlet<UdpSocket>;

impl Statsd {
    /// Create a new `StatsdClient` sending packets to the specified `address`.
    /// Sent metric keys will be prepended with `prefix`.
    /// Subsampling is performed according to `float_rate` where
    /// - 1.0 is full sampling and
    /// - 0.0 means _no_ samples will be taken
    /// See crate method `to_int_rate` for more details and a nice table
    pub fn new(address: &str, prefix_str: &str) -> Result<StatsdClient> {
        let udp_socket = UdpSocket::bind("0.0.0.0:0")?; // NB: CLOEXEC by default
        udp_socket.set_nonblocking(true)?;
        udp_socket.connect(address)?;
        StatsdOutlet::outlet(udp_socket, prefix_str, float_rate)
    }
}

impl<S: SendStats> ChannelOutput for StatsdOutlet<S> {

    /// Create a new `StatsdClient` sending packets to the specified `address`.
    /// Sent metric keys will be prepended with `prefix`.
    /// Subsampling is performed according to `float_rate` where
    /// - 1.0 is full sampling and
    /// - 0.0 means _no_ samples will be taken
    /// See crate method `to_int_rate` for more details and a nice table
    fn outlet(sender: S, prefix_str: &str, float_rate: f64) -> Result<StatsdOutlet<S>> {
        assert!(float_rate <= 1.0 && float_rate >= 0.0);
        let prefix = prefix_str.to_string();
        let rate_suffix = if float_rate < 1.0 { format!("|@{}", float_rate)} else { "".to_string() };
        Ok(StatsdOutlet {
            sender,
            prefix,
//            time_suffix: format!("|ms{}", rate_suffix),
//            gauge_suffix: format!("|g{}", rate_suffix),
//            count_suffix: format!("|c{}", rate_suffix)
        })
    }

    /// Report to statsd a count of items.
    pub fn count(&self, key: &str, value: u64) {
        if accept_sample(self.int_rate)  {
            let count = &value.to_string();
            self.send( &[key, ":", count, &self.count_suffix] )
        }
    }

    /// Report to statsd a non-cumulative (instant) count of items.
    pub fn gauge(&self, key: &str, value: u64) {
        if accept_sample(self.int_rate)  {
            let count = &value.to_string();
            self.send( &[key, ":", count, &self.gauge_suffix] )
        }
    }

    /// Report to statsd a time interval of items.
    pub fn time_interval_ms(&self, key: &str, interval_ms: u64) {
        if accept_sample(self.int_rate)  {
            self.send_time_ms(key, interval_ms);
        }
    }

    /// Query current time to use eventually with `stop_time()`
    pub fn start_time(&self) -> StartTime {
        StartTime( time::precise_time_ns() )
    }

    /// An efficient timer that skips querying for stop time if sample will not be collected.
    /// Caveat : Random sampling overhead of a few ns will be included in any reported time interval.
    pub fn stop_time(&self, key: &str, start_time: StartTime) {
        if accept_sample(self.int_rate)  {
            self.send_time_ms(key, start_time.elapsed_ms());
        }
    }

    fn send_time_ms(&self, key: &str, interval_ms: u64) {
        let value = &interval_ms.to_string();
        self.send( &[key, ":", value, &self.time_suffix] )
    }

    /// Concatenate text parts into a single buffer and send it over UDP
    fn send(&self, strings: &[&str]) {
        let mut str = String::with_capacity(MAX_UDP_PAYLOAD);
        str.push_str(&self.prefix);
        for s in strings { str.push_str(s); }
        self.sender.send_stats(str)
    }

}

struct StatsdMetric {
    port: Statsd,
    prefix: String,
}

impl Event for StatsdMetric {
    fn mark(&self) {
        // static string "barry:1|c|@0.999"
        port.send(prefix)
    }
}

impl Value for StatsdMetric {
    fn value(&self, value: ValueType) {
        // insert value between prefix and suffix "barry:44|ms|@0.999"
        port.send(format!("{}:{}|{}", prefix, value, suffix))
    }
}

impl Scope for StatsdMetric {
    fn open_scope(&self) -> OpenedScope {
        // static string "barry:1|c|@0.999"
        port.open_scope()
    }
}

impl ChannelOut for Statsd {
    fn new_value<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Value {
        let mut type_string = "|c".to_string();
        if sampling < 1.0 {
            type_string.push(format!("|{}", sampling));
        }
        StatsdMetric {
            port: self,
            name_string: name,
            type_string: type_string
        }
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Gauge {

    }

    fn new_timer<S: AsRef<str>>(&self, name: S, sampling: RateType) -> Timer {

    }
}
