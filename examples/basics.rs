//! An app demonstrating the basics of the metrics front-end.
//! Defines metrics of each kind and use them to print values to the console in multiple ways.

use dipstick::{time, Input, InputScope, Prefixed, Stream};
use std::io;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // for this demo, print metric values to the console
    let app_metrics = Stream::write_to(io::stdout()).metrics();

    // metrics can be predefined by type and name
    let counter = app_metrics.counter("counter_a");
    let level = app_metrics.level("level_a");
    let timer = app_metrics.timer("timer_b");

    // metrics can also be declared and used ad-hoc (use output.cache() if this happens often)
    app_metrics.counter("just_once").count(4);

    // metric names can be prepended with a common prefix
    let prefixed_metrics = app_metrics.named("subsystem");
    let event = prefixed_metrics.marker("event_c");
    let gauge = prefixed_metrics.gauge("gauge_d");

    // each kind of metric has a different method name to prevent errors
    counter.count(11);
    level.adjust(-4);
    level.adjust(5);

    gauge.value(22);
    gauge.value(-24);

    event.mark();

    // time can be measured multiple equivalent ways:
    // in microseconds (used internally)
    timer.interval_us(35573);

    // using the time! macro
    time!(timer, sleep(Duration::from_millis(5)));

    // using a closure
    timer.time(|| sleep(Duration::from_millis(5)));

    // using a "time handle"
    let start_time = timer.start();
    sleep(Duration::from_millis(5));
    timer.stop(start_time);
}
