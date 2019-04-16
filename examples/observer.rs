//!
//! A sample application to demonstrate flush-triggered and scheduled observation of gauge values.
//!
//! This is the expected output:
//!
//! ```
//! cargo run --example observer
//! process.threads 4
//! process.uptime 6
//! process.threads 4
//! process.uptime 6
//! ...
//! ```
//!

extern crate dipstick;

use std::time::{Duration, Instant};

use dipstick::*;

fn main() {
    let metrics = AtomicBucket::new().named("process");
    metrics.drain(Stream::to_stdout());
    metrics.flush_every(Duration::from_secs(3));

    let uptime = metrics.gauge("uptime");
    metrics.observe(uptime, |_| 6).on_flush();

    // record number of threads in pool every second
    metrics
        .observe(metrics.gauge("threads"), thread_count)
        .every(Duration::from_secs(1));

    // "heartbeat" metric
    metrics
        .observe(metrics.marker("heartbeat"), |_| 1)
        .on_flush();

    loop {
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// Query number of running threads in this process using Linux's /proc filesystem.
fn thread_count(_now: Instant) -> MetricValue {
    4
}
