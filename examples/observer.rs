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
use std::sync::Arc;
use std::time::{Duration, Instant};

use dipstick::{AtomicBucket, InputScope, Prefixed, ScheduleFlush, Stream};

fn main() {
    let start_time = Instant::now();

    let metrics = AtomicBucket::new()
        .add_prefix("process");
    metrics.set_drain(Stream::to_stderr());
    metrics.flush_every(Duration::from_secs(1));

    metrics.observe("uptime", Arc::new(move || dur2ms(start_time.elapsed())));
    metrics.observe("threads", Arc::new(threads));

    println!("Press Enter key to exit");
    io::stdin().read_line(&mut String::new()).expect("Example, ignored");
}

/// Helper to convert duration to milliseconds.
fn dur2ms(duration: Duration) -> isize {
    // Workaround for error[E0658]: use of unstable library feature 'duration_as_u128' (see issue #50202)
    // duration.as_millis()
    (duration.as_secs() * 1000 + u64::from(duration.subsec_millis())) as isize
}

/// Query number of running threads in this process using Linux's /proc filesystem.
fn threads() -> isize {
    // Example, this code is not production ready at all
    const SEARCH: &str = "Threads:\t";
    let file = File::open("/proc/self/status").unwrap();
    let lines = BufReader::new(file).lines();

    lines
        .map(|line| line.unwrap())
        .filter(|line| line.starts_with(SEARCH))
        .map(|line| {
            let value = &line[SEARCH.len()..];
            value.parse::<isize>().unwrap()
        })
        .next()
        .unwrap()
}
