//! A sample application continuously aggregating metrics,
//! printing the summary stats every three seconds

extern crate dipstick;

use std::time::Duration;
use std::thread::sleep;

use dipstick::*;

fn main() {
    let metrics = to_stdout();

    let counter = metrics.counter("counter_a");
    let timer = metrics.timer("timer_a");
    let gauge = metrics.gauge("gauge_a");
    let marker = metrics.marker("marker_a");

    loop {
        // add counts forever, non-stop
        println!("\n------- open scope");

        let ref mut scope = metrics.open_scope(true);

        counter.count(scope, 11);
        counter.count(scope, 12);
        counter.count(scope, 13);

        timer.interval_us(scope, 11_000_000);
        timer.interval_us(scope, 12_000_000);
        timer.interval_us(scope, 13_000_000);

        sleep(Duration::from_millis(1000));

        gauge.value(scope, 11);
        gauge.value(scope, 12);
        gauge.value(scope, 13);

        marker.mark(scope);

        sleep(Duration::from_millis(1000));

        println!("------- close scope: ");

        // scope metrics are printed at the end of every cycle as scope is dropped
        // use scope.flush_on_drop(false) and scope.flush() to control flushing if required
    }
}
