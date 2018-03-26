//! Decouple metric definition from configuration with trait objects.

use core::*;
use app_metrics::*;
use namespace::*;
use registry;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;


/// Create a new dispatch point for metrics.
/// All dispatch points are automatically entered in the dispatch registry.
pub fn app_delegate() -> AppSend {
    let send = AppSend {
        inner: Arc::new(RwLock::new(InnerAppSend {
            metrics: HashMap::new(),
            recv: registry::get_default_app_recv(),
        })),
    };
    registry::add_app_send(send.clone());
    send
}

/// Dynamic counterpart of a `Dispatcher`.
/// Adapter to AppMetrics<_> of unknown type.
pub trait AppRecv {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Box<AppRecvMetric + Send + Sync>;

    /// Flush the receiver's scope.
    fn flush(&self);
}

/// Dynamic counterpart of the `DispatcherMetric`.
/// Adapter to a metric of unknown type.
pub trait AppRecvMetric {
    /// Write metric value to a scope.
    /// Observers only receive previously registered handles.
    fn write(&self, value: Value);
}

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type Delegate = Arc<AppSendMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct AppSendMetric {
    kind: Kind,
    name: String,
    rate: Rate,
    #[derivative(Debug = "ignore")]
    recv_metric: AtomicRefCell<Box<AppRecvMetric + Send + Sync>>,
    #[derivative(Debug = "ignore")]
    send: AppSend,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for AppSendMetric {
    fn drop(&mut self) {
        self.send.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct AppSend {
    inner: Arc<RwLock<InnerAppSend>>,
}

struct InnerAppSend {
    metrics: HashMap<String, Weak<AppSendMetric>>,
    recv: Arc<AppRecv + Send + Sync>,
}

impl From<&'static str> for AppMetrics<Delegate> {
    fn from(prefix: &'static str) -> AppMetrics<Delegate> {
        let app_metrics: AppMetrics<Delegate> = app_delegate().into();
        app_metrics.with_prefix(prefix)
    }
}

impl From<AppSend> for AppMetrics<Delegate> {
    fn from(send: AppSend) -> AppMetrics<Delegate> {
        let send_cmd = send.clone();
        AppMetrics::new(
            // define metric
            Arc::new(move |kind, name, rate| send.define_metric(kind, name, rate)),

            // write / flush metric
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let dispatch: &Arc<AppSendMetric> = metric;
                    dispatch.recv_metric.borrow().write(value);
//                    let recv_metric: AtomicRef<Box<AppRecvMetric + Send + Sync>> = dispatch.recv_metric.borrow();
//                    recv_metric.write(value)
                }
                ScopeCmd::Flush => send_cmd.inner.write().expect("Locking Delegate").recv.flush(),
            }),
        )
    }
}

impl AppSend {
    /// Install a new metric receiver, replacing the previous one.
    pub fn set_receiver<IS: Into<AppMetrics<T>>, T: Send + Sync + Clone + 'static>(
        &self,
        receiver: IS,
    ) {
        let receiver: Arc<AppRecv + Send + Sync> = Arc::new(receiver.into());
        let inner: &mut InnerAppSend =
            &mut *self.inner.write().expect("Lock Metrics Send");

        for mut metric in inner.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let receiver_metric =
                    receiver.define_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.recv_metric.borrow_mut() = receiver_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.recv = receiver;
    }

    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Delegate {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        inner.metrics.get(name)
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let recv_metric = inner.recv.define_metric(kind, name, rate);
                let new_metric = Arc::new(AppSendMetric {
                    kind,
                    name: name.to_string(),
                    rate,
                    recv_metric: AtomicRefCell::new(recv_metric),
                    send: self.clone(),
                });
                inner.metrics.insert(
                    new_metric.name.clone(),
                    Arc::downgrade(&new_metric),
                );
                new_metric
            })
    }

    fn drop_metric(&self, metric: &AppSendMetric) {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        if inner.metrics.remove(&metric.name).is_none() {
            panic!("Could not remove DelegatingMetric weak ref from delegation point")
        }
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;
    use aggregate::*;
    use publish::*;
    use output::*;

    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        let dispatch = app_delegate();
        let sink: AppMetrics<Delegate> = dispatch.clone().into();
        dispatch.set_receiver(aggregate(summary, to_void()));
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let dispatch = app_delegate();
        let sink: AppMetrics<Delegate> = dispatch.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
