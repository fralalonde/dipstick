use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};

//////////// Log Channel

pub struct LogMetric {
    prefix: String
}

impl DefinedMetric for LogMetric {}

pub struct LogWrite {}

impl MetricWrite<LogMetric> for LogWrite {
    fn write(&self, metric: &LogMetric, value: Value) {
        // TODO format faster
        info!("{}:{}", metric.prefix, value)
    }
}

pub struct LogChannel {
    prefix: String,
    write: LogWrite
}

impl LogChannel {
    pub fn new<S: AsRef<str>>(prefix: S) -> LogChannel {
        let prefix = prefix.as_ref().to_string();
        LogChannel { prefix, write: LogWrite {}}
    }
}

impl MetricChannel for LogChannel {
    type Metric = LogMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> LogMetric {
        LogMetric { prefix: format!("{:?}:{}{}", m_type, self.prefix, name.as_ref())}
    }

    type Write = LogWrite;

    fn write<F>(&self, metrics: F ) where F: Fn(&Self::Write) {
        metrics(&self.write)
    }

}
