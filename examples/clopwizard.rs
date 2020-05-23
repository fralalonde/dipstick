//! A dropwizard-like configuration using three buckets
//! aggregating one, five and fifteen minutes of data.

use dipstick::*;
use std::thread::sleep;
use std::time::Duration;

metrics! {
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
        .named("machine_name");

    // send application metrics to aggregator
    Proxy::default().target(all_buckets);
    AtomicBucket::default_drain(Stream::write_to_stdout());
    AtomicBucket::default_stats(stats_all);

    loop {
        COUNTER.count(17);
        sleep(Duration::from_secs(3));
    }
}
