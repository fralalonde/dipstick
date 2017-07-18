use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};


////////////

pub struct DualMetric<M1: DefinedMetric, M2: DefinedMetric> {
    metric_1: M1,
    metric_2: M2,
}

impl <M1: DefinedMetric, M2: DefinedMetric> DefinedMetric for DualMetric<M1, M2> {}

pub struct DualWrite<C1: MetricChannel, C2: MetricChannel> {
    channel_a: C1,
    channel_b: C2,
}

impl <C1: MetricChannel, C2: MetricChannel> MetricWrite<DualMetric<<C1 as MetricChannel>::Metric, <C2 as MetricChannel>::Metric>> for DualWrite<C1, C2> {
    fn write(&self, metric: &DualMetric<<C1 as MetricChannel>::Metric, <C2 as MetricChannel>::Metric>, value: Value) {
        println!("Channel A");
        self.channel_a.write(|scope| scope.write(&metric.metric_1, value));
        println!("Channel B");
        self.channel_b.write(|scope| scope.write(&metric.metric_2, value));
    }
}

pub struct DualChannel<C1: MetricChannel, C2: MetricChannel> {
    write: DualWrite<C1, C2>
}

impl <C1: MetricChannel, C2: MetricChannel> DualChannel<C1, C2> {
    pub fn new(channel_a: C1, channel_b: C2) -> DualChannel<C1, C2> {
        DualChannel { write: DualWrite {channel_a, channel_b}}
    }
}

impl <C1: MetricChannel, C2: MetricChannel> MetricChannel for DualChannel<C1, C2> {
    type Metric = DualMetric<C1::Metric, C2::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> DualMetric<C1::Metric, C2::Metric> {
        let metric_1 = self.write.channel_a.define(m_type, &name, sample);
        let metric_2 = self.write.channel_b.define(m_type, &name, sample);
        DualMetric { metric_1, metric_2  }
    }

    type Write = DualWrite<C1, C2>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

