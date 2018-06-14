//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;
#[macro_use]
extern crate lazy_static;

use dipstick::*;
use std::time::Duration;

#[ignore(deprecated)]
app_metrics!(
    (Statsd, String),
    DIFFERENT_TYPES = (
        // combine outputs of different types by using a tuple
        to_statsd("localhost:8125").expect("Connecting"),
        to_stdout(),
    )
);

#[ignore(deprecated)]
app_metrics!(
    Vec<String>,
    SAME_TYPE = [
        // combine multiple outputs of the same type by using an array
        to_stdout().with_prefix("yeah"),
        to_stdout().with_prefix("ouch"),
    ]
);

#[ignore(deprecated)]
app_metrics!(
    Vec<String>,
    MUTANT_CHILD = SAME_TYPE.with_prefix("super").with_prefix("duper")
);

fn main() {
    loop {
        DIFFERENT_TYPES.counter("counter_a").count(123);
        SAME_TYPE.timer("timer_a").interval_us(2000000);
        MUTANT_CHILD.gauge("gauge_z").value(34534);
        std::thread::sleep(Duration::from_millis(40));
    }
}
