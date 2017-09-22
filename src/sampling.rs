//! Reduce the amount of data to process or transfer by statistically dropping some of it.

use core::*;
use pcg32;
use std::marker::PhantomData;

/// Perform random sampling of values according to the specified rate.
pub fn sample<'ph, M, W, S>(rate: Rate, sink: S) -> SamplingSink<'ph, M, W, S>
    where W: Writer<M>, S: Sink<M, W>
{
    SamplingSink::new(sink, rate)
}

/// The metric sampling key also holds the sampling rate to apply to it.
#[derive(Debug)]
pub struct SamplingMetric<M> {
    target: M,
    int_sampling_rate: u32,
}

/// The writer applies sampling logic each time a metric value is reported.
pub struct SamplingWriter<'ph, M: 'ph, W> {
    target: W,
    metric: PhantomData<&'ph M>,
}

impl<'ph, M, W> Writer<SamplingMetric<M>> for SamplingWriter<'ph, M, W> where W: Writer<M> {
    fn write(&self, metric: &SamplingMetric<M>, value: Value) {
        if pcg32::accept_sample(metric.int_sampling_rate) {
            self.target.write(&metric.target, value)
        }
    }
}

/// A sampling sink adapter.
pub struct SamplingSink<'ph, M: 'ph, W: 'ph, S> {
    target: S,
    sampling_rate: Rate,
    phantom: PhantomData<&'ph (M, W)>,
}

impl<'ph, M, W, S> SamplingSink<'ph, M, W, S> where W: Writer<M>, S: Sink<M, W> {
    /// Create a new sampling sink adapter.
    pub fn new(target: S, sampling_rate: Rate) -> SamplingSink<'ph, M, W, S> {
        SamplingSink { target, sampling_rate, phantom: PhantomData {} }
    }
}

impl<'ph, M, W, S> Sink<SamplingMetric<M>, SamplingWriter<'ph, M, W>> for SamplingSink<'ph, M, W, S>
    where W: Writer<M>, S: Sink<M, W>
{
    #[allow(unused_variables)]
    fn new_metric<STR: AsRef<str>>(&self, kind: MetricKind, name: STR, sampling: Rate)
                                   -> SamplingMetric<M> {
        // TODO override only if FULL_SAMPLING else warn!()
        assert_eq!(sampling, FULL_SAMPLING_RATE, "Overriding previously set sampling rate");

        let pm = self.target.new_metric(kind, name, self.sampling_rate);
        SamplingMetric {
            target: pm,
            int_sampling_rate: pcg32::to_int_rate(self.sampling_rate),
        }
    }

    fn new_writer(&self) -> SamplingWriter<'ph, M, W>   {
        SamplingWriter { target: self.target.new_writer(), metric: PhantomData {} }
    }
}
