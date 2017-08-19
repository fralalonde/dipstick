use super::{MetricKind, Rate, Value, MetricWriter, MetricKey, MetricSink};

#[derive(Debug)]
pub struct DualKey<M1: MetricKey, M2: MetricKey> {
    metric_1: M1,
    metric_2: M2,
}

impl<M1: MetricKey, M2: MetricKey> MetricKey for DualKey<M1, M2> {}

#[derive(Debug)]
pub struct DualWriter<C1: MetricSink, C2: MetricSink> {
    channel_a: C1::Writer,
    channel_b: C2::Writer,
}

impl<
    C1: MetricSink,
    C2: MetricSink,
> MetricWriter<DualKey<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>>
    for DualWriter<C1, C2> {
    fn write(
        &self,
        metric: &DualKey<<C1 as MetricSink>::Metric, <C2 as MetricSink>::Metric>,
        value: Value,
    ) {
        self.channel_a.write(&metric.metric_1, value);
        self.channel_b.write(&metric.metric_2, value);
    }
}

#[derive(Debug)]
pub struct DualSink<C1: MetricSink, C2: MetricSink> {
    channel_a: C1,
    channel_b: C2,
}

impl<C1: MetricSink, C2: MetricSink> DualSink<C1, C2> {
    pub fn new(channel_a: C1, channel_b: C2) -> DualSink<C1, C2> {
        DualSink {
            channel_a,
            channel_b,
        }
    }
}

impl<C1: MetricSink, C2: MetricSink> MetricSink for DualSink<C1, C2> {
    type Metric = DualKey<C1::Metric, C2::Metric>;
    type Writer = DualWriter<C1, C2>;

    #[allow(unused_variables)]
    fn new_metric<S: AsRef<str>>(&self, kind: MetricKind, name: S, sampling: Rate)
                                 -> Self::Metric {
        let metric_1 = self.channel_a.new_metric(kind, &name, sampling);
        let metric_2 = self.channel_b.new_metric(kind, &name, sampling);
        DualKey { metric_1, metric_2 }
    }

    fn new_writer(&self) -> Self::Writer {
        DualWriter {
            channel_a: self.channel_a.new_writer(),
            channel_b: self.channel_b.new_writer(),
        }
    }
}
