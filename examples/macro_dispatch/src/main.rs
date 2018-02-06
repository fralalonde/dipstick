//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

extern crate dipstick;
#[macro_use] extern crate lazy_static;

use dipstick::*;
use std::time::Duration;

app_metrics!(ROOT_METRICS);

app_metrics!(APP_METRICS = "sampleapp");

mod_metrics!(MOD_METRICS = APP_METRICS.with_prefix("mymodule"));

fn main() {

    let real_metrics = to_log();
    SUPER_APP.set_receiver(real_metrics);

    loop {
        SUPER_APP.counter("counter_a").count(123);
        SUPER_APP.timer("timer_a").interval_us(2000000);
        SUPER_APP.gauge("gauge_z").value(34534);
        std::thread::sleep(Duration::from_millis(40));
    }
}
