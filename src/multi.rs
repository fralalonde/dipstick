//! Dispatch metrics to multiple sinks.

use core::*;
use local_metrics::*;
use app_metrics::*;

use std::sync::Arc;

/// Two chains of different types can be combined in a tuple.
/// The chains will act as one, each receiving calls in the order the appear in the tuple.
/// For more than two types, make tuples of tuples, "Yo Dawg" style.
impl<M1, M2> From<(LocalMetrics<M1>, LocalMetrics<M2>)> for AppMetrics<(M1, M2)>
where
    M1: 'static + Clone + Send + Sync,
    M2: 'static + Clone + Send + Sync,
{
    fn from(combo: (LocalMetrics<M1>, LocalMetrics<M2>)) -> AppMetrics<(M1, M2)> {
        let scope0 = combo.0.open_scope();
        let scope1 = combo.1.open_scope();

        let scope0a = scope0.clone();
        let scope1a = scope1.clone();

        AppMetrics::new(
            Arc::new(move |kind, name, rate| (
                scope0.define_metric(kind, name, rate),
                scope1.define_metric(kind, name, rate),
            )),
            control_scope(move |cmd| { match cmd {
                ScopeCmd::Write(metric, value) => {
                    let metric: &(M1, M2) = metric;
                    scope0a.write(&metric.0, value);
                    scope1a.write(&metric.1, value);
                }
                ScopeCmd::Flush => {
                    scope0a.flush();
                    scope1a.flush();
                }
            }})
        )
    }
}

impl<'a, M> From<&'a [LocalMetrics<M>]> for AppMetrics<Vec<M>>
where
    M: 'static + Clone + Send + Sync,
{
    fn from(chains: &'a [LocalMetrics<M>]) -> AppMetrics<Vec<M>> {
        let scopes: Vec<AppMetrics<M>> = chains.iter().map(|x| x.open_scope()).collect();
        let scopes2 = scopes.clone();

        AppMetrics::new(
            Arc::new(move |kind, name, rate| {
                scopes.iter().map(|m| m.define_metric(kind, name, rate)).collect()
            }),
            control_scope(move |cmd| { match cmd {
                ScopeCmd::Write(metric, value) => {
                    let metric: &Vec<M> = metric;
                    for (i, scope) in scopes2.iter().enumerate() {
                        scope.write(&metric[i], value)
                    }
                }
                ScopeCmd::Flush => for scope in &scopes2 {
                    scope.flush()
                },
            }})
        )
    }
}
