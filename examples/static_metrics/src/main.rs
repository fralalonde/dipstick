//! Use statically defined app metrics & backend.
//! This pattern is likely to emerge
extern crate dipstick;
#[macro_use] extern crate lazy_static;

use std::time::Duration;
use dipstick::*;

/// The `metric` module should be shared across the crate and contain metrics from all modules.
/// Conventions are easier to uphold and document when all metrics are defined in the same place.
pub mod metric {

    use dipstick::*;

    // Unfortunately, Rust's `static`s assignments still force us to explicitly declare types.
    // This makes it uglier than it should be when working with generics...
    // and is even more work because IDE's such as IntelliJ can not yet see through macro blocks :(
    lazy_static! {
        /// Central metric storage
        static ref AGGREGATE: (AggregateSink, AggregateSource) = aggregate();

        /// Application metrics are send to the aggregator
        pub static ref METRICS: AppMetrics<Aggregate, AggregateSink> = metrics(AGGREGATE.0.clone());

        pub static ref COUNTER_A: Counter<Aggregate> = METRICS.counter("counter_a");
        pub static ref TIMER_B: Timer<Aggregate> = METRICS.timer("timer_b");
    }
}

fn main() {
    let (to_aggregate, _from_aggregate) = aggregate();
    let app_metrics = metrics(to_aggregate);

    loop {
        // The resulting application code is lean and clean
        metric::COUNTER_A.count(11);
        metric::TIMER_B.interval_us(654654);
    }

}
