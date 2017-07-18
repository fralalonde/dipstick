use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};

////////////

pub struct SamplingMetric<M: DefinedMetric> {
    target: M
}

impl <M: DefinedMetric> DefinedMetric for SamplingMetric<M> {}

pub struct SamplingWrite<C: MetricChannel> {
    target: C,
}

impl <C: MetricChannel> MetricWrite<SamplingMetric<<C as MetricChannel>::Metric>> for SamplingWrite<C> {

    fn write(&self, metric: &SamplingMetric<<C as MetricChannel>::Metric>, value: Value) {
        println!("Sampling");
        self.target.write(|scope| scope.write(&metric.target, value))
    }
}

pub struct SamplingChannel<C: MetricChannel> {
    write: SamplingWrite<C>
}

impl <C: MetricChannel> SamplingChannel<C> {
    pub fn new(target: C) -> SamplingChannel<C> {
        SamplingChannel { write: SamplingWrite { target }}
    }
}

impl <C: MetricChannel> MetricChannel for SamplingChannel<C> {
    type Metric = SamplingMetric<C::Metric>;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> SamplingMetric<C::Metric> {
        let pm = self.write.target.define(m_type, name, sample);
        SamplingMetric { target: pm }
    }

    type Write = SamplingWrite<C>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}
