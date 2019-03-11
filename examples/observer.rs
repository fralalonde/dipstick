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

use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::time::{Duration, Instant};

use dipstick::{AtomicBucket, InputScope, MetricValue, Prefixed, ScheduleFlush, Stream, OnFlush, Schedule};

fn main() {
    let start_time = Instant::now();

    let mut metrics = AtomicBucket::new().named("process");
    metrics.drain(Stream::to_stderr());

    let flush_handle = metrics.flush_every(Duration::from_secs(1));

    let uptime = metrics.gauge("uptime");
    metrics.on_flush(move || uptime.value(dur2ms(start_time.elapsed())));

    let threads = metrics.gauge("threads");
    metrics.schedule(Duration::from_secs(5), move || threads.value(thread_count()));

    println!("Press Enter key to exit");
    io::stdin().read_line(&mut String::new()).expect("Example, ignored");
    flush_handle.cancel();
}

/// Helper to convert duration to milliseconds.
fn dur2ms(duration: Duration) -> MetricValue {
    // Workaround for error[E0658]: use of unstable library feature 'duration_as_u128' (see issue #50202)
    // duration.as_millis()
    (duration.as_secs() * 1000 + u64::from(duration.subsec_millis())) as MetricValue
}

/// Query number of running threads in this process using Linux's /proc filesystem.
fn thread_count() -> MetricValue {
    // Example, this code is not production ready at all
    const SEARCH: &str = "Threads:\t";
    let file = File::open("/proc/self/status").unwrap();
    let lines = BufReader::new(file).lines();

    lines
        .map(|line| line.unwrap())
        .filter(|line| line.starts_with(SEARCH))
        .map(|line| {
            let value = &line[SEARCH.len()..];
            value.parse::<MetricValue>().unwrap()
        })
        .next()
        .unwrap()
}
