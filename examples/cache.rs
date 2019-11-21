//! A sample application asynchronously printing metrics to stdout.

extern crate dipstick;

use dipstick::*;
use std::io;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let metrics = Stream::write_to(io::stdout())
        .cached(5)
        .named("cache")
        .locking()
        .metrics();

    loop {
        // report some ad-hoc metric values from our "application" loop
        metrics.counter("blorf").count(1134);
        metrics.marker("burg").mark();

        sleep(Duration::from_millis(500));
    }
}
