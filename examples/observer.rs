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

use std::time::Duration;

use dipstick::*;

fn main() {
    let mut metrics = AtomicBucket::new().named("process");
    metrics.drain(Stream::to_stdout());
    metrics.flush_every(Duration::from_secs(3));

    let uptime = metrics.gauge("uptime");
    metrics.observe(uptime, || 6).on_flush();

    let threads = metrics.gauge("threads");
    metrics
        .observe(threads, thread_count)
        .every(Duration::from_secs(1));

    loop {
        std::thread::sleep(Duration::from_millis(40));
    }
}

/// Query number of running threads in this process using Linux's /proc filesystem.
fn thread_count() -> MetricValue {
    4
}
