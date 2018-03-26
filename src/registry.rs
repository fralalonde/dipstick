use metrics::Metrics;
use output;
use delegate::{MetricsRecv, MetricsSend};

use std::sync::{Arc, RwLock};

fn no_metrics() -> Arc<MetricsRecv + Send + Sync> {
    let void_metrics: Metrics<_> = output::to_void().into();
    Arc::new(void_metrics)
}

/// The registry contains a list of every metrics dispatch point in the app.
lazy_static! {
    static ref NO_RECV: Arc<MetricsRecv + Sync + Send> = no_metrics();

    static ref DEFAULT_RECV: RwLock<Arc<MetricsRecv + Sync + Send>> = RwLock::new(NO_RECV.clone());

    static ref DELEGATE_REGISTRY: RwLock<Vec<MetricsSend>> = RwLock::new(vec![]);
}

/// Register a new app send.
pub fn add_metrics_send(send: MetricsSend) {
    DELEGATE_REGISTRY.write().unwrap().push(send.clone());
}

/// Get the default app recv.
pub fn get_default_metrics_recv() -> Arc<MetricsRecv + Send + Sync> {
    DEFAULT_RECV.read().unwrap().clone()
}

/// Install a new receiver for all dispatched metrics, replacing any previous receiver.
pub fn send_metrics<IS: Into<Metrics<T>>, T: Send + Sync + Clone + 'static>(
    into_recv: IS,
) {
    let recv = Arc::new(into_recv.into());
    for d in DELEGATE_REGISTRY.read().unwrap().iter() {
        d.set_receiver(recv.clone());
    }

    *DEFAULT_RECV.write().unwrap() = recv;
}