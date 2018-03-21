//! Decouple metric definition from configuration with trait objects.

use core::*;
use local_metrics::*;
use output::*;
use namespace::*;
use registry::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;


/// Create a new dispatch point for metrics.
/// All dispatch points are automatically entered in the dispatch registry.
pub fn local_delegate() -> LocalSend {
    let delegation_point = LocalSend {
        inner_send: Arc::new(RwLock::new(InnerLocalSend {
            active_metrics: HashMap::new(),
            recv: registry.get,
            last_metric_id: 0,
        })),
    };
    register_local_delegation(delegation_point.clone());
    delegation_point
}

/// Dynamic counterpart of a `Dispatcher`.
/// Adapter to LocalMetrics<_> of unknown type.
pub trait LocalRecv {
    /// Register a new metric.
    /// Only one metric of a certain name will be defined.
    /// Observer must return a MetricHandle that uniquely identifies the metric.
    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> LocalRecvMetric;

    /// Flush the receiver's scope.
    fn open_scope(&self, buffered: bool) -> Arc<LocalRecvScope + Send + Sync>;
}

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub trait LocalRecvScope {
    fn write(&self, metric: &LocalRecvMetric, value: Value);
    fn flush(&self);
}

pub struct LocalRecvMetric (u64);

/// Shortcut name because `AppMetrics<Dispatch>`
/// looks better than `AppMetrics<Arc<DispatcherMetric>>`.
pub type LocalDelegate = Arc<LocalSendMetric>;

/// A dynamically dispatched metric.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LocalSendMetric {
    kind: Kind,
    name: String,
    rate: Rate,
    metric_id: usize,
    #[derivative(Debug = "ignore")]
    send: LocalSend,
}

/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for LocalSendMetric {
    fn drop(&mut self) {
        self.dispatcher.drop_metric(self)
    }
}


/// Dispatcher weak ref does not prevent dropping but still needs to be cleaned out.
impl Drop for LocalSendMetric {
    fn drop(&mut self) {
        self.send.drop_metric(self)
    }
}

/// A dynamic dispatch point for app and lib metrics.
/// Decouples metrics definition from backend configuration.
/// Allows defining metrics before a concrete type has been selected.
/// Allows replacing metrics backend on the fly at runtime.
#[derive(Clone)]
pub struct LocalSend {
    inner_send: Arc<RwLock<InnerLocalSend>>,
}

struct InnerLocalSend {
    recv: Box<LocalRecv + Send + Sync>,
    active_metrics: HashMap<String, Weak<LocalSendMetric>>,
    last_metric_id: usize,
}

impl From<&'static str> for LocalMetrics<LocalDelegate> {
    fn from(prefix: &'static str) -> LocalMetrics<LocalDelegate> {
        let app_metrics: LocalMetrics<LocalDelegate> = local_delegate().into();
        app_metrics.with_prefix(prefix)
    }
}

impl From<LocalSend> for LocalMetrics<LocalDelegate> {
    fn from(send: LocalSend) -> LocalMetrics<LocalDelegate> {
        let send_1 = send.clone();
        LocalMetrics::new(
            // define metric
            Arc::new(move |kind, name, rate| send.define_metric(kind, name, rate)),
            // write / flush metric
            Arc::new(move |buffered| send.open_scope(buffered))
        )
    }
}

impl LocalSend {
    /// Install a new metric receiver, replacing the previous one.
    pub fn set_receiver<IS: Into<LocalMetrics<T>>, T: Send + Sync + Clone + 'static>(
        &self,
        receiver: IS,
    ) {
        let receiver: Box<LocalRecv + Send + Sync> = Box::new(receiver.into());
        let inner: &mut InnerLocalSend =
            &mut *self.inner_send.write().expect("Lock Metrics Send");

        for mut metric in inner.active_metrics.values() {
            if let Some(metric) = metric.upgrade() {
                let receiver_metric =
                    receiver.box_metric(metric.kind, metric.name.as_ref(), metric.rate);
                *metric.receiver.borrow_mut() = receiver_metric;
            }
        }
        // TODO return old receiver (swap, how?)
        inner.recv = receiver;
    }

    fn define_metric(&self, kind: Kind, name: &str, rate: Rate) -> LocalDelegate {
        let mut inner = self.inner_send.write().expect("Lock Metrics Send");
        inner.metrics.get(name)
            .and_then(|metric_ref| Weak::upgrade(metric_ref))
            .unwrap_or_else(|| {
                let recv_metric = inner.recv.define_metric(kind, name, rate);
                let new_metric = Arc::new(LocalSendMetric {
                    kind,
                    name: name.to_string(),
                    rate,
                    metric_id: inner.last_metric_id += 1,
                    send: send.clone(),
                });
                inner.metrics.insert(
                    new_metric.name.clone(),
                    Arc::downgrade(&new_metric),
                );
                new_metric
            })
    }


    pub fn open_scope(&self, buffered: bool) -> Arc<ControlScopeFn<LocalDelegate>> {
        let mut inner = self.inner_send.write().expect("Lock Metrics Send");
        let write_scope = inner.recv.open_scope(buffered);
        let flush_scope = write_scope.clone();
        Arc::new(move |cmd| {
            match cmd {
                ScopeCmd::Write(metric, value) => write_scope.write(metric, value),
                ScopeCmd::Flush => flush_scope.flush(),
            }
        })
    }

    fn drop_metric(&self, metric: &LocalSendMetric) {
        let mut inner = self.inner_send.write().expect("Lock Metrics Send");
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
        let dispatch = local_delegate();
        let sink: LocalMetrics<LocalDelegate> = dispatch.clone().into();
        dispatch.set_receiver(aggregate(summary, to_void()));
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let dispatch = local_delegate();
        let sink: LocalMetrics<LocalDelegate> = dispatch.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
