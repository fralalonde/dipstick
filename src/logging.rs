use core::{MetricType, Rate, Value, MetricWriter, MetricKey, MetricSink};

//////////// Log Channel

#[derive(Debug)]
pub struct LoggingKey {
    prefix: String
}

impl MetricKey for LoggingKey {}

#[derive(Debug)]
pub struct LoggingWriter {}

impl MetricWriter<LoggingKey> for LoggingWriter {
    fn write(&self, metric: &LoggingKey, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

#[derive(Debug)]
pub struct LoggingSink {
    prefix: String,
    write: LoggingWriter
}

impl LoggingSink {
    pub fn new<S: AsRef<str>>(prefix: S) -> LoggingSink {
        let prefix = prefix.as_ref().to_string();
        LoggingSink { prefix, write: LoggingWriter {}}
    }
}

impl MetricSink for LoggingSink {
    type Metric = LoggingKey;
    type Writer = LoggingWriter;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> LoggingKey {
        LoggingKey { prefix: format!("{:?}:{}{}", m_type, self.prefix, name.as_ref())}
    }

    fn new_writer(&self) -> LoggingWriter {
        LoggingWriter {}
    }

}
