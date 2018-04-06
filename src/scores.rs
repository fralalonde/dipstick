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
    /// Average value (hit count / sum, non-atomic)
    Mean(f64),
    /// Mean rate (hit count / period length in seconds, non-atomic)
    Rate(f64),
}

/// A snapshot of multiple scores for a single metric.
pub type ScoreSnapshot = (Kind, String, Vec<ScoreType>);

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
pub struct Scoreboard {
    /// The kind of metric.
    kind: Kind,

    /// The metric's name.
    name: String,

    scores: [AtomicUsize; 5],
}

impl Scoreboard {
    /// Create a new Scoreboard to track summary values of a metric
    pub fn new(kind: Kind, name: String) -> Self {
        Scoreboard {
            kind,
            name,
            scores: unsafe { mem::transmute(Scoreboard::blank(TimeHandle::now().into())) },
        }
    }

    #[inline]
    fn blank(now: usize) -> [usize; 5] {
        [now, 0, 0, usize::MIN, usize::MAX]
    }

    /// Update scores with new value
    pub fn update(&self, value: Value) -> () {
        // TODO report any concurrent updates / resets for measurement of contention
        let value = value as usize;
        self.scores[1].fetch_add(1, AcqRel);
        match self.kind {
            Marker => {}
            _ => {
                // optimization - these fields are unused for Marker stats
                self.scores[2].fetch_add(value, AcqRel);
                swap_if(&self.scores[3], value, |new, current| new > current);
                swap_if(&self.scores[4], value, |new, current| new < current);
            }
        }
    }

    /// Reset scores to zero, return previous values
    fn snapshot(&self, now: usize, scores: &mut [usize; 5]) -> bool {
        // NOTE copy timestamp, count AND sum _before_ testing for data to reduce concurrent discrepancies
        scores[0] = self.scores[0].swap(now, AcqRel);
        scores[1] = self.scores[1].swap(0, AcqRel);
        scores[2] = self.scores[2].swap(0, AcqRel);

        // if hit count is zero, then no values were recorded.
        if scores[1] == 0 {
            return false;
        }

        scores[3] = self.scores[3].swap(usize::MIN, AcqRel);
        scores[4] = self.scores[4].swap(usize::MAX, AcqRel);
        true
    }

    /// Map raw scores (if any) to applicable statistics
    pub fn reset(&self) -> Option<ScoreSnapshot> {
        let now: usize = TimeHandle::now().into();
        let mut scores = Scoreboard::blank(now);
        if self.snapshot(now, &mut scores) {
            let duration_seconds = (now - scores[0]) as f64 / 1_000.0;

            let mut snapshot = Vec::new();
            match self.kind {
                Marker => {
                    snapshot.push(Count(scores[1] as u64));
                    snapshot.push(Rate(scores[1] as f64 / duration_seconds))
                }
                Gauge => {
                    snapshot.push(Max(scores[3] as u64));
                    snapshot.push(Min(scores[4] as u64));
                    snapshot.push(Mean(scores[2] as f64 / scores[1] as f64));
                }
                Timer | Counter => {
                    snapshot.push(Count(scores[1] as u64));
                    snapshot.push(Sum(scores[2] as u64));

                    snapshot.push(Max(scores[3] as u64));
                    snapshot.push(Min(scores[4] as u64));
                    snapshot.push(Mean(scores[2] as f64 / scores[1] as f64));
                    snapshot.push(Rate(scores[2] as f64 / duration_seconds))
                }
            }
            Some((self.kind, self.name.clone(), snapshot))
        } else {
            None
        }
    }
}

/// Spinlock until success or clear loss to concurrent update.
#[inline]
fn swap_if(counter: &AtomicUsize, new_value: usize, compare: fn(usize, usize) -> bool) {
    let mut current = counter.load(Acquire);
    while compare(new_value, current) {
        if counter.compare_and_swap(current, new_value, Release) == new_value {
            // update successful
            break;
        }
        // race detected, retry
        current = counter.load(Acquire);
    }
}

#[cfg(feature = "bench")]
mod bench {

    use super::*;
    use test;

    #[bench]
    fn bench_score_update_marker(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Marker, "event_a".to_string());
        b.iter(|| test::black_box(metric.update(1)));
    }

    #[bench]
    fn bench_score_update_count(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Counter, "event_a".to_string());
        b.iter(|| test::black_box(metric.update(4)));
    }

    #[bench]
    fn bench_score_empty_snapshot(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Counter, "event_a".to_string());
        let mut scores = Scoreboard::blank(0);
        b.iter(|| test::black_box(metric.snapshot(0, &mut scores)));
    }

}
