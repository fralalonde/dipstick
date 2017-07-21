use core::{MetricType, RateType, Value, MetricWriter, SinkMetric, MetricSink};

//////////// Log Channel

pub struct LogMetric {
    prefix: String
}

impl SinkMetric for LogMetric {}

pub struct LogWriter {}

impl MetricWriter<LogMetric> for LogWriter {
    fn write(&self, metric: &LogMetric, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

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

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> LogMetric {
        LogMetric { prefix: format!("{:?}:{}{}", m_type, self.prefix, name.as_ref())}
    }

    type Write = LogWriter;

    fn write<F>(&self, metrics: F ) where F: Fn(&Self::Write) {
        metrics(&self.write)
    }

}
