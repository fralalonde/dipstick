//! Task scheduling facilities.

use crate::input::InputScope;

use std::cmp::{max, Ordering};
use std::collections::BinaryHeap;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering::SeqCst;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// A guard canceling the inner handle when dropped.
///
/// See [Cancel::into_guard](trait.Cancel.html#method.into_guard) to create it.
pub struct CancelGuard<C: Cancel> {
    // This is Option, so disarm can work.
    // The problem is, Rust won't let us destructure self because we have a destructor.
    inner: Option<C>,
}

impl<C: Cancel> CancelGuard<C> {
    /// Disarms the guard.
    ///
    /// This disposes of the guard without performing the cancelation. This is similar to calling
    /// `forget` on it, but doesn't leak resources, while forget potentially could.
    pub fn disarm(mut self) -> C {
        self.inner
            .take()
            .expect("The borrowchecker shouldn't allow anyone to call disarm twice")
    }
}

impl<C: Cancel> Drop for CancelGuard<C> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            inner.cancel();
        }
    }
}

/// A deferred, repeatable, background action that can be cancelled.
pub trait Cancel {
    /// Cancel the action.
    fn cancel(&self);

    /// Create a guard that cancels when it is dropped.
    fn into_guard(self) -> CancelGuard<Self>
    where
        Self: Sized,
    {
        CancelGuard { inner: Some(self) }
    }
}

/// A handle to cancel a scheduled task if required.
#[derive(Debug, Clone)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    fn new() -> CancelHandle {
        CancelHandle(Arc::new(AtomicBool::new(false)))
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(SeqCst)
    }
}

impl Cancel for CancelHandle {
    /// Signals the task to stop.
    fn cancel(&self) {
        if self.0.swap(true, SeqCst) {
            warn!("Scheduled task was already cancelled.")
        }
    }
}

/// Enable background periodical publication of metrics
pub trait ScheduleFlush {
    /// Flush this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle;
}

impl<T: InputScope + Send + Sync + Clone + 'static> ScheduleFlush for T {
    /// Flush this scope at regular intervals.
    fn flush_every(&self, period: Duration) -> CancelHandle {
        let scope = self.clone();
        SCHEDULER.schedule(period, move |_| {
            if let Err(err) = scope.flush() {
                error!("Could not flush metrics: {}", err);
            }
        })
    }
}

lazy_static! {
    pub static ref SCHEDULER: Scheduler = Scheduler::new();
}

struct ScheduledTask {
    next_time: Instant,
    period: Duration,
    handle: CancelHandle,
    operation: Arc<dyn Fn(Instant) + Send + Sync + 'static>,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &ScheduledTask) -> Ordering {
        other.next_time.cmp(&self.next_time)
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &ScheduledTask) -> Option<Ordering> {
        Some(other.next_time.cmp(&self.next_time))
    }
}

impl PartialEq for ScheduledTask {
    fn eq(&self, other: &ScheduledTask) -> bool {
        self.next_time.eq(&other.next_time)
    }
}

impl Eq for ScheduledTask {}

pub struct Scheduler {
    next_tasks: Arc<(Mutex<BinaryHeap<ScheduledTask>>, Condvar)>,
}

pub static MIN_DELAY: Duration = Duration::from_millis(50);

impl Scheduler {
    /// Launch a new scheduler thread.
    fn new() -> Self {
        let sched: Arc<(Mutex<BinaryHeap<ScheduledTask>>, Condvar)> =
            Arc::new((Mutex::new(BinaryHeap::new()), Condvar::new()));
        let sched1 = Arc::downgrade(&sched);

        thread::Builder::new()
            .name("dipstick_scheduler".to_string())
            .spawn(move || {
                let mut wait_for = MIN_DELAY;
                while let Some(sss) = sched1.upgrade() {
                    let (heap_mutex, condvar) = &*sss;
                    let heap = heap_mutex.lock().unwrap();
                    let (mut tasks, _timed_out) = condvar.wait_timeout(heap, wait_for).unwrap();
                    'work: loop {
                        let now = Instant::now();
                        match tasks.peek() {
                            Some(task) if task.next_time > now => {
                                // next task is not ready yet, update schedule
                                wait_for = max(MIN_DELAY, task.next_time - now);
                                break 'work;
                            }
                            None => {
                                // TODO no tasks left. exit thread?
                                break 'work;
                            }
                            _ => {}
                        }
                        if let Some(mut task) = tasks.pop() {
                            if task.handle.is_cancelled() {
                                // do not execute, do not reinsert
                                continue;
                            }
                            (task.operation)(now);
                            task.next_time = now + task.period;
                            tasks.push(task);
                        }
                    }
                }
            })
            .unwrap();

