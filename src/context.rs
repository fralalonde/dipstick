//! Chain of command for unscoped metrics.

use core::*;
use metrics::Metrics;

use std::sync::Arc;

use namespace::*;

/// A pair of functions composing a twin "chain of command".
/// This is the building block for the metrics backend.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct MetricContext<M> {
    #[derivative(Debug = "ignore")]
    define_metric_fn: DefineMetricFn<M>,

    #[derivative(Debug = "ignore")]
    scope_metric_fn: OpenScopeFn<M>,
}

impl<M> MetricContext<M> {
    /// Open a new metric scope.
    /// Scope metrics allow an application to emit per-operation statistics,
    /// For example, producing a per-request performance log.
    ///
    /// Although the scope metrics can be predefined like in ['AppMetrics'], the application needs to
    /// create a scope that will be passed back when reporting scoped metric values.
    ///
    /// ```rust
    /// use dipstick::*;
    /// let scope_metrics = to_log().open_scope();
    /// let request_counter = scope_metrics.counter("scope_counter");
    /// ```
    ///
    pub fn open_scope(&self) -> Metrics<M> {
        Metrics::new(self.define_metric_fn.clone(), (self.scope_metric_fn)())
    }

}

/// Create a new metric chain with the provided metric definition and scope creation functions.
pub fn metrics_context<MF, WF, M>(make_metric: MF, make_scope: WF) -> MetricContext<M>
    where
        MF: Fn(Kind, &str, Rate) -> M + Send + Sync + 'static,
        WF: Fn() -> WriteFn<M> + Send + Sync + 'static,
{
    MetricContext {
        define_metric_fn: Arc::new(make_metric),
        scope_metric_fn: Arc::new(make_scope),
    }
}

impl<M: Send + Sync + Clone + 'static> MetricContext<M> {

    /// Intercept both metric definition and scope creation, possibly changing the metric type.
    pub fn mod_both<MF, N>(&self, mod_fn: MF) -> MetricContext<N>
    where
        MF: Fn(DefineMetricFn<M>, OpenScopeFn<M>) -> (DefineMetricFn<N>, OpenScopeFn<N>),
        N: Clone + Send + Sync,
    {
        let (metric_fn, scope_fn) =
            mod_fn(self.define_metric_fn.clone(), self.scope_metric_fn.clone());
        MetricContext {
            define_metric_fn: metric_fn,
            scope_metric_fn: scope_fn,
        }
    }

    /// Intercept scope creation.
    pub fn mod_scope<MF>(&self, mod_fn: MF) -> Self
    where
        MF: Fn(OpenScopeFn<M>) -> OpenScopeFn<M>,
    {
        MetricContext {
            define_metric_fn: self.define_metric_fn.clone(),
            scope_metric_fn: mod_fn(self.scope_metric_fn.clone()),
        }
    }

}

impl<M> From<MetricContext<M>> for Metrics<M> {
    fn from(metrics: MetricContext<M>) -> Metrics<M> {
        metrics.open_scope()
    }
}

impl<M: Send + Sync + Clone + 'static> WithNamespace for MetricContext<M> {
    fn with_name<IN: Into<Namespace>>(&self, names: IN) -> Self {
        let ref ninto = names.into();
        MetricContext {
            define_metric_fn: add_namespace(ninto, self.define_metric_fn.clone()),
            scope_metric_fn: self.scope_metric_fn.clone(),
        }
    }
}

