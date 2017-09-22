//! Write metrics to log

use core::*;

/// Send metric to a logger.
/// This uses the basic log crate as it is configured for the application.
pub fn log<STR>(log: STR) -> LoggingSink where STR: AsRef<str> {
    LoggingSink::new(log)
}

#[derive(Debug)]
/// Write metrics to log
pub struct LoggingKey {
    prefix: String,
}

/// Write metrics to log
pub struct LoggingWriter {}

impl Writer<LoggingKey> for LoggingWriter {
    fn write(&self, metric: &LoggingKey, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

/// Write metrics to the standard log with a prefix
pub struct LoggingSink {
    prefix: String,
    write: LoggingWriter,
}

impl LoggingSink {
    /// Create a new logging sink.
    pub fn new<STR>(prefix: STR) -> LoggingSink
        where STR: AsRef<str>
    {
        let prefix = prefix.as_ref().to_string();
        LoggingSink {
            prefix,
            write: LoggingWriter {},
        }
    }
}

impl Sink<LoggingKey, LoggingWriter> for LoggingSink {

    #[allow(unused_variables)]
    fn new_metric<STR>(&self, kind: MetricKind, name: STR, sampling: Rate) -> LoggingKey
        where STR: AsRef<str>
    {
        LoggingKey { prefix: format!("{:?}:{}{}", kind, self.prefix, name.as_ref()) }
    }

    fn new_writer(&self) -> LoggingWriter {
        LoggingWriter {}
    }
}
