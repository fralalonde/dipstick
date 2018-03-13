//! Decouple metric definition from configuration with trait objects.

use core::*;
use app_metrics::*;
use output::*;
use namespace::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

/// The registry contains a list of every metrics dispatch point in the app.
lazy_static! {
    static ref DELEGATE_REGISTRY: RwLock<Vec<DelegationPoint>> = RwLock::new(vec![]);
}

/// Install a new receiver for all dispatched metrics, replacing any previous receiver.
pub fn send_delegated_metrics<IS: Into<AppMetrics<T>>, T: Send + Sync + Clone + 'static>(
    receiver: IS,
) {
    let rec = receiver.into();
    for d in DELEGATE_REGISTRY.read().unwrap().iter() {
        d.set_receiver(rec.clone());
    }
}

/// Create a new dispatch point for metrics.
/// All dispatch points are automatically entered in the dispatch registry.
pub fn delegate() -> DelegationPoint {
    let delegation_point = DelegationPoint {
        inner: Arc::new(RwLock::new(InnerDelegationPoint {
            metrics: HashMap::new(),
            receiver: Box::new(app_metrics(to_void())),
        })),
    };
    DELEGATE_REGISTRY
        .write()
        .unwrap()
        .push(delegation_point.clone());
    delegation_point
}

/// Dynamic counterpart of a `Dispatcher`.
/// Adapter to AppMetrics<_> of unknown type.
pub trait Receiver {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn box_metric(&self, kind: Kind, name: &str, rate: Rate) -> Box<ReceiverMetric + Send + Sync>;

    /// Flush the receiver's scope.
    fn flush(&self);
}

/// Dynamic counterpart of the `DispatcherMetric`.
/// Adapter to a metric of unknown type.
pub trait ReceiverMetric {
    /// Write metric value to a scope.
    /// Observers only receive previously registered handles.
    fn write(&self, value: Value);
}

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type Delegate = Arc<DelegatingMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DelegatingMetric {
    kind: Kind,
    name: String,
    rate: Rate,
    #[derivative(Debug = "ignore")]
    receiver: AtomicRefCell<Box<ReceiverMetric + Send + Sync>>,
    #[derivative(Debug = "ignore")]
    dispatcher: DelegationPoint,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for DelegatingMetric {
    fn drop(&mut self) {
        self.dispatcher.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct DelegationPoint {
    inner: Arc<RwLock<InnerDelegationPoint>>,
}

struct InnerDelegationPoint {
    metrics: HashMap<String, Weak<DelegatingMetric>>,
    receiver: Box<Receiver + Send + Sync>,
}

impl From<&'static str> for AppMetrics<Delegate> {
    fn from(prefix: &'static str) -> AppMetrics<Delegate> {
        let app_metrics: AppMetrics<Delegate> = delegate().into();
        app_metrics.with_prefix(prefix)
    }
}

impl From<DelegationPoint> for AppMetrics<Delegate> {
    fn from(dispatcher: DelegationPoint) -> AppMetrics<Delegate> {
        let dispatcher_1 = dispatcher.clone();
        AppMetrics::new(
            // define metric
            Arc::new(move |kind, name, rate| dispatcher.define_metric(kind, name, rate)),
            // write / flush metric
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let dispatch: &Arc<DelegatingMetric> = metric;
                    let receiver_metric: AtomicRef<
                        Box<ReceiverMetric + Send + Sync>,
                    > = dispatch.receiver.borrow();
                    receiver_metric.write(value)
                }
                ScopeCmd::Flush => dispatcher_1
                    .inner
                    .write()
                    .expect("Locking dispatcher")
                    .receiver
                    .flush(),
            }),
        )
    }
}

impl DelegationPoint {
    /// Install a new metric receiver, replacing the previous one.
    pub fn set_receiver<IS: Into<AppMetrics<T>>, T: Send + Sync + Clone + 'static>(
        &self,
        receiver: IS,
    ) {
        let receiver: Box<Receiver + Send + Sync> = Box::new(receiver.into());
        let inner: &mut InnerDelegationPoint =
            &mut *self.inner.write().expect("Locking dispatcher");

        for mut metric in inner.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let receiver_metric =
                    receiver.box_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.receiver.borrow_mut() = receiver_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.receiver = receiver;
    }

    /// Define a dispatch metric, registering it with the current receiver.
    /// A weak ref is kept to update receiver metric if receiver is replaced.
    pub fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Delegate {
        let mut inner = self.inner.write().expect("Locking dispatcher");

        let receiver_metric = inner.receiver.box_metric(kind, name, rate);

        let delegating_metric = Arc::new(DelegatingMetric {
            kind,
            name: name.to_string(),
            rate,
            receiver: AtomicRefCell::new(receiver_metric),
            dispatcher: self.clone(),
        });

        inner.metrics.insert(
            delegating_metric.name.clone(),
            Arc::downgrade(&delegating_metric),
        );
        delegating_metric
    }

    fn drop_metric(&self, metric: &DelegatingMetric) {
        let mut inner = self.inner.write().expect("Locking delegation point");
        if inner.metrics.remove(&metric.name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;
    use core::Kind::*;
    use aggregate::*;
    use publish::*;

    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        let dispatch = delegate();
        let sink: AppMetrics<Delegate> = dispatch.clone().into();
        dispatch.set_receiver(aggregate(summary, to_void()));
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let dispatch = delegate();
        let sink: AppMetrics<Delegate> = dispatch.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
