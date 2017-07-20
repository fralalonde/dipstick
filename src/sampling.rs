use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, MetricChannel};
use pcg32;

pub struct SamplingMetric<M: DefinedMetric> {
    target: M,
    int_sampling_rate: u32,
}

impl <M: DefinedMetric> DefinedMetric for SamplingMetric<M> {}

pub struct SamplingWrite<C: MetricChannel> {
    target: C,
}

impl <C: MetricChannel> MetricWrite<SamplingMetric<<C as MetricChannel>::Metric>> for SamplingWrite<C> {

    fn write(&self, metric: &SamplingMetric<<C as MetricChannel>::Metric>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(|scope| scope.write(&metric.target, value))
        }
    }
}

pub struct SamplingChannel<C: MetricChannel> {
    write: SamplingWrite<C>,
    sampling_rate: RateType,
}

impl <C: MetricChannel> SamplingChannel<C> {
    pub fn new(target: C, sampling_rate: RateType) -> SamplingChannel<C> {
        SamplingChannel { write: SamplingWrite { target }, sampling_rate}
    }
}

impl <C: MetricChannel> MetricChannel for SamplingChannel<C> {
    type Metric = SamplingMetric<C::Metric>;
    type Write = SamplingWrite<C>;


    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> SamplingMetric<C::Metric> {
        let pm = self.write.target.define(m_type, name, self.sampling_rate);
        SamplingMetric { target: pm, int_sampling_rate: pcg32::to_int_rate(self.sampling_rate) }
    }

    fn write<F>(&self, operations: F ) where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}
