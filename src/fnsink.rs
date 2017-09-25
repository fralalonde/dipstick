//! A mechanism to dynamically define new sink implementations.

use core::*;
use std::sync::Arc;

/// Compose a sink dynamically using a generic `FnSink`.
/// Two methods have to be provided: One to make new metrics and one to create scopes.
///
/// Using this is often simpler than implementing the `Sink` trait.
/// This is especially well suited to stateless I/O sinks.
///
/// Performance impact of method delegation should be negligible
/// when compared actual method cost for most use cases.
// TODO assert actual performance vs hard-compiled impl
pub fn make_sink<M, MF, WF  >(make_metric: MF, make_scope: WF) -> FnSink<M>
    where MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
          WF: Fn(Scope<M>) + Send + Sync + 'static,
          M: Send + Sync,
{
    FnSink {
        metric_fn: Arc::new(make_metric),
        scope_fn: Arc::new(make_scope),
    }
}

/// FnSink delegates metric creation and scoping to the
/// functions or closures it was provided upon its creation.
pub struct FnSink<M> where M: Send + Sync  {
    metric_fn: MetricFn<M>,
    scope_fn: ScopeFn<M>,
}

impl <M> Sink<M> for FnSink<M> where M: Send + Sync {
    #[allow(unused_variables)]
    fn new_metric(&self, kind: Kind, name: &str, sampling: Rate) -> M {
        self.metric_fn.as_ref()(kind, name, sampling)
    }

    fn new_scope(&self) -> ScopeFn<M> {
        self.scope_fn.clone()
    }
}
