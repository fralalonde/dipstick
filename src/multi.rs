//! Dispatch metrics to multiple sinks.

use core::*;

/// Hold each sink's metric key.
pub type DoubleKey<M1, M2> = (M1, M2);

/// Write the metric values to each sink.
pub type DoubleWriter<W1, W2> = (W1, W2);

impl<M1, W1, M2, W2> Writer<DoubleKey<M1, M2>> for DoubleWriter<W1, W2> 
    where W1: Writer<M1>,
          W2: Writer<M2>,
{
    fn write(&self, metric: &DoubleKey<M1, M2>, value: Value,) {
        self.0.write(&metric.0, value);
        self.1.write(&metric.1, value);
    }
}

/// Hold the two target sinks.
/// Multiple `DoubleSink`s can be combined if more than two sinks are needed.
pub type DoubleSink<S1, S2> = (S1, S2);

impl<M1, W1, S1, M2, W2, S2> Sink<DoubleKey<M1, M2>, DoubleWriter<W1, W2>> for DoubleSink<S1, S2>
    where W1: Writer<M1>, S1: Sink<M1, W1>,
          W2: Writer<M2>, S2: Sink<M2, W2>,
{
    #[allow(unused_variables)]
    fn new_metric<STR: AsRef<str>>(&self, kind: MetricKind, name: STR, sampling: Rate) -> DoubleKey<M1, M2> {
        (self.0.new_metric(kind, &name, sampling), self.1.new_metric(kind, &name, sampling))
    }

    fn new_writer(&self) -> DoubleWriter<W1, W2> {
        (self.0.new_writer(), self.1.new_writer())
    }
}
