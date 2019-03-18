#[cfg(test)]
use std::ops::Add;

#[cfg(test)]
use std::time::Duration;

use std::time::Instant;

use crate::core::MetricValue;

#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(Instant);

impl TimeHandle {
    /// Get a handle on current time.
    /// Used by the TimerMetric start_time() method.
    pub fn now() -> TimeHandle {
        TimeHandle(now())
    }

    /// Get the elapsed time in microseconds since TimeHandle was obtained.
    pub fn elapsed_us(self) -> u64 {
        let duration = now() - self.0;
        (duration.as_secs() * 1_000_000) + duration.subsec_micros() as u64
    }

    /// Get the elapsed time in milliseconds since TimeHandle was obtained.
    pub fn elapsed_ms(self) -> MetricValue {
        (self.elapsed_us() / 1000) as isize
    }
}

/// The mock clock is thread local so that tests can run in parallel without affecting each other.
use std::cell::RefCell;
thread_local! {
    static MOCK_CLOCK: RefCell<Instant> = RefCell::new(Instant::now());
}

/// Set the mock clock to the current time.
/// Enables writing reproducible metrics tests in combination with #mock_clock_advance()
/// Should be called at beginning of test, before the metric scope is created.
/// Not feature-gated so it stays visible to outside crates but may not be used outside of tests.
#[cfg(test)]
pub fn mock_clock_reset() {
    if !cfg!(not(test)) {
        warn!("Mock clock used outside of cfg[]tests has no effect")
    }
    MOCK_CLOCK.with(|now| {
        *now.borrow_mut() = Instant::now();
    })
}

/// Advance the mock clock by a certain amount of time.
/// Enables writing reproducible metrics tests in combination with #mock_clock_reset()
/// Should be after metrics have been produced but before they are published.
/// Not feature-gated so it stays visible to outside crates but may not be used outside of tests.
#[cfg(test)]
pub fn mock_clock_advance(period: Duration) {
    MOCK_CLOCK.with(|now| {
        let mut now = now.borrow_mut();
        *now = now.add(period);
    })
}

#[cfg(not(test))]
fn now() -> Instant {
    Instant::now()
}

#[cfg(test)]
/// Metrics mock_clock enabled!
/// thread::sleep will have no effect on metrics.
/// Use advance_time() to simulate passing time.
fn now() -> Instant {
    MOCK_CLOCK.with(|now| *now.borrow())
}
