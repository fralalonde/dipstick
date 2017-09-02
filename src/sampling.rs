//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use ::*;
use pcg32;

/// The metric sampling key also holds the sampling rate to apply to it.
#[derive(Debug)]
pub struct SamplingKey<M: MetricKey> {
    target: M,
    int_sampling_rate: u32,
}

impl<M: MetricKey> MetricKey for SamplingKey<M> {}

/// The writer applies sampling logic each time a metric value is reported.
#[derive(Debug)]
pub struct SamplingWriter<C: MetricSink> {
    target: C::Writer,
}

impl<C: MetricSink> MetricWriter<SamplingKey<<C as MetricSink>::Metric>> for SamplingWriter<C> {
    fn write(&self, metric: &SamplingKey<<C as MetricSink>::Metric>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(&metric.target, value)
        }
    }
}

/// A sampling sink adapter.
#[derive(Debug)]
pub struct SamplingSink<C: MetricSink> {
    target: C,
    sampling_rate: Rate,
}

impl<C: MetricSink> SamplingSink<C> {
    /// Create a new sampling sink adapter.
    pub fn new(target: C, sampling_rate: Rate) -> SamplingSink<C> {
        SamplingSink {
            target,
            sampling_rate,
        }
    }
}

impl<C: MetricSink> MetricSink for SamplingSink<C> {
    type Metric = SamplingKey<C::Metric>;
    type Writer = SamplingWriter<C>;

    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate)
                                 -> Self::Metric {
        // TODO override only if FULL_SAMPLING else warn!()
        assert_eq!(sampling, FULL_SAMPLING_RATE, "Overriding previously set sampling rate");

        let pm = self.target.new_metric(kind, name, self.sampling_rate);
        SamplingKey {
            target: pm,
            int_sampling_rate: pcg32::to_int_rate(self.sampling_rate),
        }
    }

    fn new_writer(&self) -> Self::Writer {
        SamplingWriter { target: self.target.new_writer() }
    }
}
