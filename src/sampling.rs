use core::{MetricType, RateType, Value, SinkWriter, SinkMetric, MetricSink};
use pcg32;

pub struct RandomSamplingMetric<M: SinkMetric> {
    target: M,
    int_sampling_rate: u32,
}

impl <M: SinkMetric> SinkMetric for RandomSamplingMetric<M> {}

pub struct RandomSamplingWriter<C: MetricSink> {
    target: C,
}

impl <C: MetricSink> SinkWriter<RandomSamplingMetric<<C as MetricSink>::Metric>> for RandomSamplingWriter<C> {

    fn write(&self, metric: &RandomSamplingMetric<<C as MetricSink>::Metric>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(|scope| scope.write(&metric.target, value))
        }
    }
}

pub struct RandomSamplingSink<C: MetricSink> {
    write: RandomSamplingWriter<C>,
    sampling_rate: RateType,
}

impl <C: MetricSink> RandomSamplingSink<C> {
    pub fn new(target: C, sampling_rate: RateType) -> RandomSamplingSink<C> {
        RandomSamplingSink { write: RandomSamplingWriter { target }, sampling_rate}
    }
}

impl <C: MetricSink> MetricSink for RandomSamplingSink<C> {
    type Metric = RandomSamplingMetric<C::Metric>;
    type Write = RandomSamplingWriter<C>;


    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> RandomSamplingMetric<C::Metric> {
        let pm = self.write.target.define(m_type, name, self.sampling_rate);
        RandomSamplingMetric { target: pm, int_sampling_rate: pcg32::to_int_rate(self.sampling_rate) }
    }

    fn write<F>(&self, operations: F ) where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}
