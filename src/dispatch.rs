pub use core::{MetricType, Value, MetricWriter, MetricSink, MetricDispatch,
           EventMetric, CountMetric, GaugeMetric, TimerMetric};
use std::sync::Arc;
pub use num::ToPrimitive;

/// Base struct for all direct dispatch metrics
struct DirectMetric<C: MetricSink + 'static> {
    metric: <C as MetricSink>::Metric,
    writer: Arc<DirectWriter<C>>,
}

/// An event marker that dispatches values directly to the metrics backend
pub struct DirectEvent<C: MetricSink + 'static>(DirectMetric<C>);

/// A gauge or counter that dispatches values directly to the metrics backend
pub struct DirectGauge<C: MetricSink + 'static>(DirectMetric<C>);

/// A gauge or counter that dispatches values directly to the metrics backend
pub struct DirectCount<C: MetricSink + 'static>(DirectMetric<C>);

/// An timer that dispatches values directly to the metrics backend
pub struct DirectTimer<C: MetricSink + 'static>(DirectMetric<C>);

pub struct DirectWriter<C: MetricSink + 'static> {
    target_writer: C::Writer,
}

impl<C: MetricSink> DirectWriter<C> {
    fn write(&self, metric: &C::Metric, value: Value) {
       self.target_writer.write(metric, value)
    }
}

impl<C: MetricSink> EventMetric for DirectEvent<C> {
    fn mark(&self) {
        self.0.writer.write(&self.0.metric, 1);
    }
}

impl<C: MetricSink> CountMetric for DirectCount<C> {
    fn count<V>(&self, count: V) where V: ToPrimitive {
        self.0.writer.write(&self.0.metric, count.to_u64().unwrap());
    }
}

impl<C: MetricSink> GaugeMetric for DirectGauge<C> {
    fn value<V>(&self, value: V) where V: ToPrimitive {
        self.0.writer.write(&self.0.metric, value.to_u64().unwrap());
    }
}

impl<C: MetricSink> TimerMetric for DirectTimer<C> {
    fn interval_us<V>(&self, interval_us: V) -> V where V: ToPrimitive {
        self.0.writer.write(&self.0.metric, interval_us.to_u64().unwrap());
        interval_us
    }
}

/// A metric dispatch that writes directly to the metric backend (not queuing)
pub struct DirectDispatch<C: MetricSink + 'static> {
    prefix: String,
    target: Arc<C>,
    writer: Arc<DirectWriter<C>>,
}

impl<C: MetricSink> DirectDispatch<C> {
    /// Create a new direct metric dispatch
    pub fn new(target: C) -> DirectDispatch<C> {
        let target_writer = target.new_writer();
        DirectDispatch {
            prefix: "".to_string(),
            target: Arc::new(target),
            writer: Arc::new(DirectWriter {
                target_writer,
            }),
        }
    }

    fn add_prefix<S: AsRef<str>>(&self, name: S) -> String {
        // FIXME is there a way to return <S> in both cases?
        if self.prefix.is_empty() {
            return name.as_ref().to_string()
        }
        let mut buf:String = self.prefix.clone();
        buf.push_str(name.as_ref());
        buf.to_string()
    }
}

impl<C: MetricSink> MetricDispatch for DirectDispatch<C> {
    type Event = DirectEvent<C>;
    type Count = DirectCount<C>;
    type Gauge = DirectGauge<C>;
    type Timer = DirectTimer<C>;

    fn new_event<S: AsRef<str>>(&self, name: S) -> Self::Event {
        let metric = self.target.new_metric(MetricType::Event, self.add_prefix(name), 1.0);
        DirectEvent(DirectMetric {
            metric,
            writer: self.writer.clone(),
        })
    }

    fn new_count<S: AsRef<str>>(&self, name: S) -> Self::Count {
        let metric = self.target.new_metric(MetricType::Count, self.add_prefix(name), 1.0);
        DirectCount(DirectMetric {
            metric,
            writer: self.writer.clone(),
        })
    }

    fn new_timer<S: AsRef<str>>(&self, name: S) -> Self::Timer {
        let metric = self.target.new_metric(MetricType::Time, self.add_prefix(name), 1.0);
        DirectTimer(DirectMetric {
            metric,
            writer: self.writer.clone(),
        })
    }

    fn new_gauge<S: AsRef<str>>(&self, name: S) -> Self::Gauge {
        let metric = self.target.new_metric(MetricType::Gauge, self.add_prefix(name), 1.0);
        DirectGauge(DirectMetric {
            metric,
            writer: self.writer.clone(),
        })
    }

    fn with_prefix<S: AsRef<str>>(&self, prefix: S) -> Self {
        DirectDispatch {
            prefix: prefix.as_ref().to_string(),
            target: self.target.clone(),
            writer: self.writer.clone(),
        }
    }

}

/// Run benchmarks with `cargo +nightly bench --features bench`
#[cfg(feature = "bench")]
mod bench {

    use aggregate::MetricAggregator;
    use core::{MetricDispatch, EventMetric};
    use test::Bencher;

    #[bench]
    fn time_bench_direct_dispatch_event(b: &mut Bencher) {
        let aggregate = MetricAggregator::new().sink();
        let dispatch = super::DirectDispatch::new(aggregate);
        let event = dispatch.new_event("aaa");
        b.iter(|| event.mark());
    }

}
