//! Decouple metric definition from configuration with trait objects.

use core::*;
use namespace::*;
use scope::{self, DefineMetric, MetricScope, WriteMetric, MetricInput, Flush, ScheduleFlush, NO_METRIC_SCOPE};

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;


lazy_static! {
    static ref ROOT_DISPATCH: Arc<RwLock<InnerDispatch>> = Arc::new(RwLock::new(
        InnerDispatch {
            target: None,
            parent: None,
            metrics: HashMap::new(),
            children: HashMap::new(),
        }
    ));
}

/// Get the default dispatch point.
pub fn dispatch() -> MetricDispatch {
    MetricDispatch { inner: ROOT_DISPATCH.clone() }
}

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type Dispatch = Arc<DispatchMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DispatchMetric {
    kind: Kind,
    name: String,
    rate: Sampling,
    #[derivative(Debug = "ignore")]
    write_metric: AtomicRefCell<Box<WriteMetric + Send + Sync>>,
    #[derivative(Debug = "ignore")]
    dispatch: MetricDispatch,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for DispatchMetric {
    fn drop(&mut self) {
        self.dispatch.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct MetricDispatch {
    inner: Arc<RwLock<InnerDispatch>>,
}

struct InnerDispatch {
    target: Option<Arc<DefineMetric + Send + Sync>>,
    metrics: HashMap<String, Weak<DispatchMetric>>,
    parent: Option<Arc<RwLock<InnerDispatch>>>,
    children: HashMap<String, Arc<RwLock<InnerDispatch>>>,
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<&'static str> for MetricScope<Dispatch> {
    fn from(name: &'static str) -> MetricScope<Dispatch> {
        dispatch().into_scope().with_prefix(name)
    }
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<()> for MetricScope<Dispatch> {
    fn from(_: ()) -> MetricScope<Dispatch> {
        dispatch().into()
    }
}

impl From<MetricDispatch> for MetricScope<Dispatch> {
    fn from(dispatch: MetricDispatch) -> MetricScope<Dispatch> {
        dispatch.into_scope()
    }
}

impl InnerDispatch {
    fn switch_scope(&mut self, target_scope: Arc<DefineMetric + Send + Sync + 'static>) {
        for mut metric in self.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let target_metric = target_scope
                    .define_metric_object(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.write_metric.borrow_mut() = target_metric;
            }
        }
        for mut child in self.children.values() {
            let mut inner_child = child.write().expect("Dispatch Lock");
            inner_child.parent_set_target(target_scope.clone())
        }
    }

    fn get_parent_target(&self) -> Option<Arc<DefineMetric + Send + Sync + 'static>> {
        self.parent.clone().and_then(|p| p.read().expect("Dispatch Lock").target.clone())
    }

    fn set_target(&mut self, target: Option<Arc<DefineMetric + Send + Sync + 'static>>) {
        let new_scope = target.clone().unwrap_or_else(|| self.get_parent_target().unwrap_or(NO_METRIC_SCOPE.clone()));
        self.switch_scope(new_scope);
        self.target = target
    }

    fn parent_set_target(&mut self, target: Arc<DefineMetric + Send + Sync + 'static>) {
        if self.target.is_none() {
            // overriding target from this point downward
            self.switch_scope(target)

        }
    }

}

impl MetricDispatch {
    /// Replace target for this dispatch and it's children.
    pub fn set_target<IS: Into<Arc<DefineMetric + Send + Sync + 'static>>>(&self, target: IS) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(Some(target.into()));
    }

    /// Remove target.
    pub fn unset_target(&self) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        inner.set_target(None);
    }

    fn into_scope(&self) -> MetricScope<Dispatch> {
        let disp_0 = self.clone();
        let disp_1 = self.clone();
        MetricScope::new(
            // define metric
            Arc::new(move |kind, name, rate| disp_0.define_metric(kind, name, rate)),
            // write / flush metric
            command_fn(move |cmd| match cmd {
                Command::Write(metric, value) => {
                    let dispatch: &Arc<DispatchMetric> = metric;
                    dispatch.write_metric.borrow().write(value);
                }
                Command::Flush => if let Some(ref mut target) = disp_1.inner.write().expect("Dispatch Lock").target {
                    target.flush()
                }
            }),
        )
    }

    fn drop_metric(&self, metric: &DispatchMetric) {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        if inner.metrics.remove(&metric.name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }
}

impl MetricInput<Dispatch> for MetricDispatch {
    /// Define an event counter of the provided name.
    fn marker(&self, name: &str) -> scope::Marker {
        self.into_scope().marker(name)
    }

    /// Define a counter of the provided name.
    fn counter(&self, name: &str) -> scope::Counter {
        self.into_scope().counter(name)
    }

    /// Define a timer of the provided name.
    fn timer(&self, name: &str) -> scope::Timer {
        self.into_scope().timer(name)
    }

    /// Define a gauge of the provided name.
    fn gauge(&self, name: &str) -> scope::Gauge {
        self.into_scope().gauge(name)
    }

    /// Lookup or create a scoreboard for the requested metric.
    fn define_metric(&self, kind: Kind, name: &str, rate: Sampling) -> Dispatch {
        let mut inner = self.inner.write().expect("Dispatch Lock");
        let target_scope = inner.target.clone().unwrap_or(NO_METRIC_SCOPE.clone());
        inner
            .metrics
            .get(name)
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let metric_object = target_scope.define_metric_object(kind, name, rate);
                let define_metric = Arc::new(DispatchMetric {
                    kind,
                    name: name.to_string(),
                    rate,
                    write_metric: AtomicRefCell::new(metric_object),
                    dispatch: self.clone(),
                });
                inner
                    .metrics
                    .insert(define_metric.name.clone(), Arc::downgrade(&define_metric));
                define_metric
            })
    }

    #[inline]
    fn write(&self, metric: &Dispatch, value: Value) {
        metric.write_metric.borrow().write(value);
    }
}

impl Flush for MetricDispatch {
    fn flush(&self) {
        if let Some(ref target) = self.inner.write().expect("Dispatch Lock").target {
            target.flush()
        }
    }
}

impl ScheduleFlush for MetricDispatch {}

#[cfg(feature = "bench")]
mod bench {

    use dispatch::dispatch;
    use test;
    use aggregate::new_aggregate;
    use scope::MetricInput;


    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        dispatch().set_target(new_aggregate());
        let metric = dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let metric = dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
