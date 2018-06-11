//! Chain of command for unscoped metrics.

use core::*;
use input::MetricScope;

use std::sync::Arc;
use std::fmt::Debug;

use input::DefineMetric;
use local;

lazy_static! {
    /// The reference instance identifying an uninitialized metric config.
    pub static ref NO_METRIC_OUTPUT: Arc<OpenScope + Send + Sync> = Arc::new(local::to_void());
}

/// Wrap a MetricConfig in a non-generic trait.
pub trait OpenScope: Debug {
    /// Open a new metrics scope
    fn open_scope_object(&self) -> Arc<DefineMetric + Send + Sync + 'static>;
}

/// A pair of functions composing a twin "chain of command".
/// This is the building block for the metrics backend.
#[derive(Derivative, Clone)]
#[derivative(Debug)]
pub struct MetricOutput<M> {
    namespace: Namespace,

    #[derivative(Debug = "ignore")]
    define_metric_fn: DefineMetricFn<M>,

    #[derivative(Debug = "ignore")]
    open_scope_fn: OpenScopeFn<M>,
}

impl<M> MetricOutput<M> {
    /// Open a new metric scope.
    /// Scope metrics allow an application to emit per-operation statistics,
    /// For example, producing a per-request performance log.
    ///
    /// ```rust
    /// use dipstick::*;
    /// let scope_metrics = to_log().open_scope();
    /// let request_counter = scope_metrics.counter("scope_counter");
    /// ```
    ///
    pub fn open_scope(&self) -> MetricScope<M> {
        MetricScope::new(self.namespace.clone(), self.define_metric_fn.clone(), (self.open_scope_fn)())
    }
}

/// Create a new metric chain with the provided metric definition and scope creation functions.
pub fn metric_output<MF, WF, M>(define_fn: MF, open_scope_fn: WF) -> MetricOutput<M>
where
    MF: Fn(&Namespace, Kind, Sampling) -> M + Send + Sync + 'static,
    WF: Fn() -> CommandFn<M> + Send + Sync + 'static,
{
    MetricOutput {
        namespace: ().into(),
        define_metric_fn: Arc::new(define_fn),
        open_scope_fn: Arc::new(open_scope_fn),
    }
}

impl<M: Send + Sync + Clone + 'static> MetricOutput<M> {
    /// Intercept both metric definition and scope creation, possibly changing the metric type.
    pub fn wrap_all<MF, N>(&self, mod_fn: MF) -> MetricOutput<N>
        where
            MF: Fn(DefineMetricFn<M>, OpenScopeFn<M>) -> (DefineMetricFn<N>, OpenScopeFn<N>),
            N: Clone + Send + Sync,
    {
        let (define_metric_fn, open_scope_fn) =
            mod_fn(self.define_metric_fn.clone(), self.open_scope_fn.clone());
        MetricOutput {
            namespace: self.namespace.clone(),
            define_metric_fn,
            open_scope_fn,
        }
    }

    /// Intercept scope creation.
    pub fn wrap_scope<MF>(&self, mod_fn: MF) -> Self
        where
            MF: Fn(OpenScopeFn<M>) -> OpenScopeFn<M>,
    {
        MetricOutput {
            namespace: self.namespace.clone(),
            define_metric_fn: self.define_metric_fn.clone(),
            open_scope_fn: mod_fn(self.open_scope_fn.clone()),
        }
    }
}

impl<M> Namespaced for MetricOutput<M> {

    /// Return cloned output with appended namespace.
    fn with_namespace(&self, namespace: &Namespace) -> Self {
        MetricOutput {
            namespace: self.namespace.with_namespace(namespace),
            define_metric_fn: self.define_metric_fn.clone(),
            open_scope_fn: self.open_scope_fn.clone(),
        }
    }

}

//impl<'a, M: Send + Sync + Clone + 'static> Index<&'a str> for MetricOutput<M> {
//    type Output = Self;
//
//    fn index(&self, index: &'a str) -> &Self::Output {
//        &self.push(index)
//    }
//}

impl<M: Send + Sync + Clone + 'static> OpenScope for MetricOutput<M> {
    fn open_scope_object(&self) -> Arc<DefineMetric + Send + Sync + 'static> {
        Arc::new(self.open_scope())
    }
}

impl<M> From<MetricOutput<M>> for MetricScope<M> {
    fn from(metrics: MetricOutput<M>) -> MetricScope<M> {
        metrics.open_scope().with_namespace(&metrics.namespace)
    }
}

impl<M: Send + Sync + Clone + 'static> From<MetricOutput<M>> for Arc<DefineMetric + Send + Sync + 'static> {
    fn from(metrics: MetricOutput<M>) -> Arc<DefineMetric + Send + Sync + 'static> {
        metrics.open_scope_object()
    }
}

