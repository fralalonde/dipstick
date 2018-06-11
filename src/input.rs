//! Static metrics are used to define metrics that share a single persistent metrics scope.
//! Because the scope never changes (it is "global"), all that needs to be provided by the
//! application is the metrics values.
//!
//! Compared to [ScopeMetrics], static metrics are easier to use and provide satisfactory metrics
//! in many applications.
//!
//! If multiple [AppMetrics] are defined, they'll each have their scope.
//!
use core::{Value, Sampling, WriteFn, Namespace, Kind, DefineMetricFn, CommandFn, Namespaced};
use core::Kind::*;
use clock::TimeHandle;
use cache::{add_cache, WithCache};
use scheduler::{set_schedule, CancelHandle};
use output;

use std::sync::Arc;
use std::time::Duration;

// TODO define an 'AsValue' trait + impl for supported number types, then drop 'num' crate
pub use num::ToPrimitive;

lazy_static! {
    /// The reference instance identifying an uninitialized metric scope.
    pub static ref NO_METRIC_SCOPE: Arc<DefineMetric + Send + Sync> =
        output::NO_METRIC_OUTPUT.open_scope_object();
}

/// A non-generic trait to hide MetricScope<M>
pub trait DefineMetric: Flush {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn define_metric_object(&self, name: &Namespace, kind: Kind, rate: Sampling) -> WriteFn;

}


fn scope_write_fn<M, D>(scope: &D, kind: Kind, name: &str) -> WriteFn
    where
        M: Clone + Send + Sync + 'static,
        D: MetricInput<M> + Clone + Send + Sync + 'static
{
    let scope1 = scope.clone();
    let metric = scope.define_metric(&name.into(), kind, 1.0);
    Arc::new(move |value| scope1.write(&metric, value))
}


/// Define metrics, write values and flush them.
pub trait MetricInput: Namespaced {
    /// Define a metric of the specified type.
    fn define_metric(&self, namespace: &Namespace, kind: Kind, rate: Sampling) -> Box<WriteFn>;

    fn flush(&self) {}

    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.clone();
        set_schedule(period, move || scope.flush())
    }

}

impl<M: Send + Sync + Clone + 'static> DefineMetric for MetricScope<M> {
    fn define_metric_object(&self, namespace: &Namespace, kind: Kind, rate: Sampling) -> WriteFn
    {
        let target_metric = self.define_metric(namespace, kind, rate);
        let write_to = self.clone();
        Arc::new(move |value| write_to.write(&target_metric, value))
    }
}

//impl<M> WriteMetric for MetricWriter<M> {
//    fn write(&self, value: Value) {
//        self.command_fn.write(&self.target_metric, value);
//    }
//}

//// Mutators impl

impl<M: Send + Sync + Clone + 'static> WithCache for MetricScope<M> {
    fn with_cache(&self, cache_size: usize) -> Self {
        MetricScope {
            namespace: self.namespace.clone(),
            define_fn: add_cache(cache_size, self.define_fn.clone()),
            command_fn: self.command_fn.clone(),
        }
    }
}

