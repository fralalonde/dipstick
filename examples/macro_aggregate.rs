//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;
#[macro_use]
extern crate lazy_static;

use dipstick::*;
use std::time::Duration;

// undeclared root (un-prefixed) metrics
metrics!(<Aggregate> pub AGGREGATE = () => {
    // create counter "some_counter"
    pub Counter ROOT_COUNTER: "root_counter";
    // create counter "root_counter"
    pub Gauge ROOT_GAUGE: "root_gauge";
    // create counter "root_timer"
    pub Timer ROOT_TIMER: "root_timer";
});


metrics!(<Aggregate> AGGREGATE.with_prefix("module_prefix") => {
    // create counter "module_prefix.module_counter"
    Counter MOD_COUNTER: "module_counter";
});

fn main() {
    // print aggregated metrics to the console
    set_aggregate_default_output(to_stdout());

    // enable autoflush...
    AGGREGATE.flush_every(Duration::from_millis(4000));

    loop {
        ROOT_COUNTER.count(123);
        ROOT_TIMER.interval_us(2000000);
        ROOT_GAUGE.value(34534);
        MOD_COUNTER.count(978);

        // ...or flush manually
        AGGREGATE.flush();

        std::thread::sleep(Duration::from_millis(40));
    }
}
