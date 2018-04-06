//! Dispatch metrics to multiple sinks.

use core::*;
use output::*;
use scope::*;

use std::sync::Arc;

/// Two chains of different types can be combined in a tuple.
/// The chains will act as one, each receiving calls in the order the appear in the tuple.
/// For more than two types, make tuples of tuples, "Yo Dawg" style.
impl<M1, M2> From<(MetricOutput<M1>, MetricOutput<M2>)> for MetricScope<(M1, M2)>
where
    M1: 'static + Clone + Send + Sync,
    M2: 'static + Clone + Send + Sync,
{
    fn from(combo: (MetricOutput<M1>, MetricOutput<M2>)) -> MetricScope<(M1, M2)> {
        let scope0 = combo.0.open_scope();
        let scope1 = combo.1.open_scope();

        let scope0a = scope0.clone();
        let scope1a = scope1.clone();

        MetricScope::new(
            Arc::new(move |kind, name, rate| {
                (
                    scope0.define_metric(kind, name, rate),
                    scope1.define_metric(kind, name, rate),
                )
            }),
            command_fn(move |cmd| match cmd {
                Command::Write(metric, value) => {
                    let metric: &(M1, M2) = metric;
                    scope0a.write(&metric.0, value);
                    scope1a.write(&metric.1, value);
                }
                Command::Flush => {
                    scope0a.flush();
                    scope1a.flush();
                }
            }),
        )
    }
}

impl<'a, M> From<&'a [MetricOutput<M>]> for MetricScope<Vec<M>>
where
    M: 'static + Clone + Send + Sync,
{
    fn from(chains: &'a [MetricOutput<M>]) -> MetricScope<Vec<M>> {
        let scopes: Vec<MetricScope<M>> = chains.iter().map(|x| x.open_scope()).collect();
        let scopes2 = scopes.clone();

        MetricScope::new(
            Arc::new(move |kind, name, rate| {
                scopes
                    .iter()
                    .map(|m| m.define_metric(kind, name, rate))
                    .collect()
            }),
            command_fn(move |cmd| match cmd {
                Command::Write(metric, value) => {
                    let metric: &Vec<M> = metric;
                    for (i, scope) in scopes2.iter().enumerate() {
                        scope.write(&metric[i], value)
                    }
                }
                Command::Flush => for scope in &scopes2 {
                    scope.flush()
                },
            }),
        )
    }
}
