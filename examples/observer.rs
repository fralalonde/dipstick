//!
//! A sample application to demonstrate observing of a value.
//!
//! This is the expected output:
//!
//! ```
//! cargo run --example observer
//! ...
//! Press Enter key to exit
//! process.threads 2
//! process.uptime 1000
//! process.threads 2
//! process.uptime 2001
//! process.threads 2
//! process.uptime 3002
//! ```
//!

extern crate dipstick;

use std::time::{Duration};
use std::sync::atomic::AtomicUsize;

use dipstick::{AtomicBucket, InputScope, MetricValue, Prefixed, ScheduleFlush, Stream, OnFlush, Observe};

fn main() {
    let mut metrics = AtomicBucket::new().named("process");
    metrics.drain(Stream::to_stdout());

    metrics.flush_every(Duration::from_secs(3));

    let uptime = metrics.gauge("uptime");
    metrics.on_flush(move || uptime.value(6));

    let threads = metrics.gauge("threads");
    metrics.observe(threads, Duration::from_secs(1), thread_count);

    loop {
        metrics.counter("counter_a").count(123);
        metrics.timer("timer_a").interval_us(2000000);
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// Query number of running threads in this process using Linux's /proc filesystem.
fn thread_count() -> MetricValue {
    4
}
