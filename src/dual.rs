use core::{MetricType, RateType, Value, SinkWriter, SinkMetric, MetricSink};

pub struct DualMetric<M1: SinkMetric, M2: SinkMetric> {
    metric_1: M1,
    metric_2: M2,
}

impl <M1: SinkMetric, M2: SinkMetric> SinkMetric for DualMetric<M1, M2> {}

pub struct DualWriter<C1: MetricSink, C2: MetricSink> {
    channel_a: C1,
    channel_b: C2,
}

impl <C1: MetricSink, C2: MetricSink> SinkWriter<DualMetric<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>> for DualWriter<C1, C2> {
    fn write(&self, metric: &DualMetric<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>, value: Value) {
        self.channel_a.write(|scope| scope.write(&metric.metric_1, value));
        self.channel_b.write(|scope| scope.write(&metric.metric_2, value));
    }
}

pub struct DualSink<C1: MetricSink, C2: MetricSink> {
    write: DualWriter<C1, C2>
}

impl <C1: MetricSink, C2: MetricSink> DualSink<C1, C2> {
    pub fn new(channel_a: C1, channel_b: C2) -> DualSink<C1, C2> {
        DualSink { write: DualWriter {channel_a, channel_b}}
    }
}

impl <C1: MetricSink, C2: MetricSink> MetricSink for DualSink<C1, C2> {
    type Metric = DualMetric<C1::Metric, C2::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> DualMetric<C1::Metric, C2::Metric> {
        let metric_1 = self.write.channel_a.define(m_type, &name, sample);
        let metric_2 = self.write.channel_b.define(m_type, &name, sample);
        DualMetric { metric_1, metric_2  }
    }

    type Write = DualWriter<C1, C2>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

