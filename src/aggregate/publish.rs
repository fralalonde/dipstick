//// Aggregate Source

use core::{MetricSink, MetricType, MetricWriter};
use aggregate::sink::{AggregateSource, AggregateScore};

/// Publisher from aggregate metrics to target channel
pub struct AggregatePublisher<C: MetricSink> {
    source: AggregateSource,
    target: C,
}

impl<C: MetricSink> AggregatePublisher<C> {
    /// Create new publisher from aggregate metrics to target channel
    pub fn new(target: C, source: AggregateSource) -> AggregatePublisher<C> {
        AggregatePublisher { source, target }
    }
}

impl<C: MetricSink> AggregatePublisher<C> {
    /// Define and write metrics from aggregated scores to the target channel
    /// If this is called repeatedly it can be a good idea to use the metric cache
    /// to prevent new metrics from being created every time.
    pub fn publish(&self) {
        let scope = self.target.new_writer();
        self.source.for_each(|metric| {
            match metric.read_and_reset() {
                AggregateScore::NoData => {
                    // TODO repeat previous frame min/max ?
                    // TODO update some canary metric ?
                }
                AggregateScore::Event { hit } => {
                    let name = format!("{}.hit", &metric.name);
                    let temp_metric = self.target.new_metric(MetricType::Count, name, 1.0);
                    scope.write(&temp_metric, hit);
                }
                AggregateScore::Value { hit, sum, max, min } => {
                    if hit > 0 {
                        // do not report gauges sum and hit, they are meaningless
                        match &metric.m_type {
                            &MetricType::Gauge => {
                                // NOTE averaging badly
                                // - hit and sum are not incremented nor read as one
                                // - integer division is not rounding
                                // assuming values will still be good enough to be useful
                                let name = format!("{}.avg", &metric.name);
                                let temp_metric = self.target.new_metric(metric.m_type, name, 1.0);
                                scope.write(&temp_metric, sum / hit);
                            }
                            &MetricType::Count |
                            &MetricType::Time => {
                                let name = format!("{}.sum", &metric.name);
                                let temp_metric = self.target.new_metric(metric.m_type, name, 1.0);
                                scope.write(&temp_metric, sum);
                            }
                            _ => (),
                        }

                        let name = format!("{}.max", &metric.name);
                        let temp_metric = self.target.new_metric(MetricType::Gauge, name, 1.0);
                        scope.write(&temp_metric, max);

                        let name = format!("{}.min", &metric.name);
                        let temp_metric = self.target.new_metric(MetricType::Gauge, name, 1.0);
                        scope.write(&temp_metric, min);
                    }
                }
            }
        })
    }
}
