//! Decouple metric definition from configuration with trait objects.

use core::*;
use app_metrics::*;
use output::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

/// Create a new dispatcher.
// TODO add dispatch name for registry
pub fn dispatch() -> DispatchPoint {
    DispatchPoint {
        inner: Arc::new(RwLock::new(InnerDispatcher {
            metrics: HashMap::new(),
            receiver: Box::new(app_metrics(to_void())),
        }))
    }
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
pub type Dispatch = Arc<DispatcherMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DispatcherMetric {
    kind: Kind,
    name: String,
    rate: Rate,
    #[derivative(Debug = "ignore")]
    receiver: AtomicRefCell<Box<ReceiverMetric + Send + Sync>>,
    #[derivative(Debug = "ignore")]
    dispatcher: DispatchPoint,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for DispatcherMetric {
    fn drop(&mut self) {
        self.dispatcher.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct DispatchPoint {
    inner: Arc<RwLock<InnerDispatcher>>,
}

struct InnerDispatcher {
    metrics: HashMap<String, Weak<DispatcherMetric>>,
    receiver: Box<Receiver + Send + Sync>,
}

impl From<DispatchPoint> for AppMetrics<Dispatch> {
    fn from(dispatcher: DispatchPoint) -> AppMetrics<Dispatch> {
        let dispatcher_1 = dispatcher.clone();
        AppMetrics::new(
            // define metric
            Arc::new(move |kind, name, rate| dispatcher.define_metric(kind, name, rate)),

            // write / flush metric
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let dispatch: &Arc<DispatcherMetric> = metric;
                    let receiver_metric: AtomicRef<Box<ReceiverMetric + Send + Sync>> = dispatch.receiver.borrow();
                    receiver_metric.write(value)
                },
                ScopeCmd::Flush => {
                    dispatcher_1.inner.write().expect("Locking dispatcher").receiver.flush()
                },
            })
        )
    }
}

impl DispatchPoint {

    /// Install a new metric receiver, replacing the previous one.
    pub fn set_receiver<IS: Into<AppMetrics<T>>, T: Send + Sync + Clone + 'static>(&self, receiver: IS) {
        let receiver: Box<Receiver + Send + Sync> = Box::new(receiver.into());
        let inner: &mut InnerDispatcher = &mut *self.inner.write().expect("Locking dispatcher");

        for mut metric in inner.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let receiver_metric = receiver.box_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.receiver.borrow_mut() = receiver_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.receiver = receiver;
    }

    /// Define a dispatch metric, registering it with the current receiver.
    /// A weak ref is kept to update receiver metric if receiver is replaced.
    pub fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Dispatch {
        let mut inner = self.inner.write().expect("Locking dispatcher");

        let receiver_metric = inner.receiver.box_metric(kind, name, rate);

        let dispatcher_metric = Arc::new(DispatcherMetric {
            kind,
            name: name.to_string(),
            rate,
            receiver: AtomicRefCell::new(receiver_metric),
            dispatcher: self.clone(),
        });

        inner.metrics.insert(dispatcher_metric.name.clone(), Arc::downgrade(&dispatcher_metric));
        dispatcher_metric
    }

    fn drop_metric(&self, metric: &DispatcherMetric) {
        let mut inner = self.inner.write().expect("Locking dispatcher");
        if let None = inner.metrics.remove(&metric.name) {
            panic!("Could not remove DispatchMetric weak ref from Dispatcher")
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
        let dispatch = dispatch();
        let sink: AppMetrics<Dispatch> = dispatch.clone().into();
        dispatch.set_receiver(aggregate(summary, to_void()));
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let dispatch = dispatch();
        let sink: AppMetrics<Dispatch> = dispatch.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
