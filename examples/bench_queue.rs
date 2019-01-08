//! A sample application asynchronously printing metrics to stdout.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::*;
use std::io;
use std::thread;
use std::env::args;
use std::str::FromStr;

fn main() {
    let bucket = AtomicBucket::new();
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
    bucket.flush_to(&Stream::write_to(io::stdout()).output()).unwrap();

}
