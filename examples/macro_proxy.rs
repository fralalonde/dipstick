//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;

use dipstick::*;

use std::time::Duration;

// undeclared root (un-prefixed) metrics
metrics! {
    // create counter "some_counter"
    pub ROOT_COUNTER: Counter = "root_counter";
    // create counter "root_counter"
    pub ROOT_GAUGE: Gauge = "root_gauge";
    // create counter "root_timer"
    pub ROOT_TIMER: Timer = "root_timer";
}

// public source
metrics!(pub PUB_METRICS = "pub_lib_prefix" => {
    // create counter "lib_prefix.some_counter"
    pub PUB_COUNTER: Counter = "some_counter";
});

// declare mod source
metrics!(pub LIB_METRICS = "mod_lib_prefix" => {
    // create counter "mod_lib_prefix.some_counter"
    pub SOME_COUNTER: Counter = "some_counter";
});

// reuse declared source
metrics!(LIB_METRICS => {
    // create counter "mod_lib_prefix.another_counter"
    ANOTHER_COUNTER: Counter = "another_counter";
});

fn main() {
    dipstick::Proxy::set_default_target(Stream::to_stdout());

    loop {
        ROOT_COUNTER.count(123);
        ANOTHER_COUNTER.count(456);
        ROOT_TIMER.interval_us(2000000);
        ROOT_GAUGE.value(34534);
        std::thread::sleep(Duration::from_millis(40));
    }
}
