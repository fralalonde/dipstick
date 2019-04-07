#[cfg(test)]
use std::ops::Add;

#[cfg(test)]
use std::time::Duration;

use std::time::Instant;
use std::ops::Deref;

use core::MetricValue;

#[cfg(test)]
use std::sync::{Arc};

use std::cell::RefCell;

#[cfg(not(feature="parking_lot"))]
use std::sync::{RwLock};

#[cfg(feature="parking_lot")]
use parking_lot::{RwLock};


#[derive(Debug, Copy, Clone)]
/// A handle to the start time of a counter.
/// Wrapped so it may be changed safely later.
pub struct TimeHandle(Instant);

impl Deref for TimeHandle {
    type Target = Instant;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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

/// Use independent mock clock per thread so that tests can run concurrently without interfering.
/// Tests covering concurrent behavior can override this and share a single clock between threads.
#[cfg(test)]
thread_local! {
    static MOCK_CLOCK: RefCell<Arc<RwLock<Instant>>> = RefCell::new(Arc::new(RwLock::new(Instant::now())));
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
    // TODO mock_clock crate
    MOCK_CLOCK.with(|clock_cell| {
        let now = clock_cell.borrow();
        *write_lock!(now) = Instant::now();
    })
}


/// Advance the mock clock by a certain amount of time.
/// Enables writing reproducible metrics tests in combination with #mock_clock_reset()
/// Should be after metrics have been produced but before they are published.
/// Not feature-gated so it stays visible to outside crates but may not be used outside of tests.
#[cfg(test)]
pub fn mock_clock_advance(period: Duration) {
    MOCK_CLOCK.with(|clock_cell| {
        let now = clock_cell.borrow();
        let mut now = write_lock!(now);
        println!("advancing mock clock {:?} + {:?}", *now, period);
        *now = now.add(period);
        println!("advanced mock clock {:?}", *now);
    })
}

//#[cfg(test)]
//pub fn share_mock_clock() -> Arc<RwLock<Instant>> {
//    // TODO mock_clock crate
//    MOCK_CLOCK.with(|clock_cell| {
//        let now = clock_cell.borrow();
//        now.clone()
//    })
//}
//
//#[cfg(test)]
//pub fn use_mock_clock(shared: Arc<RwLock<Instant>>) {
//    // TODO mock_clock crate
//    MOCK_CLOCK.with(|clock_cell| {
//        let mut clock = clock_cell.borrow_mut();
//        *clock = shared
//    })
//}

#[cfg(not(test))]
fn now() -> Instant {
    Instant::now()
}

/// Metrics mock_clock enabled!
/// thread::sleep will have no effect on metrics.
/// Use advance_time() to simulate passing time.
#[cfg(test)]
fn now() -> Instant {
    MOCK_CLOCK.with(|clock_cell| {
        let clock = clock_cell.borrow();
        let now = read_lock!(clock);
        *now
    })
}
