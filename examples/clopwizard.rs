//! A demonstration of customization of exported aggregated metrics.
//! Using match on origin metric kind or score type to alter publication output.

#[macro_use]
extern crate dipstick;

#[macro_use]
extern crate lazy_static;

use std::time::Duration;
use dipstick::*;
use std::thread::sleep;

metrics!{
    APP = "application" => {
        pub COUNTER: Counter = "counter";
    }
}

fn main() {

    let one_minute = input_bucket();
    one_minute.flush_every(Duration::from_secs(60));

    let five_minutes = input_bucket();
    five_minutes.flush_every(Duration::from_secs(300));

    let fifteen_minutes = input_bucket();
    fifteen_minutes.flush_every(Duration::from_secs(900));

    let all_buckets = input_multi()
        .add_input(one_minute)
        .add_input(five_minutes)
        .add_input(fifteen_minutes)
        .add_prefix("machine_name");

    // send application metrics to aggregator
    input_proxy().set_target(all_buckets);
    Bucket::set_default_output(output_stdout());
    Bucket::set_default_stats(stats_all);

    loop {
        COUNTER.count(17);
        sleep(Duration::from_secs(3));
    }
}