        Scheduler { next_tasks: sched }
    }

    #[cfg(test)]
    pub fn task_count(&self) -> usize {
        self.next_tasks.0.lock().unwrap().len()
    }

    /// Schedule a task to run periodically.
    pub fn schedule<F>(&self, period: Duration, operation: F) -> CancelHandle
    where
        F: Fn(Instant) + Send + Sync + 'static,
    {
        let handle = CancelHandle::new();
        let new_task = ScheduledTask {
            next_time: Instant::now() + period,
            period,
            handle: handle.clone(),
            operation: Arc::new(operation),
        };
        self.next_tasks.0.lock().unwrap().push(new_task);
        self.next_tasks.1.notify_one();
        handle
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn schedule_one_and_cancel() {
        let trig1a = Arc::new(AtomicUsize::new(0));
        let trig1b = trig1a.clone();

        let sched = Scheduler::new();

        let handle1 = sched.schedule(Duration::from_millis(50), move |_| {
            trig1b.fetch_add(1, SeqCst);
        });
        assert_eq!(sched.task_count(), 1);
        thread::sleep(Duration::from_millis(170));
        assert_eq!(3, trig1a.load(SeqCst));

        handle1.cancel();
        thread::sleep(Duration::from_millis(70));
        assert_eq!(sched.task_count(), 0);
        assert_eq!(3, trig1a.load(SeqCst));
    }

    #[test]
    fn schedule_and_cancel_by_guard() {
        let trig1a = Arc::new(AtomicUsize::new(0));
        let trig1b = trig1a.clone();

        let sched = Scheduler::new();

        let handle1 = sched.schedule(Duration::from_millis(50), move |_| {
            trig1b.fetch_add(1, SeqCst);
        });
        {
            let _guard = handle1.into_guard();
            assert_eq!(sched.task_count(), 1);
            thread::sleep(Duration::from_millis(170));
            assert_eq!(3, trig1a.load(SeqCst));
        } // Here, the guard is dropped, cancelling

        thread::sleep(Duration::from_millis(70));
        assert_eq!(sched.task_count(), 0);
        assert_eq!(3, trig1a.load(SeqCst));
    }

    #[test]
    fn schedule_and_disarm_guard() {
        let trig1a = Arc::new(AtomicUsize::new(0));
        let trig1b = trig1a.clone();

        let sched = Scheduler::new();

        let handle1 = sched.schedule(Duration::from_millis(50), move |_| {
            trig1b.fetch_add(1, SeqCst);
        });
        {
            let guard = handle1.into_guard();
            assert_eq!(sched.task_count(), 1);
            thread::sleep(Duration::from_millis(170));
            assert_eq!(3, trig1a.load(SeqCst));

            guard.disarm();
        }

        thread::sleep(Duration::from_millis(70));
        assert_eq!(sched.task_count(), 1); // Not canceled
    }

    #[test]
    fn schedule_two_and_cancel() {
        let trig1a = Arc::new(AtomicUsize::new(0));
        let trig1b = trig1a.clone();

        let trig2a = Arc::new(AtomicUsize::new(0));
        let trig2b = trig2a.clone();

        let sched = Scheduler::new();

        let handle1 = sched.schedule(Duration::from_millis(50), move |_| {
            trig1b.fetch_add(1, SeqCst);
            println!("ran 1");
        });

        let handle2 = sched.schedule(Duration::from_millis(100), move |_| {
            trig2b.fetch_add(1, SeqCst);
            println!("ran 2");
        });

        thread::sleep(Duration::from_millis(110));
        assert_eq!(2, trig1a.load(SeqCst));
        assert_eq!(1, trig2a.load(SeqCst));

        handle1.cancel();
        thread::sleep(Duration::from_millis(110));
        assert_eq!(2, trig1a.load(SeqCst));
        assert_eq!(2, trig2a.load(SeqCst));

        handle2.cancel();
        thread::sleep(Duration::from_millis(160));
        assert_eq!(2, trig1a.load(SeqCst));
        assert_eq!(2, trig2a.load(SeqCst));
    }

    #[test]
    fn schedule_one_and_more() {
        let trig1a = Arc::new(AtomicUsize::new(0));
        let trig1b = trig1a.clone();

        let sched = Scheduler::new();

        let handle1 = sched.schedule(Duration::from_millis(100), move |_| {
            trig1b.fetch_add(1, SeqCst);
        });

        thread::sleep(Duration::from_millis(110));
        assert_eq!(1, trig1a.load(SeqCst));

        let trig2a = Arc::new(AtomicUsize::new(0));
        let trig2b = trig2a.clone();

        let handle2 = sched.schedule(Duration::from_millis(50), move |_| {
            trig2b.fetch_add(1, SeqCst);
        });

        thread::sleep(Duration::from_millis(110));
        assert_eq!(2, trig1a.load(SeqCst));
        assert_eq!(2, trig2a.load(SeqCst));

        handle1.cancel();
        handle2.cancel();
    }
}
