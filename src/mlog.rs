use core::{MetricType, Rate, Value, SinkWriter, SinkMetric, MetricSink};

//////////// Log Channel

#[derive(Debug)]
pub struct LogMetric {
    prefix: String
}

impl SinkMetric for LogMetric {}

#[derive(Debug)]
pub struct LogWriter {}

impl SinkWriter<LogMetric> for LogWriter {
    fn write(&self, metric: &LogMetric, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

#[derive(Debug)]
pub struct LogSink {
    prefix: String,
    write: LogWriter
}

impl LogSink {
    pub fn new<S: AsRef<str>>(prefix: S) -> LogSink {
        let prefix = prefix.as_ref().to_string();
        LogSink { prefix, write: LogWriter {}}
    }
}

impl MetricSink for LogSink {
    type Metric = LogMetric;
    type Writer = LogWriter;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> LogMetric {
        LogMetric { prefix: format!("{:?}:{}{}", m_type, self.prefix, name.as_ref())}
    }

    fn new_writer(&self) -> LogWriter {
        LogWriter {}
    }

}
