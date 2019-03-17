//! Task scheduling facilities.

use core::input::InputScope;

use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, RwLock, Condvar, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::thread::{Thread, JoinHandle};

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

lazy_static! {
    static ref SCHEDULER: Scheduler = Scheduler::default();
}

struct ScheduledTask {
    next_time: Instant,
    period: Duration,
    handle: CancelHandle,
    operation: Arc<Fn() -> () + Send + 'static>,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &ScheduledTask) -> Ordering {
        self.period.cmp(&other.period)
    }
}


#[derive(Default)]
struct Scheduler {
    thread: JoinHandle<_>,
    next_tasks: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
}

impl Scheduler {
    fn new() -> Self {
        let sched = Arc::new((Mutex::new(BinaryHeap::new()), Condvar::new()));
        let sched1 = sched.clone();

        Scheduler {
            thread: thread::Builder::new()
                .name(thread_name.to_string())
                .spawn(move || {
                    let &(ref mutex, ref trig) = &*sched1;
                    let mut guard = mutex.lock().unwrap();
                    loop {
                        let wait_result = trig.wait_timeout_until(guard, Duration::from_secs(10), |tasks| {
                           let now = Instant::now();
                           tasks.peek().map(|task| task.next_time).or(false)
                        });
                        match wait_result.unwrap() {
                            (mut tasks, timeout) if timeout == true => {
                                // timed out waiting, next task is ready to run
                                if let Some(task) = tasks.pop() {
                                    let now = Instant::now();
                                } else {
                                    // heap empty?
                                    warn!("scheduler woke up for no task")
                                }
                            },
                        }

                        task.read().unwrap().peek()
                    }
                })
                .unwrap(),
            next_tasks: Arc::new(RwLock::new(BinaryHeap::new())),
        }
    }

    fn schedule<F>(&mut self, period: Duration, operation: F) -> CancelHandle
        where F: Fn() -> () + Send + 'static {
        let handle = CancelHandle::new();
        let new_task = ScheduledTask {
            period,
            handle,
            operation: Arc::new(operation),
        };
        self.next_tasks.push(new_task);
        let handle =
            if self.thread.is_none() {}
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
