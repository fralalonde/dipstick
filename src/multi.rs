//! Dispatch metrics to multiple sinks.

use ::*;

/// Hold each sink's metric key.
#[derive(Debug)]
pub struct DoubleKey<M1: MetricKey, M2: MetricKey> {
    metric_1: M1,
    metric_2: M2,
}

impl<M1: MetricKey, M2: MetricKey> MetricKey for DoubleKey<M1, M2> {}

/// Write the metric values to each sink.
#[derive(Debug)]
pub struct DoubleWriter<C1: MetricSink, C2: MetricSink> {
    sink_a: C1::Writer,
    sink_b: C2::Writer,
}

impl<C1: MetricSink, C2: MetricSink,>
    MetricWriter<DoubleKey<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>>
    for DoubleWriter<C1, C2> {
    fn write(&self,
             metric: &DoubleKey<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>,
             value: Value,) {
        self.sink_a.write(&metric.metric_1, value);
        self.sink_b.write(&metric.metric_2, value);
    }
}

/// Hold the two target sinks.
/// Multiple `DoubleSink`s can be combined if more than two sinks are needed.
#[derive(Debug)]
pub struct DoubleSink<C1: MetricSink, C2: MetricSink> {
    sink_a: C1,
    sink_b: C2,
}

impl<C1: MetricSink, C2: MetricSink> DoubleSink<C1, C2> {
    /// Create a single sink out of two disparate sinks.
    pub fn new(sink_a: C1, sink_b: C2) -> DoubleSink<C1, C2> {
        DoubleSink {
            sink_a,
            sink_b,
        }
    }
}

impl<C1: MetricSink, C2: MetricSink> MetricSink for DoubleSink<C1, C2> {
    type Metric = DoubleKey<C1::Metric, C2::Metric>;
    type Writer = DoubleWriter<C1, C2>;

    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate)
                                 -> Self::Metric {
        let metric_1 = self.sink_a.new_metric(kind, &name, sampling);
        let metric_2 = self.sink_b.new_metric(kind, &name, sampling);
        DoubleKey { metric_1, metric_2 }
    }

    fn new_writer(&self) -> Self::Writer {
        DoubleWriter {
            sink_a: self.sink_a.new_writer(),
            sink_b: self.sink_b.new_writer(),
        }
    }
}
