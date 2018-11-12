//! A demonstration of customization of exported aggregated metrics.
//! Using match on origin metric kind or score type to alter publication output.

#[macro_use]
extern crate dipstick;

use std::time::Duration;
use dipstick::*;
use std::thread::sleep;
use std::io;

metrics!{
    APP = "application" => {
        pub COUNTER: Counter = "counter";
    }
}

fn main() {

    let one_minute = AtomicBucket::new();
    one_minute.flush_every(Duration::from_secs(60));

    let five_minutes = AtomicBucket::new();
    five_minutes.flush_every(Duration::from_secs(300));

    let fifteen_minutes = AtomicBucket::new();
    fifteen_minutes.flush_every(Duration::from_secs(900));

    let all_buckets = MultiInputScope::new()
        .add_target(one_minute)
        .add_target(five_minutes)
        .add_target(fifteen_minutes)
        .add_prefix("machine_name");

    // send application metrics to aggregator
    Proxy::default().set_target(all_buckets);
    AtomicBucket::set_default_target(Stream::write_to(io::stdout()));
    AtomicBucket::set_default_stats(stats_all);

    loop {
        COUNTER.count(17);
        sleep(Duration::from_secs(3));
    }
}
