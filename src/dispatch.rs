use core::*;
use chain::*;
use std::collections::{HashMap, LinkedList};

pub struct MetricHandle (usize);

pub struct ScopeHandle (usize);

pub trait Observer {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn metric_create(&self, kind: Kind, name: &str, rate: Rate) -> MetricHandle;

    /// Drop a previously registered metric.
    /// Drop is called once per handle.
    /// Dropped handle will never be used again.
    /// Drop is only called with previously registered handles.
    fn metric_drop(&self, metric: MetricHandle);

    /// Open a new scope.
    /// Observer must return a new ScopeHandle that uniquely identifies the scope.
    fn scope_open(&self, buffered: bool) -> ScopeHandle;

    /// Write metric value to a scope.
    /// Observers only receive previously registered handles.
    fn scope_write(&self, scope: ScopeHandle, metric: MetricHandle, value:Value);

    /// Flush a scope.
    /// Observers only receive previously registered handles.
    fn scope_flush(&self, scope: ScopeHandle);

    /// Drop a previously registered scope.
    /// Drop is called once per handle.
    /// Dropped handle will never be used again.
    /// Drop is only called with previously registered handles.
    fn scope_close(&self, scope: ScopeHandle);
}

pub struct ChainObserver<T> {
    chain: Chain<T>
}

impl<T> Observer for ChainObserver<T> {
    fn metric_create(&self, kind: Kind, name: &str, rate: Rate) -> MetricHandle {
        self.chain.define_metric(kind, name, rate)
    }

    fn metric_drop(&self, metric: MetricHandle) {}

    fn scope_open(&self, buffered: bool) -> ScopeHandle {}
    fn scope_write(&self, scope: ScopeHandle, metric: MetricHandle, value:Value) {}
    fn scope_flush(&self, scope: ScopeHandle) {}
    fn scope_close(&self, scope: ScopeHandle) {}
}

pub struct Dispatcher {
    active_observers: usize,
    metrics: HashMap<String, Dispatch>,
    observers: RwLock<Vec<Observer>>
}

/// Aggregate metrics in memory.
/// Depending on the type of metric, count, sum, minimum and maximum of values will be tracked.
/// Needs to be connected to a publish to be useful.
/// ```
/// use dipstick::*;
/// let sink = aggregate(4, summary, to_stdout());
/// let metrics = global_metrics(sink);
/// metrics.marker("my_event").mark();
/// metrics.marker("my_event").mark();
/// ```
pub fn dispatch<E, M>(stat_fn: E, to_chain: Chain<M>) -> Chain<Dispatch>
    where
        E: Fn(Kind, &str, ScoreType) -> Option<(Kind, Vec<&str>, Value)> + Send + Sync + 'static,
        M: Clone + Send + Sync + Debug + 'static,
{
    let metrics = Arc::new(RwLock::new(HashMap::new()));
    let metrics0 = metrics.clone();

    let publish = Arc::new(Publisher::new(stat_fn, to_chain));

    Chain::new(
        move |kind, name, _rate| {
            // add metric
        },
        move |_buffered| {
            // open scope
            ControlScopeFn::new(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let metric: &Aggregate = metric;
                    metric.update(value)
                },
                ScopeCmd::Flush => {
                    let metrics = metrics.read().expect("Locking metrics scoreboards");
                    let snapshot = metrics.values().flat_map(|score| score.reset()).collect();
                    publish.publish(snapshot);
                }
            })
        },
    )
}