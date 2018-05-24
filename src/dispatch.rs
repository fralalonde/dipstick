//! Decouple metric definition from configuration with trait objects.

use core::*;
use scope::{DefineMetric, MetricScope, MetricInput, Flush, ScheduleFlush, NO_METRIC_SCOPE};

use std::collections::{HashMap, BTreeMap};
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

lazy_static! {
    static ref ROOT_DISPATCH: MetricDispatch = MetricDispatch::new();
}

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type Dispatch = Arc<DispatchMetric>;

/// Provides a copy of the default dispatcher's root.
pub fn metric_dispatch() -> MetricDispatch {
    ROOT_DISPATCH.clone()
}

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DispatchMetric {
    // basic info for this metric, needed to recreate new corresponding trait object if target changes
    name: Namespace,
    kind: Kind,
    rate: Sampling,

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
    targets: HashMap<Namespace, Arc<DefineMetric + Send + Sync>>,
    // last part of the namespace is the metric's name
    metrics: BTreeMap<Namespace, Weak<DispatchMetric>>,
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<&'static str> for MetricScope<Dispatch> {
    fn from(name: &'static str) -> MetricScope<Dispatch> {
        metric_dispatch().into_scope().with_suffix(name)
    }
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<()> for MetricScope<Dispatch> {
    fn from(_: ()) -> MetricScope<Dispatch> {
        metric_dispatch().into()
    }
}

impl From<MetricDispatch> for MetricScope<Dispatch> {
    fn from(dispatch: MetricDispatch) -> MetricScope<Dispatch> {
        dispatch.into_scope()
    }
}

impl InnerDispatch {

    fn new() -> Self {
        Self {
            targets: HashMap::new(),
            metrics: BTreeMap::new(),
        }
    }

    fn set_target(&mut self, target_name: Namespace, target_scope: Arc<DefineMetric + Send + Sync + 'static>) {
        self.targets.insert(target_name.clone(), target_scope.clone());
        for (metric_name, metric) in self.metrics.range_mut(target_name.clone()..) {
            if let Some(metric) = metric.upgrade() {
                // check for range end
                if !metric_name.starts_with(&target_name) { break }

                // check if metric targeted by _lower_ namespace
                if metric.write_metric.borrow().1 > target_name.len() { continue }

                let target_metric = target_scope
                    .define_metric_object(&metric.name, metric.kind, metric.rate);
                *metric.write_metric.borrow_mut() = (target_metric, target_name.len());
            }
        }
    }

    fn get_effective_target(&self, namespace: &Namespace)
            -> Option<(Arc<DefineMetric + Send + Sync>, usize)> {
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
            .unwrap_or_else(|| (NO_METRIC_SCOPE.clone(), 0));

        // update all affected metrics to next upper targeted namespace
        for (name, metric) in self.metrics.range_mut(namespace..) {
            // check for range end
            if !name.starts_with(namespace) { break }

            if let Some(mut metric) = metric.upgrade() {
                // check if metric targeted by _lower_ namespace
                if metric.write_metric.borrow().1 > namespace.len() { continue }

                let new_metric = up_target.define_metric_object(name, metric.kind, metric.rate);
                *metric.write_metric.borrow_mut() = (new_metric, up_nslen);
            }
        }
    }

    fn drop_metric(&mut self, name: &Namespace) {
        if self.metrics.remove(name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }

    fn flush(&self, namespace: &Namespace) {
        if let Some((target, _nslen)) = self.get_effective_target(namespace) {
            target.flush();
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
    pub fn set_target<IS: Into<Arc<DefineMetric + Send + Sync + 'static>>>(&self, target: IS) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(self.namespace.clone(), target.into());
    }

    /// Replace target for this dispatch and it's children.
    pub fn unset_target(&self) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.unset_target(&self.namespace);
    }

    fn into_scope(&self) -> MetricScope<Dispatch> {
        let disp_0 = self.clone();
        let disp_1 = self.clone();
        MetricScope::new(
            self.namespace.clone(),
            // define metric
            Arc::new(move |name, kind, rate| disp_0.define_metric(name, kind, rate)),
            // write / flush metric
            command_fn(move |cmd| match cmd {
                Command::Write(metric, value) => {
                    let dispatch: &Arc<DispatchMetric> = metric;
                    dispatch.write_metric.borrow().0(value);
                }
                Command::Flush => disp_1.inner.write().expect("Dispatch Lock").flush(&disp_1.namespace),
            }),
        )
    }

}

impl MetricInput<Dispatch> for MetricDispatch {

    /// Lookup or create a dispatch stub for the requested metric.
    fn define_metric(&self, name: &Namespace, kind: Kind, rate: Sampling) -> Dispatch {
        let mut zname = self.namespace.clone();
        zname.extend(name);
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner
            .metrics
            .get(&zname)
            // TODO validate that Kind & Sample match existing
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let (target, target_namespace_length) = inner.get_effective_target(name)
                    .unwrap_or_else(|| (NO_METRIC_SCOPE.clone(), 0));
                let metric_object = target.define_metric_object(name, kind, rate);
                let define_metric = Arc::new(DispatchMetric {
                    name: zname,
                    kind,
                    rate,
                    write_metric: AtomicRefCell::new((metric_object, target_namespace_length)),
                    dispatch: self.inner.clone(),
                });
                inner
                    .metrics
                    .insert(name.clone(), Arc::downgrade(&define_metric));
                define_metric
            })
    }

    #[inline]
    fn write(&self, metric: &Dispatch, value: Value) {
        metric.write_metric.borrow().0(value);
    }

    fn with_suffix(&self, name: &str) -> Self {
        if name.is_empty() {
            return self.clone()
        }
        MetricDispatch {
            namespace: self.namespace.with_suffix(name),
            inner: self.inner.clone()
        }
    }
}

//impl<'a> Index<&'a str> for MetricDispatch {
//    type Output = Self;
//
//    fn index(&self, index: &'a str) -> &Self::Output {
//        &self.push(index)
//    }
//}

impl Flush for MetricDispatch {
    fn flush(&self) {
        self.inner.write().expect("Dispatch Lock").flush(&self.namespace)
    }
}

impl ScheduleFlush for MetricDispatch {}

#[cfg(feature = "bench")]
mod bench {

    use dispatch::metric_dispatch;
    use test;
    use aggregate::MetricAggregator;
    use scope::MetricInput;

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
