use core::{MetricType, Rate, Value, MetricWriter, MetricKey, MetricSink};
use pcg32;

#[derive(Debug)]
pub struct SamplingKey<M: MetricKey> {
    target: M,
    int_sampling_rate: u32,
}

impl <M: MetricKey> MetricKey for SamplingKey<M> {}

#[derive(Debug)]
pub struct SamplingWriter<C: MetricSink> {
    target: C::Writer,
}

impl <C: MetricSink> MetricWriter<SamplingKey<<C as MetricSink>::Metric>> for SamplingWriter<C> {

    fn write(&self, metric: &SamplingKey<<C as MetricSink>::Metric>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(&metric.target, value)
        }
    }
}

#[derive(Debug)]
pub struct SamplingSink<C: MetricSink> {
    target: C,
    sampling_rate: Rate,
}

impl <C: MetricSink> SamplingSink<C> {
    pub fn new(target: C, sampling_rate: Rate) -> SamplingSink<C> {
        SamplingSink { target, sampling_rate}
    }
}

impl <C: MetricSink> MetricSink for SamplingSink<C> {
    type Metric = SamplingKey<C::Metric>;
    type Writer = SamplingWriter<C>;


    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sampling: Rate) -> SamplingKey<C::Metric> {
        let pm = self.target.define(m_type, name, self.sampling_rate);
        SamplingKey { target: pm, int_sampling_rate: pcg32::to_int_rate(self.sampling_rate) }
    }

    fn new_writer(&self) -> SamplingWriter<C> {
        SamplingWriter { target: self.target.new_writer() }
    }

}
