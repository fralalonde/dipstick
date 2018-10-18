//! A sample application asynchronously printing metrics to stdout.

#[macro_use]
extern crate dipstick;

use std::thread::sleep;
use std::time::Duration;
use dipstick::{Proxy, Stream, Counter, InputScope, Input, SimpleFormat, Formatting, AppLabel};

metrics!{
    COUNTER: Counter = "counter_a";
}

fn main() {
    Proxy::set_default_target(
        Stream::stderr().formatting(SimpleFormat::default()).input());

    AppLabel::set("abc", "xyz");
    loop {
        // report some metric values from our "application" loop
        COUNTER.count(11);
        sleep(Duration::from_millis(500));
    }

}
