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
        let now = time::precise_time_ns() as usize;
        Scoreboard {
            kind,
            name,
            scores: unsafe { mem::transmute(Scoreboard::blank(now)) },
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
        self.scores[1].fetch_add(1, Acquire);
        match self.kind {
            Marker => {}
            _ => {
                // optimization - these fields are unused for Marker stats
                self.scores[2].fetch_add(value, Acquire);
                swap_if_more(&self.scores[3], value);
                swap_if_less(&self.scores[4], value);
            }
        }
    }

    /// Reset scores to zero, return previous values
    fn snapshot(&self, now: usize, scores: &mut [usize; 5]) -> bool {
        // NOTE copy timestamp, count AND sum _before_ testing for data to reduce concurrent discrepancies
        scores[0] = self.scores[0].swap(now, Release);
        scores[1] = self.scores[1].swap(0, Release);
        scores[2] = self.scores[2].swap(0, Release);

        // if hit count is zero, then no values were recorded.
        if scores[1] == 0 {
            return false;
        }

        scores[3] = self.scores[3].swap(usize::MIN, Release);
        scores[4] = self.scores[4].swap(usize::MAX, Release);
        true
    }

    /// Map raw scores (if any) to applicable statistics
    pub fn reset(&self) -> Option<ScoreSnapshot> {
        let now = time::precise_time_ns() as usize;
        let mut scores = Scoreboard::blank(now);
        if self.snapshot(now, &mut scores) {
            let duration_seconds = (now - scores[0]) as f64 / 1_000_000_000.0;

            let mut snapshot = Vec::new();
            match self.kind {
                Marker => {
                    snapshot.push(Count(scores[1] as u64));
                    snapshot.push(Rate(average(scores[1], duration_seconds, &scores, now)))
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
                    snapshot.push(Rate(average(scores[2], duration_seconds, &scores, now)))
                }
            }
            Some((self.kind, self.name.clone(), snapshot))
        } else {
            None
        }
    }
}

fn average(count: usize, time: f64, scores: &[usize], now: usize) -> f64 {
    let avg = count as f64 / time;
    if avg > 10_000_000.0 {
        eprintln!("Computed anomalous rate of '{}'/s, count '{}'  / time '{}'s, start {}, stop {}", avg, count, time, scores[0], now);
    }
    avg
}

/// Spinlock until success or clear loss to concurrent update.
#[inline]
fn swap_if_more(counter: &AtomicUsize, new_value: usize) {
    let mut current = counter.load(Acquire);
    while current < new_value {
        if counter.compare_and_swap(current, new_value, Release) == new_value {
            break;
        }
        current = counter.load(Acquire);
    }
}

/// Spinlock until success or clear loss to concurrent update.
#[inline]
fn swap_if_less(counter: &AtomicUsize, new_value: usize) {
    let mut current = counter.load(Acquire);
    while current > new_value {
        if counter.compare_and_swap(current, new_value, Release) == new_value {
            break;
        }
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
    fn bench_score_snapshot(b: &mut test::Bencher) {
        let metric = Scoreboard::new(Counter, "event_a".to_string());
        let now = time::precise_time_ns() as usize;
        let mut scores = Scoreboard::blank(now);
        b.iter(|| test::black_box(metric.snapshot(now, &mut scores)));
    }

}
