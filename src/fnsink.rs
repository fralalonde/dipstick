//! A mechanism to dynamically define new sink implementations.

use core::*;
use std::sync::Arc;

/// Dynamic metric definition function.
/// Metrics can be defined from any thread, concurrently (Fn is Sync).
/// The resulting metrics themselves can be also be safely shared across threads (<M> is Send + Sync).
/// Concurrent usage of a metric is done using threaded scopes.
/// Shared concurrent scopes may be provided by some backends (aggregate).
pub type MetricFn<M> = Box<Fn(Kind, &str, Rate) -> M + Send + Sync>;

/// Compose a sink dynamically using a generic `FnSink`.
/// Two methods have to be provided: One to make new metrics and one to create scopes.
///
/// Using `make_sink` is often simpler than implementing the `Sink` trait and
/// is especially well suited to stateless I/O sinks.
///
/// Performance impact of method delegation should be negligible
/// when compared actual method cost for most use cases.
// TODO assert actual performance vs hard-compiled impl
pub fn make_sink<M, MF, WF>(make_metric: MF, make_scope: WF) -> FnSink<M>
where
    MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
    WF: Fn(Scope<M>) + Send + Sync + 'static,
    M: Send + Sync,
{
    FnSink {
        // capture the provided closures in Arc to provide cheap clones
        metric_fn: Box::new(make_metric),
        scope_fn: Arc::new(make_scope),
    }
}

/// FnSink delegates metric creation and scoping to the
/// functions or closures it was provided upon its creation.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct FnSink<M>
where
    M: Send + Sync,
{
    #[derivative(Debug = "ignore")]
    metric_fn: MetricFn<M>,
    #[derivative(Debug = "ignore")]
    scope_fn: ScopeFn<M>,
}

impl<M> Sink<M> for FnSink<M>
where
    M: Clone + Send + Sync,
{
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        self.metric_fn.as_ref()(kind, name, sampling)
    }

    #[allow(unused_variables)]
    fn new_scope(&self, auto_flush: bool) -> ScopeFn<M> {
        self.scope_fn.clone()
    }
}
