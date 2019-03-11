//! Task scheduling facilities.

use core::input::{InputScope};

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
///
/// # Panics
///
/// Panics if the OS fails to create a thread.
pub fn set_schedule<F>(thread_name: &str, every: Duration, operation: F) -> CancelHandle
where
    F: Fn() -> () + Send + 'static,
{
    let handle = CancelHandle::new();
    let inner_handle = handle.clone();

    thread::Builder::new()
        .name(thread_name.to_string())
        .spawn(move || loop {
            thread::sleep(every);
            if inner_handle.is_cancelled() {
                break;
            }
            operation();
        })
        .unwrap(); // TODO: Panic, change API to return Result?
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
        set_schedule("dipstick-flush", period, move || {
            if let Err(err) = scope.flush() {
                error!("Could not flush metrics: {}", err);
            }
        })
    }
}

//use std::net::{SocketAddr, ToSocketAddrs};
//
//use tiny_http::{Server, StatusCode, self};
//
//pub fn http_serve<A: ToSocketAddrs, F: Fn()>(addresses: A) -> CancelHandle {
//    let handle = CancelHandle::new();
//    let inner_handle = handle.clone();
//    let server = tiny_http::Server::http("0.0.0.0:0")?;
//
//    thread::spawn(move || loop {
//        match server.recv_timeout(Duration::from_secs(1)) {
//            Ok(Some(req)) => {
//                let response = tiny_http::Response::new_empty(StatusCode::from(200));
//                if let Err(err) = req.respond(response) {
//                    warn!("Metrics response error: {}", err)
//                }
//            }
//            Ok(None) => if inner_handle.is_cancelled() { break; }
//            Err(err) => warn!("Metrics request error: {}", err)
//        };
//    });
//    handle
//}
