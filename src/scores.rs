use time;
use std::mem;

use core::*;
use core::Kind::*;

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::*;
use std::usize;

use self::ScoreType::*;

#[derive(Debug, Clone, Copy)]
/// Possibly aggregated scores.
pub enum ScoreType {
    /// Number of times the metric was used.
    Count(u64),
    /// Sum of metric values reported.
    Sum(u64),
    /// Biggest value reported.
    Max(u64),
    /// Smallest value reported.
    Min(u64),
    /// Approximative average value (hit count / sum, non-atomic)
    Mean(f64),
    /// Approximative mean rate (hit count / period length in seconds, non-atomic)
    Rate(f64),
}

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
pub struct Scoreboard {
    /// The kind of metric.
    pub kind: Kind,

    /// The metric's name.
    pub name: String,

    scores: [AtomicUsize; 5]
}

impl Scoreboard {
    /// Create a new Scoreboard to track summary values of a metric
    pub fn new(kind: Kind, name: String) -> Self {
        Scoreboard {
            kind,
            name,
            scores: unsafe { mem::transmute(Scoreboard::blank()) }
        }
    }

    #[inline]
    fn blank() -> [usize; 5] {
        [time::precise_time_ns() as usize, 0, 0, usize::MIN, usize::MAX]
    }

    /// Update scores with new value
    pub fn update(&self, value: Value) -> () {
        // TODO report any concurrent updates / resets for measurement of contention
        let value = value as usize;
        self.scores[1].fetch_add(1, Acquire);
        self.scores[2].fetch_add(value, Acquire);
        swap_if_more(&self.scores[3], value);
        swap_if_less(&self.scores[4], value);
    }

    /// Reset aggregate values, return previous values
    /// To-be-published snapshot of aggregated score values for a metric.
    pub fn reset(&self) -> Vec<ScoreType> {
        let mut scores = Scoreboard::blank();
        let now = scores[0];

        for i in 0..5 {
            scores[i] = self.scores[i].swap(scores[i], Release);
        }

        let duration_seconds = (now - scores[0]) as f64  / 1_000_000_000.0;

        // if hit count is zero, then no values were recorded.
        if scores[1] == 0 {
            return vec![]
        }

        let mut snapshot = Vec::new();
        match self.kind {
            Marker => {
                snapshot.push(Count(scores[1] as u64));
                snapshot.push(Rate(scores[2] as f64 / duration_seconds))
            },
            Gauge => {
                snapshot.push(Max(scores[3] as u64));
                snapshot.push(Min(scores[4] as u64));
                snapshot.push(Mean(scores[2] as f64 / scores[1] as f64));
            },
            Timer | Counter => {
                snapshot.push(Count(scores[1] as u64));
                snapshot.push(Sum(scores[2] as u64));

                snapshot.push(Max(scores[3] as u64));
                snapshot.push(Min(scores[4] as u64));
                snapshot.push(Mean(scores[2] as f64 / scores[1] as f64));
                snapshot.push(Rate(scores[2] as f64 / duration_seconds))
            },
        }
        snapshot
    }


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
