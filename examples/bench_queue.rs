//! A sample application asynchronously printing metrics to stdout.

use dipstick::{AtomicBucket, Input, InputQueueScope, InputScope, Stream};
use std::env::args;
use std::str::FromStr;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let bucket = AtomicBucket::new();
    // NOTE: Wrapping an AtomicBucket with a Queue probably useless, as it is very fast and performs no I/O.
    let queue = InputQueueScope::wrap(bucket.clone(), 10000);
    let event = queue.marker("a");
    let args = &mut args();
    args.next();
    let tc: u8 = u8::from_str(&args.next().unwrap()).unwrap();
    for _ in 0..tc {
        let event = event.clone();
        thread::spawn(move || {
            loop {
                // report some metric values from our "application" loop
                event.mark();
            }
        });
    }
    sleep(Duration::from_secs(5));
    bucket
        .flush_to(&Stream::write_to_stdout().metrics())
        .unwrap();
}
