use core::{MetricType, RateType, Value, SinkWriter, SinkMetric, MetricSink};
use pcg32;

#[derive(Debug)]
pub struct RandomSamplingMetric<M: SinkMetric> {
    target: M,
    int_sampling_rate: u32,
}

impl <M: SinkMetric> SinkMetric for RandomSamplingMetric<M> {}

#[derive(Debug)]
pub struct RandomSamplingWriter<C: MetricSink> {
    target: C::Writer,
}

impl <C: MetricSink> SinkWriter<RandomSamplingMetric<<C as MetricSink>::Metric>> for RandomSamplingWriter<C> {

    fn write(&self, metric: &RandomSamplingMetric<<C as MetricSink>::Metric>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(&metric.target, value)
        }
    }
}

#[derive(Debug)]
pub struct RandomSamplingSink<C: MetricSink> {
    target: C,
    sampling_rate: RateType,
}

impl <C: MetricSink> RandomSamplingSink<C> {
    pub fn new(target: C, sampling_rate: RateType) -> RandomSamplingSink<C> {
        RandomSamplingSink { target, sampling_rate}
    }
}

impl <C: MetricSink> MetricSink for RandomSamplingSink<C> {
    type Metric = RandomSamplingMetric<C::Metric>;
    type Writer = RandomSamplingWriter<C>;


    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> RandomSamplingMetric<C::Metric> {
        let pm = self.target.define(m_type, name, self.sampling_rate);
        RandomSamplingMetric { target: pm, int_sampling_rate: pcg32::to_int_rate(self.sampling_rate) }
    }

    fn new_writer(&self) -> RandomSamplingWriter<C> {
        RandomSamplingWriter { target: self.target.new_writer() }
    }

}
