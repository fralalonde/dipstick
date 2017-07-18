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
        println!("LOG TAGS {} | Value {}", metric.prefix, value)
    }
}

pub struct LogChannel {
    write: LogWrite
}

impl LogChannel {
    pub fn new() -> LogChannel {
        LogChannel { write: LogWrite {}}
    }
}

impl MetricChannel for LogChannel {
    type Metric = LogMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> LogMetric {
        LogMetric { prefix: format!("Type {:?} | Name {} | Sample {}", m_type, name.as_ref(), sample)}
    }

    type Write = LogWrite;

    fn write<F>(&self, metrics: F ) where F: Fn(&Self::Write) {
        metrics(&self.write)
    }

}
