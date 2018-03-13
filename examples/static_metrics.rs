//! Use statically defined app metrics & backend.
//! This pattern is likely to emerge
extern crate dipstick;
#[macro_use]
extern crate lazy_static;

/// The `metric` module should be shared across the crate and contain metrics from all modules.
/// Conventions are easier to uphold and document when all metrics are defined in the same place.
pub mod metric {

    use dipstick::*;

    // Unfortunately, Rust's `static`s assignments still force us to explicitly declare types.
    // This makes it uglier than it should be when working with generics...
    // and is even more work because IDE's such as IntelliJ can not yet see through macro blocks :(
    lazy_static! {
        pub static ref METRICS: AppMetrics<String> = app_metrics(to_stdout());
        pub static ref COUNTER_A: AppCounter<String> = METRICS.counter("counter_a");
        pub static ref TIMER_B: AppTimer<String> = METRICS.timer("timer_b");
    }
}

fn main() {
    // The resulting application code is lean and clean
    metric::COUNTER_A.count(11);
    metric::TIMER_B.interval_us(654654);
}
