//! A sample application asynchronously printing metrics to stdout.

extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::*;

fn main() {
    let metrics = to_stdout().cache(5).new_input().add_name("cache");

    loop {
        // report some ad-hoc metric values from our "application" loop
        metrics.count("blorf", 1134);
        metrics.mark("burg");

        sleep(Duration::from_millis(500));
    }
}
