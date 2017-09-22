//! Write metrics to stdout

use core::*;

/// Print metrics to stdout.
pub fn stdout<STR>(prefix: STR) -> StdoutSink where STR: AsRef<str> {
    StdoutSink::new(prefix)
}

#[derive(Debug)]
pub struct StdoutKey {
    prefix: String,
}

/// Write metrics to log
pub struct StdoutWriter {}

impl Writer<StdoutKey> for StdoutWriter {
    fn write(&self, metric: &StdoutKey, value: Value) {
        // TODO format faster
        println!("{}:{}", metric.prefix, value)
    }
}

/// Write metrics to stdout with a prefix
pub struct StdoutSink {
    prefix: String,
    write: StdoutWriter,
}

impl StdoutSink {
    /// Create a new stdout sink.
    pub fn new<STR>(prefix: STR) -> StdoutSink
        where STR: AsRef<str>
    {
        let prefix = prefix.as_ref().to_string();
        StdoutSink {
            prefix,
            write: StdoutWriter {},
        }
    }
}

impl Sink<StdoutKey, StdoutWriter> for StdoutSink {

    #[allow(unused_variables)]
    fn new_metric<STR>(&self, kind: MetricKind, name: STR, sampling: Rate) -> StdoutKey
        where STR: AsRef<str>
    {
        StdoutKey { prefix: format!("{:?}:{}{}", kind, self.prefix, name.as_ref()) }
    }

    fn new_writer(&self) -> StdoutWriter {
        StdoutWriter {}
    }
}
