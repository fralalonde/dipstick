//! A sample application sending ad-hoc counter values both to statsd _and_ to stdout.

//#[macro_use]
//extern crate dipstick;
//#[macro_use]
//extern crate lazy_static;

//use dipstick::*;
//use std::time::Duration;
//
//#[ignore(deprecated)]
//app_metrics!(
//    MultiOutput, DIFFERENT_TYPES = to_multi()
//        .with_output(to_statsd("localhost:8125").expect("Statsd"))
//        .with_output(to_stdout())
//);
//
//#[ignore(deprecated)]
//app_metrics!(
//    MultiOutput, SAME_TYPE = to_multi()
//        .with_output(to_stdout().with_prefix("yeah"))
//        .with_output(to_stdout().with_prefix("ouch"))
//);
//
//#[ignore(deprecated)]
//app_metrics!(
//    MultiOutput, MUTANT_CHILD = SAME_TYPE.with_prefix("super").with_prefix("duper")
//);

fn main() {
//    let mmm: &OpenScope = &to_stdout();
//
//    loop {
//        DIFFERENT_TYPES.counter("counter_a").count(123);
//        SAME_TYPE.timer("timer_a").interval_us(2000000);
//        MUTANT_CHILD.gauge("gauge_z").value(34534);
//        std::thread::sleep(Duration::from_millis(40));
//    }
}
