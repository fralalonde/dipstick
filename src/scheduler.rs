//! Task scheduling facilities.

use core::InputScope;

use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

/// A handle to cancel a scheduled task if required.
#[derive(Debug, Clone)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    fn new() -> CancelHandle {
        CancelHandle(Arc::new(AtomicBool::new(false)))
    }

    /// Signals the task to stop.
    pub fn cancel(&self) {
        self.0.store(true, SeqCst);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(SeqCst)
    }
}

/// Schedule a task to run periodically.
/// Starts a new thread for every task.
pub fn set_schedule<F>(every: Duration, operation: F) -> CancelHandle
where
    F: Fn() -> () + Send + 'static,
{
    let handle = CancelHandle::new();
    let inner_handle = handle.clone();

    thread::spawn(move || loop {
        thread::sleep(every);
        if inner_handle.is_cancelled() {
            break;
        }
        operation();
    });
    handle
}

/// Enable background periodical publication of metrics
pub trait ScheduleFlush {
    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle;
}

impl<T: InputScope + Send + Sync + Clone + 'static> ScheduleFlush for T {
    /// Start a thread dedicated to flushing this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.clone();
        set_schedule(period, move || {
            if let Err(err) = scope.flush() {
                error!("Could not flush metrics: {}", err);
            }
        })
    }
}
