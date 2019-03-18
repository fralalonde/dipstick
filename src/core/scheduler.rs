//! Task scheduling facilities.

use core::input::InputScope;
use core::clock::{TimeHandle};

use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, RwLock, Condvar, Mutex};
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::collections::{BinaryHeap, HashMap};
use std::cmp::{Ordering, min};
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
    static ref SCHEDULER: Scheduler = Scheduler::new();
}

struct ScheduledTask {
    next_time: Instant,
    period: Duration,
    handle: CancelHandle,
    operation: Arc<Fn() -> () + Send + Sync + 'static>,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &ScheduledTask) -> Ordering {
        self.next_time.cmp(&other.next_time)
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &ScheduledTask) -> Option<Ordering> {
        self.next_time.partial_cmp(&other.next_time)
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &ScheduledTask) -> bool {
        self.next_time.eq(&other.next_time)
    }
}

impl Eq for ScheduledTask {}

struct Scheduler {
    thread: JoinHandle<()>,
    next_tasks: Arc<(Mutex<BinaryHeap<ScheduledTask>>, Condvar)>,
}

pub static MIN_DELAY: Duration = Duration::from_millis(50);

impl Scheduler {
    fn new() -> Self {
        let sched: Arc<(Mutex<BinaryHeap<ScheduledTask>>, Condvar)> = Arc::new((Mutex::new(BinaryHeap::new()), Condvar::new()));
        let sched1 = sched.clone();

        Scheduler {
            thread: thread::Builder::new()
                .name("dipstick_scheduler".to_string())
                .spawn(move || {
                    let &(ref mutex, ref trig) = &*sched1;
                    let mut next_delay = MIN_DELAY;
                    loop {
                        let guard = mutex.lock().unwrap();
                        let (mut tasks, timeout) = trig.wait_timeout(guard, next_delay).unwrap();
                        'work:
                        while let Some(mut task) = tasks.pop() {
                            if task.handle.is_cancelled() { continue }
                            let now = *TimeHandle::now();
                            if task.next_time <= now {
                                // execute & schedule next incantation
                                (task.operation)();
                                task.next_time = now + task.period;
                                tasks.push(task);
                            } else {
                                next_delay = min(MIN_DELAY, task.next_time - now);
                                tasks.push(task);
                                break 'work
                            }
                        };
                    }
                })
                .unwrap(),
            next_tasks: sched
        }
    }

    pub fn task_count(&self) -> usize {
        self.next_tasks.0.lock().unwrap().len()
    }

    pub fn schedule<F>(&self, period: Duration, operation: F) -> CancelHandle
        where F: Fn() -> () + Send + Sync + 'static {
        let handle = CancelHandle::new();
        let new_task = ScheduledTask {
            next_time: *TimeHandle::now() + period,
            period,
            handle: handle.clone(),
            operation: Arc::new(operation),
        };
        self.next_tasks.0.lock().unwrap().push(new_task);
        handle
    }

}

#[cfg(test)]
pub mod test {
    use super::*;
    use core::clock::{mock_clock_advance, mock_clock_reset};

    #[test]
    fn schedule_one_and_cancel() {
        let trig = Arc::new(AtomicBool::new(false));
        let trig1 = trig.clone();

        let sched = Scheduler::new();
        assert_eq!(sched.task_count(), 0);

        let handle = sched.schedule(Duration::from_secs(5), move || trig1.store(true, SeqCst));
        assert_eq!(sched.task_count(), 1);

        thread::sleep(MIN_DELAY);
        mock_clock_advance(Duration::from_millis(4999));
        assert_eq!(false, trig.load(SeqCst));
        assert_eq!(sched.task_count(), 1);


        thread::sleep(MIN_DELAY);
        thread::sleep(MIN_DELAY);
        mock_clock_advance(Duration::from_millis(600));
        assert_eq!(true, trig.load(SeqCst));
        assert_eq!(sched.task_count(), 1);

        handle.cancel();
        assert_eq!(sched.task_count(), 0);
    }
}

