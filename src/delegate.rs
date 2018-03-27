//! Decouple metric definition from configuration with trait objects.

use core::*;
use metrics::{MetricScope, DefineMetric, WriteMetric, NO_RECV_METRICS};
use namespace::*;

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Weak};

use atomic_refcell::*;

/// Define delegate metrics.
#[macro_export]
macro_rules! delegate_metrics {
    (pub $METRIC_ID:ident = $e:expr $(;)*) => { metrics! {<Delegate> pub $METRIC_ID = $e; } };
    (pub $METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => { metrics! {<Delegate> pub $METRIC_ID = $e => { $($REMAINING)* } } };
    ($METRIC_ID:ident = $e:expr $(;)*) => { metrics! {<Delegate> $METRIC_ID = $e; } };
    ($METRIC_ID:ident = $e:expr => { $($REMAINING:tt)+ }) => { metrics! {<Delegate> $METRIC_ID = $e => { $($REMAINING)* } } };
    ($METRIC_ID:ident => { $($REMAINING:tt)+ }) => { metrics! {<Delegate> $METRIC_ID => { $($REMAINING)* } } };
    ($e:expr => { $($REMAINING:tt)+ }) => { metrics! {<Delegate> $e => { $($REMAINING)* } } };
}

lazy_static! {
    pub static ref DELEGATE_REGISTRY: RwLock<Vec<MetricsSend>> = RwLock::new(vec![]);
    pub static ref DEFAULT_METRICS: RwLock<Arc<DefineMetric + Sync + Send>> = RwLock::new(NO_RECV_METRICS.clone());
}

/// Install a new receiver for all dispatched metrics, replacing any previous receiver.
pub fn set_default_metric_scope<IS: Into<MetricScope<T>>, T: Send + Sync + Clone + 'static>(into_recv: IS) {
    let recv = Arc::new(into_recv.into());
    for d in DELEGATE_REGISTRY.read().unwrap().iter() {
        d.set_receiver(recv.clone());
    }
    *DEFAULT_METRICS.write().unwrap() = recv;
}

/// Create a new dispatch point for metrics.
/// All dispatch points are automatically entered in the dispatch registry.
pub fn delegate_metrics() -> MetricsSend {
    let send = MetricsSend {
        inner: Arc::new(RwLock::new(InnerMetricsSend {
            metrics: HashMap::new(),
            recv: DEFAULT_METRICS.read().unwrap().clone(),
        })),
    };
    DELEGATE_REGISTRY.write().unwrap().push(send.clone());
    send
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
    rate: Sampling,
    #[derivative(Debug = "ignore")]
    recv_metric: AtomicRefCell<Box<WriteMetric + Send + Sync>>,
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
    recv: Arc<DefineMetric + Send + Sync>,
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<&'static str> for MetricScope<Delegate> {
    fn from(prefix: &'static str) -> MetricScope<Delegate> {
        let app_metrics: MetricScope<Delegate> = delegate_metrics().into();
        if !prefix.is_empty() {
            app_metrics.with_prefix(prefix)
        } else {
            app_metrics
        }
    }
}

/// Allow turning a 'static str into a Delegate, where str is the prefix.
impl From<()> for MetricScope<Delegate> {
    fn from(_: ()) -> MetricScope<Delegate> {
        let app_metrics: MetricScope<Delegate> = delegate_metrics().into();
        app_metrics
    }
}

impl From<MetricsSend> for MetricScope<Delegate> {
    fn from(send: MetricsSend) -> MetricScope<Delegate> {
        let send_cmd = send.clone();
        MetricScope::new(
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
    pub fn set_receiver<R: DefineMetric + Send + Sync + 'static>(&self, recv: Arc<R>) {
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

    fn define_metric(&self, kind: Kind, name: &str, rate: Sampling) -> Delegate {
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

    use delegate::{delegate_metrics, set_default_metric_scope, Delegate};
    use test;
    use metrics::MetricScope;
    use aggregate::aggregate;

    #[bench]
    fn dispatch_marker_to_aggregate(b: &mut test::Bencher) {
        set_default_metric_scope(aggregate());
        let sink: MetricScope<Delegate> = delegate_metrics().into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

    #[bench]
    fn dispatch_marker_to_void(b: &mut test::Bencher) {
        let metrics = delegate_metrics();
        let sink: MetricScope<Delegate> = metrics.into();
        let metric = sink.marker("event_a");
        b.iter(|| test::black_box(metric.mark()));
    }

}
