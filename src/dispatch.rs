//! Decouple metric definition from configuration with trait objects.

use core::{Namespace, WithPrefix, Kind, MetricInput, WriteFn, NO_METRIC_OUTPUT, Flush, WithAttributes, Attributes};
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
pub struct MetricProxy {
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
impl Drop for MetricProxy {
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
    attributes: Attributes,
    inner: Arc<RwLock<InnerDispatch>>,
}

struct InnerDispatch {
    // namespaces can target one, many or no metrics
    targets: HashMap<Namespace, Arc<MetricInput + Send + Sync>>,
    // last part of the namespace is the metric's name
    metrics: BTreeMap<Namespace, Weak<MetricProxy>>,
}
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
            attributes: Attributes::default(),
            inner: Arc::new(RwLock::new(InnerDispatch::new())),
        }
    }

    /// Replace target for this dispatch and it's children.
    pub fn set_target<IS: Into<Arc<MetricInput + Send + Sync + 'static>>>(&self, target: IS) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(self.get_namespace().clone(), target.into());
    }

    /// Replace target for this dispatch and it's children.
    pub fn unset_target(&self) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.unset_target(self.get_namespace());
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
        let name = self.qualified_name(name);
        let mut inner = self.inner.write().expect("Dispatch Lock");
        let proxy = inner
            .metrics
            .get(&name)
            // TODO validate that Kind matches existing
            .and_then(|proxy_ref| Weak::upgrade(proxy_ref))
            .unwrap_or_else(|| {
                let name2 = name.clone();
                // not found, define new
                let (target, target_namespace_length) = inner.get_effective_target(&name)
                    .unwrap_or_else(|| (NO_METRIC_OUTPUT.open_scope(), 0));
                let metric_object = target.define_metric(&name, kind);
                let proxy = Arc::new(MetricProxy {
                    name,
                    kind,
                    write_metric: AtomicRefCell::new((metric_object, target_namespace_length)),
                    dispatch: self.inner.clone(),
                });
                inner.metrics.insert(name2, Arc::downgrade(&proxy));
                proxy
            });
        WriteFn::new(move |value| (proxy.write_metric.borrow().0)(value))
    }
}

impl Flush for MetricDispatch {
    fn flush(&self) -> error::Result<()> {
        self.inner.write().expect("Dispatch Lock").flush(self.get_namespace())
    }
}

impl WithAttributes for MetricDispatch {
    fn get_attributes(&self) -> &Attributes { &self.attributes }
    fn mut_attributes(&mut self) -> &mut Attributes { &mut self.attributes }
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
