//! Write metrics to log

use ::*;

#[derive(Debug)]
/// Write metrics to log
pub struct LoggingKey {
    prefix: String,
}

impl Metric for LoggingKey {}

#[derive(Debug, Copy, Clone)]
/// Write metrics to log
pub struct LoggingWriter {}

impl Writer<LoggingKey> for LoggingWriter {
    fn write(&self, metric: &LoggingKey, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

#[derive(Debug)]
/// Write metrics to the standard log with a prefix
pub struct LoggingSink {
    prefix: String,
    write: LoggingWriter,
}

impl LoggingSink {
    /// Create a new logging sink.
    pub fn new<S: AsRef<str>>(prefix: S) -> LoggingSink {
        let prefix = prefix.as_ref().to_string();
        LoggingSink {
            prefix,
            write: LoggingWriter {},
        }
    }
}

impl Sink for LoggingSink {
    type Metric = LoggingKey;
    type Writer = LoggingWriter;

    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate)
                                 -> Self::Metric {
        LoggingKey { prefix: format!("{:?}:{}{}", kind, self.prefix, name.as_ref()) }
    }

    fn new_writer(&self) -> Self::Writer {
        LoggingWriter {}
    }
}
