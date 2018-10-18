//! A sample application asynchronously printing metrics to stdout.

#[macro_use]
extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::{Proxy, Stream, Counter, Marker, InputScope, QueuedOutput, Input};
use std::io;
use std::thread;

metrics!{
    COUNTER: Counter = "counter_a";
    EVENT: Marker = "event_c";
}

fn main() {
    Proxy::set_default_target(
        Stream::write_to(io::stdout()).queued(100).input());
    for _ in 0..4 {
        thread::spawn(move || {
            loop {
                // report some metric values from our "application" loop
                COUNTER.count(11);
                EVENT.mark();
                sleep(Duration::from_millis(5));
            }
        });
    }
    sleep(Duration::from_secs(500000));

}
