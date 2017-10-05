//! Task scheduling facilities.

use std::time::Duration;
use std::thread;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;

/// A handle to cancel a scheduled task if required.
#[derive(Debug, Clone)]
pub struct CancelHandle (Arc<AtomicBool>);

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
pub fn schedule<F>(every: Duration, operation: F) -> CancelHandle
    where F: Fn() -> () + Send + 'static
{
    let handle = CancelHandle::new();
    let inner_handle = handle.clone();

    thread::spawn(move || {
        loop {
            thread::sleep(every);
            if inner_handle.is_cancelled() {
                break
            }
            operation();
        }
    });
    handle
}
