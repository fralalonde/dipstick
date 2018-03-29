//! Decouple metric definition from configuration with trait objects.

use core::*;
use scope::{MetricScope, DefineMetric, WriteMetric, NO_METRICS_SCOPE};

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

/// Define delegate metrics.
#[macro_export]
macro_rules! dispatch_metrics {
    (pub $METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {<Dispatch> pub $METRIC_ID = $e; }
    };
    (pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Dispatch> pub $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($METRIC_ID:ident = $e:expr $(;)*) => {
        metrics! {<Dispatch> $METRIC_ID = $e; }
    };
    ($METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Dispatch> $METRIC_ID = $e => { $($REMAINING)* } }
    };
    ($METRIC_ID:ident => { $($REMAINING:tt)+ }) => {
        metrics! {<Dispatch> $METRIC_ID => { $($REMAINING)* } }
    };
    ($e:expr => { $($REMAINING:tt)+ }) => {
        metrics! {<Dispatch> $e => { $($REMAINING)* } }
    };
}

lazy_static! {
    static ref DISPATCHER_REGISTRY: RwLock<HashMap<String, Arc<RwLock<InnerDispatch>>>> = RwLock::new(HashMap::new());
    static ref DEFAULT_DISPATCH_SCOPE: RwLock<Arc<DefineMetric + Sync + Send>> = RwLock::new(NO_METRICS_SCOPE.clone());
}

/// Install a new receiver for all dispatched metrics, replacing any previous receiver.
pub fn set_dispatch_default<IS: Into<MetricScope<T>>, T: Send + Sync + Clone + 'static>(into_scope: IS) {
    let new_scope = Arc::new(into_scope.into());
    for inner in DISPATCHER_REGISTRY.read().unwrap().values() {
        MetricDispatch {inner: inner.clone()}.set_scope(new_scope.clone());
    }
    *DEFAULT_DISPATCH_SCOPE.write().unwrap() = new_scope;
}

/// Get the named dispatch point.
/// Uses the stored instance if it already exists, otherwise creates it.
/// All dispatch points are automatically entered in the dispatch registry and kept FOREVER.
pub fn dispatch_name(name: &str) -> MetricScope<Dispatch> {
    let inner = DISPATCHER_REGISTRY.write().expect("Dispatch Registry")
        .entry(name.into())
        .or_insert_with(|| {
            Arc::new(RwLock::new(InnerDispatch {
                metrics: HashMap::new(),
                scope: DEFAULT_DISPATCH_SCOPE.read().unwrap().clone(),
            }))})
        .clone();
    MetricDispatch { inner }.into()
}

/// Get the default dispatch point.
pub fn dispatch() -> MetricScope<Dispatch> {
    dispatch_name("_DEFAULT")
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
    metrics: HashMap<String, Weak<DispatchMetric>>,
    scope: Arc<DefineMetric + Send + Sync>,
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<&'static str> for MetricScope<Dispatch> {
    fn from(prefix: &'static str) -> MetricScope<Dispatch> {
        dispatch_name(prefix).into()
    }
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<()> for MetricScope<Dispatch> {
    fn from(_: ()) -> MetricScope<Dispatch> {
        let app_metrics: MetricScope<Dispatch> = dispatch_name("").into();
        app_metrics
    }
}

impl From<MetricDispatch> for MetricScope<Dispatch> {
    fn from(send: MetricDispatch) -> MetricScope<Dispatch> {
        let send_cmd = send.clone();
        MetricScope::new(
            // define metric
            Arc::new(move |kind, name, rate| send.define_metric(kind, name, rate)),

            // write / flush metric
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let dispatch: &Arc<DispatchMetric> = metric;
                    dispatch.write_metric.borrow().write(value);
                }
                ScopeCmd::Flush => send_cmd.inner.write().expect("Locking Delegate").scope.flush(),
            }),
        )
    }
}

impl MetricDispatch {
    /// Install a new metric receiver, replacing the previous one.
    pub fn set_scope<R: DefineMetric + Send + Sync + 'static>(&self, recv: Arc<R>) {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        for mut metric in inner.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let recv_metric = recv.define_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.write_metric.borrow_mut() = recv_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.scope = recv.clone()
        
    }

    fn define_metric(&self, kind: Kind, name: &str, rate: Sampling) -> Dispatch {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        inner.metrics.get(name)
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let recv_metric = inner.scope.define_metric(kind, name, rate);
                let define_metric = Arc::new(DispatchMetric {
                    kind,
                    name: name.to_string(),
                    rate,
                    write_metric: AtomicRefCell::new(recv_metric),
                    dispatch: self.clone(),
                });
                inner.metrics.insert(
                    define_metric.name.clone(),
                    Arc::downgrade(&define_metric),
                );
                define_metric
            })
    }

    fn drop_metric(&self, metric: &DispatchMetric) {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        if inner.metrics.remove(&metric.name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use dispatch;
    use test;
    use aggregate::aggregate;

    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        dispatch::set_dispatch_default(aggregate());
        let metric = super::dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let metric = dispatch::dispatch().marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
