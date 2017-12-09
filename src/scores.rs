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
        Scoreboard {
            scores: unsafe { mem::transmute(Scoreboard::blank()) }
        }
    }

    #[inline]
    fn blank() -> [usize; 5] {
        [time::precise_time_ns() as usize, 0, 0, usize::MIN, usize::MAX]
    }

    pub fn reset(&self) -> Snapshot {
        let mut scores = Scoreboard::blank();
        let now = scores[0];

        for i in 0..5 {
            scores[i] = self.scores[i].swap(scores[i], Release);
        }

        scores[0] = now - scores[0];
        Snapshot { scores }
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
    scores: [usize; 5],
}

impl Snapshot {

    pub fn duration_ns(&self) -> Value {
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

    pub fn average(&self) -> f64 {
        self.sum() as f64 / self.hit_count() as f64
    }

    pub fn mean_rate(&self) -> f64 {
        self.sum() as f64 / (self.duration_ns() as f64 / 1_000_000_000.0)
    }

}