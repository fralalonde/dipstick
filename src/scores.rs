use time;
use std::mem;

use core::*;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::usize;

#[derive(Debug)]
pub struct Scoreboard {
    scores: [AtomicUsize; 5]
}

impl Scoreboard {
    pub fn new() -> Self {
        unsafe {
            // AtomicUsize does not impl Default
            let scores: [AtomicUsize; 5] = mem::uninitialized();
            scores[1].store(0, SeqCst);
            scores[2].store(0, SeqCst);
            scores[3].store(usize::MIN, SeqCst);
            scores[4].store(usize::MAX, SeqCst);
            scores[0].store(time::precise_time_ns() as usize, SeqCst);
            Scoreboard {
                scores
            }
        }
    }

    pub fn reset(&self) -> (Snapshot, u64) {
        let mut snapshot = Snapshot{ scores: [0;5] };
        // SNAPSHOTS OF ATOMICS IN PROGRESS, HANG TIGHT
        for i in 0..5 {
            snapshot.scores[i] = self.scores[i].swap(snapshot.scores[i], Release);
        }
        // END OF ATOMICS SNAPSHOT, YOU CAN RELAX NOW

        (snapshot, self.scores[0].load(SeqCst) as u64)
    }

    /// Update scores with new value
    pub fn update(&self, value: Value) -> () {
        // TODO report any concurrent updates / resets for measurement of contention
        let value = value as usize;
        self.scores[1].fetch_add(1, Acquire);
        self.scores[2].fetch_add(value, Acquire);
        Scoreboard::swap_if_more(&self.scores[3], value);
        Scoreboard::swap_if_less(&self.scores[4], value);
    }

    /// Spinlock until success or clear loss to concurrent update.
    #[inline]
    fn swap_if_more(counter: &AtomicUsize, new_value: usize) {
        let mut current = counter.load(Acquire);
        while current < new_value {
            if counter.compare_and_swap(current, new_value, Release) == new_value { break }
            current = counter.load(Acquire);
        }
    }

    /// Spinlock until success or clear loss to concurrent update.
    #[inline]
    fn swap_if_less(counter: &AtomicUsize, new_value: usize) {
        let mut current = counter.load(Acquire);
        while current > new_value {
            if counter.compare_and_swap(current, new_value, Release) == new_value { break }
            current = counter.load(Acquire);
        }
    }
}

pub struct Snapshot {
    scores: [usize; 5]
}

impl Snapshot {

    pub fn start_time_ns(&self) -> Value {
        self.scores[0] as Value
    }

    pub fn hit_count(&self) -> Value {
        self.scores[1] as Value
    }

    pub fn sum(&self) -> Value {
        self.scores[2] as Value
    }

    pub fn max(&self) -> Value {
        self.scores[3] as Value
    }

    pub fn min(&self) -> Value {
        self.scores[4] as Value
    }

}