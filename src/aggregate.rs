use core::{MetricType, RateType, Value, MetricWrite, DefinedMetric, Channel};
use std::sync::atomic::{AtomicUsize, Ordering};

//// Aggregate

enum StatsType {
    HitCount,
    Sum,
    MeanValue,
    Max,
    Min,
    MeanRate
}

struct AggregateMetric {
    hit_count: AtomicUsize,
    value_sum: AtomicUsize,
    value_max: AtomicUsize,
    value_min: AtomicUsize,
}

impl AggregateMetric {
    fn new() -> AggregateMetric {
        AggregateMetric{ hit_count: AtomicUsize::new(0), value_sum: AtomicUsize::new(0), value_max: AtomicUsize::new(0), value_min: AtomicUsize::new(0)}
    }
}

impl DefinedMetric for AggregateMetric {

}

struct AggregateWrite<C: Channel> {
    target: C,
}

impl <C: Channel> MetricWrite<AggregateMetric> for AggregateWrite<C> {
    fn write(&self, metric: &AggregateMetric, value: Value) {
        println!("Aggregate");
        metric.hit_count.fetch_add(1, Ordering::Relaxed);
        metric.value_sum.fetch_add(value as usize, Ordering::Relaxed);

        //        self.target.write(|scope| scope.write(metric, value, tags))
    }
}

struct AggregateChannel<C: Channel> {
    write: AggregateWrite<C>,
    stats: Vec<AggregateMetric>
}

impl <C: Channel> AggregateChannel<C> {
    fn new(target: C) -> AggregateChannel<C> {
        AggregateChannel { write: AggregateWrite { target }, stats: Vec::new()}
    }
}

impl <C: Channel> Channel for AggregateChannel<C> {
    type Metric = AggregateMetric;

    fn define<S: AsRef<str>>(&self, m_type: MetricType, name: S, sample: RateType) -> AggregateMetric {
        let pm = self.write.target.define(m_type, name, sample);
        let mut exp = match m_type {
            MetricType::Gauge => {vec!(
                self.write.target.define(m_type, format!("{}.avg", String::from(name.as_ref())), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample)
            )}
            MetricType::Count => {vec!(
                self.write.target.define(m_type, format!("{}.avg", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.sum", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
            )}
            MetricType::Time => {vec!(
                self.write.target.define(m_type, format!("{}.avg", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.sum", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.max", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
            )}
            MetricType::Event => {vec!(
                self.write.target.define(m_type, format!("{}.count", name.as_ref()), sample),
                self.write.target.define(m_type, format!("{}.rate", name.as_ref()), sample)
            )}
        };
        AggregateMetric::new()
    }

    type Write = AggregateWrite<C>;

    fn write<F>(&self, operations: F )
        where F: Fn(&Self::Write) {
        operations(&self.write)
    }
}

