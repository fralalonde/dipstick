//! A sample application asynchronously printing metrics to stdout.

#[macro_use]
extern crate dipstick;

#[macro_use]
extern crate lazy_static;

use std::thread::sleep;
use std::time::Duration;
use dipstick::*;

use std::thread;

metrics!{
    Counter COUNTER: "counter_a";
    Marker EVENT: "event_c";
}

fn main() {
    input_proxy().set_target(output_stdout().async(100));
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
