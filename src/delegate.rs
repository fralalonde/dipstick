//! Decouple metric definition from configuration with trait objects.

use core::*;
use metrics::*;
use namespace::*;
use registry;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;


/// Create a new dispatch point for metrics.
/// All dispatch points are automatically entered in the dispatch registry.
pub fn delegate_metrics() -> MetricsSend {
    let send = MetricsSend {
        inner: Arc::new(RwLock::new(InnerMetricsSend {
            metrics: HashMap::new(),
            recv: registry::get_default_metrics_recv(),
        })),
    };
    registry::add_metrics_send(send.clone());
    send
}

/// Dynamic counterpart of a `Dispatcher`.
/// Adapter to AppMetrics<_> of unknown type.
pub trait MetricsRecv {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Box<RecvMetric + Send + Sync>;

    /// Flush the receiver's scope.
    fn flush(&self);
}

/// Dynamic counterpart of the `DispatcherMetric`.
/// Adapter to a metric of unknown type.
pub trait RecvMetric {
    /// Write metric value to a scope.
    /// Observers only receive previously registered handles.
    fn write(&self, value: Value);
}

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type Delegate = Arc<SendMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SendMetric {
    kind: Kind,
    name: String,
    rate: Rate,
    #[derivative(Debug = "ignore")]
    recv_metric: AtomicRefCell<Box<RecvMetric + Send + Sync>>,
    #[derivative(Debug = "ignore")]
    send: MetricsSend,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for SendMetric {
    fn drop(&mut self) {
        self.send.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct MetricsSend {
    inner: Arc<RwLock<InnerMetricsSend>>,
}

struct InnerMetricsSend {
    metrics: HashMap<String, Weak<SendMetric>>,
    recv: Arc<MetricsRecv + Send + Sync>,
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<&'static str> for Metrics<Delegate> {
    fn from(prefix: &'static str) -> Metrics<Delegate> {
        let app_metrics: Metrics<Delegate> = delegate_metrics().into();
        app_metrics.with_prefix(prefix)
    }
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<()> for Metrics<Delegate> {
    fn from(_: ()) -> Metrics<Delegate> {
        let app_metrics: Metrics<Delegate> = delegate_metrics().into();
        app_metrics
    }
}


impl From<MetricsSend> for Metrics<Delegate> {
    fn from(send: MetricsSend) -> Metrics<Delegate> {
        let send_cmd = send.clone();
        Metrics::new(
            // define metric
            Arc::new(move |kind, name, rate| send.define_metric(kind, name, rate)),

            // write / flush metric
            control_scope(move |cmd| match cmd {
                ScopeCmd::Write(metric, value) => {
                    let dispatch: &Arc<SendMetric> = metric;
                    dispatch.recv_metric.borrow().write(value);
                }
                ScopeCmd::Flush => send_cmd.inner.write().expect("Locking Delegate").recv.flush(),
            }),
        )
    }
}

impl MetricsSend {
    /// Install a new metric receiver, replacing the previous one.
    pub fn set_receiver<R: MetricsRecv + Send + Sync + 'static>(&self, recv: Arc<R>) {
        let inner = &mut self.inner.write().expect("Lock Metrics Send");

        for mut metric in inner.metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let recv_metric = recv.define_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.recv_metric.borrow_mut() = recv_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.recv = recv.clone()
    }

    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> Delegate {
        let mut inner = self.inner.write().expect("Lock Metrics Send");
        inner.metrics.get(name)
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let recv_metric = inner.recv.define_metric(kind, name, rate);
                let new_metric = Arc::new(SendMetric {
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

    fn drop_metric(&self, metric: &SendMetric) {
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
        let dispatch = delegate_metrics();
        let sink: Metrics<Delegate> = dispatch.clone().into();
        dispatch.set_receiver(aggregate(summary, to_void()));
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let dispatch = delegate_metrics();
        let sink: Metrics<Delegate> = dispatch.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
