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

/// A metric that holds aggregated values.
/// Some fields are kept public to ease publishing.
#[derive(Debug)]
pub struct Scoreboard {
    /// The kind of metric
    kind: Kind,
    /// The actual recorded metric scores
    scores: [AtomicUsize; 4],
}

impl Scoreboard {
    /// Create a new Scoreboard to track summary values of a metric
    pub fn new(kind: Kind) -> Self {
        Scoreboard {
            kind,
            scores: unsafe { mem::transmute(Scoreboard::blank()) },
        }
    }

    /// Returns the metric's kind.
    pub fn metric_kind(&self) -> Kind {
        self.kind
    }

    #[inline]
    fn blank() -> [usize; 4] {
        [0, 0, usize::MIN, usize::MAX]
    }

    /// Update scores with new value
    pub fn update(&self, value: Value) -> () {
        // TODO report any concurrent updates / resets for measurement of contention
        let value = value as usize;
        self.scores[0].fetch_add(1, AcqRel);
        match self.kind {
            Marker => {}
            _ => {
                // optimization - these fields are unused for Marker stats
                self.scores[1].fetch_add(value, AcqRel);
                swap_if(&self.scores[2], value, |new, current| new > current);
                swap_if(&self.scores[3], value, |new, current| new < current);
            }
        }
    }

    /// Reset scores to zero, return previous values
    fn snapshot(&self, scores: &mut [usize; 4]) -> bool {
        // NOTE copy timestamp, count AND sum _before_ testing for data to reduce concurrent discrepancies
        scores[0] = self.scores[0].swap(0, AcqRel);
        scores[1] = self.scores[1].swap(0, AcqRel);

        // if hit count is zero, then no values were recorded.
        if scores[0] == 0 {
            return false;
        }

        scores[2] = self.scores[2].swap(usize::MIN, AcqRel);
        scores[3] = self.scores[3].swap(usize::MAX, AcqRel);
        true
    }

    /// Map raw scores (if any) to applicable statistics
    pub fn reset(&self, duration_seconds: f64) -> Option<Vec<ScoreType>> {
        let mut scores = Scoreboard::blank();
        if self.snapshot(&mut scores) {

            let mut snapshot = Vec::new();
            match self.kind {
                Marker => {
                    snapshot.push(Count(scores[0] as u64));
                    snapshot.push(Rate(scores[0] as f64 / duration_seconds))
                }
                Gauge => {
                    snapshot.push(Max(scores[2] as u64));
                    snapshot.push(Min(scores[3] as u64));
                    snapshot.push(Mean(scores[1] as f64 / scores[0] as f64));
                }
                Timer => {
                    snapshot.push(Count(scores[0] as u64));
                    snapshot.push(Sum(scores[1] as u64));

                    snapshot.push(Max(scores[2] as u64));
                    snapshot.push(Min(scores[3] as u64));
                    snapshot.push(Mean(scores[1] as f64 / scores[0] as f64));
                    // timer rate uses the COUNT of timer calls per second (not SUM)
                    snapshot.push(Rate(scores[0] as f64 / duration_seconds))
                }
                Counter => {
                    snapshot.push(Count(scores[0] as u64));
                    snapshot.push(Sum(scores[1] as u64));

                    snapshot.push(Max(scores[2] as u64));
                    snapshot.push(Min(scores[3] as u64));
                    snapshot.push(Mean(scores[1] as f64 / scores[0] as f64));
                    // counter rate uses the SUM of values per second (e.g. to get bytes/s)
                    snapshot.push(Rate(scores[1] as f64 / duration_seconds))
                }
            }
            Some(snapshot)
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
    fn update_marker(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Marker);
        b.iter(|| test::black_box(metric.update(1)));
    }

    #[bench]
    fn update_count(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Counter);
        b.iter(|| test::black_box(metric.update(4)));
    }

    #[bench]
    fn empty_snapshot(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Counter);
        let scores = &mut Scoreboard::blank();
        b.iter(|| test::black_box(metric.snapshot(scores)));
    }

}
