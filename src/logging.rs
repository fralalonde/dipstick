use ::*;

#[derive(Debug)]
pub struct LoggingKey {
    prefix: String,
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
    write: LoggingWriter,
}

impl LoggingSink {
    pub fn new<S: AsRef<str>>(prefix: S) -> LoggingSink {
        let prefix = prefix.as_ref().to_string();
        LoggingSink {
            prefix,
            write: LoggingWriter {},
        }
    }
}

impl MetricSink for LoggingSink {
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
