//! Decouple metric definition from configuration with trait objects.

use core::{Namespace, Namespaced, Kind, MetricInput, WriteFn, ROOT_NS, NO_METRIC_OUTPUT, Flush};
use error;

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

lazy_static! {
    /// Root of the default metrics dispatch, usable by all libraries and apps.
    /// Libraries should create their metrics into sub subspaces of this.
    /// Applications should configure on startup where the dispatched metrics should go.
    /// Exceptionally, one can create its own MetricDispatch root, separate from this one.
    pub static ref ROOT_DISPATCH: MetricDispatch = MetricDispatch::new();
}

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DispatchMetric {
    // basic info for this metric, needed to recreate new corresponding trait object if target changes
    name: Namespace,
    kind: Kind,

    // the metric trait object to dispatch metric values to
    // the second part can be up to namespace.len() + 1 if this metric was individually targeted
    // 0 if no target assigned
    #[derivative(Debug = "ignore")]
    write_metric: (AtomicRefCell<(WriteFn, usize)>),

    // a reference to the the parent dispatcher to remove the metric from when it is dropped
    #[derivative(Debug = "ignore")]
    dispatch: Arc<RwLock<InnerDispatch>>,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for DispatchMetric {
    fn drop(&mut self) {
        self.dispatch.write().expect("Dispatch Lock").drop_metric(&self.name)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct MetricDispatch {
    namespace: Namespace,
    inner: Arc<RwLock<InnerDispatch>>,
}

struct InnerDispatch {
    // namespaces can target one, many or no metrics
    targets: HashMap<Namespace, Arc<MetricInput + Send + Sync>>,
    // last part of the namespace is the metric's name
    metrics: BTreeMap<Namespace, Weak<DispatchMetric>>,
}

///// Allow turning a 'static str into a Delegate, where str is the prefix.
//impl<M: AsRef<str>> From<M> for MetricDispatch {
//    fn from(name: M) -> MetricScope<Dispatch> {
//        ROOT_DISPATCH.with_prefix(name.as_ref())
//    }
//}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
//impl From<()> for MetricDispatch {
//    fn from(_: ()) -> MetricScope<Dispatch> {
//        metric_dispatch().into()
//    }
//}

impl InnerDispatch {

    fn new() -> Self {
        Self {
            targets: HashMap::new(),
            metrics: BTreeMap::new(),
        }
    }

    fn set_target(&mut self, target_name: Namespace, target_scope: Arc<MetricInput + Send + Sync>) {
        self.targets.insert(target_name.clone(), target_scope.clone());
        for (metric_name, metric) in self.metrics.range_mut(target_name.clone()..) {
            if let Some(metric) = metric.upgrade() {
                // check for range end
                if !metric_name.starts_with(&target_name) { break }

                // check if metric targeted by _lower_ namespace
                if metric.write_metric.borrow().1 > target_name.len() { continue }

                let target_metric = target_scope.define_metric(&metric.name, metric.kind);
                *metric.write_metric.borrow_mut() = (target_metric, target_name.len());
            }
        }
    }

    fn get_effective_target(&self, namespace: &Namespace)
            -> Option<(Arc<MetricInput + Send + Sync>, usize)> {
        if let Some(target) = self.targets.get(namespace) {
            return Some((target.clone(), namespace.len()));
        }

        // no 1:1 match, scan upper namespaces
        let mut name = namespace.clone();
        while let Some(_popped) = name.pop() {
            if let Some(target) = self.targets.get(&name) {
                return Some((target.clone(), name.len()))
            }
        }
        None
    }

    fn unset_target(&mut self, namespace: &Namespace) {
        if self.targets.remove(namespace).is_none() {
            // nothing to do
            return
        }

        let (up_target, up_nslen) = self.get_effective_target(namespace)
            .unwrap_or_else(|| (NO_METRIC_OUTPUT.open_scope(), 0));

        // update all affected metrics to next upper targeted namespace
        for (name, metric) in self.metrics.range_mut(namespace..) {
            // check for range end
            if !name.starts_with(namespace) { break }

            if let Some(mut metric) = metric.upgrade() {
                // check if metric targeted by _lower_ namespace
                if metric.write_metric.borrow().1 > namespace.len() { continue }

                let new_metric = up_target.define_metric(name, metric.kind);
                *metric.write_metric.borrow_mut() = (new_metric, up_nslen);
            }
        }
    }

    fn drop_metric(&mut self, name: &Namespace) {
        if self.metrics.remove(name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }

    fn flush(&self, namespace: &Namespace) -> error::Result<()> {
        if let Some((target, _nslen)) = self.get_effective_target(namespace) {
            target.flush()
        } else {
            Ok(())
        }
    }

}

impl MetricDispatch {

    /// Create a new "private" metric dispatch root. This is usually not what you want.
    /// Since this dispatch will not be part of the standard dispatch tree,
    /// it will need to be configured independently and since downstream code may not know about
    /// its existence this may never happen and metrics will not be dispatched anywhere.
    /// If you want to use the standard dispatch tree, use #metric_dispatch() instead.
    pub fn new() -> Self {
        MetricDispatch {
            namespace: ROOT_NS.clone(),
            inner: Arc::new(RwLock::new(InnerDispatch::new())),
        }
    }

    /// Replace target for this dispatch and it's children.
    pub fn set_target<IS: Into<Arc<MetricInput + Send + Sync + 'static>>>(&self, target: IS) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(self.namespace.clone(), target.into());
    }

    /// Replace target for this dispatch and it's children.
    pub fn unset_target(&self) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.unset_target(&self.namespace);
    }

}

impl<S: AsRef<str>> From<S> for MetricDispatch {
    fn from(name: S) -> MetricDispatch {
        MetricDispatch::new().with_prefix(name.as_ref())
    }
}

impl MetricInput for MetricDispatch {
    /// Lookup or create a dispatch stub for the requested metric.
    fn define_metric(&self, name: &Namespace, kind: Kind) -> WriteFn {
        let mut zname = self.namespace.clone();
        zname.extend(name);
        let mut inner = self.inner.write().expect("Dispatch Lock");
        let z = inner
            .metrics
            .get(&zname)
            // TODO validate that Kind & Sample match existing
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let (target, target_namespace_length) = inner.get_effective_target(name)
                    .unwrap_or_else(|| (NO_METRIC_OUTPUT.open_scope(), 0));
                let metric_object = target.define_metric(name, kind);
                let define_metric = Arc::new(DispatchMetric {
                    name: zname,
                    kind,
                    write_metric: AtomicRefCell::new((metric_object, target_namespace_length)),
                    dispatch: self.inner.clone(),
                });
                inner
                    .metrics
                    .insert(name.clone(), Arc::downgrade(&define_metric));
                define_metric
            });
        WriteFn::new(move |value| (z.write_metric.borrow().0)(value))
    }
}

impl Flush for MetricDispatch {
    fn flush(&self) -> error::Result<()> {
        self.inner.write().expect("Dispatch Lock").flush(&self.namespace)
    }

}

impl Namespaced for MetricDispatch {

    fn with_prefix(&self, name: &str) -> Self {
        MetricDispatch {
            namespace: self.namespace.with_prefix(name),
            inner: self.inner.clone()
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use dispatch::metric_dispatch;
    use test;
    use aggregate::MetricAggregator;
    use input::MetricInput;

    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        metric_dispatch().set_target(MetricAggregator::new());
        let metric = metric_dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let metric = metric_dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
