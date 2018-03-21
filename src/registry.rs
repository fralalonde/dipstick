use app_metrics::AppMetrics;
use output;
use app_delegate::{AppRecv, AppSend};

use std::sync::{Arc, RwLock};

fn no_app_metrics() -> Arc<AppRecv + Send + Sync> {
    let void_metrics: AppMetrics<_> = output::to_void().into();
    Arc::new(void_metrics)
}

/// The registry contains a list of every metrics dispatch point in the app.
lazy_static! {
    static ref NO_APP_METRICS: Arc<AppRecv + Sync + Send> = no_app_metrics();

    static ref DEFAULT_APP_RECEIVER: RwLock<Arc<AppRecv + Sync + Send>> = RwLock::new(NO_APP_METRICS.clone());

    static ref DELEGATE_REGISTRY: RwLock<Vec<AppSend>> = RwLock::new(vec![]);
}

/// Register a new app send.
pub fn add_app_send(send: AppSend) {
    DELEGATE_REGISTRY.write().unwrap().push(send.clone());
}

/// Get the default app recv.
pub fn get_default_app_recv() -> Arc<AppRecv + Send + Sync> {
    DEFAULT_APP_RECEIVER.read().unwrap().clone()
}

/// Install a new receiver for all dispatched metrics, replacing any previous receiver.
pub fn send_app_metrics<IS: Into<AppMetrics<T>>, T: Send + Sync + Clone + 'static>(
    into_recv: IS,
) {
    let recv = into_recv.into();
    for d in DELEGATE_REGISTRY.read().unwrap().iter() {
        d.set_receiver(recv.clone());
    }
}