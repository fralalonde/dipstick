//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;
#[macro_use]
extern crate lazy_static;

use dipstick::*;
use std::time::Duration;

metrics!{
    pub ROOT: Bucket {
        SUB_1 = "sub" {
            SUB_1A = "1a" {
                COUNTER: Counter = "counter";
            }
            SUB_1B = "sub1b";
        }
    }
}

metric_define!{ ROOT => {
    // create counter "some_counter"
    pub ROOT_COUNTER: Counter = "root_counter";
    // create gauge "root_gauge"
    pub ROOT_GAUGE: Gauge = "root_gauge";
    // create timer "root_timer"
    pub ROOT_TIMER: Timer = "root_timer";
}}

// undeclared root (un-prefixed) metrics
metrics!(pub AGGREGATE: Aggregate {
    // create counter "some_counter"
    pub ROOT_COUNTER: Counter = "root_counter";
    // create gauge "root_gauge"
    pub ROOT_GAUGE: Gauge = "root_gauge";
    // create timer "root_timer"
    pub ROOT_TIMER: Timer = "root_timer";
});


metrics!( AGGREGATE.with_prefix("module_prefix") => {
    // create counter "module_prefix.module_counter"
    MOD_COUNTER: Counter = "module_counter";
});

fn main() {
    // print aggregated metrics to the console
    MetricAggregator::set_default_output(to_stdout());

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
